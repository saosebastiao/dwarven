---
id: 20
title: 'docs/architecture/testing.md: test layout + harness patterns'
type: doc
state: done
priority: p2
epic: code-docs
created: 2026-05-11T15:49:35Z
created_by: maintainer
updated: 2026-05-11T15:53:50Z
---
The test surface is ~270 tests across unit + integration + adapter-specific + eval-framework, with reusable patterns (tempdir fixtures, `dwarven()` builder, daemon spawn-and-stop, byte-reproducibility assertions). None of this is documented; a contributor adding a feature has to scour tests/* to find the right pattern.

Scope:
- Test layout: src/<module>/<file>.rs:#[cfg(test)] for unit, tests/<area>.rs for integration, evals/<agent>/*.yaml for eval scenarios.
- Harness patterns:
  * `tests/common/dwarven()` — assert_cmd builder with --repo + tempdir.
  * tempfile::tempdir + assert_cmd for isolated per-test repos (always; no shared state).
  * daemon spawn + drop semantics in tests/daemon.rs (PID + port + cleanup).
  * Reading + asserting on issue.md frontmatter shape in storage tests.
  * Mocking the Anthropic API surface in eval (mock_tools).
- Determinism rules: byte-reproducibility on reindex, frontmatter ordering, comment-file naming.
- "Why no mocking of SQLite" (we test against the real engine; speed is fine).
- "Why no mocking of file IO" (atomic-write semantics matter).
- Where to put a new test for a new mutation, scheduler tweak, adapter, etc.

Audience: contributor writing a test for a new feature.
