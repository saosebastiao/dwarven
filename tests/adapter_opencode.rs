//! Adapter integration tests for `dwarven init --host opencode`.
//! Mirrors `tests/adapter_claude_code.rs` parallel structure.

use std::path::Path;

use assert_cmd::prelude::*;

mod common;
use common::dwarven;

const AGENT_NAMES: &[&str] = &[
    "spec", "architect", "gap", "pm", "plan", "test", "implement", "review", "doc", "triage",
];

fn init_opencode(repo: &Path) {
    dwarven(repo)
        .args(["init", "--host", "opencode"])
        .assert()
        .success();
}

#[test]
fn creates_all_subagent_files() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    let agents = tmp.path().join(".opencode/agents");
    for name in AGENT_NAMES {
        let p = agents.join(format!("{name}.md"));
        assert!(p.is_file(), "missing {p:?}");
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("mode: subagent"));
        assert!(content.contains("model: inherit"));
        assert!(content.contains("permission:"));
        // R4.6: --actor baked into bash patterns
        assert!(
            content.contains(&format!("\"dwarven --actor {name} issue view*\": allow")),
            "{name} missing actor-baked dwarven pattern"
        );
        // Closing wildcard deny
        assert!(content.contains("\"*\": deny"));
    }
}

#[test]
fn creates_maintainer_primary() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    let build = tmp.path().join(".opencode/agents/build.md");
    assert!(build.is_file());
    let content = std::fs::read_to_string(&build).unwrap();
    assert!(content.contains("mode: primary"));
    assert!(content.contains("\"dwarven --actor maintainer issue view*\": allow"));
    // shell is read-only
    assert!(content.contains("edit: deny"));
    assert!(content.contains("write: deny"));
    // can dispatch any subagent
    assert!(content.contains("task:") && content.contains("\"*\": allow"));
}

#[test]
fn creates_opencode_json_at_repo_root() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    let p = tmp.path().join("opencode.json");
    assert!(p.is_file());
    let content = std::fs::read_to_string(&p).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    // R4.8: R13 deny patterns in the global permission.bash block
    let bash = &v["permission"]["bash"];
    for must_deny in [
        "git push --force*",
        "git reset --hard*",
        "rm -rf*",
        "dwarven serve*",
        "dwarven init*",
        "dwarven issue priority*",
        "dwarven issue priority-override*",
    ] {
        assert_eq!(
            bash.get(must_deny).and_then(|v| v.as_str()),
            Some("deny"),
            "missing global deny for {must_deny}"
        );
    }
    // R4.5: instructions reference loads AGENTS.md at session start
    let instructions = v["instructions"].as_array().unwrap();
    assert!(instructions
        .iter()
        .any(|i| i.as_str() == Some(".opencode/AGENTS.md")));
}

#[test]
fn creates_agents_md_orientation() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    let p = tmp.path().join(".opencode/AGENTS.md");
    assert!(p.is_file());
    let content = std::fs::read_to_string(&p).unwrap();
    assert!(content.contains("@spec"));
    assert!(content.contains("@architect"));
    assert!(content.contains("@triage"));
    assert!(content.contains("dwarven daemon status"));
}

#[test]
fn dialogue_agents_allow_question() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    for name in &["spec", "architect", "gap", "pm", "plan"] {
        let p = tmp.path().join(format!(".opencode/agents/{name}.md"));
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(
            content.contains("question: allow"),
            "{name} should allow question"
        );
    }
}

#[test]
fn discrete_agents_deny_question() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    for name in &["test", "implement", "review", "doc", "triage"] {
        let p = tmp.path().join(format!(".opencode/agents/{name}.md"));
        let content = std::fs::read_to_string(&p).unwrap();
        assert!(
            content.contains("question: deny"),
            "{name} should deny question"
        );
    }
}

#[test]
fn second_run_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path());
    let snapshot = snapshot_dir(tmp.path());
    init_opencode(tmp.path());
    let after = snapshot_dir(tmp.path());
    assert_eq!(snapshot, after);
}

fn snapshot_dir(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.join(".opencode")];
    if root.join("opencode.json").exists() {
        let content = std::fs::read_to_string(root.join("opencode.json")).unwrap();
        out.push(("opencode.json".to_string(), content));
    }
    while let Some(dir) = stack.pop() {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().to_string();
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                out.push((rel, content));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn dual_install_with_claude_code_does_not_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    dwarven(tmp.path())
        .args(["init", "--host", "claude-code", "--host", "opencode"])
        .assert()
        .success();
    assert!(tmp.path().join(".claude/agents/spec.md").is_file());
    assert!(tmp.path().join(".opencode/agents/spec.md").is_file());
    assert!(tmp.path().join(".claude/settings.json").is_file());
    assert!(tmp.path().join("opencode.json").is_file());
    assert!(tmp.path().join(".opencode/AGENTS.md").is_file());
}

#[test]
fn unknown_host_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    dwarven(tmp.path())
        .args(["init", "--host", "wibble"])
        .assert()
        .failure();
}

#[test]
fn each_agent_has_no_pattern_shadowing_r13() {
    // The validator runs at init time; a successful init means no
    // shadow. This is a defense-in-depth regression test that asserts
    // production ROSTER patterns stay clean of R13 shadows.
    let tmp = tempfile::tempdir().unwrap();
    init_opencode(tmp.path()); // would fail if shadowing detected
}
