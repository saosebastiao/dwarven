---
spec_ref: web-api.md (R5), coordination-hub.md (R6.3)
date: 2026-05-10
issue: 8
---

# Event stream

The hub's real-time change stream: who emits, who subscribes, how reconnects recover from missed events. Implementation companion to `web-api.md#R5`.

## The bus

A single `EventBus` per daemon process. Owned by `AppState`, shared (via `Arc`) with every HTTP handler and with the watcher thread.

```rust
pub struct EventBus {
    inner: Arc<EventBusInner>,
}
struct EventBusInner {
    tx: broadcast::Sender<EventEnvelope>,  // live stream
    ring: Mutex<VecDeque<EventEnvelope>>,  // replay buffer
    next_seq: AtomicU64,                   // monotonic seq counter
}
```

Three pieces under one Arc:

- **`broadcast::Sender`** — `tokio::sync::broadcast` channel. Each subscriber gets its own `Receiver` and receives every message sent after subscription.
- **`Mutex<VecDeque<EventEnvelope>>`** — bounded ring buffer (256 entries) of recent events for `Last-Event-ID` replay.
- **`AtomicU64`** — monotonic sequence counter, allocated on each `emit`. Resets to 1 on daemon restart (no cross-restart persistence; `web-api.md#R5.6` explicitly defers that).

## Emitter contract

`EventBus::emit(kind, payload)` is the only way to produce an event:

1. Allocate a new `seq` via `fetch_add(1, Relaxed)`.
2. Push the envelope into the ring buffer (evict oldest if at capacity).
3. Broadcast to all live subscribers.

The send to the broadcast channel can fail if there are no subscribers — that's not an error, the event still lands in the ring for future replay.

Callers: mutation handlers in `src/api/mutations.rs` and `src/api/scheduler.rs`, plus the watcher's `daemon.reindexed` emits. Each call is best-effort; we never `await` the broadcast or fall back to retry logic.

## Subscriber contract

`EventBus::subscribe() -> broadcast::Receiver<EventEnvelope>` returns a new receiver that sees every event broadcast *after* the call. Order: `subscribe()` is called *before* the ring snapshot is taken so any emit between snapshot and subscribe is captured by the live receiver. The post-snapshot filter `seq > snapshot_max` drops duplicates that appear in both the ring and the broadcast.

This race-free protocol was a careful choice when issue #3 (`Last-Event-ID` replay) was added — see `src/api/events.rs#sse_handler` for the exact sequencing.

## Last-Event-ID replay

When a client reconnects with the `Last-Event-ID` header:

```
EventBus::replay_since(last_id) -> Replay::Events { events, snapshot_max }
                                | Replay::RefreshRequired { available_oldest }
```

`Replay::Events` is the normal case: the ring still has every event with seq > last_id. The handler yields these as the prefix of the SSE stream, then attaches to the live broadcast filtered to seq > snapshot_max.

`Replay::RefreshRequired` is the lossy case: last_id is older than the ring's oldest entry (i.e., some events have been evicted). The handler emits a single `stream.refresh-required` event with payload `{ available_oldest }`, then attaches to live. The client (or the maintainer's eyeballs) is expected to refetch state from the JSON API instead of trusting the partial tail.

The 256-entry ring is sized for "brief disconnects during normal use." Sustained disconnects past 256 events trigger the refresh path. That's a conservative buffer at hundreds-of-issues scale; if real usage shows clients regularly miss past the buffer, the size should grow.

## Why not WAL-based replay?

A SQLite-backed event log (with per-event commit, persistence across restart) was considered and rejected for v1:

- **Persistence cost.** Every event becomes a SQLite commit. At the watcher's reindex rate during bursty edits, this is non-trivial overhead.
- **Restart semantics get awkward.** Across restart, what should the "live cursor" be? Replay from start? Skip to live? Either choice surprises someone.
- **Spec explicitly defers it** (`web-api.md#R5.6`). The future-extension language is unambiguous.

In-memory ring is enough for the "browser tab momentarily disconnected" case the spec actually targets.

## Event vocabulary

`EventKind` enum (`src/api/events.rs`) maps 1:1 to the names in `web-api.md#R5.4`:

```rust
pub enum EventKind {
    IssueCreated,       // → "issue.created"
    IssueChanged,       // → "issue.changed"
    IssueClosed,        // → "issue.closed"
    CommentAdded,       // → "comment.added"
    DependencyAdded,    // → "dependency.added"
    DependencyRemoved,  // → "dependency.removed"
    DaemonReindexed,    // → "daemon.reindexed"
}
```

The `stream.lagged` and `stream.refresh-required` event names are NOT in this enum; they're synthesized by the SSE handler directly. Adding a new EventKind variant is a minor-version change (`web-api.md#R6.2`); renames or removals are major.

## Payload conventions

Payloads are intentionally thin — id and the bare minimum identifying information. The spec says (`web-api.md#R5.5`): "Each event carries enough information for the web UI to update its local view without re-fetching. Detailed payloads (full issue object) are not sent in events; the UI fetches when it needs detail."

This is a tradeoff: heavier payloads would let trivial UI updates skip the fetch round-trip, but they'd also tie the event-stream schema to the resource schema, making field additions to issues require corresponding event-version bumps. Lean payloads keep the two evolutions independent.

## Lagging subscribers

Slow consumers can fall behind the broadcast channel's per-receiver buffer (`CHANNEL_CAPACITY = 256`). When this happens, `BroadcastStream::next` returns `Err(BroadcastStreamRecvError::Lagged(n))`. The SSE handler synthesizes a `stream.lagged` event with `{ skipped: n }` so the client can either refresh or accept the gap.

In practice, the web UI's renderers complete in milliseconds and almost never lag. The mechanism is a safety net for pathologically slow consumers (e.g., a debug session attached to the SSE endpoint via curl in a slow tty).

## Watcher integration

The file watcher (`src/daemon/watcher.rs`) emits `EventKind::DaemonReindexed` after every reindex, with a `reason` field (`startup`, `events`, `reconcile`, `reconcile-fallback`). This is the only event the watcher emits — file-system events themselves are not surfaced because they're noisy (a single `dwarven issue create` produces multiple FS events for issue.md, comment files, etc., which the debounce collapses into one reindex).

The mutation handlers emit the per-mutation events (`issue.created`, `issue.changed`, `comment.added`, etc.). Direct file edits that bypass the API (the maintainer's escape hatch per `storage-model.md#R6.2`) produce only `daemon.reindexed` events — the UI must re-fetch to see the new state.

## What's not in v1

`web-api.md#R7.4` defers outbound webhooks; the SSE stream is the only event consumer surface. Slack notifications, email, GitHub mirroring, etc. are all future adapter work that would subscribe to the same in-process bus.
