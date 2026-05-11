use std::path::Path;

use assert_cmd::prelude::*;

mod common;
use common::dwarven;

const AGENT_NAMES: &[&str] = &[
    "spec", "architect", "gap", "pm", "plan", "test", "implement", "review", "doc", "triage",
];

fn init_with_adapter(repo: &Path) {
    dwarven(repo)
        .args(["init", "--host", "claude-code"])
        .assert()
        .success();
}

#[test]
fn adapter_creates_all_agent_files() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    let agents = tmp.path().join(".claude/agents");
    for name in AGENT_NAMES {
        let p = agents.join(format!("{name}.md"));
        assert!(p.is_file(), "missing agent file {p:?}");
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains(&format!("name: {name}\n")));
        assert!(content.contains("model: inherit"));
        // Every agent's --actor allowlist patterns carry their own name.
        assert!(
            content.contains(&format!("Bash(dwarven --actor {name} issue view:*)")),
            "agent {name} missing --actor pattern"
        );
    }
}

#[test]
fn adapter_creates_all_command_files() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    let commands = tmp.path().join(".claude/commands");
    for name in AGENT_NAMES {
        let p = commands.join(format!("{name}.md"));
        assert!(p.is_file(), "missing command file {p:?}");
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.contains("allowed-tools: Agent"));
        assert!(content.contains(&format!("subagent_type=\"{name}\"")));
    }
}

#[test]
fn adapter_settings_contains_universal_deny() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    let settings = std::fs::read_to_string(tmp.path().join(".claude/settings.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&settings).unwrap();

    let allow = parsed["permissions"]["allow"]
        .as_array()
        .expect("allow array");
    let allow: Vec<&str> = allow.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        allow.contains(&"Agent"),
        "maintainer shell must have Agent: {allow:?}"
    );
    assert!(
        allow.contains(&"Bash(dwarven --actor maintainer issue view:*)"),
        "maintainer read-only dwarven pattern missing"
    );

    let deny = parsed["permissions"]["deny"]
        .as_array()
        .expect("deny array");
    let deny: Vec<&str> = deny.iter().filter_map(|v| v.as_str()).collect();
    let must_deny = [
        "Bash(git push --force:*)",
        "Bash(git reset --hard:*)",
        "Bash(rm -rf:*)",
        "Bash(dwarven serve:*)",
        "Bash(dwarven init:*)",
        "Bash(dwarven config set:*)",
        "Bash(dwarven issue priority:*)",
        "Bash(dwarven issue priority-override:*)",
    ];
    for pat in must_deny {
        assert!(
            deny.contains(&pat),
            "missing universal deny pattern '{pat}' in {deny:?}"
        );
    }

    let hooks = &parsed["hooks"];
    assert!(hooks["SessionStart"].is_array());
    assert!(hooks["PreToolUse"].is_array());
}

#[cfg(unix)]
#[test]
fn adapter_hook_scripts_are_executable() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    for name in &["session-start.sh", "pre-tool-use.sh"] {
        let p = tmp.path().join(".claude/hooks").join(name);
        let perms = std::fs::metadata(&p).unwrap().permissions();
        assert_eq!(
            perms.mode() & 0o111,
            0o111,
            "{name} not marked executable"
        );
    }
}

#[test]
fn adapter_idempotent_second_run() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    // Snapshot every file.
    let snapshot = walk_files(tmp.path().join(".claude").as_path());

    dwarven(tmp.path())
        .args(["init", "--host", "claude-code"])
        .assert()
        .success();

    let after = walk_files(tmp.path().join(".claude").as_path());
    assert_eq!(snapshot, after, "second adapter run mutated existing files");
}

fn walk_files(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                out.push((rel, content));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn dialogue_agents_include_ask_user_question() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    for name in &["spec", "architect", "gap", "pm", "plan"] {
        let p = tmp.path().join(format!(".claude/agents/{name}.md"));
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(
            content.contains("AskUserQuestion"),
            "dialogue agent {name} should declare AskUserQuestion"
        );
    }
}

#[test]
fn every_agent_carries_red_flags_and_verification() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    for name in &[
        "spec", "architect", "gap", "pm", "plan", "test", "implement", "review", "doc", "triage",
    ] {
        let p = tmp.path().join(format!(".claude/agents/{name}.md"));
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(
            content.contains("## Red flags"),
            "{name} should have a Red flags section"
        );
        assert!(
            content.contains("## Verification before exit"),
            "{name} should have a Verification before exit section"
        );
        // Sanity: at least one anti-rationalization row, format
        // "| Tempting thought | Reality |" sets up; we look for the
        // header row's separator as a proxy for table presence.
        assert!(
            content.contains("| Tempting thought | Reality |"),
            "{name} red-flags should be a Markdown table"
        );
    }
}

#[test]
fn discrete_agents_exclude_ask_user_question() {
    let tmp = tempfile::tempdir().unwrap();
    init_with_adapter(tmp.path());

    for name in &["test", "implement", "review", "doc", "triage"] {
        let p = tmp.path().join(format!(".claude/agents/{name}.md"));
        let content = std::fs::read_to_string(&p).unwrap();
        // Look in the tools: line specifically, since the body might mention
        // it descriptively. Header parse: split on "tools: ".
        let tools_line = content
            .lines()
            .find(|l| l.starts_with("tools: "))
            .expect("tools line");
        assert!(
            !tools_line.contains("AskUserQuestion"),
            "discrete agent {name} should NOT declare AskUserQuestion: {tools_line}"
        );
    }
}

#[test]
fn unknown_host_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    dwarven(tmp.path())
        .args(["init", "--host", "wibble"])
        .assert()
        .failure();
}
