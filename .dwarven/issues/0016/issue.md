---
id: 16
title: 'docs/web-ui.md: screen-by-screen walkthrough'
type: doc
state: doc
priority: p2
epic: user-docs
created: 2026-05-11T05:16:45Z
created_by: maintainer
updated: 2026-05-11T05:16:45Z
---
User-facing walkthrough of the web UI. The spec (web-ui.md is in docs/specs/) defines what each screen should do; this doc is for someone USING the UI.

NOTE: spec file is docs/specs/web-ui.md; this user doc is docs/web-ui.md (distinct).

Scope per screen:
- Inbox: what shows up, sort order, badge meaning, what it means when empty
- Issues list: filters available, URL persistence, sort
- Issue detail: full mutation forms (transition / close / blocker / priority / edit / dep / comment), real-time refresh behavior, terminal-state read-only mode
- Deps: layered DAG visual encoding (state color, priority size, blocker dot), focus + N-hop, epic clusters + collapse, URL state
- Schedule: ranked queue, score/override/effective columns, override controls
- Daemon: status fields, reindex + shutdown buttons
- Config: form structure, restart-required flagging, raw JSON view

For each: 1-2 sentence purpose, what to do here, common pitfalls.

Defer screenshots until the visual design stabilizes.
