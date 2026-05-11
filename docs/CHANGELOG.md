# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Per Dwarven's spec versioning model: major spec versions migrate to versioned directories (`docs/specs/v1/`, `docs/specs/v2/`); minor spec versions are annotated inline; patch-level changes are tracked here.

## [Unreleased]

### Added

- `docs/configuration.md`: `.dwarven/config.toml` reference. Section-
  by-section: `[repo]`, `[counters]`, `[daemon]`, `[scheduler]` with
  `[scheduler.priority_weights]` sub-table, `[triage]`. Each key with
  type, default, restart-required flag, and notes. Honestly flags two
  known limitations: `daemon.bind` is validated but ignored (daemon
  hardcodes `127.0.0.1`), and `triage.stale_threshold_days` is
  validated but not yet consumed. Tuning guidance for the scheduler
  weights and `alpha`. Validation-error table. Cross-refs into
  `coordination-hub.md#R10`, `dep-graph.md`, and troubleshooting. (#14)
- `docs/cli-reference.md`: standalone reference for every `dwarven`
  subcommand, flag, and exit code with realistic examples. Covers
  global flags, init (including dual host install), the full issue
  surface (create/view/list/transition/close/comment/blocker/priority/
  priority-override/edit/dep), serve, daemon control, schedule next,
  reindex, and config get/set. Forward-linked from
  `docs/getting-started.md`. (#13)
- `docs/getting-started.md`: end-to-end quickstart walkthrough.
  Covers prerequisites, `cargo install --path .`, `dwarven init`,
  host adapter choice (Claude Code vs opencode) with the materialize-time
  vs PreToolUse-hook enforcement trade-off, starting the daemon
  foreground or detached, the web UI tour, filing a first issue,
  dispatching the first agent (`/spec 1` vs `@spec please pick up
  issue 1`), and walking the pipeline once. Linked from README and
  forward to the other user docs. (#12)

### Changed

- README rewritten to reflect v1+v2 shipped state. Status callout now
  describes the actual shipped surface (CLI, daemon, HTTP API, SSE,
  both host adapters, scheduler, web UI, eval framework, ~270 tests)
  rather than "implementation pending." Hosts section updated to
  document both adapters as shipped. Repository layout, what's-shipped
  table, and documentation link map added. (#11)
- `dwarven init --host` help text corrected: previously said "Not yet
  implemented"; now lists the shipped adapters (`claude-code`, `opencode`).

### Added

- opencode adapter: `dwarven init --host opencode` materializes
  the full `.opencode/` + root-level surface per
  `host-adapter.md#R4`. 13 files: `opencode.json` at repo root
  with the R13 universal-deny floor and `instructions` ref;
  `.opencode/AGENTS.md` orientation copy (auto-loaded at session
  start); `.opencode/agents/<name>.md` × 10 with `mode: subagent`
  and per-agent `permission` blocks; `.opencode/agents/build.md`
  for the maintainer primary. Materialize-time validation rejects
  any per-agent allow pattern that would shadow a global deny
  (opencode has no PreToolUse hook as a runtime second line).
  Coexists with the Claude Code adapter — both can be installed
  in the same repository. (#4, #5)
- Agent prompt eval framework + runner per
  `docs/architecture/agent-eval.md`. `cargo run --example
  eval-runner` consumes YAML scenarios under `evals/<agent>/`,
  drives a Claude API tool-use loop against mock implementations
  (Edit/Write/AskUserQuestion mocked; Read real; Bash mocks
  `dwarven` read commands against fixture_issues), and asserts
  on tool-call patterns (required + forbidden). Optional
  response_judge runs a second Claude call to evaluate response
  text where it's load-bearing. Refuses to run without
  `ANTHROPIC_API_KEY`. Reference scenarios live at
  `evals/spec/refuses-out-of-scope-edits.yaml` (dialogue agent)
  and `evals/test/declines-to-write-source.yaml` (discrete-work
  agent). Library-target side: `src/eval/{scenario,matcher,mock_tools}`
  with 11 unit tests; the Anthropic client lives in the example
  itself (reqwest is a dev-dep). (#7, #10)
- Dependencies graph view groups nodes by epic per
  `web-ui.md#R7.4`. Each epic cluster renders a translucent
  background rectangle with a clickable label; collapsed clusters
  appear as a single placeholder node showing `<epic> (<N>)`.
  Edge rewiring on collapse: in/out edges of cluster members are
  redirected to the placeholder; intra-cluster edges are dropped.
  Collapsed state is part of the URL hash (e.g.
  `#/deps?collapsed=alpha,beta`) so views are shareable. (#9)
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
