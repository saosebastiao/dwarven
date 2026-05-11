---
id: 14
title: 'docs/configuration.md: config.toml reference'
type: doc
state: done
priority: p2
epic: user-docs
created: 2026-05-11T05:16:33Z
created_by: maintainer
updated: 2026-05-11T05:22:36Z
---
Reference for every key in .dwarven/config.toml. The spec (coordination-hub.md#R10) defines the schema; this doc is the user-facing reference with explanations.

Scope per section:
- [repo]: id, name — identity (immutable on init)
- [counters]: next_issue_id — internal counter, don't edit
- [daemon]: port, bind, reconciliation_interval_seconds — restart-required flags noted
- [scheduler]: alpha (range, default, tuning advice), priority_weights (p0/p1/p2/unset with p0 >= p1 >= p2 invariant)
- [triage]: stale_threshold_days
- [adapters.*]: extension points (currently unused)

For each key:
- Type + valid range
- Default
- What changing it does
- Whether daemon restart is required
- Example value

Plus a section on \`dwarven config get/set\` ergonomics and the live-vs-restart-required distinction.
