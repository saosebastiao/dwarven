---
issue: 2
date: 2026-05-10
---

# Plan: Validate config.toml at daemon startup; fatal on invalid

**Issue:** #2 — `Daemon: validate config.toml at startup; fatal on invalid`
**Specs:** `coordination-hub.md#R10.6`

## Goal

When `dwarven serve` starts, fully parse and validate `.dwarven/config.toml` before binding HTTP, starting the watcher, or doing any work. On invalid config, exit non-zero with a clear one-line error citing the offending key. Per R10.6: "Validation failures on hub start are fatal."

## Validation rules (R10.6)

- `daemon.port` ∈ `[1, 65535]`
- `daemon.bind` is a string (any valid value; we don't validate IP form)
- `daemon.reconciliation_interval_seconds` is a positive integer
- `scheduler.alpha` ∈ `[0.0, 1.0]`
- `scheduler.priority_weights.{p0,p1,p2,unset}` are all positive numbers; `p0 ≥ p1 ≥ p2`
- `triage.stale_threshold_days` is a positive integer

Missing optional keys fall back to defaults (R10.3).

## Code changes

`src/daemon/config.rs` (new module):
- `pub struct DaemonConfig { port: u16, bind: String, reconciliation_interval_seconds: u64 }`
- `pub struct TriageConfig { stale_threshold_days: u64 }`
- `pub struct FullConfig { daemon: DaemonConfig, scheduler: SchedulerConfig, triage: TriageConfig }` — re-uses `scheduler::SchedulerConfig` for the `[scheduler]` validation already in `src/scheduler/config.rs`.
- `pub fn read_full_config(paths) -> Result<FullConfig>` — parses, validates each section, returns or fails.

`src/daemon/serve.rs` calls `read_full_config` early (after `require_initialized`, before opening the PID file). Failure exits via the existing UserError mapping (exit code 1).

## Tests

`tests/daemon_config_validation.rs`:
- `serve_rejects_invalid_port` — set `daemon.port = 70000`, run `dwarven serve` (no `&`), expect non-zero exit + stderr mentions port.
- `serve_rejects_alpha_out_of_range` — set `scheduler.alpha = 2.0`, expect non-zero exit + stderr mentions alpha.
- `serve_rejects_p0_lt_p1` — set weights so p0 < p1, expect non-zero exit.
- `serve_rejects_zero_reconcile_interval` — set 0, expect non-zero.
- `serve_rejects_zero_stale_threshold` — set 0, expect non-zero.
- `serve_accepts_default_config` — happy path: existing fresh repo's defaults pass; daemon comes up. (Already covered by other tests; may not need a dedicated case.)

We don't need to spawn the daemon long-term for the failure cases — `dwarven serve` exits immediately on invalid config. `cmd.assert().failure()` is enough.

## Branch + commit

`feat/2-startup-config-validation`. Two commits: tests RED, then impl GREEN.
