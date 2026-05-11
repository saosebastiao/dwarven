---
id: 11
title: 'README rewrite: reflect v1+v2 shipped state'
type: doc
state: done
priority: p1
epic: user-docs
created: 2026-05-11T05:14:37Z
created_by: maintainer
updated: 2026-05-11T05:16:11Z
---
Current README.md still frames the project as "Pre-release, mid-pivot... implementation is pending. The current agents/, skills/, commands/, and hooks/ directories implement v0.1 and are flagged stale." That's stale by ~30 slices — v1 + v2 are both shipped, the inherited directories are gone, both adapters exist.

Scope:
- Replace the "Status" framing with a current state summary: v1+v2 shipped, X tests passing, Claude Code + opencode adapters available.
- Quickstart: install (cargo), \`dwarven init\` + \`dwarven init --host claude-code\` (or --host opencode), \`dwarven serve\`, open http://127.0.0.1:7777.
- Replace any reference to inherited \`agents/\` etc. with the new \`.claude/\` / \`.opencode/\` materialization model.
- Update the agent roster table to reference docs/specs/agent-roster.md anchors (already in place but verify links).
- Add a "What's shipped" section: CLI subcommands, daemon, HTTP API, SSE, web UI, scheduler, both adapters, eval framework.
- Link map: getting-started, CLI reference, configuration, etc. (those docs will be filed as follow-on issues #12–#17).
- Remove the "Status" subsection that lists v2-pending items — all shipped.

Keep the philosophy / core-ideas section largely intact; that content is timeless.
