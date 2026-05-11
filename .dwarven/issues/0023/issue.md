---
id: 23
title: 'rustdoc: api + daemon modules'
type: doc
state: done
priority: p1
epic: code-docs
created: 2026-05-11T15:54:39Z
created_by: maintainer
updated: 2026-05-11T16:03:15Z
---
Add rustdoc to the daemon-side modules: HTTP API, SSE event bus, daemon lifecycle.

Files in scope:
- src/api/mod.rs
- src/api/server.rs (router, AppState wiring)
- src/api/error.rs (ApiError envelope)
- src/api/state.rs (AppState)
- src/api/issues.rs (list, view, list_comments_for_issue handlers + ListQuery / CommentQuery)
- src/api/mutations.rs (create_issue, edit_issue, append_comment, transition, set/clear_blocker, set/clear_priority, add/remove_dep + their *Body structs)
- src/api/scheduler.rs (queue, set_override + OverrideBody)
- src/api/config.rs (get_config, patch_config)
- src/api/daemon_ops.rs (shutdown, reindex)
- src/api/events.rs (EventKind, EventBus, sse_handler, Last-Event-ID replay, EventEnvelope)
- src/api/types.rs (Issue, Comment DTOs)
- src/api/web.rs (embedded asset handlers, spa_fallback)
- src/daemon/mod.rs
- src/daemon/serve.rs (orchestration)
- src/daemon/pidfile.rs (acquire, release, pid-present-but-stale)
- src/daemon/watcher.rs (watch_loop, debounce semantics, reconcile interval)
- src/daemon/control.rs (run_status, run_stop, run_restart)
- src/daemon/config.rs (FullConfig, read_full_config, validation rules)

The api/events.rs Last-Event-ID protocol especially deserves a thorough /// block — the race-free subscribe-snapshot-replay sequence is subtle and the architecture doc already explains it; the code-side doc should be the precise contract.
