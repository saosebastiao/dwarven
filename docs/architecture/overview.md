---
spec_ref: dwarven.md, coordination-hub.md
date: 2026-05-11
issue: 18
---

# Architecture overview

Orientation for someone reading the source. The other architecture docs
in this directory each focus on one concern; this doc ties them together
and points back to the specs they realize.

If you have not yet read the top-level spec at
[`docs/specs/dwarven.md`](../specs/dwarven.md), start there — this doc
assumes that mental model.

## Build targets

The repository compiles three artifacts from one source tree:

| Target | Source root | What it is |
|---|---|---|
| `dwarven` binary | `src/main.rs` | The single shipped binary. Both the CLI (one-shot subcommands) and the daemon (`dwarven serve`) live here. |
| `dwarven` library | `src/lib.rs` | A library re-export of the same modules. Lets `examples/` and `tests/` import internals without duplication. `lib.rs` and `main.rs` declare the same `mod` set (a known minor cost; see `lib.rs` header comment). |
| `eval-runner` example | `examples/eval_runner.rs` | Standalone binary built only with `cargo run --example eval-runner`. Consumes the library's `eval` module + a `reqwest` Anthropic client kept out of the production binary. |

## Top-level modules

Module name → one-paragraph summary. All under `src/`.

### `storage`

Files-of-record IO. `atomic` writes via temp+rename, `comment_file`
and `issue_file` serializers, `config` reads `config.toml` and gives
out `RepoPaths` (the resolved-paths convenience struct that every other
module takes). This module holds the advisory `flock` semantics
documented in [`storage-layout.md`](storage-layout.md).

### `issue`

Per-subcommand business logic for issue mutations. One file per CLI
verb: `create.rs`, `view.rs`, `list.rs`, `transition.rs`, `comment.rs`,
`close.rs`, `blocker.rs`, `priority.rs`, `priority_override.rs`,
`edit.rs`, `dep.rs`. Each file exposes an `Args` struct and a `run(args)`
that performs the mutation. The HTTP mutation handlers in `api/mutations.rs`
call into these same `run` functions; the CLI is not a re-implementation
of the HTTP path.

### `index`

SQLite index lifecycle. `rebuild(paths)` drops and rebuilds
`.index.sqlite` from files. Health probe (`IndexHealth`) for the
daemon's startup. Schema version constant. The index is read-many,
write-once-per-rebuild; see [`storage-layout.md`](storage-layout.md) for
the byte-reproducibility story.

### `api`

Daemon-side HTTP and SSE surface, plus the embedded web UI assets.

- `server.rs` — axum router, port binding, shutdown wiring.
- `error.rs` — the unified `{error, message, details}` envelope.
- `state.rs` — `AppState` shared across handlers (paths, events bus).
- `issues.rs`, `mutations.rs`, `scheduler.rs`, `config.rs`,
  `daemon_ops.rs` — per-resource handlers. Mutations call into `issue::`.
- `events.rs` — the `EventBus` (broadcast channel + ring buffer + seq counter)
  and SSE handler with `Last-Event-ID` replay.
- `types.rs` — JSON DTOs (`Issue`, `Comment`).
- `web.rs` — `include_str!`-embedded HTML/JS/CSS and the SPA fallback.

### `daemon`

Long-running process lifecycle. `serve.rs` orchestrates: validates
config, takes PID lock, installs signal handlers, spawns the HTTP server,
spawns the file watcher, blocks on shutdown. `pidfile.rs` is the
advisory-lock-on-`.daemon.pid` mechanism. `watcher.rs` debounces file
events into reindex operations. `control.rs` powers `dwarven daemon
status/stop/restart`. `config.rs` is the startup-time config validator.

### `scheduler`

Dep-graph scheduler. `compute.rs` is the pure function: DFS-with-memo
over the `blocks` graph, cycle detection, override-vs-score separation.
`config.rs` reads `[scheduler]` from `config.toml` and validates the
weight invariants. See [`scheduler.md`](scheduler.md).

### `adapter`

Host adapters. `registry.rs` holds the host-agnostic `AgentDef` roster.
Two host modules — `claude_code/` and `opencode/` — render the registry
into per-host files (agent `.md`, command/slash, settings/permission,
hooks). `mod.rs::install(host, root)` is the entry point called by
`dwarven init --host`. See [`host-adapter.md`](host-adapter.md).

### `eval`

Agent-prompt eval framework. `scenario.rs` and `matcher.rs` define the
scenario shape and the required/forbidden tool-call matchers.
`mock_tools.rs` are pure mock implementations of agent-facing tools
(Edit/Write/AskUserQuestion mocked; Bash mocked for `dwarven` reads).
The Anthropic HTTP client and run loop live in `examples/eval_runner.rs`
to keep `reqwest` as a dev-dependency. See [`agent-eval.md`](agent-eval.md).

### `init`, `config`, `schedule_cli`, `time`

Smaller modules that don't warrant their own architecture doc.

- `init.rs` — `dwarven init`, scaffolds `.dwarven/config.toml`.
- `config.rs` — `dwarven config get/set` CLI surface (distinct from
  `daemon/config.rs`, which validates, and `scheduler/config.rs` and
  `storage/config.rs`, which read sections). The duplicate name is
  awkward but each file is in a different module path.
- `schedule_cli.rs` — `dwarven schedule next` formatter.
- `time.rs` — UTC ISO-8601 formatters used by frontmatter and SSE event ids.

## Cross-cutting invariants

Things that are true everywhere in the codebase.

### Files are the source of truth

`.dwarven/issues/` is the canonical state. `.index.sqlite` is derived;
the daemon can drop and rebuild it at any time. If they disagree, the
files win. This is `coordination-hub.md#R7` made structural — there is
no code path that updates the index without also (or first) updating
the file.

### CLI is filesystem-only; daemon owns SQLite

Every CLI subcommand reads from `.dwarven/issues/*.md` directly. None
write to `.index.sqlite`. The one exception (`dwarven reindex`) drops
and rebuilds the index, which is "rebuild from files" semantically — it
doesn't violate the single-writer rule. See
[`cli-vs-daemon.md`](cli-vs-daemon.md).

### Atomic write via temp + rename

Every write to a hub-tracked file goes through
`crate::storage::atomic::write_atomic`. Writers always:

1. Write to `.<name>.tmp.<pid>` in the same directory.
2. `fsync` the temp file.
3. `rename(2)` over the target.

POSIX `rename` within a directory is atomic. Readers always see either
the old file or the complete new one.

### Advisory lock for counter increments

`dwarven issue create` (the `next_issue_id` bump), comment append (the
per-issue `seq` scan), and frontmatter mutations all serialize on
`.dwarven/.config.lock` via `crate::storage::config::with_repo_lock`.
This is one lock for the whole repo, not per-issue; at the spec's
"hundreds of issues" scale, the contention is fine.

### Single daemon per repo

`dwarven serve` takes an exclusive `flock` on `.dwarven/.daemon.pid` for
its entire lifetime. A second `serve` invocation against the same repo
fails fast. See [`cli-vs-daemon.md`](cli-vs-daemon.md) for the
pidfile-absence-as-stop-signal mechanism.

### Actor attribution at the CLI boundary

Every mutation carries an `actor` (defaulting to `maintainer`). The CLI
exposes it as `--actor`; the HTTP API accepts an `actor` body field.
Host adapters bake the agent's name into the agent's allowlist patterns
(e.g., `Bash(dwarven --actor spec issue view:*)`), so an agent cannot
fake another agent's attribution at the shell layer.

## Data flow: a CLI mutation

A mutation issued via the CLI travels:

```
$ dwarven issue transition 42 plan
        │
        ▼
src/main.rs (clap dispatch)
        │
        ▼
src/issue/transition.rs::run(args)
        │
        ├── storage/config.rs::with_repo_lock  (advisory flock)
        ├── storage/issue_file.rs::read_issue  (read .dwarven/issues/42/issue.md)
        ├── (validate transition against work-states graph)
        ├── storage/issue_file.rs::write_issue (atomic write)
        └── storage/comment_file.rs::append_comment (atomic write)
        │
        ▼
        EXIT
```

The daemon, if running, observes via its file watcher:

```
notify (inotify/FSEvents)
        │
        ▼
src/daemon/watcher.rs::watch_loop  (debounce, batch)
        │
        ▼
src/index.rs::rebuild  (drop + rebuild .index.sqlite)
        │
        ▼
src/api/events.rs::EventBus::emit(DaemonReindexed)
        │
        ▼
SSE subscribers (web UI re-renders)
```

The CLI never knows the daemon exists. The daemon never coordinates with
the CLI directly — only via files.

## Data flow: an HTTP mutation

```
$ curl -X POST .../api/v1/issues/42/transitions -d '{"to":"plan"}'
        │
        ▼
src/api/server.rs (axum router)
        │
        ▼
src/api/mutations.rs::transition  (extractor: Path, State, Json)
        │
        ▼
src/issue/transition.rs::run(args)   ◄── same function the CLI calls
        │  (advisory flock, read, validate, atomic write)
        │
        ▼
src/api/events.rs::EventBus::emit(IssueChanged)
        │
        ▼
SSE subscribers receive event with seq id
```

The HTTP mutation paths and the CLI mutation paths share their business
logic. The HTTP layer is extractor → translate to CLI `Args` → call
`issue::*::run` → emit event.

## Where each spec is realized

| Spec | Primary code location | Architecture doc |
|---|---|---|
| `dwarven.md` (top-level) | spans the whole codebase | this file |
| `storage-model.md` | `src/storage/`, `src/index.rs` | [`storage-layout.md`](storage-layout.md) |
| `work-states.md` | `src/issue/transition.rs`, `src/issue/close.rs` | — |
| `coordination-hub.md` | `src/daemon/`, `src/api/` | [`cli-vs-daemon.md`](cli-vs-daemon.md), [`event-stream.md`](event-stream.md) |
| `dwarven-cli.md` | `src/main.rs`, `src/issue/`, `src/config.rs`, `src/init.rs` | [`cli-vs-daemon.md`](cli-vs-daemon.md) |
| `web-api.md` | `src/api/` | [`event-stream.md`](event-stream.md) |
| `web-ui.md` | `assets/web/`, `src/api/web.rs` | [`web-ui-spa.md`](web-ui-spa.md) |
| `dialogue.md` | agent-prompt copy (not Rust); enforced via roster + adapter | — |
| `agent-roster.md` | `src/adapter/registry.rs` | [`host-adapter.md`](host-adapter.md) |
| `host-adapter.md` | `src/adapter/claude_code/`, `src/adapter/opencode/` | [`host-adapter.md`](host-adapter.md) |
| `dep-graph.md` | `src/scheduler/` | [`scheduler.md`](scheduler.md) |
| (eval framework) | `src/eval/`, `examples/eval_runner.rs` | [`agent-eval.md`](agent-eval.md) |

## What is *not* here

- **Configuration values:** see [`docs/configuration.md`](../configuration.md).
- **CLI flags:** see [`docs/cli-reference.md`](../cli-reference.md).
- **API request/response shapes:** see [`docs/http-api-reference.md`](../http-api-reference.md).
- **Agent prompt copy:** lives in `src/adapter/registry.rs` (the
  agent-`description` and `prompt` fields). The behavior-shaping framing
  (Red Flags tables etc., per CLAUDE.md "Skills are behavior-shaping
  code") is gated on eval evidence and is the subject of open issue #6.

## Reading order

If you are new to the codebase:

1. **This file.**
2. [`cli-vs-daemon.md`](cli-vs-daemon.md) — the load-bearing split.
3. [`storage-layout.md`](storage-layout.md) — what's on disk.
4. The architecture doc for whichever subsystem you're touching first.
5. The matching spec under [`docs/specs/`](../specs/).
6. Source.
