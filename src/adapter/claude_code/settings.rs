//! `.claude/settings.json` — maintainer shell allowlist + universal R13
//! deny patterns. The shell is read-only by design (`host-adapter.md#R3.3`):
//! all mutations route through dispatched agents, each of which carries
//! its own `--actor` allowlist patterns.

use serde_json::json;

pub fn render() -> String {
    let value = json!({
        "permissions": {
            "allow": [
                "Agent",
                "Read",
                "AskUserQuestion",
                "Bash(git status:*)",
                "Bash(git diff:*)",
                "Bash(git log:*)",
                "Bash(git show:*)",
                "Bash(git branch:*)",
                "Bash(git rev-parse:*)",
                "Bash(git merge-base:*)",
                "Bash(dwarven --actor maintainer issue view:*)",
                "Bash(dwarven --actor maintainer issue list:*)",
                "Bash(dwarven --actor maintainer daemon status:*)",
                "Bash(dwarven --actor maintainer config get:*)"
            ],
            "deny": [
                // R13.1 destructive git
                "Bash(git push --force:*)",
                "Bash(git push --force-with-lease:*)",
                "Bash(git push -f:*)",
                "Bash(git reset --hard:*)",
                "Bash(git checkout -- .)",
                "Bash(git restore .)",
                "Bash(git clean -f:*)",
                "Bash(git branch -D:*)",
                "Bash(git push origin --delete:*)",
                // R13.2 destructive filesystem
                "Bash(rm -rf:*)",
                "Bash(rm -f:*)",
                // R13.3 hub admin (maintainer-only)
                "Bash(dwarven serve:*)",
                "Bash(dwarven daemon:*)",
                "Bash(dwarven init:*)",
                "Bash(dwarven config set:*)",
                "Bash(dwarven reindex:*)",
                "Bash(dwarven issue priority:*)",
                "Bash(dwarven issue priority-override:*)",
                "Bash(dwarven issue transition * --override:*)"
            ]
        },
        "hooks": {
            "SessionStart": [
                {
                    "command": ".claude/hooks/session-start.sh"
                }
            ],
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "command": ".claude/hooks/pre-tool-use.sh"
                }
            ]
        }
    });

    let mut s = serde_json::to_string_pretty(&value).expect("serializing settings");
    s.push('\n');
    s
}
