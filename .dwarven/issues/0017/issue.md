---
id: 17
title: 'docs/troubleshooting.md: common issues + recovery'
type: doc
state: done
priority: p2
epic: user-docs
created: 2026-05-11T05:16:51Z
created_by: maintainer
updated: 2026-05-11T05:26:53Z
---
Common-issues-and-recovery reference. Builds on the rest of the docs landing.

Scope (each as its own section with symptom + cause + fix):
- "dwarven serve" exits with "another daemon is already running" — stale pidfile or live process
- Web UI shows "connecting..." indefinitely — daemon down, port mismatch, firewall
- "illegal transition" errors when working through the pipeline — work-states.md graph reference
- "would create a cycle" on dep add — how to find + break the cycle
- Index out of sync (CLI shows different state than web UI) — dwarven reindex
- Daemon won't start: port in use — config set daemon.port + restart
- Config validation fails at startup — read the error, fix the value
- Adapter materialization fails ("shadows global deny") — pattern conflict, narrowing
- Eval runner: ANTHROPIC_API_KEY missing or invalid
- SSE stream reconnect dropped events — Last-Event-ID protocol behavior + when to refresh
- "stale PID" in daemon status — what happened + reclaim path

Plus a "where to file a bug" section with the right places to look first.
