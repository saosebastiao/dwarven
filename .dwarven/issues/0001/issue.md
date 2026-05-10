---
id: 1
title: 'Daemon: SQLite integrity check + schema-version mismatch rebuild on startup'
type: feature
state: done
priority: p1
epic: daemon-polish
created: 2026-05-10T02:02:52Z
created_by: maintainer
updated: 2026-05-10T05:46:26Z
---
Per coordination-hub.md#R7.3 the daemon should run `PRAGMA integrity_check` on the index at startup and rebuild from `.dwarven/` files on failure. R7.4 adds: the index records the schema version in a meta table; if the recorded version doesn't match the daemon's expected version, rebuild from files.

Current state (slice 11, src/index.rs): we DO record `schema_version = 1` in a `meta` table on every reindex, and the daemon's startup reindex (slice 13, src/daemon/watcher.rs) rebuilds unconditionally. So R7.3/R7.4 are *partially* satisfied as a side effect — but we never explicitly check integrity or version; if a corrupt `.index.sqlite` exists from a prior crashed run we'd happily overwrite it. The "happy" path covers the spec; the explicit checks make the failure modes legible.

Scope:
- On daemon startup, BEFORE the unconditional rebuild, open the existing index (if present), check integrity_check, check meta.schema_version. Log results.
- If mismatch, log and proceed with rebuild (current behavior). If match + integrity ok, the unconditional rebuild is currently still wasteful — consider trusting the existing index and only rebuilding on event/reconcile.
- Decide whether to keep the unconditional startup rebuild or trust the index when checks pass. Spec R3.3 step 5 says "Perform a full reindex from .dwarven/ files" so it currently always rebuilds; that may be conservative-correct.
