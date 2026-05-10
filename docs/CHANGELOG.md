# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Per Dwarven's spec versioning model: major spec versions migrate to versioned directories (`docs/specs/v1/`, `docs/specs/v2/`); minor spec versions are annotated inline; patch-level changes are tracked here.

## [Unreleased]

### Added

- SSE event stream now supports `Last-Event-ID`-based replay on
  reconnect (`web-api.md#R5.6`). Each emitted event carries a
  monotonic seq id; clients reconnecting with the header receive
  any missed events from a 256-entry ring buffer before the live
  stream attaches. If the requested id is older than the buffer's
  oldest entry, the server emits a `stream.refresh-required`
  event so the client refetches state. (#3)
- Daemon now validates `.dwarven/config.toml` at startup per
  `coordination-hub.md#R10.6`. Out-of-range `daemon.port`, invalid
  `scheduler.alpha`, mis-ordered `scheduler.priority_weights`, and
  zero `triage.stale_threshold_days` / `daemon.reconciliation_interval_seconds`
  cause `dwarven serve` to exit with code 1 and a clear error
  before any side effects (PID lock, HTTP bind, watcher start).
  (#2)
- Daemon startup now probes the existing SQLite index for health
  (`coordination-hub.md#R7.3` + `#R7.4`) before performing the
  unconditional reindex. One of four log lines reports whether the
  prior index was missing, healthy, corrupt, or had a schema
  version mismatch. The unconditional rebuild per R3.3 step 5 is
  preserved; the probe is observability only. (#1)
- v0.1 architecture locked in `README.md` and `CLAUDE.md`.
- Formal v0.1 specification at `docs/specs/dwarven.md`.
- Bootstrap scaffolding: `docs/specs/`, `docs/architecture/`, `docs/plans/`.
- All 10 agent definitions in `agents/` per R3.1–R3.10.
- All 10 dispatch commands in `commands/`.
- `skills/using-dwarven/` rewritten for the v0.1 shell+agent model.
- `skills/dispatching-parallel-agents/` updated for the Claude Code `Agent` tool.
- `.claude/settings.json` with project-wide allow + R6.5 deny patterns.
- `hooks/pre-tool-use` PreToolUse hook as defense-in-depth on the R6.5 never-list.
- `skills/repository-setup/` per R8: scaffolds target repos with directories, label set (R5.1), issue/PR templates, `.claude/settings.json`, and `main` branch protection (R5.5).

### Changed

- `.gitignore` updated from `.claude/` to `.claude/*` + `!.claude/settings.json` so the project settings file is tracked.

### Removed

- Inherited skills folded into agent prompts and deleted: `brainstorming`, `executing-plans`, `finishing-a-development-branch`, `receiving-code-review`, `requesting-code-review`, `writing-plans`.
- Inherited skill deleted as vestigial (the architecture IS this): `subagent-driven-development`.
- Inherited deprecated commands deleted: `brainstorm.md`, `write-plan.md`, `execute-plan.md`.
- Inherited `agents/code-reviewer.md` superseded by `agents/code-review.md`.
