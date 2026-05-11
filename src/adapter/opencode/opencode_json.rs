//! `opencode.json` at the repository root. Carries:
//! - The R4.8 / R13 universal-deny permission floor.
//! - An `instructions` reference loading `.opencode/AGENTS.md` (R4.5).

use serde_json::json;

/// The list of bash patterns that no agent may ever invoke. Materialized
/// into `opencode.json`'s `permission.bash` map as `"<pattern>": "deny"`.
/// Mirrors `agent-roster.md#R13` + the slice 22 amendment for
/// `priority-override`.
pub fn deny_patterns() -> Vec<&'static str> {
    vec![
        // R13.1 destructive git
        "git push --force*",
        "git push --force-with-lease*",
        "git push -f*",
        "git reset --hard*",
        "git checkout -- .*",
        "git restore .*",
        "git clean -f*",
        // R13.2 destructive filesystem
        "rm -rf*",
        "rm -f*",
        // R13.3 hub admin (maintainer-only)
        "dwarven serve*",
        "dwarven daemon*",
        "dwarven init*",
        "dwarven reindex*",
        "dwarven config set*",
        "dwarven issue priority*",
        "dwarven issue priority-override*",
        "dwarven issue transition * --override*",
    ]
}

pub fn render() -> String {
    let bash_map: serde_json::Map<String, serde_json::Value> = deny_patterns()
        .iter()
        .map(|p| (p.to_string(), json!("deny")))
        .collect();

    let value = json!({
        "permission": {
            "bash": bash_map,
        },
        "instructions": [".opencode/AGENTS.md"]
    });

    let mut s = serde_json::to_string_pretty(&value).expect("serializing opencode.json");
    s.push('\n');
    s
}
