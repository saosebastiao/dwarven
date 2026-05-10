use std::convert::Infallible;

use axum::extract::State;
use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use futures_util::stream::Stream;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::state::AppState;

/// In-process broadcast channel for change events. The daemon's writers
/// (HTTP mutation handlers, file watcher) `send`; the SSE handler
/// subscribes per-connection.
pub type EventTx = broadcast::Sender<EventEnvelope>;

/// Capacity bound. Slow subscribers that lag past this drop messages and
/// the SSE handler reports the gap to the client.
pub const CHANNEL_CAPACITY: usize = 256;

#[derive(Debug, Clone, Serialize)]
pub struct EventEnvelope {
    pub kind: EventKind,
    pub payload: Value,
}

impl EventEnvelope {
    pub fn new(kind: EventKind, payload: Value) -> Self {
        Self { kind, payload }
    }
}

/// Event types per `web-api.md#R5.4`. New types may be added in minor
/// versions; renames are major.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    IssueCreated,
    IssueChanged,
    IssueClosed,
    CommentAdded,
    DependencyAdded,
    DependencyRemoved,
    DaemonReindexed,
}

impl EventKind {
    pub fn as_sse_event(&self) -> &'static str {
        match self {
            EventKind::IssueCreated => "issue.created",
            EventKind::IssueChanged => "issue.changed",
            EventKind::IssueClosed => "issue.closed",
            EventKind::CommentAdded => "comment.added",
            EventKind::DependencyAdded => "dependency.added",
            EventKind::DependencyRemoved => "dependency.removed",
            EventKind::DaemonReindexed => "daemon.reindexed",
        }
    }
}

pub fn make_channel() -> EventTx {
    let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
    tx
}

/// Best-effort emit. A send to a channel with no current subscribers is
/// not an error — events are dropped on the floor until someone connects.
pub fn emit(tx: &EventTx, kind: EventKind, payload: Value) {
    let _ = tx.send(EventEnvelope::new(kind, payload));
}

pub async fn sse_handler(
    State(app): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = app.events.subscribe();
    let stream = BroadcastStream::new(rx).map(|res| match res {
        Ok(envelope) => Ok(Event::default()
            .event(envelope.kind.as_sse_event())
            .data(envelope.payload.to_string())),
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => Ok(Event::default()
            .event("stream.lagged")
            .data(json!({ "skipped": n }).to_string())),
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
