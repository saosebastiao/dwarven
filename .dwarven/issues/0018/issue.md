---
id: 18
title: 'docs/architecture/overview.md: module map + cross-cutting invariants'
type: doc
state: done
priority: p1
epic: code-docs
created: 2026-05-11T15:49:33Z
created_by: maintainer
updated: 2026-05-11T15:50:41Z
---
The architecture docs directory has six concern-specific docs (cli-vs-daemon, storage-layout, event-stream, scheduler, web-ui-spa, agent-eval) but no top-level orientation that ties them together. New users currently land on architecture/ and don't know where to start.

Scope:
- Crate structure: top-level modules (storage, issue, index, api, daemon, scheduler, adapter, eval) with one-paragraph what-and-why per module.
- Build targets: dwarven binary, library target, eval-runner example.
- Cross-cutting invariants: files-of-record vs derived index, single-writer SQLite, atomic-write-via-rename, advisory-lock semantics.
- Data flow: a mutation through CLI → file → watcher → index → SSE; a query through CLI → file or through HTTP → file → JSON.
- The CLI/daemon boundary (point at architecture/cli-vs-daemon.md).
- Where each spec is realized in code (one row per spec).
- Pointer map to the other architecture docs.

Audience: a new contributor or future-you who has read the README + getting-started and now wants to read the source.
