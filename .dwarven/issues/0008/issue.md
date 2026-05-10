---
id: 8
title: docs/architecture/*.md write-ups for v1+v2 implementation decisions
type: doc
state: doc
priority: p2
epic: docs-architecture
created: 2026-05-10T02:04:22Z
created_by: maintainer
updated: 2026-05-10T02:04:22Z
---
30 commits of v1+v2 implementation have shipped without corresponding docs/architecture/ entries. The architectural choices are spread across commit messages and code comments. Posterity needs a more findable record.

Scope (one architecture doc per topic, drafted in dialogue with /architect):
- docs/architecture/storage-layout.md — the .dwarven/ tree, atomic-rename writes, lock file, why DELETE journal mode for byte-reproducibility.
- docs/architecture/cli-vs-daemon.md — why the CLI never opens the SQLite index; lock semantics for concurrent CLI invocations; why pidfile-absence (not kill -0) is the stop signal.
- docs/architecture/scheduler.md — DFS-with-memo + cycle detection; α tunability rationale; override-vs-score separation; the motivating example walkthrough.
- docs/architecture/web-ui-spa.md — vanilla-JS choice (no build step), embedded include_str! assets, hash-routing with URL filter persistence.
- docs/architecture/event-stream.md — broadcast channel + per-handler subscriber + lagged event; why no replay in v1 + path to R5.6.

These are all documentation of *implementation* decisions, not specification changes. Spec amendments (storage-model.md#R4.4.4, etc.) are recorded as such; this issue is about the design narrative.
