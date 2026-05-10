//! Scheduler endpoints per `web-api.md#R4.9` + `dep-graph.md#R5`.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::error::ApiError;
use crate::api::events::{EventKind, emit};
use crate::api::state::AppState;
use crate::issue;
use crate::issue::create::{NotFoundError, UserError};
use crate::scheduler::{ScheduledIssue, compute};

#[derive(Debug, Deserialize)]
pub struct QueueQuery {
    state: Option<String>,
    r#type: Option<String>,
    priority: Option<String>,
    epic: Option<String>,
    /// If `?actionable=true`, restrict to rows with `actionable: true`.
    actionable: Option<bool>,
}

pub async fn queue(
    State(app): State<AppState>,
    Query(q): Query<QueueQuery>,
) -> Result<Json<Vec<ScheduledIssue>>, ApiError> {
    let paths = app.paths.clone();
    let rows = tokio::task::spawn_blocking(move || compute(&paths))
        .await
        .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))?
        .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;

    let filtered: Vec<ScheduledIssue> =
        rows.into_iter().filter(|r| matches(r, &q)).collect();
    Ok(Json(filtered))
}

fn matches(row: &ScheduledIssue, q: &QueueQuery) -> bool {
    if let Some(s) = &q.state {
        if !comma_match(&row.state, s) {
            return false;
        }
    }
    if let Some(t) = &q.r#type {
        if !comma_match(&row.r#type, t) {
            return false;
        }
    }
    if let Some(p) = &q.priority {
        let pm = match &row.priority {
            Some(v) => comma_match(v, p),
            None => comma_match("unset", p),
        };
        if !pm {
            return false;
        }
    }
    if let Some(epic) = &q.epic {
        // ScheduledIssue doesn't currently carry epic; for v1 of the
        // scheduler API we don't filter on it. The full Issue object is
        // already available via /issues if richer filtering is needed.
        let _ = epic;
    }
    if q.actionable.unwrap_or(false) && !row.actionable {
        return false;
    }
    true
}

fn comma_match(value: &str, csv: &str) -> bool {
    csv.split(',').any(|s| s == value)
}

#[derive(Debug, Deserialize)]
pub struct OverrideBody {
    pub issue: u64,
    /// Set the override to this value. Mutually exclusive with `clear`.
    pub value: Option<f64>,
    /// Clear an existing override. Mutually exclusive with `value`.
    #[serde(default)]
    pub clear: bool,
}

pub async fn set_override(
    State(app): State<AppState>,
    Json(body): Json<OverrideBody>,
) -> Result<Json<Value>, ApiError> {
    if body.clear == body.value.is_some() {
        return Err(ApiError::bad_request(
            "exactly one of `value` or `clear: true` must be provided",
        ));
    }
    let paths = app.paths.clone();
    let id = body.issue;
    let value = body.value;
    let clear = body.clear;

    let result = tokio::task::spawn_blocking(move || -> Result<(), ApiError> {
        if clear {
            let args = issue::priority_override::ClearArgs {
                repo_root: paths.root.clone(),
                quiet: true,
                json: false,
                id,
            };
            issue::priority_override::run_clear(args).map_err(classify)
        } else {
            let args = issue::priority_override::SetArgs {
                repo_root: paths.root.clone(),
                quiet: true,
                json: false,
                id,
                value: value.expect("checked above"),
            };
            issue::priority_override::run_set(args).map_err(classify)
        }
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))?;

    result?;

    // Emit an issue.changed event so subscribers refresh affected views.
    emit(
        &app.events,
        EventKind::IssueChanged,
        json!({
            "id": id,
            "kind": if clear { "priority-override-cleared" } else { "priority-override-set" },
            "value": value,
        }),
    );

    Ok(Json(json!({"issue": id, "ok": true})))
}

fn classify(err: anyhow::Error) -> ApiError {
    if err.downcast_ref::<NotFoundError>().is_some() {
        ApiError::not_found(format!("{err:#}"))
    } else if err.downcast_ref::<UserError>().is_some() {
        let msg = format!("{err:#}");
        if msg.contains("terminal state") {
            ApiError {
                status: StatusCode::CONFLICT,
                error: "conflict".to_string(),
                message: msg,
                details: None,
            }
        } else {
            ApiError::bad_request(msg)
        }
    } else {
        ApiError::hub_error(format!("{err:#}"))
    }
}
