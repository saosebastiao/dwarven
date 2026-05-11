# Web UI

A screen-by-screen walkthrough of the local web UI served by
`dwarven serve`. Default URL: `http://127.0.0.1:7777`.

This is the user-facing reference for the shipped UI. The spec — what
each screen *must* do — lives at [`docs/specs/web-ui.md`](specs/web-ui.md).

## Conventions

- **Hash routing.** The UI is a single-page app; screens are switched
  by URL hash (`#/inbox`, `#/issues`, `#/deps`, `#/schedule`, `#/daemon`,
  `#/config`). The hash is the canonical view state — filters and
  collapsed clusters are encoded there, so URLs are shareable.
- **Real-time.** The UI subscribes to the SSE event stream and rerenders
  on relevant events. Mutating from the CLI shows up within a tick.
- **Connection indicator.** A status pill in the top nav reflects the
  SSE connection. "connecting…" means no live stream is attached yet;
  "live" means events are flowing.
- **Terminal-state read-only.** When viewing an issue in `done` or
  `dropped`, mutation forms hide; the issue is shown for archive only.

The nav has six top-level entries: **Inbox**, **Issues**, **Deps**,
**Schedule**, **Daemon**, **Config**.

---

## Inbox

`#/inbox` — the landing screen.

**Purpose:** what currently needs the maintainer's attention next, in
ranked order. Active issues whose blocker is `maintainer-input`, plus
issues in maintainer-owned states (per `work-states.md#R3`), surface
here.

**What to do here:** triage the top item. Open it (click the row),
read the latest comment, and either resolve the blocker
(`Clear blocker` → unblocks the agent that was waiting) or transition
it forward yourself.

**When empty:** "Nothing waiting on you." — the agents are unblocked and
the scheduler has work for them.

**Common pitfall:** a stale `maintainer-input` blocker can linger after
you replied. If you commented but didn't clear the blocker, the inbox
still shows the issue. Clear the blocker explicitly.

---

## Issues

`#/issues` — full list with filters.

**Filter bar** (each persists in the URL):

| Filter | Effect |
|---|---|
| State (multi) | Comma-separated state values. |
| Type (multi) | Comma-separated type values. |
| Priority (multi) | `p0`, `p1`, `p2`, or `unset`. |
| Epic | One epic slug. |
| Grep | Literal-string match across title and body. |
| Open/Closed/All | Terminal-state inclusion. |

**Sort:** by `updated` (default), `id`, `created`, or `priority`.

Click a row to open the issue detail. The "New issue" button opens an
inline form (same fields as `dwarven issue create`).

**URL persistence:** every filter goes into the hash, so you can share
a URL like `#/issues?state=plan,test&priority=p0,p1`.

---

## Issue detail

`#/issues/<id>` — single-issue view, with the full mutation surface.

**Header:** id, title, type, state, priority, blocker, epic. The blocks /
blocked-by lists are clickable links to the related issues.

**Body** rendered as Markdown.

**Comment thread:** in chronological order. State-change comments are
visually distinct; you can filter the thread to state-changes only.

**Mutation forms** appear inline as long as the issue is non-terminal:

| Form | What it does |
|---|---|
| Comment | Append a comment. Actor defaults to `maintainer`. |
| Transition | Pick a target state; required `comment` when targeting `done` or `dropped`. Validated against the graph; `override` is offered as a checkbox (maintainer-only). |
| Close | Convenience shortcut that targets `done` or `dropped` with a closure comment. |
| Set / clear blocker | Pick `maintainer-input`, `external`, or `upstream`. |
| Set / clear priority | Pick `p0`/`p1`/`p2`. |
| Edit frontmatter | Title, type, epic. |
| Add dep | "this blocks #N" or "#N blocks this" with an optional rationale. |
| Remove dep | One-click on the dependency edge. |

**Real-time refresh:** any change to this issue (from CLI, another tab,
or another agent) rerenders the screen.

**Common pitfall:** if you start typing in a form and the issue gets
mutated elsewhere, the form is preserved on re-render — but your
text-in-progress may briefly flash. Submit promptly or copy your text.

---

## Deps

`#/deps` — the dependency DAG, grouped by epic.

**Visual encoding:**

- **Node color** = state (one color per state from `work-states.md`).
- **Node size** = priority (p0 largest).
- **Blocker dot** = small marker if the issue has any non-`none` blocker.
- **Edge** = "from blocks to."

**Layered layout:** roots (no `blocked_by`) at the top; nodes flow down.

**Epic clustering:** issues with the same `epic` are wrapped in a
translucent rectangle with a clickable label. Click the label to
collapse the cluster — the cluster becomes a single placeholder node
showing `<epic> (<N>)`; in-cluster edges are dropped; in/out edges are
redirected to the placeholder. URL state: `#/deps?collapsed=alpha,beta`.

**Focus + N-hop:** click a node to focus it. The view filters to that
node plus its N-hop neighborhood (default N = 2). Adjust N with the
hop-distance control, or clear focus to see the whole graph.

**Common pitfall:** a dense graph can look chaotic. Collapse the epics
you aren't actively working in, or focus a single node.

---

## Schedule

`#/schedule` — the dep-graph scheduler's ranked queue.

**Columns:** id, state, title, base score, downstream score, total
score, override, effective rank. Rows are sorted by effective rank.

**Override controls:** click the override cell on any row to enter or
clear an absolute override. Active overrides are visually distinct so
you can find what you've pinned.

**Actionable filter:** toggle "actionable only" to show issues whose
`blocked_by` is empty.

**State filter:** restrict to issues currently in a given state (e.g.
`plan` to find the next planning candidate).

**Common pitfall:** an override pinning an issue at the top doesn't
unblock it. If the issue is `blocked_by` something open, the scheduler
still considers it not-actionable.

---

## Daemon

`#/daemon` — status and admin.

**Status fields:**
- Process: PID, port, uptime.
- Index: last reindex outcome (healthy / corrupt / mismatch / missing)
  and number of issues / dependencies indexed.
- SSE buffer: current occupancy of the ring buffer.
- Version.

**Buttons:**
- **Reindex** — drops and rebuilds the SQLite index from files. Use
  after manual file surgery to `.dwarven/issues/`.
- **Shutdown** — graceful exit. You will need to start the daemon
  again from a terminal.

**Common pitfall:** "Shutdown" does what it says. There is no
"restart" button in the UI; restart from the terminal with
`dwarven daemon restart` (which can be run from outside the now-stopped
session).

---

## Config

`#/config` — read and edit `.dwarven/config.toml`.

**Form view:** every config section is rendered as a labeled form.
Restart-required keys (`daemon.port`, `daemon.bind`) are visually
flagged. Editing them writes to file but the running daemon continues
on the old value until restart.

**Raw view:** toggle to see the file as TOML for direct editing. Saves
go through the same validation; an invalid value returns an error
without writing.

See [`docs/configuration.md`](configuration.md) for every key, its
default, and whether changing it requires a restart.

---

## What's not in the UI

Some operations are CLI-only by design (per `dwarven.md#R2.10`):

- `--actor` attribution for non-maintainer mutations (agents do not use
  the UI).
- Initial repo setup (`dwarven init`).
- Host adapter installation (`dwarven init --host …`).

Use the CLI for these and the UI for triage, browsing, and ad-hoc
adjustments.
