---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Coordination Hub

This document specifies the coordination hub binary's responsibilities, lifecycle, and operational guarantees. Per `dwarven.md#R3.1`, the hub is a local Rust binary that owns the workflow state — reading/writing files (`storage-model.md`), maintaining a SQLite index, exposing a local HTTP API (`web-api.md`), and serving the web UI (`web-ui.md`).

This spec covers the daemon's behavior. The CLI surface that talks to the hub (or to the file system directly) is specified in `dwarven-cli.md`.

---

## R1 — Scope

R1.1 — This spec defines:
- The hub binary's responsibilities and division of labor with the CLI;
- Daemon lifecycle (start, stop, restart, crash recovery);
- Per-repo binding;
- File-watching behavior;
- Failure modes and recovery.

R1.2 — Out of scope: HTTP endpoint shapes (`web-api.md`); CLI subcommands (`dwarven-cli.md`); on-disk artifact format (`storage-model.md`); web UI design (`web-ui.md`).

---

## R2 — Architecture: CLI vs. daemon division of labor

R2.1 — The hub binary has two distinct execution modes: **CLI mode** (one-shot subcommand, exits on completion) and **daemon mode** (long-running, serves HTTP and watches files).

R2.2 — In CLI mode, the binary reads and writes `.dwarven/` files directly using the file system. CLI mode does not require the daemon to be running and does not connect to it.

R2.3 — In daemon mode, the binary maintains the SQLite index, watches `.dwarven/` for changes, serves the local HTTP API, and serves the web UI. The daemon is the only process that writes to the SQLite index.

R2.4 — The CLI does not read or write the SQLite index. CLI queries are answered by direct file-system reads. This is acceptable because CLI operations are typically single-issue (`dwarven issue view 42`) or small list scans (`dwarven issue list --state=plan`); the operational scale is hundreds of issues, not millions.

R2.5 — The daemon's SQLite index serves the web UI and any future high-aggregation queries. The index is derived state and is reproducible from files at any time (`storage-model.md#R8`).

R2.6 — This division means the CLI is fully functional whether or not the daemon is running. The daemon is required only for the web UI and any HTTP-API consumers.

---

## R3 — Daemon lifecycle

R3.1 — The daemon is started by `dwarven serve` (`dwarven-cli.md`). v1 does not auto-start the daemon on first CLI call; explicit start is required. This trades a small UX cost for predictable lifecycle management.

R3.2 — `dwarven serve` runs in the foreground by default. `dwarven serve --background` (or shell `&`) detaches.

R3.3 — On startup the daemon must, in order:
1. Read `.dwarven/config.toml`;
2. Acquire an exclusive lock on the daemon's PID file `.dwarven/.daemon.pid` (gitignored);
3. Bind the HTTP listener to its configured address (default `127.0.0.1:7777`);
4. Open or create the SQLite index file (`storage-model.md#R8.4`);
5. Perform a full reindex from `.dwarven/` files (`storage-model.md#R9.3`);
6. Start the file watcher;
7. Begin serving HTTP requests.

R3.4 — On graceful shutdown (SIGINT, SIGTERM, or HTTP shutdown endpoint), the daemon must, in order: stop accepting new HTTP requests; finish in-flight requests with a deadline; stop the file watcher; commit any pending index updates; release the PID lock; exit 0.

R3.5 — If the daemon finds a stale PID file (the recorded PID does not correspond to a live process), it must reclaim the lock and overwrite the file. If the recorded PID *does* correspond to a live process, startup fails with a clear error pointing to the existing daemon.

R3.6 — The daemon binds to `127.0.0.1` only by default. Binding to other interfaces requires an explicit configuration override and is out of scope for v1 (no auth model exists yet).

R3.7 — Idle behavior: in v1 the daemon does not self-terminate after idle. The maintainer stops it explicitly.

---

## R4 — Per-repo binding

R4.1 — A daemon instance is bound to exactly one repository workspace, identified by the directory containing `.dwarven/`. The daemon's working directory must be that root or a descendant.

R4.2 — Multiple repositories are served by multiple daemon processes, each on its own port. The default port (7777) applies only to the first; subsequent daemons must be started with explicit `--port` overrides.

R4.3 — There is no central registry of running daemons in v1. Each maintainer tracks their own running instances.

R4.4 — A repository may have at most one running daemon (enforced by the PID lock in R3.5). Attempting to start a second daemon for the same repository fails.

---

## R5 — HTTP serving

R5.1 — The daemon serves the local HTTP API specified in `web-api.md` and the web UI static assets specified in `web-ui.md`.

R5.2 — Web UI assets are embedded in the binary at compile time. The daemon does not require external static files at runtime.

R5.3 — The HTTP listener binds to `127.0.0.1` (R3.6). No TLS in v1.

R5.4 — No authentication in v1. Any process on the local machine that can reach `127.0.0.1:<port>` is trusted. Multi-user machine considerations are deferred.

R5.5 — On startup, the daemon prints the URL (`http://127.0.0.1:<port>`) to stdout for the maintainer to open in a browser.

---

## R6 — File watcher

R6.1 — The daemon watches `.dwarven/` recursively using a platform-native file-event API (FSEvents on macOS, inotify on Linux, ReadDirectoryChangesW on Windows). The choice of library is an implementation detail.

R6.2 — On any create, modify, or delete event affecting a tracked path, the daemon reads the affected file(s) and updates the SQLite index. Index updates must be atomic at the artifact level (one issue's update either fully succeeds or fully fails and is retried).

R6.3 — The daemon must broadcast change events to connected web UI clients (e.g., via Server-Sent Events or WebSocket; specific transport defined in `web-api.md`). The web UI uses these to reflect changes made by CLI invocations or direct maintainer file edits without manual refresh.

R6.4 — The file watcher must tolerate transient failures (dropped events, watcher restart): the daemon must perform a periodic reconciliation scan (default: every 60 seconds) that compares the on-disk state against the index and corrects drift.

R6.5 — On macOS, the daemon must handle the FSEvents coalescing behavior (events may report a directory rather than the specific file). The reconciliation in R6.4 acts as a safety net for any missed precision.

---

## R7 — SQLite index ownership

R7.1 — The SQLite index file is written exclusively by the daemon. CLI invocations do not open it for write.

R7.2 — The daemon uses SQLite in WAL mode to allow concurrent reads (e.g., HTTP API requests) while index updates commit.

R7.3 — On startup, the daemon performs an integrity check (SQLite `PRAGMA integrity_check`). On failure, it deletes the index file and rebuilds from `.dwarven/` files (`storage-model.md#R8.1`).

R7.4 — The index schema version is recorded in a `_meta` table. On startup, if the recorded version does not match the daemon's expected version, the daemon rebuilds the index from files. There is no in-place index migration in v1.

R7.5 — The index file (`.dwarven/.index.sqlite` per `storage-model.md#R8.4`) is single-machine: the daemon does not synchronize the index across machines or checkouts. Each checkout's daemon builds its own.

---

## R8 — Failure modes and recovery

R8.1 — **Port in use.** Daemon startup fails with a clear error naming the conflicting process (where detectable). Maintainer chooses a different port via `--port`.

R8.2 — **PID file stale.** Reclaimed automatically (R3.5).

R8.3 — **Index corruption.** Detected via integrity check (R7.3); index rebuilt from files.

R8.4 — **File watcher failure.** Reconciliation scan (R6.4) covers gaps. If the file watcher cannot start at all, the daemon falls back to pure polling (reconciliation every 5 seconds) and logs a warning.

R8.5 — **Crash.** PID file may be stale on next start; reclaimed per R3.5. Index may be inconsistent if the crash occurred mid-write; reconciled on next startup's full reindex (R3.3 step 5).

R8.6 — **`.dwarven/` deleted while daemon is running.** Daemon detects directory absence on next file event or reconciliation scan, logs a fatal error, and exits. The hub does not silently re-create the directory.

R8.7 — **Disk full on file write.** CLI writes that fail mid-rename (`storage-model.md#R6.4`) leave the original file intact (atomic rename guarantee). Daemon writes (index updates) fail and are retried on next reconciliation.

---

## R9 — Out of scope for v1

R9.1 — **Multi-machine sync.** The index is single-machine (R7.5). Maintainers working from multiple machines rely on git for `.dwarven/` synchronization; each machine's daemon rebuilds its own index.

R9.2 — **Multi-user / concurrent maintainers.** v1 assumes a single maintainer per repo. Concurrent edits to `.dwarven/` files by multiple humans are handled by git merge, not by the hub.

R9.3 — **Authentication and remote access.** Local-only, unauthenticated (R5.3–R5.4). Remote access requires future spec work.

R9.4 — **Auto-start of daemon from CLI.** Explicit `dwarven serve` only (R3.1). May be revisited if friction is real in practice.

R9.5 — **Hot reload of `config.toml`.** Configuration changes require a daemon restart in v1.

R9.6 — **Index sharing across checkouts.** Each checkout has its own index file; they are not shared via symlink or central store.

---

## R10 — Configuration schema (`.dwarven/config.toml`)

R10.1 — `.dwarven/config.toml` is the per-repo hub configuration. It is created by `dwarven init` (`dwarven-cli.md#R6.1`) with default values and edited by the maintainer (or via `dwarven config set` — `dwarven-cli.md#R6.15`).

R10.2 — The full v2.0 schema:

```toml
# Identity
[repo]
id = "<uuid>"                # auto-generated at init; immutable
name = "<string>"            # human-readable; defaults to directory name

# Atomic counters managed by the hub
[counters]
next_issue_id = 1            # incremented atomically on `dwarven issue create` (storage-model.md#R5.2)

# Daemon
[daemon]
port = 7777                  # HTTP listen port (R3.3)
bind = "127.0.0.1"           # bind address (R3.6); changing is unsupported in v1
reconciliation_interval_seconds = 60   # file-watcher safety-net scan (R6.4)

# Scheduler (consumed in v2)
[scheduler]
alpha = 0.5                  # propagation factor for downstream value (dep-graph.md#R3.4.1)
[scheduler.priority_weights]
p0 = 4
p1 = 2
p2 = 1
unset = 1

# Triage
[triage]
stale_threshold_days = 14    # cadence for transitioning quiet issues to state:maintainer (agent-roster.md#R12.3)

# Per-host adapter settings (free-form extension point)
[adapters.claude_code]
# adapter-specific keys; see host-adapter.md#R3
[adapters.opencode]
# adapter-specific keys; see host-adapter.md#R4 (v3)
```

R10.3 — **Required keys**: `[repo]` table (created at init), `[counters]` table (created at init). All other tables are optional; the hub uses documented defaults for missing keys.

R10.4 — **Restart-required keys**: `daemon.port`, `daemon.bind`. Changing these via `dwarven config set` while the daemon is running prints a warning (per R9.5 and `dwarven-cli.md#R6.15.2`); the change does not take effect until restart.

R10.5 — **Live-reload keys**: all `scheduler.*` and `triage.*` keys take effect on the next computation/scan that consumes them (`dep-graph.md#R3.4.3`). The hub re-reads `config.toml` on each consumption rather than caching.

R10.6 — **Validation.** The hub validates keys on read and rejects malformed values with a clear error: `daemon.port` must be in `[1, 65535]`; `scheduler.alpha` must be in `[0.0, 1.0]`; `scheduler.priority_weights` must be positive integers with `p0 ≥ p1 ≥ p2`; `triage.stale_threshold_days` must be a positive integer. Validation failures on hub start are fatal; failures on `dwarven config set` produce a CLI error and leave the file unchanged.

R10.7 — **Unknown keys** under known tables are preserved on write but produce a warning. Unknown top-level tables are preserved silently (extension point for adapters).

R10.8 — **Schema versioning.** The schema described here is v2.0. Future minor versions add optional keys; major versions follow `dwarven.md#R5.3`. The schema version itself is *not* recorded in the file — the hub binary version implies it, and migrations are at major-version boundaries via `dwarven.md` migration rules.
