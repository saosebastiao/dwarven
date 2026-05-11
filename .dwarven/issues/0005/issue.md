---
id: 5
title: Implement opencode adapter (v3)
type: feature
state: done
priority: p2
blocked_by:
- 4
epic: opencode-adapter
created: 2026-05-10T02:03:36Z
created_by: maintainer
updated: 2026-05-11T04:59:47Z
---
v3 deliverable per host-adapter.md#R4. Mirror the Claude Code adapter (slice 18, src/adapter/claude_code/) for opencode's primitives. Specifics resolve under issue #4 (the spec gap).

Scope (assumes #4 is resolved):
- src/adapter/opencode/ module rendering agents/commands/hooks/settings to .opencode/ paths.
- dwarven init --host opencode wires through the existing adapter::install dispatch.
- Tests parallel to tests/adapter_claude_code.rs.

Per host-adapter.md#R4.5 the adapters share no on-disk artifacts; a repo may install both simultaneously.
