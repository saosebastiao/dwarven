//! `.claude/hooks/*.sh` — POSIX-shell hook scripts.
//!
//! `session-start.sh` orients the maintainer at session start
//! (`host-adapter.md#R3.6`).
//!
//! `pre-tool-use.sh` is the second-line defense for the universal
//! constraints in `agent-roster.md#R13` (`host-adapter.md#R3.7`). The
//! per-agent allowlist is the first line; this hook catches the few
//! patterns that are universally forbidden regardless of agent.

pub fn session_start_script() -> String {
    r#"#!/bin/sh
# Dwarven session-start orientation hook (host-adapter.md#R3.6).
# Prints a one-line note about the daemon's status and the web UI URL.
set -e

if command -v dwarven >/dev/null 2>&1; then
    if dwarven daemon status --quiet >/dev/null 2>&1; then
        port=$(dwarven config get daemon.port 2>/dev/null || echo 7777)
        echo "dwarven daemon: running — http://127.0.0.1:${port}"
    else
        echo "dwarven daemon: stopped — start with 'dwarven serve' (foreground) or 'dwarven serve &' (detached)"
    fi
else
    echo "dwarven CLI not on PATH; install before working with hub-tracked artifacts."
fi

echo "agent dispatch: /spec /architect /gap /pm /plan /test /implement /review /doc /triage"
"#
    .to_string()
}

pub fn pre_tool_use_script() -> String {
    // The hook receives the tool invocation on stdin (Claude Code
    // contract: see Claude Code's hooks docs). For v1 we intentionally
    // keep the matcher list compact and grep-based — the per-agent
    // allowlist already enforces 95% of the surface; this catches
    // a handful of patterns that no agent may invoke regardless.
    r#"#!/bin/sh
# Dwarven PreToolUse hook (host-adapter.md#R3.7, agent-roster.md#R13).
# Reads the JSON tool payload on stdin and rejects forbidden patterns.

set -eu

payload=$(cat)
cmd=$(printf '%s' "$payload" | sed -n 's/.*"command":[[:space:]]*"\([^"]*\)".*/\1/p')

deny() {
    printf '%s\n' "$1" >&2
    exit 2
}

case "$cmd" in
    *"git push --force"*|*"git push -f"*|*"git push --force-with-lease"*)
        deny "blocked by R13.1: destructive git push"
        ;;
    *"git reset --hard"*)
        deny "blocked by R13.1: git reset --hard"
        ;;
    *"git clean -f"*)
        deny "blocked by R13.1: git clean -f"
        ;;
    *"rm -rf"*|*"rm -f "*)
        deny "blocked by R13.2: destructive rm"
        ;;
    *"dwarven serve"*|*"dwarven daemon"*|*"dwarven init"*|*"dwarven reindex"*)
        deny "blocked by R13.3: maintainer-only hub admin"
        ;;
    *"dwarven config set"*)
        deny "blocked by R13.3: maintainer-only config mutation"
        ;;
    *"dwarven issue priority"*)
        deny "blocked by R13.3: priority is maintainer-only (dwarven-cli.md#R6.9)"
        ;;
    *"dwarven issue transition"*"--override"*)
        deny "blocked by R13.3: --override is maintainer-only (dwarven-cli.md#R6.5.3)"
        ;;
esac

# Default: allow.
exit 0
"#
    .to_string()
}
