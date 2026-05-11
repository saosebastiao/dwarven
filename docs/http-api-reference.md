# HTTP API reference

Endpoint catalog for the local coordination hub's HTTP API. Bound by
default at `http://127.0.0.1:7777` while `dwarven serve` is running.

The formal contract is [`docs/specs/web-api.md`](specs/web-api.md). This
doc is the user-facing reference with realistic request/response payloads.

## Conventions

- **Base URL:** `http://127.0.0.1:<port>` where `<port>` comes from
  `daemon.port` in [`docs/configuration.md`](configuration.md).
- **API prefix:** `/api/v1/`.
- **Authentication:** none. v1 is local-only; do not expose the daemon
  on a non-loopback interface without your own auth layer.
- **Content type:** request bodies and responses are JSON unless noted.

### Error envelope

Every non-2xx response uses:

```json
{
  "error": "<machine code>",
  "message": "<human-readable detail>",
  "details": null
}
```

| HTTP status | `error` code | Typical cause |
|---|---|---|
| 400 | `bad_request` | Validation failure (illegal transition, missing required body field). |
| 404 | `not_found` | Issue id, dependency edge, or comment not found. |
| 500 | `hub_error` | Unexpected hub-side IO or storage error. |

### Actor attribution

Mutation endpoints accept an optional `actor` field on the body
(`"actor": "spec"` etc.). If omitted, attribution falls back to
`"maintainer"`. The CLI's `--actor` flag and the web UI both feed this
field.

---

## Daemon

### `GET /api/v1/daemon`

Liveness + version info.

```bash
curl http://127.0.0.1:7777/api/v1/daemon
```

```json
{
  "state": "running",
  "pid": 51234,
  "port": 7777,
  "uptime_seconds": 1284,
  "version": "0.1.0"
}
```

### `POST /api/v1/daemon/shutdown`

Asks the daemon to exit cleanly. The HTTP response is sent before the
server begins teardown.

```bash
curl -X POST http://127.0.0.1:7777/api/v1/daemon/shutdown
```

### `POST /api/v1/daemon/reindex`

Drops and rebuilds the SQLite index from the on-disk files.

```bash
curl -X POST http://127.0.0.1:7777/api/v1/daemon/reindex
```

Useful after direct file edits to `.dwarven/issues/`.

---

## Issues

### `GET /api/v1/issues`

List issues. Supports the same filters as `dwarven issue list`.

| Query param | Effect |
|---|---|
| `state=<csv>` | Filter to one or more states. |
| `type=<csv>` | Filter by type. |
| `blocker=<csv>` | Filter by blocker value. |
| `priority=<csv>` | Filter by priority (use `unset` for no-priority). |
| `epic=<slug>` | Single epic. |
| `grep=<str>` | Literal-string match against title and body. |
| `closed=true` | Include closed (terminal-state) issues. |
| `all=true` | Include all states; supersedes `closed`. |

```bash
curl 'http://127.0.0.1:7777/api/v1/issues?state=plan,test&priority=p0,p1'
```

```json
[
  {
    "id": 12,
    "title": "docs/getting-started.md: quickstart walkthrough",
    "type": "doc",
    "state": "done",
    "priority": "p1",
    "blocker": null,
    "epic": "user-docs",
    "blocks": [],
    "blocked_by": [],
    "created": "2026-05-11T05:16:26Z",
    "updated": "2026-05-11T05:19:00Z"
  }
]
```

### `POST /api/v1/issues`

Create an issue.

```bash
curl -X POST http://127.0.0.1:7777/api/v1/issues \
  -H 'Content-Type: application/json' \
  -d '{
    "type": "feature",
    "title": "rename Export to Share",
    "priority": "p1",
    "epic": "v2-launch",
    "body": "Marketing copy update.",
    "blocked_by": [],
    "blocks": []
  }'
```

`201 Created` with the full issue record. The created issue includes its
assigned `id`.

### `GET /api/v1/issues/:id`

Fetch a single issue with its frontmatter and body.

```bash
curl http://127.0.0.1:7777/api/v1/issues/12
```

`404 not_found` if the issue does not exist.

### `PATCH /api/v1/issues/:id`

Edit low-churn frontmatter (`title`, `type`, `epic`). All fields
optional; omitted fields are unchanged.

```bash
curl -X PATCH http://127.0.0.1:7777/api/v1/issues/12 \
  -H 'Content-Type: application/json' \
  -d '{"title": "new title", "epic": "v2-launch"}'
```

### Issue comments

#### `GET /api/v1/issues/:id/comments`

```bash
curl http://127.0.0.1:7777/api/v1/issues/12/comments
```

Optional `?kind=state-change` filters to state-change records.

#### `POST /api/v1/issues/:id/comments`

```bash
curl -X POST http://127.0.0.1:7777/api/v1/issues/12/comments \
  -H 'Content-Type: application/json' \
  -d '{"body": "Spotted a regression.", "actor": "review"}'
```

Returns the full comment thread (so the caller does not need a separate
fetch).

### Transitions

#### `POST /api/v1/issues/:id/transitions`

Move an issue to a new state. Validated against the work-states graph.

```bash
curl -X POST http://127.0.0.1:7777/api/v1/issues/12/transitions \
  -H 'Content-Type: application/json' \
  -d '{"to": "plan", "comment": "PM accepted scope.", "actor": "pm"}'
```

| Body field | Type | Required? | Notes |
|---|---|---|---|
| `to` | string | yes | Target state. Pass `done` or `dropped` to close. |
| `comment` | string | optional | Rationale; recorded on the state-change comment. Required when `to` is terminal. |
| `override` | bool | optional | Force a transition not in the graph. Maintainer-only; cannot target terminal states. |
| `actor` | string | optional | Attribution. |

`400 bad_request` on illegal transition (e.g., `spec → review`) unless
`override: true`.

### Blocker

#### `PUT /api/v1/issues/:id/blocker`

Set a blocker.

```bash
curl -X PUT http://127.0.0.1:7777/api/v1/issues/12/blocker \
  -H 'Content-Type: application/json' \
  -d '{"blocker": "maintainer-input", "comment": "Need design approval."}'
```

#### `DELETE /api/v1/issues/:id/blocker`

Clear the blocker. Optional body `{"comment": "Resolved."}`.

```bash
curl -X DELETE http://127.0.0.1:7777/api/v1/issues/12/blocker
```

### Priority

#### `PUT /api/v1/issues/:id/priority`

```bash
curl -X PUT http://127.0.0.1:7777/api/v1/issues/12/priority \
  -H 'Content-Type: application/json' \
  -d '{"priority": "p1"}'
```

#### `DELETE /api/v1/issues/:id/priority`

Clears the asserted priority (returns the issue to "unset").

```bash
curl -X DELETE http://127.0.0.1:7777/api/v1/issues/12/priority
```

---

## Dependencies

### `GET /api/v1/dependencies`

All dependency edges, repo-wide.

```bash
curl http://127.0.0.1:7777/api/v1/dependencies
```

```json
[
  {"from": 5, "to": 7, "rationale": "7 imports the API surface 5 defines."}
]
```

### `GET /api/v1/issues/:id/dependencies`

Edges touching this issue (both directions).

```bash
curl http://127.0.0.1:7777/api/v1/issues/7/dependencies
```

### `POST /api/v1/dependencies`

Add an edge.

```bash
curl -X POST http://127.0.0.1:7777/api/v1/dependencies \
  -H 'Content-Type: application/json' \
  -d '{"from": 5, "to": 7, "rationale": "7 depends on 5."}'
```

`400 bad_request` if adding the edge would create a cycle (the message
identifies the cycle).

### `DELETE /api/v1/dependencies/:from/:to`

Remove an edge.

```bash
curl -X DELETE http://127.0.0.1:7777/api/v1/dependencies/5/7
```

---

## Scheduler

### `GET /api/v1/scheduler/queue`

Ranked queue from the dep-graph scheduler.

| Query param | Effect |
|---|---|
| `state=<csv>` | Restrict to issues in these states. |
| `actionable=true` | Only return issues whose `blocked_by` is empty. |
| `count=<n>` | Limit. |

```bash
curl 'http://127.0.0.1:7777/api/v1/scheduler/queue?count=5&actionable=true'
```

```json
[
  {
    "id": 7,
    "state": "plan",
    "title": "...",
    "priority": "p1",
    "base_score": 2.0,
    "downstream_score": 1.0,
    "score": 2.5,
    "override": null,
    "effective_rank": 2.5,
    "actionable": true
  }
]
```

### `POST /api/v1/scheduler/override`

Set or clear the absolute scheduler override on an issue. Body must
have exactly one of `value` or `clear: true`.

```bash
# Pin to the top
curl -X POST http://127.0.0.1:7777/api/v1/scheduler/override \
  -H 'Content-Type: application/json' \
  -d '{"issue": 12, "value": 100}'

# Restore algorithmic ranking
curl -X POST http://127.0.0.1:7777/api/v1/scheduler/override \
  -H 'Content-Type: application/json' \
  -d '{"issue": 12, "clear": true}'
```

---

## Config

### `GET /api/v1/config`

Return the validated config as JSON.

```bash
curl http://127.0.0.1:7777/api/v1/config
```

### `PATCH /api/v1/config`

Partial update. Keys not in the body are unchanged. Validated server-side;
restart-required keys (e.g. `daemon.port`) are accepted into the file but
the running daemon continues with the old value until restart.

```bash
curl -X PATCH http://127.0.0.1:7777/api/v1/config \
  -H 'Content-Type: application/json' \
  -d '{"scheduler": {"alpha": 0.7}}'
```

See [`docs/configuration.md`](configuration.md) for every key and its
restart-required flag.

---

## Events (SSE)

### `GET /api/v1/events`

Server-Sent Events stream. The daemon emits an event each time the file
watcher observes a change.

```bash
curl -N http://127.0.0.1:7777/api/v1/events
```

```
id: 142
event: issue.changed
data: {"id":12,"kind":"transition","to":"plan"}

id: 143
event: comment.added
data: {"issue":12,"seq":3}
```

### Event vocabulary

| Event name | Payload | Emitted when |
|---|---|---|
| `issue.created` | `{"id":<n>}` | A new issue is filed. |
| `issue.changed` | `{"id":<n>,"kind":"<transition\|edit\|blocker-set\|blocker-cleared\|priority-set\|priority-cleared>", ...}` | The issue's frontmatter changed. |
| `issue.closed` | `{"id":<n>,"to":"<done\|dropped>"}` | Terminal transition. |
| `comment.added` | `{"issue":<n>,"seq":<n>}` | A comment was appended. |
| `dependency.added` | `{"from":<n>,"to":<n>}` | A new edge. |
| `dependency.removed` | `{"from":<n>,"to":<n>}` | An edge removed. |
| `daemon.reindexed` | `{}` | The SQLite index was rebuilt (file watcher reconcile or explicit reindex). |
| `stream.refresh-required` | `{}` | The client's `Last-Event-ID` is older than the ring buffer; refetch state. |

### Last-Event-ID replay

On reconnect, clients should send `Last-Event-ID: <n>` with the most
recent seq id received. The daemon replays any events from a 256-entry
ring buffer with `id > n` before attaching the live stream. If `n` is
older than the buffer's oldest entry, the daemon emits a single
`stream.refresh-required` event so the client knows to refetch state
from scratch instead of relying on incremental events.

```bash
curl -N -H 'Last-Event-ID: 140' http://127.0.0.1:7777/api/v1/events
```

---

## Quick recipes

### Walk a fresh issue through the pipeline

```bash
# Create
ID=$(curl -s -X POST http://127.0.0.1:7777/api/v1/issues \
  -H 'Content-Type: application/json' \
  -d '{"type":"feature","title":"demo","priority":"p2"}' | jq -r .id)

# Advance through states
for state in pm plan test implement review doc; do
  curl -X POST http://127.0.0.1:7777/api/v1/issues/$ID/transitions \
    -H 'Content-Type: application/json' \
    -d "{\"to\":\"$state\"}"
done

# Close
curl -X POST http://127.0.0.1:7777/api/v1/issues/$ID/transitions \
  -H 'Content-Type: application/json' \
  -d '{"to":"done","comment":"Demo complete."}'
```

### Tail the event stream into a script

```bash
curl -N http://127.0.0.1:7777/api/v1/events | while read line; do
  echo "$line"
  # parse + react...
done
```
