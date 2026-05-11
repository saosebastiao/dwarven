//! YAML scenario types per `docs/architecture/agent-eval.md`.

use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Scenario {
    pub agent: String,
    #[serde(default)]
    pub context: ScenarioContext,
    pub user_message: String,
    #[serde(default)]
    pub follow_up_answers: Vec<String>,
    #[serde(default)]
    pub required_tool_calls: Vec<ToolPattern>,
    #[serde(default)]
    pub forbidden_tool_calls: Vec<ToolPattern>,
    #[serde(default)]
    pub response_judge: Option<JudgeRubric>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ScenarioContext {
    pub description: Option<String>,
    #[serde(default)]
    pub fixture_issues: Vec<FixtureIssue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureIssue {
    pub id: u64,
    pub title: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub state: String,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub epic: Option<String>,
    #[serde(default)]
    pub blocked_by: Vec<u64>,
    #[serde(default)]
    pub blocks: Vec<u64>,
    #[serde(default)]
    pub body: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolPattern {
    /// Tool name: `Edit`, `Write`, `Read`, `Bash`, `AskUserQuestion`, `Agent`.
    pub pattern: String,
    /// For Edit/Write/Read: glob against the `file_path` argument.
    #[serde(default)]
    pub path_glob: Option<String>,
    /// For Bash: substring of the `command` argument.
    #[serde(default)]
    pub contains: Option<String>,
    /// Only match invocations at or after this turn (1-indexed).
    #[serde(default)]
    pub after_turn: Option<usize>,
    /// Only match invocations strictly before this turn (1-indexed).
    #[serde(default)]
    pub before_turn: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JudgeRubric {
    pub prompt: String,
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

pub fn load(path: &Path) -> Result<Scenario> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let s: Scenario = serde_yaml::from_str(&raw)
        .with_context(|| format!("parsing scenario YAML {}", path.display()))?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_reference_spec_scenario() {
        let s = load(std::path::Path::new(
            "evals/spec/refuses-out-of-scope-edits.yaml",
        ))
        .unwrap();
        assert_eq!(s.agent, "spec");
        assert!(s.user_message.contains("refactor"));
        assert!(s.required_tool_calls.is_empty());
        assert!(s.forbidden_tool_calls.len() >= 2);
        let edit_src = s
            .forbidden_tool_calls
            .iter()
            .find(|p| p.pattern == "Edit" && p.path_glob.as_deref() == Some("src/**"));
        assert!(edit_src.is_some(), "expected Edit forbidden on src/**");
        let judge = s.response_judge.as_ref().expect("judge");
        assert!(judge.required);
        assert!(judge.prompt.contains("decline"));
    }

    #[test]
    fn loads_reference_test_scenario() {
        let s = load(std::path::Path::new(
            "evals/test/declines-to-write-source.yaml",
        ))
        .unwrap();
        assert_eq!(s.agent, "test");
        let req = s
            .required_tool_calls
            .iter()
            .find(|p| p.pattern == "Bash" && p.contains.as_deref().is_some_and(|c| c.contains("transition")));
        assert!(req.is_some(), "expected required Bash transition");
    }

    #[test]
    fn judge_required_defaults_to_true() {
        let raw = r#"
agent: spec
user_message: hi
response_judge:
  prompt: did the agent refuse?
"#;
        let s: Scenario = serde_yaml::from_str(raw).unwrap();
        assert!(s.response_judge.unwrap().required);
    }
}
