---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
target_release: v2
---

# Dependency Graph and Prioritization

This document specifies the dependency model and the dynamic prioritization algorithm. It is a **v2 deliverable**: the data model and edge creation surface land in v1 (already specified across `storage-model.md`, `dwarven-cli.md`, `web-api.md`), but the scheduler that consumes the graph and computes effective priority is v2.

The thesis (per `dwarven.md#R2.9` and the maintainer's stated motivation): priority should reflect not only the maintainer's product priorities but also each issue's *downstream-unblocking value* — how much other valuable work it enables. Working on a `p1` that unblocks five `p0`s should outrank a `p1` that unblocks nothing.

---

## R1 — Scope

R1.1 — This spec defines: the dependency graph's structure; the prioritization algorithm; the maintainer-override mechanism; and the scheduler's API surface.

R1.2 — Out of scope: the visualization of the dep graph (`web-ui.md#R7`); the on-disk format of edges (`storage-model.md#R3.4`); the CLI commands for edge management (`dwarven-cli.md#R6.11`); the HTTP endpoints (`web-api.md#R4.6`, `R4.9`).

---

## R2 — The dependency model

R2.1 — Edges are directed and carry the semantic *blocks*: edge `(i → j)` means "issue *i* blocks issue *j*". Equivalently: *j* is blocked by *i* and cannot proceed until *i* is in a terminal state (`done` or `dropped`).

R2.2 — Edges are stored in the frontmatter of *both* endpoints (`storage-model.md#R4.3.5–R4.3.6`): *i* lists *j* in its `blocks`, *j* lists *i* in its `blocked_by`. Asymmetric edges are flagged by the hub for triage (`storage-model.md#R4.3.6`).

R2.3 — The graph is a DAG. Cycle detection on edge creation rejects any addition that would close a cycle (`dwarven-cli.md#R6.11.3`).

R2.4 — Edges have no weight or type in v1. All edges are equal "blocks" relationships. Future extensions (soft-blocks, "should-precede" relationships) require a spec change.

R2.5 — Edges may carry an optional `rationale` recorded as a comment on the source issue when added (`dwarven-cli.md#R6.11.2`). The rationale does not affect prioritization; it is documentation.

---

## R3 — The prioritization algorithm

### R3.1 — Inputs

R3.1.1 — For each active issue *i* (not in a terminal state):
- *base_priority(i)*: a numeric weight derived from the maintainer-asserted `priority` field. Default mapping: `p0 → 4`, `p1 → 2`, `p2 → 1`, unset → `1`.
- *blocks(i)*: the set of issues *j* such that `(i → j)` is an edge.
- *override(i)*: the maintainer's explicit rank override, if set (R4).

R3.1.2 — The graph used for computation excludes terminal-state issues. An edge `(i → j)` where *j* is `done` or `dropped` is treated as removed.

### R3.2 — The score function

R3.2.1 — Define *score(i)* recursively:

```
score(i) = base_priority(i) + α · Σ_{j ∈ blocks(i)} score(j)
```

where α (the propagation factor) is a hub-configured constant. Default: `α = 0.5`.

R3.2.2 — Because the graph is a DAG (R2.3), the recursion terminates. The hub computes scores via a single topological sweep from sinks to sources.

R3.2.3 — *effective_priority(i)*:
- If *override(i)* is set: *effective_priority(i) = override(i)*.
- Otherwise: *effective_priority(i) = score(i)*.

### R3.3 — Interpretation

R3.3.1 — Higher *effective_priority* means higher rank. Sort descending to produce the scheduler's output queue.

R3.3.2 — Issues whose immediate prerequisites are not yet complete (i.e., issues with one or more `blocked_by` edges to non-terminal issues) appear in the queue but are flagged as "not yet actionable." The scheduler does not hide them — visibility into the upstream chain helps the maintainer reason about why something is ranked where it is.

R3.3.3 — Issues with `state: maintainer` or any active `blocker` are excluded from the scheduler's *agent dispatch* queue (no agent can work on them). They remain visible in the Inbox (`web-ui.md#R4`).

### R3.4 — Tunability

R3.4.1 — α is configurable in `.dwarven/config.toml` under `scheduler.alpha`. Permitted range: `[0.0, 1.0]`. Higher values mean downstream value dominates; lower values mean immediate base priority dominates.

R3.4.2 — The base-priority mapping is configurable under `scheduler.priority_weights` (e.g., `{p0=10, p1=2, p2=1}` to make `p0` overwhelm everything else). The hub validates that weights are positive and `p0 ≥ p1 ≥ p2`.

R3.4.3 — Configuration changes to `scheduler.*` take effect on the next scheduler computation; no daemon restart required.

---

## R4 — Maintainer override

R4.1 — The maintainer may set `effective_priority_override` on any issue's frontmatter to bypass the computed score. The override is an absolute number; the issue's rank is determined by this value directly (R3.2.3).

R4.2 — Overrides are set via `dwarven issue priority-override <id> <value>` (CLI extension landing in v2) or `POST /api/v1/scheduler/override` (`web-api.md#R4.9`).

R4.3 — Overrides are intended for "I know better than the algorithm" scenarios — typically for one-off urgent items or strategic bets the algorithm cannot see. They are not intended to be set on every issue; doing so reduces the scheduler to a manual ranker.

R4.4 — The web UI surfaces overrides visually (`web-ui.md#R8.3`) so the maintainer can audit what they've overridden.

R4.5 — Clearing an override (`dwarven issue priority-override <id> --clear`) restores algorithmic ranking.

---

## R5 — Scheduler API surface

R5.1 — The hub exposes the scheduler via the v2 endpoints:

- `GET /api/v1/scheduler/queue` — returns the ranked list of active issues with their *effective_priority*, *score*, *base_priority*, *override* (if any), and *blocked_by* status. Query params filter (e.g., `?state=plan` to see what Planning would pick up next).
- `POST /api/v1/scheduler/override` — sets an override.

R5.2 — The scheduler computation is performed on demand (per request) in v2. Caching with file-watcher invalidation may be added later.

R5.3 — `dwarven schedule next [--state=<state>]` (CLI extension in v2) is a thin wrapper that hits the queue endpoint and prints the top item (or top N with `--count <n>`).

---

## R6 — Edge creation conventions

R6.1 — Edges are created by:
- **Architect** when an architectural decision implies a sequencing constraint (e.g., "auth refactor must precede mobile login").
- **PM** when decomposing a parent into children with sequencing constraints among them.
- **Planning** when an implementation plan reveals a dependency on another in-flight piece.
- **Maintainer** at any time, via CLI or web UI.

R6.2 — Test Dev, Implementation, Code Review, and Doc must not create edges. If they discover a missing dependency, they escalate via `dialogue.md#R4` so the appropriate upstream agent (or the maintainer) can record it.

R6.3 — Triage may flag asymmetric edges (`storage-model.md#R4.3.6`) but does not create or remove edges directly. Asymmetries are reported in the triage chore for human resolution.

R6.4 — Edges should reflect *true* sequencing constraints — work that genuinely cannot proceed until the prerequisite is done. Avoid edges that merely express preference ("would be nicer to do X first"); those distort the ranking.

---

## R7 — Cycle handling

R7.1 — Cycle prevention is at edge creation (`dwarven-cli.md#R6.11.3`). The hub rejects edges that would close a cycle and reports the cycle path.

R7.2 — If a cycle slips in via direct file editing (the maintainer escape hatch — `storage-model.md#R6.2`), the next reindex (`coordination-hub.md#R3.3`) detects it. The hub logs the cycle and refuses to compute scores for any issue in the affected strongly-connected component until the cycle is resolved. Such issues are surfaced in the Inbox with a `cycle:detected` synthetic indicator.

R7.3 — Resolution is the maintainer's job: remove one edge in the cycle. There is no automatic resolution.

---

## R8 — Out of scope for v2

R8.1 — **Edge weights or types.** All edges are uniform "blocks" in v2 (R2.4). Soft-blocks, ordering hints, and other relationships are future spec work.

R8.2 — **Effort estimates and parallelism modeling.** The scheduler does not consider issue size or how many agents could work in parallel. Output is a strict total order; the maintainer or future automation chooses what to dispatch.

R8.3 — **Multi-objective optimization.** The single *effective_priority* number conflates "important" and "unblocking." Pareto-style multi-objective ranking is out of scope.

R8.4 — **Learning / adaptive weighting.** α and base-priority weights are hub-configured constants; they do not adapt based on outcomes. Hand-tunable, not learned.

R8.5 — **Dependency on external systems.** Edges connect Dwarven issues only. A dependency on an upstream library version, vendor delivery, etc. is captured as `blocker: external` or `blocker: upstream` (`work-states.md#R4.2`), not as a graph edge.

R8.6 — **Auto-dispatch from the scheduler.** The scheduler ranks; it does not dispatch. Auto-dispatch on schedule events is future automation work and ties into detached dispatch (`host-adapter.md#R2.9`).
