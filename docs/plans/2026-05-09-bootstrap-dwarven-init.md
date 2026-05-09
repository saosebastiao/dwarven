# Plan: Bootstrap `dwarven init`

**Date:** 2026-05-09
**Scope:** Stand up the `dwarven` Rust binary skeleton and implement the minimum-viable `dwarven init` subcommand. No host-adapter materialization in this slice; just `.dwarven/` filesystem scaffolding.

**Why this slice first:** the smallest end-to-end exercise of the v2 stack. `dwarven init` produces the on-disk surface (`storage-model.md`) that all subsequent subcommands operate against. Until init exists, no other subcommand can be exercised on a real repo. Once init works, we can run it on this repo and start filing issues for further implementation work — moving from spec-drafting bootstrap into spec-driven workflow.

## Scope of this slice

In:
- `Cargo.toml` with minimum dependency set
- `src/main.rs` — clap-based CLI entry, dispatches to subcommand modules
- `src/init.rs` — `dwarven init` implementation
- Manual verification via `cargo run -- init` against a temp directory

Out (deferred to future slices):
- `dwarven init --host <h>` adapter materialization
- All other subcommands (`issue create`, `serve`, etc.)
- SQLite index
- HTTP server / web UI
- Tests (no test framework in place yet; this slice is bootstrap)

## Spec references

- `dwarven-cli.md#R6.1` — init contract: idempotent, creates `config.toml`, `issues/`, `.gitignore`
- `coordination-hub.md#R10` — `config.toml` schema (defaults documented)
- `storage-model.md#R2.5` — `config.toml` holds next-issue counter, repo identity, defaults
- `storage-model.md#R2.6` — hub creates `.dwarven/.gitignore` for derived artifacts (SQLite index)

## Concrete output of `dwarven init`

Operating on the current directory, creates:

```
.dwarven/
├── config.toml          # populated with defaults per coordination-hub.md#R10.2
├── .gitignore           # contains .index.sqlite, .index.sqlite-*, .daemon.pid
└── issues/              # empty directory (placeholder; .gitkeep optional)
```

`config.toml` initial content:

```toml
[repo]
id = "<generated-uuid>"
name = "<dir-name>"

[counters]
next_issue_id = 1

[daemon]
port = 7777
bind = "127.0.0.1"
reconciliation_interval_seconds = 60

[scheduler]
alpha = 0.5

[scheduler.priority_weights]
p0 = 4
p1 = 2
p2 = 1
unset = 1

[triage]
stale_threshold_days = 14
```

## Idempotence (`dwarven-cli.md#R6.1.2`)

Re-running on a directory that already has `.dwarven/`: print current state (e.g., "Dwarven already initialized at .dwarven; repo id: <uuid>; next issue id: <n>") and exit 0 without writing.

## Dependencies

- `clap` (with derive feature) — CLI parsing
- `anyhow` — error handling
- `toml` — config serialization
- `serde` (with derive) — config struct
- `uuid` (with v4 feature) — repo id generation

## Out-of-scope considerations

- **Cross-platform path handling.** v1 targets macOS + Linux per `coordination-hub.md` implication; Windows support is best-effort.
- **Logging.** No structured logging in this slice; `eprintln!` for warnings, `println!` for output.
- **Output format.** Human-readable only in this slice; `--json` global flag (`dwarven-cli.md#R4.2`) deferred.
