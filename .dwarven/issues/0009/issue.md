---
id: 9
title: 'Web UI: epic-clustered grouping in deps graph'
type: feature
state: pm
priority: p2
epic: web-ui-polish
created: 2026-05-10T02:04:31Z
created_by: maintainer
updated: 2026-05-10T02:04:31Z
---
web-ui.md#R7.4 specifies cluster grouping by epic in the dependencies graph: "nodes are grouped by `epic` when set; clusters can be collapsed."

Current state (slice 26, assets/web/app.js renderDeps): the deps graph layouts a layered DAG without epic awareness. Click panel shows the issue's epic, but graph layout doesn't group by it.

Scope:
- Within each layer, sort/cluster nodes by epic (nodes with same epic adjacent).
- Render a translucent rectangle behind each epic-cluster, labeled with the epic slug.
- Collapsible: click an epic label to collapse the cluster into a single placeholder showing "epic-name (N)" with an arrow expanding it back. Collapsed state is part of the URL (e.g. `?collapsed=foo,bar`).
