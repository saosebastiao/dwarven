---
spec_ref: dep-graph.md
date: 2026-05-10
issue: 8
---

# Dependency-graph scheduler

Implementation of `docs/specs/dep-graph.md`. The spec gives the score function and the queue's API surface; this doc captures the implementation choices, the motivating example, and the maintainer-override mechanics.

## The score function

```
score(i) = base_priority(i) + α · Σ_{j ∈ blocks(i)} score(j)
```

`base_priority` maps the `priority` field to a number per `scheduler.priority_weights` (defaults: `p0=4`, `p1=2`, `p2=1`, unset=1). `α` is `scheduler.alpha` (default 0.5). `blocks(i)` is the set of issues *i* blocks (i → j edges in the graph).

Higher `score` ranks higher. The thesis from `dep-graph.md` and the maintainer's stated motivation: priority should reflect not only the maintainer's product priorities but also each issue's *downstream-unblocking value*.

## DFS-with-memoization, cycle detection

`crate::scheduler::compute::compute_with` (the pure function) walks each active issue and computes its score via depth-first traversal with a memo. The traversal is on the `blocks` graph: visiting *i* requires visiting all *j* in `blocks(i)` first.

Cycle detection uses a "white / gray / black" coloring: a node is white before visit, gray while it's on the recursion stack, and black after its subtree completes. Re-entering a gray node means a cycle. Nodes participating in a cycle have their scores fixed to 0 and are flagged `in_cycle: true` and `actionable: false` in the output. The graph is DAG-by-construction (edge creation rejects cycles per `dwarven-cli.md#R6.11.3`) but maintainers can introduce cycles via direct file edit; this detection is the safety net for that case (`dep-graph.md#R7.2`).

The implementation uses a separate `on_stack: HashMap<u64, bool>` instead of an enum because the existing `memo` map already serves as the "black" marker (presence = computed).

## Override-vs-score separation

`effective_priority_override` lives in the issue's frontmatter as an `Option<f64>` field. When set, it replaces the algorithmic score in the queue's `effective_priority` column. The `score` column always reflects the algorithm — so the maintainer can see what the algorithm would have computed alongside their override.

The output struct keeps both columns visible:

```rust
pub struct ScheduledIssue {
    pub score: f64,
    pub override_value: Option<f64>,
    pub effective_priority: f64,  // == override.unwrap_or(score)
    // ...
}
```

Sort is by `effective_priority` descending, with `id` ascending as a deterministic tiebreaker.

## "Actionable" semantics

An issue is `actionable: true` iff:

- it's not in a cycle, AND
- it has no active `blocked_by` upstream, AND
- it has no `blocker:*` field set, AND
- its state is not `maintainer`

This matches `dep-graph.md#R3.3.2` + `R3.3.3`. Crucially, non-actionable issues *still appear in the queue with their score* — the maintainer can see the upstream chain by inspecting the queue. This is a deliberate departure from "hide what can't be worked on now" patterns in similar tools; visibility into why something is ranked where it is matters more.

## The motivating example

A p1 that unblocks five p0s should outrank a p1 that unblocks nothing. With α=0.5 and default weights:

- Each downstream p0 has score 4.
- The p1 blocker: `score = 2 + 0.5 × (5 × 4) = 12`.
- An isolated p1: `score = 2`.

The blocker outranks the isolated p1 by 6×. The integration test `tests/api_scheduler.rs#queue_returns_ranked_list` asserts the resulting order. The unit test `src/scheduler/compute.rs#tests::p1_unblocking_five_p0s_outranks_isolated_p1` exercises the algebra directly.

## Why DFS-with-memo over Kahn-like topological sort

Two algorithms produce the same scores on a DAG:

- **Topological sort, propagate forward**: reverse-topo-order the graph, compute `score(j)` first for sinks, then propagate to parents.
- **DFS-with-memo**: visit each node, recursively visit successors, memoize.

The DFS approach is chosen because it handles cycles cleanly (the on-stack marker makes detection trivial) and because the recursion structure mirrors the score function's recursive definition. Topo-sort would require a separate cycle-detection pass.

The "scheduler's API surface is on-demand" choice (`dep-graph.md#R5.2`) means we compute scores per request. At ~hundreds of issues, the DFS is sub-millisecond. Caching with file-watcher invalidation is future polish if the request rate ever justifies it.

## Configuration mutability

`scheduler.alpha` and `scheduler.priority_weights` are live-reload keys (`coordination-hub.md#R10.5`). The hub re-reads `config.toml` on each consumption rather than caching. The tradeoff is one extra file read per `compute()` call — negligible at the file's ~30-line size.

The startup-time validation (`src/daemon/config.rs`, issue #2) re-validates `[scheduler]` at daemon start so malformed configs are caught immediately rather than on first queue request.

## Maintainer override discipline

Per `dep-graph.md#R4.3`, overrides are intended for one-off urgent items or strategic bets the algorithm cannot see. They're not intended to be applied broadly. The web UI's Schedule screen surfaces the `override` column visually so the maintainer can audit what they've overridden; the column is empty for the vast majority of issues by intent.

`dwarven issue priority-override` (CLI) and `POST /api/v1/scheduler/override` (HTTP) carry the same actor-attribution constraint as `dwarven issue priority` itself: maintainer-only by structural convention. The Claude Code adapter's universal deny list (`agent-roster.md#R13.3`) excludes `priority-override` from every agent's allowlist.
