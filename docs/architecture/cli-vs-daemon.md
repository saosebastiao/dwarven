---
spec_ref: coordination-hub.md, dwarven-cli.md
date: 2026-05-10
issue: 8
---

# CLI / daemon division of labor

Two execution modes share the same Rust binary. Why the split looks the way it does, and the load-bearing rules each side observes.

## The split

**CLI mode** (`dwarven init`, `dwarven issue ...`, `dwarven config ...`, `dwarven reindex`, `dwarven schedule ...`) is one-shot. Each invocation:

- reads and writes `.dwarven/` files directly
- never opens `.dwarven/.index.sqlite` for write (the daemon owns it)
- holds the repo lock only for the duration of a single mutation
- exits when the operation completes

**Daemon mode** (`dwarven serve`) is long-running. It:

- holds the PID lock for its entire lifetime
- watches `.dwarven/issues/` and rebuilds the SQLite index on file events
- serves the HTTP API
- broadcasts events on the SSE stream
- exits cleanly on SIGINT/SIGTERM/SIGHUP or `POST /api/v1/daemon/shutdown`

The CLI is fully functional whether or not the daemon is running. The daemon is required only for the web UI and any external HTTP consumer. This is `coordination-hub.md#R2.6` made concrete.

## Why the CLI doesn't write the SQLite index

The daemon is the only writer (`coordination-hub.md#R7.1`). The CLI exclusively reads `.dwarven/` files for queries. Two reasons:

1. **Single-writer simplifies concurrency.** If both the CLI and the daemon could write to the same SQLite file with WAL mode, conflicts and write contention become a real concern. With the daemon as sole writer, lock-free reads from any number of CLI invocations are safe.
2. **CLI works without the daemon.** The CLI shouldn't fall over if the daemon isn't running. Forcing the CLI to write the index would either require launching the daemon transparently (an auto-start mechanism the spec explicitly defers per `coordination-hub.md#R9.4`) or making CLI-write semantics graceful when the daemon is concurrently writing — both worse than just reading files.

**Exception:** `dwarven reindex` writes the index *because* the spec's intent is "rebuild the index" and that work is the same regardless of who runs it. When the daemon is also running, `reindex` deletes and rebuilds the file while the daemon is watching; the next file event triggers the daemon's own rebuild. Eventual consistency, no harm done. A future polish slice could route the CLI's reindex request through the daemon when present.

## The repo lock vs. the PID lock

Two different files, two different invariants.

**`.dwarven/.config.lock`** (used by CLI mutations) — exclusive `flock`, taken and released per operation. Serializes counter increments and frontmatter writes across concurrent CLI invocations. The daemon also takes it for its own mutations (none in v1; reserved).

**`.dwarven/.daemon.pid`** (used only by `dwarven serve`) — exclusive `flock` held for the daemon's entire lifetime. Prevents two daemons from running on the same repo (`coordination-hub.md#R4.4`). The PID file's contents are the running daemon's process id; `dwarven daemon stop` reads them to send `SIGTERM`.

The two locks intentionally do not interfere. A CLI mutation while the daemon is running takes the config lock, completes its file write, and returns; the daemon's file watcher then picks up the change and reindexes. The daemon is not blocked while the CLI mutation is in progress.

## Pidfile-absence as the stop signal

`dwarven daemon stop` cannot use `kill(pid, 0)` (the conventional "is this process alive" probe) because that returns `Ok` for zombie processes. Under `cargo test` the daemon is a child of a test binary that doesn't `wait()`, so a graceful exit leaves a zombie whose pid `kill -0` reports as alive indefinitely. The stop command would loop forever.

The actual signal is **pidfile absence**: the daemon's clean-shutdown path removes `.daemon.pid` after releasing the lock, before exiting. `dwarven daemon stop` sends `SIGTERM` and then polls for `!pidfile.exists()` with a 5-second timeout. Stale pidfiles (the recorded PID is not a live process) are handled separately — `daemon stop` detects them and just removes the file.

This decision is documented in code at `src/daemon/control.rs#run_stop`. The test that motivated it is `tests/daemon.rs#lifecycle_serve_status_stop` — it was the canary that caught the zombie issue under cargo test's spawn pattern.

## Signal handling under inherited masks

Cargo test's harness inherits a signal mask into spawned children that includes `SIGTERM`. The daemon must explicitly unblock the termination signals via `sigprocmask(SIG_UNBLOCK, &set, NULL)` at startup — otherwise the signal-hook handler is installed but never fires because the kernel never delivers the signal.

This is done unconditionally at `dwarven serve` startup (`src/daemon/serve.rs#spawn_signal_thread`), so the daemon behaves identically when spawned from a shell, from `Command::spawn`, or from a test harness.

Signal-hook's iterator pattern (a dedicated thread that calls a synchronous wait inside `Signals::new(...)`) is used instead of the flag-only pattern. Both work in normal shell invocations; only the iterator pattern works under cargo test's inherited mask.

## When the daemon validates config

Slice 28 (issue #2) added startup-time validation of `[daemon]`, `[scheduler]`, and `[triage]` per `coordination-hub.md#R10.6`. The validator runs **before** PID lock acquisition, signal handler install, HTTP bind, or file watcher start. The motivation: a malformed `daemon.port` previously silently truncated via `as u16` and bound the daemon to a nonsense port; now it exits cleanly with a clear error.

The CLI's `dwarven config set` does its own validation for `scheduler.*` keys (slice 20 onward). `daemon.*` keys aren't validated by `config set` because the spec treats them as restart-required (`coordination-hub.md#R10.4`); the daemon's next startup is the right validation point.
