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

/// GET /api/v1/scheduler/queue — v1 stub per `web-api.md#R4.9.1`.
pub async fn scheduler_queue() -> Result<Json<Value>, ApiError> {
    Err(ApiError {
        status: StatusCode::NOT_IMPLEMENTED,
        error: "not_implemented".to_string(),
        message: "scheduler queue lands in v2 per dep-graph.md".to_string(),
        details: None,
    })
}

/// POST /api/v1/scheduler/override — v1 stub per `web-api.md#R4.9.1`.
pub async fn scheduler_override() -> Result<Json<Value>, ApiError> {
    Err(ApiError {
        status: StatusCode::NOT_IMPLEMENTED,
        error: "not_implemented".to_string(),
        message: "scheduler override lands in v2 per dep-graph.md".to_string(),
        details: None,
    })
}
