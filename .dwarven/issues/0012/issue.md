---
id: 12
title: 'docs/getting-started.md: quickstart walkthrough'
type: doc
state: doc
priority: p1
epic: user-docs
created: 2026-05-11T05:16:26Z
created_by: maintainer
updated: 2026-05-11T05:16:26Z
---
End-to-end quickstart for a new user: install, init, choose adapter, start daemon, open web UI, file first issue, dispatch first agent, walk pipeline once.

Scope:
- Prerequisites (Rust toolchain version)
- Build + install from source
- \`dwarven init\` + \`--host <claude-code|opencode>\` choice with trade-offs
- \`dwarven serve\` (foreground vs detached)
- Web UI tour (one paragraph; link to docs/web-ui.md when it exists)
- Filing a first issue via CLI
- Dispatching the first agent (host-specific examples)
- One full pipeline walk: pm → plan → test → implement → review → doc → done
- Where to go next (CLI reference, configuration, troubleshooting links)

Audience: someone landing on the repo who has not used Dwarven before. Assume Rust familiarity but no AI-coding-agent host familiarity.
