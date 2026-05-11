//! Dwarven: host-agnostic system for specification-driven development
//! with strongly decoupled agents and a local coordination hub.
//!
//! This crate compiles to:
//!
//! - The `dwarven` binary (`src/main.rs`), which exposes both the CLI
//!   (`dwarven issue ...`, `dwarven config ...`, etc.) and the daemon
//!   (`dwarven serve`).
//! - A library target (this file), reused by `examples/eval_runner.rs`
//!   and the integration test suite.
//!
//! Module declarations are intentionally duplicated between this file
//! and `src/main.rs` — cargo compiles them under both lib and bin
//! targets.
//!
//! # Where to start
//!
//! - [`docs/architecture/overview.md`](../../docs/architecture/overview.md)
//!   is the new-contributor orientation.
//! - [`docs/specs/dwarven.md`](../../docs/specs/dwarven.md) is the
//!   top-level spec; per-concern specs live alongside it.
//! - [`docs/getting-started.md`](../../docs/getting-started.md) is the
//!   user-facing walkthrough.
//!
//! # Module map
//!
//! - [`storage`]: files-of-record IO. Atomic writes, advisory locks,
//!   issue/comment serialization.
//! - [`issue`]: per-CLI-verb business logic (one module per `dwarven
//!   issue <verb>`). The HTTP API mutation handlers call into these
//!   same `run()` functions.
//! - [`index`]: SQLite derived index, rebuilt from files.
//! - [`scheduler`]: dep-graph scheduler.
//! - [`api`]: daemon-side HTTP, SSE, and embedded web UI.
//! - [`daemon`]: long-running process lifecycle.
//! - [`adapter`]: host adapters (Claude Code, opencode) +
//!   host-agnostic agent registry.
//! - [`eval`]: agent-prompt eval framework (consumed by the
//!   `eval-runner` example).
//! - [`init`], [`config`], [`schedule_cli`], [`time`]: smaller
//!   modules supporting the above.

pub mod adapter;
pub mod api;
pub mod config;
pub mod daemon;
pub mod eval;
pub mod index;
pub mod init;
pub mod issue;
pub mod schedule_cli;
pub mod scheduler;
pub mod storage;
pub mod time;
