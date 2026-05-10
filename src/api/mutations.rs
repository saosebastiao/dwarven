//! HTTP mutation handlers. Each handler translates a JSON request body into
//! the corresponding `issue::*` Args struct and delegates to the run()
//! function we already use from the CLI. Errors are mapped to HTTP via
//! `map_err`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::Value;

use crate::api::error::ApiError;
use crate::api::events::EventKind;
use crate::api::state::AppState;
use crate::api::types::{Comment, Issue};
use crate::issue;
use crate::issue::create::{BodyInput, NotFoundError, UserError};
use crate::storage::comment_file::list_comments;
use crate::storage::config::RepoPaths;
use crate::storage::issue_file::read_issue;

/// Translate a downstream anyhow error into the right HTTP shape.
fn classify(err: anyhow::Error) -> ApiError {
    if err.downcast_ref::<NotFoundError>().is_some() {
        ApiError::not_found(format!("{err:#}"))
    } else if err.downcast_ref::<UserError>().is_some() {
        // bad_request for input-shaped failures; conflict for things like
        // illegal transitions / cycles. We can't tell them apart from the
        // string content alone, so default to bad_request. Specific
        // handlers (transition, dep add) override this where needed.
        ApiError::bad_request(format!("{err:#}"))
    } else {
        ApiError::hub_error(format!("{err:#}"))
    }
}

fn classify_conflict(err: anyhow::Error) -> ApiError {
    if err.downcast_ref::<NotFoundError>().is_some() {
        ApiError::not_found(format!("{err:#}"))
    } else if err.downcast_ref::<UserError>().is_some() {
        ApiError {
            status: StatusCode::CONFLICT,
            error: "conflict".to_string(),
            message: format!("{err:#}"),
            details: None,
        }
    } else {
        ApiError::hub_error(format!("{err:#}"))
    }
}

fn actor_or_default(actor: Option<String>) -> String {
    actor
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| "maintainer".to_string())
}

async fn run_blocking<F, T>(f: F) -> Result<T, ApiError>
where
    F: FnOnce() -> Result<T, ApiError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))?
}

fn read_issue_for_response(paths: &RepoPaths, id: u64) -> Result<Issue, ApiError> {
    let p = paths.issue_md(id);
    if !p.exists() {
        return Err(ApiError::not_found(format!("issue #{id} not found")));
    }
    let f = read_issue(&p).map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
    Ok(Issue::from_file(f))
}

// ---------- Issues: create + edit ----------

#[derive(Debug, Deserialize)]
pub struct CreateIssueBody {
    pub title: String,
    pub r#type: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub blocker: Option<String>,
    #[serde(default)]
    pub blocked_by: Vec<u64>,
    #[serde(default)]
    pub blocks: Vec<u64>,
    #[serde(default)]
    pub epic: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

pub async fn create_issue(
    State(app): State<AppState>,
    Json(body): Json<CreateIssueBody>,
) -> Result<(StatusCode, Json<Issue>), ApiError> {
    let paths = app.paths.clone();
    let actor = actor_or_default(body.actor);
    let blocker = body.blocker;

    let id = run_blocking(move || {
        let body_input = match body.body {
            Some(s) if !s.is_empty() => BodyInput::Inline(s),
            _ => BodyInput::None,
        };
        let args = issue::create::CreateArgs {
            repo_root: paths.root.clone(),
            actor: actor.clone(),
            quiet: true,
            json: false,
            issue_type: body.r#type,
            title: body.title,
            body: body_input,
            state: body.state,
            priority: body.priority,
            blocked_by: body.blocked_by,
            blocks: body.blocks,
            epic: body.epic,
        };
        // The CLI surface doesn't currently print the assigned id back
        // through a return value; for v1 we re-derive the id by reading
        // config.toml's previous next_issue_id, but it's simpler to read
        // the most-recent issue dir after the call.
        issue::create::run(args).map_err(classify)?;
        let id = latest_issue_id(&paths).map_err(|e| ApiError::hub_error(format!("{e:#}")))?;

        // If the request also asked for a blocker, set it now.
        if let Some(b) = blocker {
            let set_args = issue::blocker::SetArgs {
                repo_root: paths.root.clone(),
                actor: actor.clone(),
                quiet: true,
                json: false,
                id,
                blocker: b,
                comment: None,
            };
            issue::blocker::run_set(set_args).map_err(classify)?;
        }
        Ok(id)
    })
    .await?;

    let paths = app.paths.clone();
    let issue =
        run_blocking(move || read_issue_for_response(&paths, id)).await?;
    app.events.emit(
        EventKind::IssueCreated,
        serde_json::json!({"id": id}),
    );
    Ok((StatusCode::CREATED, Json(issue)))
}

fn latest_issue_id(paths: &RepoPaths) -> anyhow::Result<u64> {
    let ids = crate::storage::issue_file::enumerate_issue_ids(&paths.issues_dir())?;
    ids.into_iter()
        .max()
        .ok_or_else(|| anyhow::anyhow!("no issues found after create"))
}

#[derive(Debug, Deserialize)]
pub struct EditIssueBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub epic: Option<String>,
}

pub async fn edit_issue(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<EditIssueBody>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    run_blocking(move || {
        let args = issue::edit::EditArgs {
            repo_root: paths.root.clone(),
            quiet: true,
            json: false,
            id,
            title: body.title,
            issue_type: body.r#type,
            epic: body.epic,
        };
        issue::edit::run(args).map_err(classify)
    })
    .await?;

    let paths = app.paths.clone();
    let issue = run_blocking(move || read_issue_for_response(&paths, id)).await?;
    app.events.emit(
        EventKind::IssueChanged,
        serde_json::json!({"id": id, "kind": "edit"}),
    );
    Ok(Json(issue))
}

// ---------- Comments ----------

#[derive(Debug, Deserialize)]
pub struct AppendCommentBody {
    pub body: String,
    #[serde(default)]
    pub actor: Option<String>,
}

pub async fn append_comment(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<AppendCommentBody>,
) -> Result<(StatusCode, Json<Vec<Comment>>), ApiError> {
    let paths = app.paths.clone();
    let actor = actor_or_default(body.actor);

    run_blocking(move || {
        let args = issue::comment::CommentArgs {
            repo_root: paths.root.clone(),
            actor,
            quiet: true,
            json: false,
            id,
            body: BodyInput::Inline(body.body),
        };
        issue::comment::run(args).map_err(classify)
    })
    .await?;

    // Return the full comment thread so the UI doesn't need a separate fetch.
    let paths = app.paths.clone();
    let comments = run_blocking(move || -> Result<Vec<Comment>, ApiError> {
        let raw = list_comments(&paths.comments_dir(id))
            .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
        Ok(raw.into_iter().map(Comment::from_file).collect())
    })
    .await?;

    let seq = comments.last().map(|c| c.seq).unwrap_or(0);
    app.events.emit(
        EventKind::CommentAdded,
        serde_json::json!({"issue": id, "seq": seq}),
    );
    Ok((StatusCode::CREATED, Json(comments)))
}

// ---------- Transitions ----------

#[derive(Debug, Deserialize)]
pub struct TransitionBody {
    pub to: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default, rename = "override")]
    pub override_graph: bool,
    #[serde(default)]
    pub actor: Option<String>,
}

pub async fn transition(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<TransitionBody>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    let actor = actor_or_default(body.actor);
    let target = body.to.clone();
    let target_for_event = target.clone();

    run_blocking(move || {
        // R4.3.3: closing == transition with to=done|dropped. Route
        // appropriately so the close path's "from any active state"
        // freedom applies.
        if target == "done" || target == "dropped" {
            let close_args = issue::close::CloseArgs {
                repo_root: paths.root.clone(),
                actor,
                quiet: true,
                json: false,
                id,
                dropped: target == "dropped",
                body: match body.comment {
                    Some(s) if !s.is_empty() => BodyInput::Inline(s),
                    _ => BodyInput::None,
                },
            };
            issue::close::run(close_args).map_err(classify_conflict)
        } else {
            let trans_args = issue::transition::TransitionArgs {
                repo_root: paths.root.clone(),
                actor,
                quiet: true,
                json: false,
                id,
                new_state: target,
                comment: body.comment,
                override_graph: body.override_graph,
            };
            issue::transition::run(trans_args).map_err(classify_conflict)
        }
    })
    .await?;

    let paths = app.paths.clone();
    let issue = run_blocking(move || read_issue_for_response(&paths, id)).await?;
    let kind = if target_for_event == "done" || target_for_event == "dropped" {
        EventKind::IssueClosed
    } else {
        EventKind::IssueChanged
    };
    app.events.emit(
        kind,
        serde_json::json!({"id": id, "to": target_for_event}),
    );
    Ok(Json(issue))
}

// ---------- Blocker ----------

#[derive(Debug, Deserialize)]
pub struct SetBlockerBody {
    pub blocker: String,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

pub async fn set_blocker(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<SetBlockerBody>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    let actor = actor_or_default(body.actor);
    let blocker_for_event = body.blocker.clone();

    run_blocking(move || {
        let args = issue::blocker::SetArgs {
            repo_root: paths.root.clone(),
            actor,
            quiet: true,
            json: false,
            id,
            blocker: body.blocker,
            comment: body.comment,
        };
        issue::blocker::run_set(args).map_err(classify)
    })
    .await?;

    let paths = app.paths.clone();
    let issue = run_blocking(move || read_issue_for_response(&paths, id)).await?;
    app.events.emit(
        EventKind::IssueChanged,
        serde_json::json!({"id": id, "kind": "blocker-set", "blocker": blocker_for_event}),
    );
    Ok(Json(issue))
}

#[derive(Debug, Deserialize, Default)]
pub struct ClearBlockerBody {
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

pub async fn clear_blocker(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    body: Option<Json<ClearBlockerBody>>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    let body = body.map(|b| b.0).unwrap_or_default();
    let actor = actor_or_default(body.actor);

    run_blocking(move || {
        let args = issue::blocker::ClearArgs {
            repo_root: paths.root.clone(),
            actor,
            quiet: true,
            json: false,
            id,
            comment: body.comment,
        };
        issue::blocker::run_clear(args).map_err(classify)
    })
    .await?;

    let paths = app.paths.clone();
    let issue = run_blocking(move || read_issue_for_response(&paths, id)).await?;
    app.events.emit(
        EventKind::IssueChanged,
        serde_json::json!({"id": id, "kind": "blocker-cleared"}),
    );
    Ok(Json(issue))
}

// ---------- Priority ----------

#[derive(Debug, Deserialize)]
pub struct SetPriorityBody {
    pub priority: String,
}

pub async fn set_priority(
    State(app): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<SetPriorityBody>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    let priority_for_event = body.priority.clone();
    run_blocking(move || {
        let args = issue::priority::PriorityArgs {
            repo_root: paths.root.clone(),
            quiet: true,
            json: false,
            id,
            priority: body.priority,
        };
        issue::priority::run(args).map_err(classify)
    })
    .await?;
    let paths = app.paths.clone();
    let issue = run_blocking(move || read_issue_for_response(&paths, id)).await?;
    app.events.emit(
        EventKind::IssueChanged,
        serde_json::json!({"id": id, "kind": "priority-set", "priority": priority_for_event}),
    );
    Ok(Json(issue))
}

pub async fn clear_priority(
    State(app): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Issue>, ApiError> {
    let paths = app.paths.clone();
    run_blocking(move || {
        let args = issue::priority::ClearArgs {
            repo_root: paths.root.clone(),
            quiet: true,
            json: false,
            id,
        };
        issue::priority::run_clear(args).map_err(classify)
    })
    .await?;
    let paths = app.paths.clone();
    let issue = run_blocking(move || read_issue_for_response(&paths, id)).await?;
    app.events.emit(
        EventKind::IssueChanged,
        serde_json::json!({"id": id, "kind": "priority-cleared"}),
    );
    Ok(Json(issue))
}

// ---------- Dependencies ----------

#[derive(Debug, Deserialize)]
pub struct AddDepBody {
    pub from: u64,
    pub to: u64,
    #[serde(default)]
    pub rationale: Option<String>,
    #[serde(default)]
    pub actor: Option<String>,
}

pub async fn add_dep(
    State(app): State<AppState>,
    Json(body): Json<AddDepBody>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let paths = app.paths.clone();
    let actor = actor_or_default(body.actor);
    let (from_id, to_id) = (body.from, body.to);
    let rationale_input = match body.rationale {
        Some(s) if !s.is_empty() => BodyInput::Inline(s),
        _ => BodyInput::None,
    };

    run_blocking(move || {
        let args = issue::dep::AddArgs {
            repo_root: paths.root.clone(),
            actor,
            quiet: true,
            json: false,
            from_id,
            to_id,
            rationale: rationale_input,
        };
        issue::dep::run_add(args).map_err(classify_conflict)
    })
    .await?;

    app.events.emit(
        EventKind::DependencyAdded,
        serde_json::json!({"from": from_id, "to": to_id}),
    );
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"from": from_id, "to": to_id})),
    ))
}

pub async fn remove_dep(
    State(app): State<AppState>,
    Path((from, to)): Path<(u64, u64)>,
) -> Result<impl IntoResponse, ApiError> {
    let paths = app.paths.clone();
    run_blocking(move || {
        let args = issue::dep::RemoveArgs {
            repo_root: paths.root.clone(),
            quiet: true,
            json: false,
            from_id: from,
            to_id: to,
        };
        issue::dep::run_remove(args).map_err(classify)
    })
    .await?;
    app.events.emit(
        EventKind::DependencyRemoved,
        serde_json::json!({"from": from, "to": to}),
    );
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_deps_all(State(app): State<AppState>) -> Result<Json<Value>, ApiError> {
    let paths = app.paths.clone();
    let edges = run_blocking(move || -> Result<Value, ApiError> {
        let ids = crate::storage::issue_file::enumerate_issue_ids(&paths.issues_dir())
            .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
        let mut edges = Vec::new();
        for id in ids {
            let issue = read_issue(&paths.issue_md(id))
                .map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
            for blocked in issue.frontmatter.blocks {
                edges.push(serde_json::json!({"from": id, "to": blocked}));
            }
        }
        Ok(Value::Array(edges))
    })
    .await?;
    Ok(Json(edges))
}

pub async fn list_deps_for_issue(
    State(app): State<AppState>,
    Path(id): Path<u64>,
) -> Result<Json<Value>, ApiError> {
    let paths = app.paths.clone();
    let result = run_blocking(move || -> Result<Value, ApiError> {
        let p = paths.issue_md(id);
        if !p.exists() {
            return Err(ApiError::not_found(format!("issue #{id} not found")));
        }
        let issue = read_issue(&p).map_err(|e| ApiError::hub_error(format!("{e:#}")))?;
        Ok(serde_json::json!({
            "id": id,
            "blocks": issue.frontmatter.blocks,
            "blocked_by": issue.frontmatter.blocked_by,
        }))
    })
    .await?;
    Ok(Json(result))
}
