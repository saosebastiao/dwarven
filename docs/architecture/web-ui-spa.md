---
spec_ref: web-ui.md, coordination-hub.md
date: 2026-05-10
issue: 8
---

# Web UI single-page-app architecture

How the embedded web UI is built and shipped. Implementation companion to `docs/specs/web-ui.md` (which specifies *what* the screens do; this doc captures *how*).

## No framework, no build step

The UI is plain `index.html` + `app.css` + `app.js` (≈700 LoC of vanilla JS as of this writing). No React, no Svelte, no bundler. The decision is deliberate:

- **Zero build pipeline.** No `package.json`, no `node_modules/`, no transpile step. `cargo build` produces the entire artifact.
- **Embedded via `include_str!`** at compile time. The daemon binary ships the UI in its `.rodata` section; the bytes never leave the binary.
- **No supply-chain surface.** The UI carries zero JavaScript dependencies. Browser APIs (fetch, EventSource, URLSearchParams, ES2017) are the entire surface.

The cost is verbosity: rendering issue lists is `template-literal + map`, not JSX. At the scale we have (a half-dozen screens, ~700 lines), this is fine; if the UI grows past ~2000 lines a framework starts paying off.

## Embedding via `include_str!`

`src/api/web.rs` references each asset:

```rust
const INDEX_HTML: &str = include_str!("../../assets/web/index.html");
const APP_JS: &str = include_str!("../../assets/web/app.js");
const APP_CSS: &str = include_str!("../../assets/web/app.css");
```

axum routes wrap them in `Response` objects with the right `Content-Type`. There is no filesystem read at runtime — the assets are static `&'static str` baked into the binary at compile time. Editing the assets requires a rebuild; this is acceptable because we don't develop the UI iteratively (no hot reload).

## SPA fallback for client-side routes

The daemon serves `/` (the index), `/app.js`, and `/app.css` directly. Any other path that doesn't start with `/api/` falls back to serving `index.html` (the SPA shell), per `web-ui.md#R2.2`. This makes client-side deep links work: `http://127.0.0.1:7777/issues/42` reaches `index.html`, which then reads `window.location.hash` and renders the issue detail.

Paths under `/api/` are NOT subject to the SPA fallback — they return a JSON 404 envelope if unmatched, so typos against the API don't get papered over with HTML.

## Hash-based routing

Routes live in `window.location.hash`, parsed by `parseHash()`:

```
#/inbox
#/issues?state=plan&priority=p0
#/issues/42
#/deps?focus=42&hops=2
#/schedule
#/config
#/daemon
```

The query-string portion of the hash carries filter / focus state. Two consequences:

- **Shareable URLs.** Copy-paste a hash into a browser; the same view renders.
- **Back/forward navigation works.** Each `setHash()` call pushes onto the history stack; the `hashchange` listener re-renders on pop.

`setHash(route, params)` is the only internal API that mutates the hash. Filter forms call it with a fresh `URLSearchParams`; clicking an issue row calls `window.location.hash = #/issues/${id}` (deliberately without `setHash` to keep the issue-detail route param-free).

## State management

There is no client-side store. Each route renderer fetches what it needs from the HTTP API on entry. The cost is one round-trip per navigation; the benefit is no stale-data bugs.

The one piece of cross-render state is the **SSE subscription**. The connection is established once on page load (`connectSSE()`) and persists across navigations. On any event of interest, the active renderer re-runs and re-fetches. This is the entire real-time-update mechanism per `web-ui.md#R11`.

The Config screen explicitly opts *out* of SSE-driven re-render — a maintainer editing config doesn't want their unsaved input clobbered by a competing actor's change. This is a per-screen choice in the `refresh()` callback inside `connectSSE()`.

## SSE replay protocol (post-issue #3)

The browser's `EventSource` natively tracks the latest `id:` field across events and resends it as `Last-Event-ID` on automatic reconnect. The server's replay protocol (issue #3, `web-api.md#R5.6`) means brief disconnects don't lose events:

- Disconnect for < 5s of activity: server replays from ring buffer, EventSource transparently catches up.
- Disconnect long enough to evict events: server emits a `stream.refresh-required` event with the oldest available seq; the client refreshes affected views from the API.

The UI's event handler treats `stream.refresh-required` identically to other change events: re-run the active renderer. The renderer's fresh fetch sees the post-disconnect state correctly.

## Dependency graph (SVG layered DAG)

The Deps screen renders nodes and edges as a single inline `<svg>` element, no graph library. Layout is layered — nodes assigned to layers by longest-path-from-source — and the SVG is drawn top-to-bottom (sources at the top, sinks at the bottom; edges point downward).

Why hand-rolled: the visualization is simple (small DAG, < ~100 nodes typical), the layout algorithm is straightforward, and adding a graph library (`vis-network`, `cytoscape`) would force either a build step or a runtime CDN dependency. The hand-rolled version is ~250 lines including all interaction.

Node visual encoding follows `web-ui.md#R7.3`: state → fill color, priority → circle radius (p0 largest), blocker presence → orange dot badge. Click a node → inline panel below the graph with summary + deep link to detail.

Focus mode (`?focus=42&hops=2`) filters the graph to an N-hop neighborhood of a chosen issue. Both incoming and outgoing edges are walked.

Deferred: epic-clustered grouping (`web-ui.md#R7.4`) is filed as issue #9.

## Mutation UI

The issue detail screen exposes seven collapsible `<details>` sections, one per mutation type (comment, transition, close, blocker, priority, edit, dep). Each form maps to an existing HTTP endpoint:

| UI form | HTTP endpoint |
|---|---|
| Comment | `POST /issues/:id/comments` |
| Transition | `POST /issues/:id/transitions` |
| Close | `POST /issues/:id/transitions` (`to: done|dropped`) |
| Blocker set / clear | `PUT|DELETE /issues/:id/blocker` |
| Priority set / clear | `PUT|DELETE /issues/:id/priority` |
| Edit | `PATCH /issues/:id` (only changed fields sent) |
| Dep add / remove | `POST /dependencies` / `DELETE /dependencies/:from/:to` |

On success, the renderer re-fetches the issue and the comment thread and re-renders. Errors surface via `alert()`. Terminal-state issues hide the actions section entirely.

## Tradeoffs and known limits

- **No optimistic UI.** Every mutation waits for the server round-trip before updating the view. At local-network latencies this is imperceptible; over a future remote API it would be.
- **No client-side validation beyond required-field.** Title length, state transitions, and edge cycles are validated server-side and surfaced via HTTP error messages.
- **No accessibility audit.** Keyboard navigation works for forms and links but isn't tested. `web-ui.md#R1.2` explicitly defers this.
- **Desktop layout only.** Mobile breakpoints aren't implemented (`web-ui.md#R12.2`).

These are spec-aligned omissions, not oversights.
