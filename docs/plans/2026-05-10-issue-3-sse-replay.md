---
issue: 3
date: 2026-05-10
---

# Plan: SSE Last-Event-ID-based replay on reconnect

**Issue:** #3
**Specs:** `web-api.md#R5.6`, `R5.7`

## Goal

Make the SSE stream resilient to brief disconnects. On reconnect, the client sends `Last-Event-ID: <n>` and the server replays any events emitted with `seq > n` from an in-memory ring buffer, then attaches to the live stream. If the requested id is older than the buffer's oldest entry, the server emits one `stream.refresh-required` event so the client knows it must refetch instead of trusting partial state.

In-memory only; no persistence across daemon restarts (R5.6 explicitly).

## Surface change

`EventEnvelope` gains `seq: u64` (monotonic, allocated by the bus).

The `EventTx = broadcast::Sender<EventEnvelope>` becomes a thin wrapper struct `EventBus` that owns:
- `broadcast::Sender<EventEnvelope>` — live stream
- `Arc<Mutex<VecDeque<EventEnvelope>>>` — ring buffer, capacity 256
- `AtomicU64` — next seq to assign

Public API:
- `EventBus::new() -> EventBus`
- `EventBus::emit(kind: EventKind, payload: Value)` — allocates seq, pushes to ring (evicting oldest if full), broadcasts.
- `EventBus::subscribe() -> Receiver`
- `EventBus::replay_since(last_id: u64) -> Replay`
   - `Replay::Events(Vec<EventEnvelope>)` — replay from buffer
   - `Replay::RefreshRequired { available_oldest: u64 }` — last_id < oldest in ring

Existing call sites:
- `crate::api::events::emit(&tx, kind, payload)` becomes `bus.emit(kind, payload)`.
- AppState's `events: EventTx` becomes `events: EventBus` (cloneable Arc-holding type).
- watcher and mutation handlers use `app.events.emit(...)`.

SSE handler change:
- Read `Last-Event-ID` header from the request.
- If present and parses as u64, call `bus.replay_since(id)`.
   - On `Events`: yield replay events first (with their `seq` as the SSE id), then attach to live.
   - On `RefreshRequired`: yield one `stream.refresh-required` event with payload `{available_oldest}`, then attach to live.
- If absent: just attach to live.
- For each event yielded (replay or live), set `Event::id(seq.to_string())` so the client tracks `Last-Event-ID` automatically.

## Tests

`tests/api_events_replay.rs`:

1. `events_carry_monotonic_id` — connect, trigger 2 mutations, assert SSE `id:` lines are 1, 2.
2. `replay_emits_missed_events` — connect, get id N, disconnect, mutate twice (server-side), reconnect with `Last-Event-ID: N`, assert next 2 events received are seq N+1, N+2.
3. `replay_refresh_required_when_id_too_old` — emit > 256 events between disconnect and reconnect; reconnect with id=1; expect a `stream.refresh-required` event before live attach.
4. `no_last_event_id_streams_live_only` — first connect without header → no replay attempted.

The "emit > 256 events" test is feasible because the test's mutation loop runs synchronously and the bus's emit is non-blocking.

## Code organization

Refactor `src/api/events.rs`:
- Keep `EventEnvelope`, `EventKind` types.
- Add `EventBus` struct + `Replay` enum.
- Move `make_channel()` to `EventBus::new()`; deprecate the old name (callers updated).
- Move `emit()` free function to `EventBus::emit` method; deprecate.
- The SSE handler stays in this file but takes `&EventBus` via `AppState`.

`src/api/state.rs`:
- `AppState.events: EventTx` → `AppState.events: EventBus`. EventBus must be `Clone` for AppState's clone-per-handler semantics — wrap fields in `Arc` accordingly.

## Branch

`feat/3-sse-replay`. Two commits: tests RED, then impl GREEN. Merge into main.
