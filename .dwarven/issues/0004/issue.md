---
id: 4
title: Spec the opencode adapter (host-adapter.md#R4 sketch → full)
type: spec-gap
state: pm
priority: p2
blocks:
- 5
epic: opencode-adapter
created: 2026-05-10T02:03:27Z
created_by: maintainer
updated: 2026-05-10T02:03:36Z
---
host-adapter.md#R4 currently sketches the opencode adapter contract and lists open questions in R4.2:

- Does opencode expose subagent isolation equivalent to Claude Code's `Agent`?
- Does opencode support per-tool allowlists with prefix-matched patterns? (Required for `Bash(dwarven --actor <name> ...)`)
- Does opencode have an interactive question primitive analogous to AskUserQuestion?
- Does opencode support hooks (SessionStart, PreToolUse equivalents)?

Scope (this issue resolves the spec; implementation is a separate issue):
- Investigate opencode's primitives and either expand R4 into a full per-host adapter spec analogous to R3 (Claude Code), OR document the gaps with prompt-level / process-level workarounds per R4.3.
- Add concrete file paths for opencode adapter outputs (R4.4 currently says ".opencode/<adapter-files>" TBD).
- Expand the agent-roster.md crosswalk: which agents work under which constraints; which require workarounds.

This is a v3 deliverable per host-adapter.md but blocking the implementation issue.
