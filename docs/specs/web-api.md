---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Web API

This document specifies the local HTTP API exposed by the coordination hub daemon (`coordination-hub.md#R5`). The API is the surface consumed by the web UI (`web-ui.md`) and by any future HTTP consumer (CI integrations, IDE extensions).

Per `dwarven.md#R2.10`, **agents do not consume this API directly**; they use the `dwarven` CLI (`dwarven-cli.md`). The API exists for the web UI.

---

## R1 — Scope

R1.1 — This spec defines: base URL conventions; resource layout; request and response formats; the real-time event stream; error format; and stability guarantees.

R1.2 — Out of scope: HTTP server implementation (`coordination-hub.md`); the on-disk artifact format the API exposes (`storage-model.md`); the web UI consumer (`web-ui.md`).

R1.3 — The API exposes the same operations as the CLI surface defined in `dwarven-cli.md`. This spec does not re-specify operation semantics; it specifies the HTTP shape.

---

## R2 — Invocation conventions

R2.1 — All endpoints live under the path prefix `/api/v1`. Major-version bumps add a new prefix (`/api/v2`); the old prefix continues to work until the next breaking change.

R2.2 — The daemon binds to `127.0.0.1` only by default (`coordination-hub.md#R3.6`). No CORS headers are emitted in v1 — same-origin access from the bundled web UI is the only supported consumption pattern.

R2.3 — Request and response bodies are JSON unless explicitly noted. `Content-Type: application/json` is required on requests with a body.

R2.4 — Authentication: none in v1 (`coordination-hub.md#R5.4`). Any process that can reach the bind address is trusted.

R2.5 — Every mutating request accepts an optional `actor` field in its JSON body, mirroring the CLI's `--actor` (`dwarven-cli.md#R3`). When omitted, the actor is `maintainer` (the web UI is the maintainer's surface).

---

## R3 — Response and error format

R3.1 — Successful responses use HTTP status codes 200 (read), 201 (created), 204 (no content). Response body is the affected resource as JSON, except for 204.

R3.2 — Error responses use the JSON shape:

```json
{
  "error": "<machine-readable code>",
  "message": "<human-readable explanation>",
  "details": { /* optional, error-specific */ }
}
```

R3.3 — Error codes correspond to CLI exit codes (`dwarven-cli.md#R5`):

| HTTP status | Error code | Meaning |
|---|---|---|
| 400 | `bad_request` | Malformed input, invalid flags, illegal transition |
| 404 | `not_found` | Issue / comment / resource does not exist |
| 409 | `conflict` | Cycle detection, terminal-state mutation, lock contention |
| 500 | `hub_error` | Filesystem failure, corrupted state |

R3.4 — All timestamps are ISO 8601 UTC with millisecond precision (`2026-05-09T14:22:00.123Z`).

R3.5 — Issue and comment field shapes mirror the on-disk frontmatter (`storage-model.md#R4`) one-for-one, with the body included as a `body` field. Derived fields (e.g., effective priority from `dep-graph.md`) are added under documented keys; their absence on a freshly-rebuilt index is permissible.

---

## R4 — Resources

### R4.1 — Issues

| Method | Path | Operation |
|---|---|---|
| `GET` | `/api/v1/issues` | List with filter query params |
| `POST` | `/api/v1/issues` | Create (mirrors `dwarven issue create`) |
| `GET` | `/api/v1/issues/:id` | View |
| `PATCH` | `/api/v1/issues/:id` | Edit non-state fields (title, type, epic) |
| `DELETE` | `/api/v1/issues/:id` | Disallowed in v1 (returns 405); use close + dropped instead |

R4.1.1 — `GET /api/v1/issues` accepts query params: `state`, `type`, `blocker`, `priority`, `epic`, `open` (boolean), `closed` (boolean), `grep`, `sort`. Multi-valued filters use comma separation (`?state=plan,test`). Defaults match `dwarven-cli.md#R6.4`.

R4.1.2 — `POST /api/v1/issues` body is the issue's frontmatter fields (minus `id`, `created`, `updated`, which the hub assigns) plus a `body` field.

R4.1.3 — `PATCH /api/v1/issues/:id` accepts only `title`, `type`, `epic`. Mutating `state`, `priority`, or `blocker` requires the dedicated endpoints (R4.4–R4.6).

### R4.2 — Comments

| Method | Path | Operation |
|---|---|---|
| `GET` | `/api/v1/issues/:id/comments` | List comments for an issue |
| `POST` | `/api/v1/issues/:id/comments` | Append a comment (`kind: comment`) |

R4.2.1 — `GET` accepts `kind` filter (e.g., `?kind=state-change` for the state history view).

R4.2.2 — `POST` body has `body` and optional `actor`. `kind` is always `comment` for this endpoint; specialized comment kinds are produced by the dedicated mutation endpoints (R4.4–R4.6).

### R4.3 — State transitions

| Method | Path | Operation |
|---|---|---|
| `POST` | `/api/v1/issues/:id/transitions` | Perform a state transition |

R4.3.1 — Body: `{ "to": "<new-state>", "comment": "<text>", "override": <bool> }`. `comment` is recommended; `override` defaults false and requires `actor: maintainer` (or absent → maintainer; see R2.5).

R4.3.2 — Returns 200 with the updated issue + the new state-change comment. Returns 400 if the transition is illegal per `work-states.md#R6.2`; the response includes a `details.allowed_transitions` list.

R4.3.3 — Closing an issue is `POST /api/v1/issues/:id/transitions` with `to: "done"` or `to: "dropped"`.

### R4.4 — Blockers

| Method | Path | Operation |
|---|---|---|
| `PUT` | `/api/v1/issues/:id/blocker` | Set blocker; body `{ "blocker": "...", "comment": "..." }` |
| `DELETE` | `/api/v1/issues/:id/blocker` | Clear blocker; body `{ "comment": "..." }` |

### R4.5 — Priority

| Method | Path | Operation |
|---|---|---|
| `PUT` | `/api/v1/issues/:id/priority` | Set priority; body `{ "priority": "p0\|p1\|p2" }` |
| `DELETE` | `/api/v1/issues/:id/priority` | Clear priority |

R4.5.1 — Mirrors `dwarven issue priority`. By convention this is maintainer-only; the API does not enforce.

### R4.6 — Dependencies

| Method | Path | Operation |
|---|---|---|
| `GET` | `/api/v1/dependencies` | List all edges (full graph) |
| `GET` | `/api/v1/issues/:id/dependencies` | List edges for one issue (both incoming and outgoing) |
| `POST` | `/api/v1/dependencies` | Add edge; body `{ "from": <id>, "to": <id>, "rationale": "..." }` |
| `DELETE` | `/api/v1/dependencies/:from/:to` | Remove edge |

R4.6.1 — `POST` rejects cycles per `dwarven-cli.md#R6.11.3` with HTTP 409 and `details.cycle_path: [<ids>]`.

R4.6.2 — `from` blocks `to` (i.e., `to` is `blocked_by` `from`).

### R4.7 — Daemon

| Method | Path | Operation |
|---|---|---|
| `GET` | `/api/v1/daemon` | Status: PID, port, uptime, version, index health |
| `POST` | `/api/v1/daemon/shutdown` | Graceful shutdown |
| `POST` | `/api/v1/daemon/reindex` | Trigger full reindex (synchronous; large repos may block) |

### R4.8 — Config

| Method | Path | Operation |
|---|---|---|
| `GET` | `/api/v1/config` | Read full `.dwarven/config.toml` as JSON |
| `PATCH` | `/api/v1/config` | Merge-update keys |

R4.8.1 — Mutations to config that require a daemon restart (per `coordination-hub.md#R9.5`) include `requires_restart: true` in the response.

### R4.9 — Scheduler (v2 placeholder)

| Method | Path | Operation |
|---|---|---|
| `GET` | `/api/v1/scheduler/queue` | Ranked list of active issues by effective priority |
| `POST` | `/api/v1/scheduler/override` | Maintainer-asserted ranking override |

R4.9.1 — These endpoints are stubs in v1 (return 501 `not_implemented`). The full contract is specified in `dep-graph.md` and lands in v2.

---

## R5 — Real-time event stream

R5.1 — The daemon exposes a Server-Sent Events stream at `GET /api/v1/events`. Clients connect with `Accept: text/event-stream`.

R5.2 — Events are emitted on any change the daemon observes — whether triggered by the API itself, by CLI invocations, or by direct file edits picked up via the file watcher (`coordination-hub.md#R6.3`).

R5.3 — Event payload format:

```
event: issue.changed
data: {"id": 42, "kind": "state-change", "from": "plan", "to": "test", "actor": "plan", "ts": "2026-05-09T14:22:00.123Z"}

event: issue.created
data: {"id": 43, ...}

event: dependency.added
data: {"from": 42, "to": 50}
```

R5.4 — Event types in v1: `issue.created`, `issue.changed`, `issue.closed`, `comment.added`, `dependency.added`, `dependency.removed`, `daemon.reindexed`.

R5.5 — Each event carries enough information for the web UI to update its local view without re-fetching. Detailed payloads (full issue object) are not sent in events; the UI fetches when it needs detail.

R5.6 — Disconnected clients reconnect at will. The stream does not replay missed events in v1; on reconnect, the UI may need to refresh affected views. (A `Last-Event-ID`-based replay mechanism is a possible future extension.)

R5.7 — The stream uses the standard SSE retry mechanism (`retry: 5000\n\n`) for client-driven reconnection.

---

## R6 — Stability guarantees

R6.1 — Within a major version (v1.x of the API, equivalent to v2.x of the spec), endpoint paths, request shapes, response shapes, and error codes are stable. Field additions are minor; removals or renames are major.

R6.2 — The event-stream event type vocabulary (R5.4) is stable within a major version. New event types may be added.

R6.3 — Path prefix `/api/v1` remains responsive at minimum until `/api/v3` ships, providing one full major-version overlap.

---

## R7 — Out of scope for v1

R7.1 — **Authentication and authorization.** Local-only, single-trust-zone (R2.4). Multi-user or remote-access scenarios require a future spec.

R7.2 — **Pagination.** All list endpoints return full results in v1. At expected scale (hundreds of issues) this is fine; pagination is added when needed.

R7.3 — **Bulk operations.** No batch-create or batch-transition endpoints. Web UI performs these as N individual requests.

R7.4 — **Webhooks / outbound notifications.** Only the inbound SSE stream. Outbound delivery to external systems is a future adapter concern.

R7.5 — **GraphQL or alternative protocols.** REST-only in v1.

R7.6 — **Schema discovery (OpenAPI document).** Not produced in v1. The spec is the contract.

R7.7 — **Issue deletion (R4.1, DELETE).** Disallowed; use close+dropped. Hard deletion would only be for accidentally-created issues; manual file removal is the escape hatch.
