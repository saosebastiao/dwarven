---
issue: 9
date: 2026-05-10
---

# Plan: epic-clustered grouping in the dependencies graph

**Issue:** #9
**Specs:** `web-ui.md#R7.4`

## Goal

Within each layer of the dependencies-graph SVG, group nodes that share an `epic` value adjacent to each other. Render a translucent rectangle behind each epic cluster, labeled with the epic slug. Click the label to collapse the cluster into a single placeholder node; click again to expand. Collapsed state is part of the URL hash so views are shareable and the back button works.

## Layout changes

Current behavior (`assets/web/app.js#layerize` and `#layoutLayers`):

- nodes are sorted by `id` within each layer
- no awareness of `epic`
- positions are computed and SVG nodes drawn

New behavior:

- nodes within a layer first split into groups: one per distinct epic value, plus one "no-epic" group at the start (so unsorted/epicless nodes come first per layer)
- within each group, sort by id (same as today)
- layer's node sequence is `[no-epic group] + [groups sorted by epic slug]`
- groups are drawn with a background rect (rounded, translucent, distinct hue per epic) extending across the cluster, with the epic label above the rect

## Collapsing

Each epic cluster has a label that's actually a `<text>` with a click handler. Click toggles the epic in the URL `collapsed=...` CSV. Collapsed cluster:

- replaced by a single circle (radius derived from member count, capped) labeled with `${epic} (${n})`
- still participates in the layered layout — its layer is the median layer of the cluster's members (or just the min/max — use min for simplicity, "earliest layer the cluster reaches")
- edges to/from collapsed-cluster members redirect to the placeholder node (no doubled edges; deduplicate)
- click the placeholder to expand

The no-epic group is never collapsible.

## URL state

`#/deps?collapsed=epic-a,epic-b` collapses those two epics. Combined with existing `focus`, `hops`, `all` params.

## Test coverage

`tests/api_web.rs`: add bundled-content assertions for the new functions:

- `groupByEpic` (helper) is in the JS bundle
- `renderCollapsedPlaceholder` (helper) is in the JS bundle
- `collapsed` query param is referenced in the deps-route handler

The dependency-graph UI doesn't have semantic-correctness integration tests (it's a visual feature), and the assertions above are deliberately minimal — they catch a missing-export regression but not visual regressions. Visual regressions would require headless-browser tests, deferred per `web-ui.md#R1.2`.

## Code organization

All changes in `assets/web/app.js#renderDeps` and its helpers. CSS additions in `assets/web/app.css`:

- `.epic-cluster-bg` — fill with low opacity
- `.epic-label` — color + cursor + bold

No new files; this is a UI-only slice.

## Branch

`feat/9-epic-clustering`. Tests + impl in same commit (the test assertions are trivial existence checks that pass once the code exists).
