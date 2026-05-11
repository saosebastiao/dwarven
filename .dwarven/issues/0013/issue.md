---
id: 13
title: 'docs/cli-reference.md: every subcommand + flag + example'
type: doc
state: done
priority: p1
epic: user-docs
created: 2026-05-11T05:16:29Z
created_by: maintainer
updated: 2026-05-11T05:21:15Z
---
Standalone CLI reference. The spec (docs/specs/dwarven-cli.md) defines the contract; this doc is the user-facing reference with realistic examples.

Scope per subcommand:
- One-paragraph synopsis
- Flags (global + per-subcommand) with semantic descriptions
- Exit codes
- 2-3 concrete examples (with expected output where short)

Subcommands to cover (all R6.x):
- init (with/without --host)
- issue: create, view, list, transition, comment, close, blocker {set,clear}, priority, priority-override, edit, dep {add,remove}
- serve
- daemon: status, stop, restart
- reindex
- config: get, set
- schedule next

Global flags: --repo, --actor, --json, --quiet.
