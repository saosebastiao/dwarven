---
id: 21
title: 'rustdoc: foundational modules (storage, index, scheduler)'
type: doc
state: done
priority: p1
epic: code-docs
created: 2026-05-11T15:54:38Z
created_by: maintainer
updated: 2026-05-11T15:58:26Z
---
Add rustdoc to the foundational pure-ish modules. These hold the storage primitives, the SQLite index, and the dep-graph scheduler.

Files in scope (file-level //! comments + per-pub-item ///):
- src/storage/mod.rs
- src/storage/atomic.rs (write_atomic + helpers)
- src/storage/config.rs (RepoPaths, with_repo_lock, require_initialized)
- src/storage/issue_file.rs (IssueFrontmatter, read_issue, write_issue, enumerate_issue_ids)
- src/storage/comment_file.rs (CommentFrontmatter, append_comment, list_comments)
- src/index.rs (IndexHealth, rebuild, SCHEMA_VERSION)
- src/scheduler/mod.rs
- src/scheduler/compute.rs (ScheduledIssue, compute, compute_with)
- src/scheduler/config.rs (SchedulerConfig, PriorityWeights, read_scheduler_config)

For each pub type/function: one-line summary, longer paragraph for any non-obvious invariants or constraints (advisory lock semantics, byte-reproducibility constraints, error envelope, validation rules). No need for runnable doctest examples — the integration tests already cover the runnable contract.

Existing module-level //! comments are present in some files; preserve and extend rather than rewriting unless they're stale.
