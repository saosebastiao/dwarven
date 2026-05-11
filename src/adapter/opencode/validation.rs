//! Materialize-time validation per `host-adapter.md#R4.8.3`. opencode
//! has no runtime PreToolUse hook to enforce R13 as a second line of
//! defense, so the adapter checks at `dwarven init --host opencode`
//! time that no per-agent `bash` allow pattern would shadow a global
//! deny pattern.

use anyhow::{Result, anyhow};
use globset::Glob;

use crate::adapter::opencode::agents::to_opencode_pattern;
use crate::adapter::registry::AgentDef;

/// For each global deny pattern, take its literal prefix (the part
/// before the first `*`) and test it against every per-agent allow
/// pattern. If any allow pattern matches the deny prefix, the agent
/// would circumvent a global deny.
pub fn check_no_shadowing(agents: &[AgentDef], deny_patterns: Vec<&str>) -> Result<()> {
    for agent in agents {
        let allows: Vec<String> = agent
            .bash_patterns
            .iter()
            .map(|p| to_opencode_pattern(p))
            .collect();
        for deny in &deny_patterns {
            let prefix = deny.split('*').next().unwrap_or("").trim();
            if prefix.is_empty() {
                continue;
            }
            for allow in &allows {
                let matcher = match Glob::new(allow) {
                    Ok(g) => g.compile_matcher(),
                    Err(_) => continue,
                };
                if matcher.is_match(prefix) {
                    return Err(anyhow!(
                        "agent '{}' bash pattern '{}' shadows global deny '{}' \
                         (matches the prefix '{}'). \
                         Tighten the agent's allow pattern so it does not subsume \
                         the deny; opencode has no runtime second-line defense.",
                        agent.name,
                        allow,
                        deny,
                        prefix
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::registry::{AgentDef, Mode};

    fn agent_with_bash(name: &'static str, patterns: &'static [&'static str]) -> AgentDef {
        AgentDef {
            name,
            spec_section: "test",
            description: "",
            mode: Mode::Discrete,
            trigger: "",
            inputs: "",
            outputs: "",
            exit_conditions: "",
            scope_fences: "",
            non_bash_tools: &["Read"],
            bash_patterns: patterns,
        }
    }

    #[test]
    fn passes_when_no_overlap() {
        let agents = vec![agent_with_bash("safe", &["git status:*", "git diff:*"])];
        let denies = vec!["rm -rf*", "dwarven serve*"];
        assert!(check_no_shadowing(&agents, denies).is_ok());
    }

    #[test]
    fn fails_when_agent_pattern_shadows_deny() {
        let agents = vec![agent_with_bash("rogue", &["dwarven serve:*"])];
        let denies = vec!["dwarven serve*"];
        let err = check_no_shadowing(&agents, denies).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("rogue"));
        assert!(msg.contains("shadows global deny"));
    }

    #[test]
    fn fails_when_agent_wildcard_subsumes_deny() {
        let agents = vec![agent_with_bash("loose", &["dwarven:*"])];
        let denies = vec!["dwarven serve*"];
        let err = check_no_shadowing(&agents, denies).unwrap_err();
        assert!(format!("{err:#}").contains("shadows global deny"));
    }

    #[test]
    fn production_roster_does_not_shadow_r13() {
        let denies = crate::adapter::opencode::opencode_json::deny_patterns();
        check_no_shadowing(crate::adapter::registry::roster(), denies)
            .expect("production roster shadows a R13 deny pattern");
    }
}
