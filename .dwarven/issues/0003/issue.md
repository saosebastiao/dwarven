---
id: 3
title: 'SSE: Last-Event-ID-based replay on reconnect'
type: feature
state: pm
priority: p2
epic: daemon-polish
created: 2026-05-10T02:03:15Z
created_by: maintainer
updated: 2026-05-10T02:03:15Z
---
web-api.md#R5.6 acknowledges this as a future extension: "The stream does not replay missed events in v1; on reconnect, the UI may need to refresh affected views. (A Last-Event-ID-based replay mechanism is a possible future extension.)"

Current state (slice 17, src/api/events.rs): the broadcast channel has capacity 256 and lagged subscribers receive a `stream.lagged` event with the skip count. The web UI handles it as a generic refresh. We do NOT track per-event sequence numbers or persist anything.

Scope:
- Assign each EventEnvelope a monotonic seq id (u64).
- Optionally retain a small ring buffer (~256 events) in memory.
- SSE handler honors `Last-Event-ID` header on reconnect: replay events with seq > Last-Event-ID from the ring buffer, then attach to the live stream. If the requested seq is older than the buffer's oldest, return a single `stream.refreshed-required` event so the client knows to do a full refetch.
- Persistence across daemon restarts is NOT in scope; ring buffer is in-memory only.
