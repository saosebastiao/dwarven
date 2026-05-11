//! Pure-function tool-call pattern matching per
//! `docs/architecture/agent-eval.md`. Operates on captured tool
//! invocations and the scenario's declared patterns; produces
//! pass/fail + offending-invocation lists.

use globset::Glob;
use serde_json::Value;

use crate::eval::scenario::ToolPattern;

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    /// Tool name as returned by the Claude API (e.g., "Edit", "Bash").
    pub name: String,
    /// The raw input JSON value as sent by the model.
    pub input: Value,
    /// 1-indexed turn within the scenario's conversation loop.
    pub turn: usize,
}

/// True if `inv` matches every condition declared on `pattern`.
pub fn pattern_matches(pattern: &ToolPattern, inv: &ToolInvocation) -> bool {
    if inv.name != pattern.pattern {
        return false;
    }
    if let Some(after) = pattern.after_turn {
        if inv.turn < after {
            return false;
        }
    }
    if let Some(before) = pattern.before_turn {
        if inv.turn >= before {
            return false;
        }
    }
    if let Some(glob) = &pattern.path_glob {
        let file_path = inv
            .input
            .get("file_path")
            .or_else(|| inv.input.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let compiled = match Glob::new(glob) {
            Ok(g) => g.compile_matcher(),
            Err(_) => return false,
        };
        if !compiled.is_match(file_path) {
            return false;
        }
    }
    if let Some(needle) = &pattern.contains {
        let command = inv
            .input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !command.contains(needle) {
            return false;
        }
    }
    true
}

/// True if at least one invocation matches the required pattern.
pub fn required_call_satisfied(pattern: &ToolPattern, invocations: &[ToolInvocation]) -> bool {
    invocations.iter().any(|inv| pattern_matches(pattern, inv))
}

/// Indices of invocations that violate a forbidden pattern (i.e. matched
/// when they shouldn't have).
pub fn forbidden_call_violations(pattern: &ToolPattern, invocations: &[ToolInvocation]) -> Vec<usize> {
    invocations
        .iter()
        .enumerate()
        .filter_map(|(i, inv)| if pattern_matches(pattern, inv) { Some(i) } else { None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn inv(name: &str, input: Value, turn: usize) -> ToolInvocation {
        ToolInvocation {
            name: name.into(),
            input,
            turn,
        }
    }

    fn pat(name: &str) -> ToolPattern {
        ToolPattern {
            pattern: name.into(),
            path_glob: None,
            contains: None,
            after_turn: None,
            before_turn: None,
        }
    }

    #[test]
    fn matches_on_tool_name_only() {
        let p = pat("Edit");
        assert!(pattern_matches(&p, &inv("Edit", json!({}), 1)));
        assert!(!pattern_matches(&p, &inv("Write", json!({}), 1)));
    }

    #[test]
    fn matches_path_glob_against_file_path() {
        let mut p = pat("Edit");
        p.path_glob = Some("src/**".into());
        assert!(pattern_matches(
            &p,
            &inv("Edit", json!({"file_path": "src/foo/bar.rs"}), 1)
        ));
        assert!(!pattern_matches(
            &p,
            &inv("Edit", json!({"file_path": "docs/specs/x.md"}), 1)
        ));
        assert!(!pattern_matches(
            &p,
            &inv("Edit", json!({}), 1) // missing file_path
        ));
    }

    #[test]
    fn matches_contains_against_bash_command() {
        let mut p = pat("Bash");
        p.contains = Some("issue transition".into());
        assert!(pattern_matches(
            &p,
            &inv("Bash", json!({"command": "dwarven --actor test issue transition 42 implement"}), 1)
        ));
        assert!(!pattern_matches(
            &p,
            &inv("Bash", json!({"command": "git status"}), 1)
        ));
    }

    #[test]
    fn after_turn_filters_earlier_invocations() {
        let mut p = pat("Edit");
        p.after_turn = Some(2);
        assert!(!pattern_matches(&p, &inv("Edit", json!({}), 1)));
        assert!(pattern_matches(&p, &inv("Edit", json!({}), 2)));
        assert!(pattern_matches(&p, &inv("Edit", json!({}), 3)));
    }

    #[test]
    fn before_turn_filters_later_invocations() {
        let mut p = pat("Edit");
        p.before_turn = Some(2);
        assert!(pattern_matches(&p, &inv("Edit", json!({}), 1)));
        assert!(!pattern_matches(&p, &inv("Edit", json!({}), 2)));
        assert!(!pattern_matches(&p, &inv("Edit", json!({}), 3)));
    }

    #[test]
    fn required_call_satisfied_iff_any_invocation_matches() {
        let p = pat("AskUserQuestion");
        assert!(!required_call_satisfied(&p, &[]));
        assert!(!required_call_satisfied(
            &p,
            &[inv("Edit", json!({}), 1)]
        ));
        assert!(required_call_satisfied(
            &p,
            &[inv("Edit", json!({}), 1), inv("AskUserQuestion", json!({}), 2)]
        ));
    }

    #[test]
    fn forbidden_call_violations_returns_offending_indices() {
        let mut p = pat("Edit");
        p.path_glob = Some("src/**".into());
        let invs = vec![
            inv("Edit", json!({"file_path": "docs/plans/x.md"}), 1),
            inv("Edit", json!({"file_path": "src/foo.rs"}), 2),
            inv("Write", json!({"file_path": "src/bar.rs"}), 3),
            inv("Edit", json!({"file_path": "src/baz.rs"}), 4),
        ];
        let violations = forbidden_call_violations(&p, &invs);
        assert_eq!(violations, vec![1, 3]);
    }

    #[test]
    fn invalid_glob_does_not_match() {
        let mut p = pat("Edit");
        p.path_glob = Some("[invalid".into());
        // Garbage globs match nothing; the matcher silently returns false
        // rather than erroring (the scenario is malformed but a single
        // bad pattern shouldn't crash a full eval run).
        assert!(!pattern_matches(&p, &inv("Edit", json!({"file_path": "any"}), 1)));
    }
}
