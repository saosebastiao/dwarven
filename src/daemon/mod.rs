//! Daemon process lifecycle.
//!
//! - [`serve`] is the long-running entry point invoked by
//!   `dwarven serve`. It validates config, acquires the PID lock,
//!   installs signal handlers, spawns the HTTP server + file watcher,
//!   then blocks until shutdown.
//! - [`control`] powers `dwarven daemon {status,stop,restart}`.
//! - [`pidfile`] is the advisory-lock-on-`.daemon.pid` mechanism
//!   used by both `serve` (to take the lock) and `control` (to talk
//!   to a running daemon).
//! - [`watcher`] debounces filesystem events into reindex operations
//!   and runs the periodic reconciliation pass.
//! - [`config`] is the startup-time configuration validator.
//!
//! Spec: [`docs/specs/coordination-hub.md`](../../../docs/specs/coordination-hub.md).
//! Architecture: [`docs/architecture/cli-vs-daemon.md`](../../../docs/architecture/cli-vs-daemon.md),
//! [`docs/architecture/event-stream.md`](../../../docs/architecture/event-stream.md).

pub mod config;
pub mod control;
pub mod pidfile;
pub mod serve;
pub mod watcher;
