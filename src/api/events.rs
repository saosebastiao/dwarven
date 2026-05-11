//! SSE event stream + `Last-Event-ID` replay.
//!
//! [`EventBus`] owns a `tokio::sync::broadcast` channel for live
//! delivery, a 256-entry [`VecDeque`] ring buffer for short-window
//! replay, and an [`AtomicU64`] monotonic sequence counter.
//!
//! Emitters: each HTTP mutation handler calls [`EventBus::emit`] after
//! a successful mutation; the file watcher calls it with
//! [`EventKind::DaemonReindexed`] after each reindex.
//!
//! Subscribers: [`sse_handler`] is the GET `/api/v1/events` endpoint.
//! On reconnect with a `Last-Event-ID` header, it replays missed
//! events from the ring buffer before attaching to the live stream.
//! If the requested id is older than the ring's oldest entry, it emits
//! a single `stream.refresh-required` event so the client knows to
//! refetch state instead of trusting an incomplete tail.
//!
//! Architecture: [`docs/architecture/event-stream.md`](../../../docs/architecture/event-stream.md).

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use futures_util::stream::{self, Stream, StreamExt};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use crate::api::state::AppState;

/// Broadcast capacity per-subscriber. Slow consumers that lag past this drop
/// messages; the SSE handler emits a `stream.lagged` event so the client
/// can decide whether to refresh.
pub const CHANNEL_CAPACITY: usize = 256;

/// Replay buffer capacity. Reconnects with `Last-Event-ID` older than this
/// many events back receive a single `stream.refresh-required` event
/// (`web-api.md#R5.6`). 256 covers a few seconds of bursty mutation
/// traffic at hundreds-of-issues scale.
pub const REPLAY_CAPACITY: usize = 256;

#[derive(Debug, Clone, Serialize)]
pub struct EventEnvelope {
    /// Monotonic per-bus sequence number, allocated in `EventBus::emit`.
    pub seq: u64,
    pub kind: EventKind,
    pub payload: Value,
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

/// In-process bus of change events. Owns a broadcast::Sender for live
/// delivery, a ring buffer of recent events for `Last-Event-ID` replay,
/// and a monotonic seq counter.
///
/// `Clone` is cheap (Arc-only) so `AppState` clones get the same bus.
#[derive(Clone)]
pub struct EventBus {
    inner: Arc<EventBusInner>,
}

struct EventBusInner {
    tx: broadcast::Sender<EventEnvelope>,
    ring: Mutex<VecDeque<EventEnvelope>>,
    next_seq: AtomicU64,
}

/// Result of `EventBus::replay_since`.
#[derive(Debug)]
pub enum Replay {
    /// `last_id` is recent enough that the ring covers everything since.
    /// `events` is the missed-events list (may be empty); `snapshot_max`
    /// is the highest seq in this snapshot — live events with seq `<=`
    /// this are duplicates and should be filtered.
    Events { events: Vec<EventEnvelope>, snapshot_max: u64 },
    /// `last_id` is older than the ring's oldest entry; the missing
    /// events have been evicted. Caller should refetch.
    RefreshRequired { available_oldest: u64 },
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            inner: Arc::new(EventBusInner {
                tx,
                ring: Mutex::new(VecDeque::with_capacity(REPLAY_CAPACITY)),
                next_seq: AtomicU64::new(1),
            }),
        }
    }

    /// Allocate a seq, push into the ring (evicting oldest if full), and
    /// broadcast. Best-effort: a `send` with no current subscribers is not
    /// an error, but the event still lands in the ring for future replay.
    pub fn emit(&self, kind: EventKind, payload: Value) {
        let seq = self.inner.next_seq.fetch_add(1, Ordering::Relaxed);
        let envelope = EventEnvelope { seq, kind, payload };

        // Push to ring. Locking under `tokio::sync::Mutex` requires
        // `.await`; we're called from sync handler paths, so use
        // `try_lock` and fall back to a brief blocking lock via the
        // executor's `block_in_place` if needed. Practically the ring is
        // never contended for long, so try_lock succeeds.
        match self.inner.ring.try_lock() {
            Ok(mut ring) => Self::push(&mut ring, envelope.clone()),
            Err(_) => {
                // Rare: another emitter or a snapshot read is holding the
                // lock. Spin briefly via blocking_lock — this is acceptable
                // because emitters are infrequent vs. broadcast rate.
                let mut ring = self.inner.ring.blocking_lock();
                Self::push(&mut ring, envelope.clone());
            }
        }

        let _ = self.inner.tx.send(envelope);
    }

    fn push(ring: &mut VecDeque<EventEnvelope>, e: EventEnvelope) {
        if ring.len() == REPLAY_CAPACITY {
            ring.pop_front();
        }
        ring.push_back(e);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.inner.tx.subscribe()
    }

    /// Take a snapshot of events with seq > `last_id`. See `Replay` doc.
    pub async fn replay_since(&self, last_id: u64) -> Replay {
        let ring = self.inner.ring.lock().await;
        let oldest = ring.front().map(|e| e.seq);
        let newest = ring.back().map(|e| e.seq);
        // Empty ring → nothing to replay; not refresh-required (nothing
        // has happened since last_id either, by construction).
        let Some(oldest) = oldest else {
            return Replay::Events {
                events: Vec::new(),
                snapshot_max: last_id,
            };
        };
        // last_id+1 must be present in ring for replay to be lossless.
        // Equivalent: oldest <= last_id + 1, i.e. last_id + 1 >= oldest.
        if last_id + 1 < oldest {
            return Replay::RefreshRequired {
                available_oldest: oldest,
            };
        }
        let events: Vec<EventEnvelope> =
            ring.iter().filter(|e| e.seq > last_id).cloned().collect();
        Replay::Events {
            events,
            snapshot_max: newest.unwrap_or(last_id),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn sse_handler(
    State(app): State<AppState>,
    headers: HeaderMap,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let last_id: Option<u64> = headers
        .get("last-event-id")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok());

    // Subscribe to live FIRST so events emitted between snapshot read and
    // subscription register are not missed.
    let live_rx = app.events.subscribe();

    // Replay snapshot only if Last-Event-ID was supplied. Without the
    // header, fresh subscribers see only future events (existing
    // behavior).
    let (prefix, snapshot_max): (Vec<EventEnvelope>, u64) = match last_id {
        Some(id) => match app.events.replay_since(id).await {
            Replay::Events { events, snapshot_max } => (events, snapshot_max),
            Replay::RefreshRequired { available_oldest } => {
                // Synthesize a single "stream.refresh-required" event in
                // the prefix; live attach proceeds as normal.
                let synthetic_seq = available_oldest.saturating_sub(1);
                let synthetic = EventEnvelope {
                    seq: synthetic_seq,
                    // Re-using IssueChanged kind would be wrong; we emit
                    // a sentinel via a non-EventKind path: see prefix
                    // mapping below where we special-case seq < 1.
                    // Encode the marker via a special payload.
                    kind: EventKind::DaemonReindexed, // placeholder; prefix mapping treats as_sse_event differently below
                    payload: json!({ "available_oldest": available_oldest }),
                };
                // We rebuild the SSE Event directly for the synthetic
                // entry below using a closure-friendly representation:
                // the prefix vec carries it, and the `to_sse_event`
                // mapping checks for the sentinel payload to override
                // the event-name.
                (vec![synthetic], synthetic_seq)
            }
        },
        None => (Vec::new(), 0),
    };

    let prefix_stream = stream::iter(prefix.into_iter().map(to_sse_event));
    let live_stream = BroadcastStream::new(live_rx).filter_map(move |res| {
        let item = match res {
            Ok(envelope) => {
                if envelope.seq <= snapshot_max {
                    None
                } else {
                    Some(to_sse_event(envelope))
                }
            }
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                Some(Ok(Event::default()
                    .event("stream.lagged")
                    .data(json!({ "skipped": n }).to_string())))
            }
        };
        async move { item }
    });

    let combined = prefix_stream.chain(live_stream);
    Sse::new(combined).keep_alive(KeepAlive::default())
}

fn to_sse_event(envelope: EventEnvelope) -> Result<Event, Infallible> {
    // `replay_since` synthesizes a refresh-required marker by setting
    // payload = {"available_oldest": _}. We detect that here and override
    // the event name to "stream.refresh-required" rather than introducing
    // a new EventKind variant (which would expand the public R5.4 enum).
    let is_refresh_marker = envelope
        .payload
        .get("available_oldest")
        .map(|_| envelope.payload.as_object().map(|m| m.len() == 1).unwrap_or(false))
        .unwrap_or(false);
    let name = if is_refresh_marker {
        "stream.refresh-required"
    } else {
        envelope.kind.as_sse_event()
    };
    Ok(Event::default()
        .event(name)
        .id(envelope.seq.to_string())
        .data(envelope.payload.to_string()))
}
