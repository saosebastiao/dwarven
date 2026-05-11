//! Daemon-side HTTP API + SSE event stream + embedded web UI.
//!
//! The router is built in [`server::router`] and bound by
//! [`server::spawn`]. Per-resource handlers live in `issues.rs`,
//! `mutations.rs`, `scheduler.rs`, `config.rs`, and `daemon_ops.rs`.
//! Error envelope lives in [`error`]. Shared app state lives in
//! [`state::AppState`]. The SSE event bus and the
//! `Last-Event-ID`-replay handler live in [`events`]. The embedded
//! HTML/JS/CSS for the web UI is served by [`web`].
//!
//! Spec: [`docs/specs/web-api.md`](../../../docs/specs/web-api.md).
//! Architecture: [`docs/architecture/event-stream.md`](../../../docs/architecture/event-stream.md),
//! [`docs/architecture/web-ui-spa.md`](../../../docs/architecture/web-ui-spa.md).

pub mod config;
pub mod daemon_ops;
pub mod error;
pub mod events;
pub mod issues;
pub mod mutations;
pub mod scheduler;
pub mod server;
pub mod state;
pub mod types;
pub mod web;
