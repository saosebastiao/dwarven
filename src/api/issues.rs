use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::state::AppState;
use crate::api::types::{Comment, Issue};
use crate::storage::comment_file::list_comments;
use crate::storage::issue_file::{enumerate_issue_ids, read_issue};

const TERMINAL_STATES: &[&str] = &["done", "dropped"];

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    state: Option<String>,
    r#type: Option<String>,
    blocker: Option<String>,
    priority: Option<String>,
    epic: Option<String>,
    grep: Option<String>,
    closed: Option<bool>,
    all: Option<bool>,
}

pub async fn list(
    State(app): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<Issue>>, ApiError> {
    let paths = app.paths.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<Vec<Issue>, ApiError> {
        let ids = enumerate_issue_ids(&paths.issues_dir())
            .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let issue = read_issue(&paths.issue_md(id))
                .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
            out.push(Issue::from_file(issue));
        }
        Ok(out)
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))??;

    let filtered = apply_filters(result, &q);
    Ok(Json(filtered))
}

pub async fn view(
    State(app): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    let issue = tokio::task::spawn_blocking(move || -> Result<Issue, ApiError> {
        let p = paths.issue_md(id);
        if !p.exists() {
            return Err(ApiError::not_found(format!("issue #{id} not found")));
        }
        let f = read_issue(&p).map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
        Ok(Issue::from_file(f))
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))??;
    Ok(Json(issue))
}

#[derive(Debug, Deserialize)]
pub struct CommentQuery {
    kind: Option<String>,
}

pub async fn list_comments_for_issue(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    Query(q): Query<CommentQuery>,
) -> Result<Json<Vec<Comment>>, ApiError> {
    let paths = app.paths.clone();
    let comments = tokio::task::spawn_blocking(move || -> Result<Vec<Comment>, ApiError> {
        let issue_path = paths.issue_md(id);
        if !issue_path.exists() {
            return Err(ApiError::not_found(format!("issue #{id} not found")));
        }
        let raw = list_comments(&paths.comments_dir(id))
            .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
        Ok(raw.into_iter().map(Comment::from_file).collect())
    })
    .await
    .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))??;

    let kind_filter = q.kind;
    let filtered: Vec<Comment> = match kind_filter {
        Some(k) => comments.into_iter().filter(|c| c.kind == k).collect(),
        None => comments,
    };
    Ok(Json(filtered))
}

fn apply_filters(rows: Vec<Issue>, q: &ListQuery) -> Vec<Issue> {
    rows.into_iter()
        .filter(|fm| matches(fm, q))
        .collect()
}

fn matches(issue: &Issue, q: &ListQuery) -> bool {
    let all = q.all.unwrap_or(false);
    let closed = q.closed.unwrap_or(false);
    let is_terminal = TERMINAL_STATES.contains(&issue.state.as_str());

    if !all {
        if closed {
            if !is_terminal {
                return false;
            }
        } else if is_terminal {
            // implicit --open
            return false;
        }
    }
    if let Some(states) = &q.state {
        if !comma_match(&issue.state, states) {
            return false;
        }
    }
    if let Some(types) = &q.r#type {
        if !comma_match(&issue.r#type, types) {
            return false;
        }
    }
    if let Some(blockers) = &q.blocker {
        let bm = match &issue.blocker {
            Some(b) => comma_match(b, blockers),
            None => comma_match("unset", blockers),
        };
        if !bm {
            return false;
        }
    }
    if let Some(priorities) = &q.priority {
        let pm = match &issue.priority {
            Some(p) => comma_match(p, priorities),
            None => comma_match("unset", priorities),
        };
        if !pm {
            return false;
        }
    }
    if let Some(epic) = &q.epic {
        match &issue.epic {
            Some(e) if e == epic => {}
            _ => return false,
        }
    }
    if let Some(needle) = &q.grep {
        let in_title = issue.title.contains(needle);
        let in_body = issue.body.contains(needle);
        if !in_title && !in_body {
            return false;
        }
    }
    true
}

fn comma_match(value: &str, csv: &str) -> bool {
    csv.split(',').any(|s| s == value)
}
