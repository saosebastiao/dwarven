---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Web UI

This document specifies the web UI's screens, flows, and interaction model. The web UI is the maintainer's primary visual surface for the coordination hub (`coordination-hub.md`); it consumes the local HTTP API (`web-api.md`).

This spec defines what the UI *does* — which screens exist, what each screen surfaces, what mutations the maintainer can perform from each. It does not prescribe styling, framework, or layout choices; those are an architecture/implementation concern.

---

## R1 — Scope

R1.1 — This spec defines: the UI's top-level navigation; per-screen content and operations; the real-time-update behavior; and the input flows for the maintainer's two most common tasks (responding to detached agent dialogue, and reranking work).

R1.2 — Out of scope: visual design (colors, typography, layout); the JavaScript framework or rendering approach; keyboard shortcuts (a future spec extension); accessibility audits; mobile or small-screen layouts.

R1.3 — The UI is the maintainer's surface only. Agents do not consume the UI (`dwarven.md#R2.10`).

---

## R2 — Deployment

R2.1 — UI assets are embedded in the daemon binary at compile time and served from `/` (`coordination-hub.md#R5.2`). The maintainer opens `http://127.0.0.1:<port>/` in a browser.

R2.2 — The UI is a single-page application. Client-side routing handles per-screen navigation; the daemon serves `index.html` for any unknown path under `/` so deep links work.

R2.3 — The UI assumes a single browser session per maintainer per repo. Concurrent browser sessions on the same daemon are functional but not coordinated (no cursor sharing, no edit-conflict resolution beyond what the API provides).

---

## R3 — Top-level navigation

R3.1 — The UI exposes the following top-level screens, accessible from a persistent navigation surface:

| Screen | Purpose |
|---|---|
| Inbox | Items awaiting maintainer attention (`state: maintainer`, `blocker: maintainer-input`, etc.) |
| Issues | Full filterable issue list |
| Issue detail | One issue's full state, body, comments, history (linked from anywhere an issue is named) |
| Dependencies | Graph view of the dependency network |
| Schedule | Ranked queue by effective priority (v2) |
| Daemon | Daemon status, reindex, shutdown |
| Config | View/edit `.dwarven/config.toml` |

R3.2 — Inbox is the default landing screen. Rationale: the maintainer's most common entry point is "what needs my attention?"

R3.3 — Every screen surfaces a global "create issue" affordance.

---

## R4 — Inbox screen

R4.1 — The Inbox lists all issues where the maintainer is the next required actor:

- Issues with `state: maintainer`;
- Issues with any `blocker:*` set, regardless of state;
- Issues whose latest comment author is an agent in the previous 24 hours and whose state has not changed since (i.e., agent has spoken and is awaiting acknowledgment).

R4.2 — Each row shows: ID, title, type, state, blocker (if any), priority, latest-comment timestamp, latest-comment author.

R4.3 — Clicking a row navigates to the Issue Detail screen.

R4.4 — The Inbox surfaces a "blocker count" badge in the navigation, updated in real time via the SSE stream (`web-api.md#R5`).

R4.5 — Inbox sort: blockers (`maintainer-input` first), then by latest-comment age (oldest first — the maintainer should not let things rot).

---

## R5 — Issues screen

R5.1 — The Issues screen is the full filterable issue list, with the same query semantics as `dwarven issue list` (`dwarven-cli.md#R6.4`).

R5.2 — Filters available: state (multi), type (multi), blocker (multi), priority (multi), epic, open/closed/all, free-text search (titles + bodies).

R5.3 — Sortable columns: ID, title, state, type, priority, created, updated.

R5.4 — Filter and sort state is reflected in the URL (query parameters) so views are shareable and back/forward navigation works.

R5.5 — Bulk-action support: none in v1 (`web-api.md#R7.3`). Each mutation is per-issue.

---

## R6 — Issue detail screen

R6.1 — Single screen showing: frontmatter (title, type, state, priority, blocker, epic, created, updated, blocked_by, blocks); body (rendered Markdown); chronological comment thread (with state-change comments interleaved and visually distinct).

R6.2 — Operations available from this screen:

| Action | Notes |
|---|---|
| Post comment | Markdown editor; submits via `POST /comments` |
| Transition state | Dropdown of legal targets per `work-states.md#R6.2`; with override toggle for maintainer |
| Set/clear blocker | Dropdown of values per `work-states.md#R4.2` |
| Set/clear priority | `p0` / `p1` / `p2` / clear |
| Edit title / type / epic | Inline edit; submits via `PATCH /issues/:id` |
| Add/remove dep edge | "blocks" picker showing other issue IDs/titles |
| Close (done / dropped) | Terminal transition with required closure comment |

R6.3 — The dependency section shows both directions: "Blocks" (this issue blocks N others) and "Blocked by" (this issue is blocked by N others). Each linked issue is clickable.

R6.4 — Real-time updates: if another actor (CLI, file edit) modifies the displayed issue, the UI receives an SSE event (`web-api.md#R5.3`) and updates the displayed state without page reload. If the maintainer has unsaved input (e.g., a partially-typed comment), the UI surfaces a non-disruptive notice rather than overwriting.

R6.5 — The state-history section is a filtered comment view (`kind: state-change` only), rendered as a vertical timeline.

---

## R7 — Dependencies screen

R7.1 — A graph visualization of the dependency network. Nodes are issues; directed edges are "A blocks B" relationships.

R7.2 — Default view: subgraph filtered to active issues only (terminal-state issues hidden, with toggle to include).

R7.3 — Node visual encoding: state (color or shape), priority (size or border weight), and `blocker` presence (badge).

R7.4 — Cluster grouping: nodes are grouped by `epic` when set; clusters can be collapsed.

R7.5 — Selecting a node opens an inline panel with the issue summary and a deep-link to its detail screen.

R7.6 — Edge management (add/remove) is performed from the detail screen (R6.2), not from the graph view in v1.

R7.7 — At scale beyond what the layout can handle clearly, the UI offers a focused-on-issue mode (show only the selected issue and its N-hop neighborhood). This is the primary navigation pattern at scale.

---

## R8 — Schedule screen (v2)

R8.1 — A ranked list of active issues by effective priority (computed per `dep-graph.md`).

R8.2 — Each row shows: rank, ID, title, state, maintainer-priority, downstream-unblocking value, effective priority, override (if set).

R8.3 — Maintainer override: per-row affordance to assert a custom rank (R4.5 of `web-api.md`). Overrides are recorded and surfaced visually.

R8.4 — Ranking explanation: clicking a rank reveals the inputs that produced it (which dependencies, which downstream issues counted, etc.).

R8.5 — In v1 this screen renders an "available in v2" placeholder with a link to `dep-graph.md`.

---

## R9 — Daemon screen

R9.1 — Shows: PID, port, uptime, hub binary version, index file size, last reindex timestamp, file-watcher health, count of currently-connected SSE clients.

R9.2 — Operations: trigger reindex; graceful shutdown.

R9.3 — Reindex shows a progress indicator; the UI remains responsive (read-only) while reindexing.

R9.4 — Shutdown disconnects all clients with a "daemon stopped" notice and stops the SSE stream.

---

## R10 — Config screen

R10.1 — Renders `.dwarven/config.toml` as a form, with current values pre-populated.

R10.2 — Fields requiring a daemon restart (per `coordination-hub.md#R9.5`) are flagged in the UI; saving them triggers a confirmation dialog and reports `requires_restart: true` from the API.

R10.3 — The raw TOML is also viewable in a read-only code view for advanced inspection.

---

## R11 — Real-time updates

R11.1 — On load, every screen that displays issue data subscribes to the SSE event stream (`web-api.md#R5`).

R11.2 — On `issue.created`, `issue.changed`, `issue.closed`, `comment.added`: the UI updates the affected list rows or detail view in place.

R11.3 — On `dependency.added` / `dependency.removed`: the Dependencies screen updates the graph; detail screens for either endpoint refresh their dependency sections.

R11.4 — On `daemon.reindexed`: the UI refreshes all visible queries.

R11.5 — Disconnection: the SSE retry mechanism (`web-api.md#R5.7`) handles transient drops. Sustained disconnect (>10s) surfaces a persistent banner indicating the UI may be stale; the banner clears on reconnect.

---

## R12 — Out of scope for v1

R12.1 — **Authentication and per-user views.** Single-maintainer model.

R12.2 — **Mobile / small-screen layouts.** Desktop browser only in v1.

R12.3 — **Spec / architecture / plan editing in the UI.** These are file-based docs edited in the maintainer's editor. The UI may render previews of linked spec/plan paths but does not host an editor for them.

R12.4 — **Custom dashboards or saved views.** Filter state is URL-shareable (R5.4) but there's no named "saved view" concept.

R12.5 — **Comment editing.** Comments are append-only via the UI in v1. To revise, post a follow-up comment. (The CLI does not enforce immutability either — direct file edits remain the escape hatch — but the UI does not surface an edit affordance.)

R12.6 — **Issue templates.** Free-form body composition only (`dwarven-cli.md#R9.5`).

R12.7 — **Notifications outside the browser.** No system notifications, email, etc. The Inbox screen is the notification surface.
