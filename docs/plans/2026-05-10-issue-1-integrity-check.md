---
issue: 1
date: 2026-05-10
status: drafted (pending test)
---

# Plan: SQLite integrity check + schema-version mismatch handling on daemon startup

**Issue:** #1 — `Daemon: SQLite integrity check + schema-version mismatch rebuild on startup`
**Specs:** `coordination-hub.md#R7.3`, `#R7.4`, `#R3.3` step 5

## Goal

When the daemon starts, before performing the unconditional reindex, inspect any pre-existing `.dwarven/.index.sqlite` and decide:

1. **No index file:** rebuild (current behavior).
2. **Integrity check fails (`PRAGMA integrity_check != "ok"`):** delete + rebuild, log the failure.
3. **Schema version mismatch (recorded `meta.schema_version != index::SCHEMA_VERSION`):** delete + rebuild, log the version delta.
4. **Integrity ok + version match:** rebuild anyway in v1 to satisfy R3.3 step 5's "Perform a full reindex from `.dwarven/` files" — but log that the existing index passed checks. The watcher's debounce + reconciliation already keep the index fresh during normal operation; the redundant startup rebuild is a defense against unobserved file-system mutations between daemon runs (e.g., the maintainer ran `git pull`).

The choice to keep the unconditional rebuild matches the spec literally and matches the conservative posture of the daemon ("on conflict, files win"). What this slice changes is **legibility of failure modes** — a corrupt or stale-schema index now produces a visible log line, not silent overwrite.

## Out-of-scope decisions

- **Don't** change R3.3 step 5 to "rebuild only when checks fail." That's a useful optimization for repos with thousands of issues but the spec wants the full reindex. Track separately if needed.
- **Don't** persist anything across daemon restarts beyond what's already there.

## Code changes

### `src/index.rs` — public API

Add three pure functions consuming an open `Connection`:

```rust
pub fn check_integrity(conn: &Connection) -> Result<bool>
pub fn read_schema_version(conn: &Connection) -> Result<Option<i64>>
pub enum IndexHealth { Missing, Corrupt(String), VersionMismatch { found: i64, expected: i64 }, Ok }
pub fn probe_health(index_path: &Path) -> Result<IndexHealth>
```

`probe_health` opens the file (if present) read-only, runs `PRAGMA integrity_check` and reads `meta.schema_version`, and returns the diagnosis. Errors during probe map to `Corrupt` (we treat any open-or-query failure on an existing file as corruption).

### `src/daemon/watcher.rs` — startup hook

Before calling `index::rebuild` for the initial pass, call `index::probe_health` and emit one of four log lines:

- `[daemon] no existing index; building fresh`
- `[daemon] existing index corrupt ({reason}); rebuilding`
- `[daemon] existing index schema version mismatch (found={found}, expected={expected}); rebuilding`
- `[daemon] existing index passed integrity + version checks; rebuilding per R3.3 step 5`

In all four cases, proceed to `rebuild`. Subsequent (event-driven and reconciliation) reindexes are unaffected.

## Tests

### Unit (`src/index.rs#tests`)

- `probe_health_missing_file_returns_missing`
- `probe_health_ok_after_clean_rebuild` — fresh repo, rebuild, probe, expect `Ok`.
- `probe_health_corrupt_returns_corrupt` — write garbage bytes to `.index.sqlite`, probe, expect `Corrupt(...)`.
- `probe_health_version_mismatch` — manually `UPDATE meta SET value = '999' WHERE key = 'schema_version'`, probe, expect `VersionMismatch { found: 999, expected: 1 }`.

### Integration (extend `tests/reindex.rs` or new `tests/index_health.rs`)

- A test that pre-populates a corrupt index, runs the daemon, verifies the log line + a fresh index is produced.

The integration test is harder (daemon-spawning) and lower-value than the unit tests; defer if time-bound. The unit tests cover the meaningful logic.

## Branch + commit

Branch: `feat/1-integrity-check`. Commits scoped to:

1. `src/index.rs`: add probe_health + IndexHealth + tests.
2. `src/daemon/watcher.rs`: wire probe_health into startup; one log line per branch.
3. `docs/plans/2026-05-10-issue-1-integrity-check.md`: this file.

Code Review merges `feat/1-integrity-check` into `main` on approval.

## Verification

- `cargo test` green (existing 196 + new unit/integration).
- Manual smoke against a tempdir: kill the daemon mid-write to produce a corrupt index, restart, observe the corrupt-rebuild log line + functioning index after.
