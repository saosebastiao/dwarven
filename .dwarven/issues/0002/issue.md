---
id: 2
title: 'Daemon: validate config.toml at startup; fatal on invalid'
type: feature
state: pm
priority: p2
epic: daemon-polish
created: 2026-05-10T02:03:04Z
created_by: maintainer
updated: 2026-05-10T02:03:04Z
---
coordination-hub.md#R10.6 specifies validation (daemon.port in [1,65535]; scheduler.alpha in [0.0, 1.0]; scheduler.priority_weights positive with p0 >= p1 >= p2; triage.stale_threshold_days positive). It says "Validation failures on hub start are fatal".

Current state: src/scheduler/config.rs validates `[scheduler]` keys when read on every queue computation (slice 20). The daemon does NOT validate `[daemon]`/`[triage]` at startup, and a malformed config.toml will silently default for missing keys but may surface as runtime errors on first scheduler computation rather than at startup.

Scope:
- On daemon::serve::run, after require_initialized, parse config.toml fully and validate every documented key per R10.6.
- Surface invalid config as a one-line error and exit non-zero before binding HTTP / starting the watcher. Per R10.6 startup validation is fatal (vs. CLI `dwarven config set` which exits 1 and leaves the file unchanged — already implemented).
- The existing scheduler::config::SchedulerConfig::validate() can be reused; add validators for daemon and triage tables.
