//! Daemon admin endpoints: shutdown + reindex.
//!
//! `POST /api/v1/daemon/shutdown` flips the shared shutdown flag; the
//! `serve` loop notices and exits cleanly. `POST /api/v1/daemon/reindex`
//! drops and rebuilds the SQLite index in-process so the watcher and
//! the request see a consistent view.

use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::index;

/// POST /api/v1/daemon/shutdown — flip the term flag and return immediately.
/// The caller observes the daemon disappear shortly after (the watcher and
/// HTTP server both wind down on the next tick).
pub async fn shutdown(State(app): State<AppState>) -> (StatusCode, Json<Value>) {
    app.term_flag.store(true, Ordering::Relaxed);
    (StatusCode::ACCEPTED, Json(json!({"ok": true})))
}

/// POST /api/v1/daemon/reindex — synchronous full reindex. v1 has no
/// bounded SLA; large repos block until done. Returns the resulting counts.
pub async fn reindex(State(app): State<AppState>) -> Result<Json<Value>, ApiError> {
    let paths = app.paths.clone();
    let stats = tokio::task::spawn_blocking(move || index::rebuild(&paths))
        .await
        .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))?
        .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
    Ok(Json(json!({
        "issues": stats.issues,
        "comments": stats.comments,
        "edges": stats.edges,
    })))
}

// Scheduler endpoints moved to crate::api::scheduler in slice 21.
