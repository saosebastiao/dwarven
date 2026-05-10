# Dwarven — Project Context

## What this is

Dwarven is a **host-agnostic system for specification-driven development** with **strongly decoupled agents** and a **local coordination hub**. The project is mid-pivot (announced 2026-05-09): the v2 architecture replaces the inherited v0.1 design (Claude-Code-only, GitHub-coupled). v2 specifications are drafted; **the v1+v2 implementation is feature-complete** at the CLI surface, daemon, HTTP API, SSE event stream, Claude Code adapter, dep-graph scheduler, and a minimum-viable web UI.

The full v2 architecture is documented in `README.md` and decomposed across `docs/specs/*.md` (top-level + 9 constituent specs). This file is the load-bearing in-session reference: design decisions with rationale, working conventions, what's stale vs. settled, and pointers.

## Working state

| Area | Status |
|---|---|
| v2 architecture | **Specs drafted; three amendments shipped from implementation.** Source of truth: `docs/specs/dwarven.md` plus 9 constituent specs. Amendments: `storage-model.md#R4.4.4` (creation-comment `from: created` sentinel), `work-states.md#R6.2.4` + `dwarven-cli.md#R6.7.3` (universal `done` reachability via close), `agent-roster.md#R13.3` (priority-override added to universal deny). |
| `dwarven` CLI | **Shipped.** `init` (R6.1, `--host claude-code` materializes the adapter), `issue {create,view,list,transition,comment,close,blocker {set,clear},priority,priority-override,edit,dep {add,remove}}` (R6.2–R6.11 + dep-graph#R4), `serve` + `daemon {status,stop,restart}` (R6.12–R6.13), `reindex` (R6.14), `config {get,set}` (R6.15), `schedule next` (dep-graph#R5.3). |
| Daemon | **Shipped.** PID-locked single-instance per repo, signal handling, file watcher with debounced reindex, periodic reconciliation. SQLite index byte-reproducible from files (storage-model.md#R8). |
| HTTP API | **Shipped.** Full web-api.md surface: issues / comments / transitions / blocker / priority / dependencies / daemon / config / scheduler endpoints + SSE event stream at `/api/v1/events`. |
| Claude Code adapter | **Shipped.** `dwarven init --host claude-code` materializes 23 files: 10 agents (`.claude/agents/<name>.md`), 10 slash commands, `settings.json` (allowlist + R13 deny + hooks block), session-start + pre-tool-use hooks. Per-agent `--actor` baked into allowlist patterns. Substantive Red-Flag-style framing intentionally not synthesized — those need eval evidence per "Skills are behavior-shaping code" below. |
| Web UI | **Shipped.** Vanilla-JS SPA embedded in the binary at compile time (assets/web/). All web-ui.md screens: Inbox, Issues list (with URL-state filter persistence), Issue detail (with full mutation UI: transition / close / blocker / priority / edit / dep add-remove / comment), Dependencies graph (hand-rolled SVG, layered DAG, click-to-panel, focus + N-hop mode), Schedule (with override controls), Daemon ops, Config form. Real-time updates via SSE. Deferred: epic-clustered grouping in the deps graph (R7.4), saved views (R12.4). |
| `.dwarven/` in this repo | Bootstrapped at `5d0ede4`; `next_issue_id = 1` (no real issues filed yet — all verification used scratch tempdirs). |
| opencode adapter | v3 deliverable. The host-agnostic contract in `host-adapter.md#R2` is published. |
| Dep-graph scheduler | **Shipped (v2).** `score(i) = base + α · Σ score(blocked)` topo-sort with cycle detection. CLI: `dwarven schedule next`, `dwarven issue priority-override`. HTTP: `GET /api/v1/scheduler/queue`, `POST /api/v1/scheduler/override`. Web UI: Schedule screen with override controls. Defaults from config.toml: `α=0.5`, weights `{p0=4, p1=2, p2=1, unset=1}`. |
| Test coverage | **196 tests, all green** (18 unit + 178 integration across 17 test files). `cargo test` runs in ~6s. The web-UI tests assert that key renderers + endpoint paths are present in the embedded JS bundle; behavioral correctness rides on the underlying API tests. |

## The pivot (2026-05-09)

The project was announced as a Claude Code plugin coupled to GitHub. After the v0.1 implementation reached the milestone documented in earlier git history, the maintainer announced a substantial pivot:

1. **Decouple from GitHub.** Replace `gh` CLI + GH issues/PRs/labels with a lightweight local server + UI backed by file-based storage.
2. **Dynamic dep-graph prioritization.** Priorities driven by both product priority *and* downstream-unblocking value.
3. **Multi-host.** Target Claude Code (v1) and opencode (v3), not CC only.

The v2 specs reflect this pivot end-to-end. When you read code/files/comments referencing `gh issue *`, `agent:*` labels, or "GitHub as the coordination surface," that content is from v0.1 and is being retired.

## Architectural decisions (v2, with rationale)

### Files are the source of truth

Hub-tracked artifacts (issues, comments, state transitions, dependency edges) live as Markdown files with frontmatter under `.dwarven/`, committed alongside code. SQLite is a derived index for fast queries and the web UI; it is reproducible from files at any time.

*Why files over DB-canonical:* maintainer-readable, git-diffable, PR-able, branch-scoped, no DB-vs-file sync nightmares. The escape hatch is just opening the file in your editor.

### CLI ↔ daemon split

The `dwarven` CLI is filesystem-only — no SQLite access. The daemon owns SQLite, HTTP, web UI, and the file watcher. CLI works fully without daemon; daemon is required only for the web UI.

*Why this split:* CLI is robust regardless of daemon state. No CLI-vs-daemon write coordination. Trade-off: CLI list operations are O(n) file reads, fine at hundreds-of-issues scale.

### Agents talk to the hub via CLI only

Per `dwarven.md#R2.10`: agents use `dwarven` CLI exclusively, not the HTTP API. The web UI uses the HTTP API; agents do not.

*Why CLI-only for agents:* per-agent allowlist patterns stay granular and host-portable (`Bash(dwarven --actor spec issue view:*)`). HTTP would weaken allowlist enforcement.

### Actor attribution at the allowlist boundary

Each agent's allowlist patterns embed `--actor <agent-name>` literally (e.g., `Bash(dwarven --actor spec issue view:*)`). Per-host adapters generate these patterns when materializing the agent definitions.

*Why baked into patterns:* attribution is structural, not prompt-level. An agent cannot fake another agent's actor name.

### State == owner

The `state` field on an issue IS who currently owns the issue (the next required action). One field, one concept. Single-owner is structural — a value cannot represent two simultaneous owners.

*Why merged:* v0.1 had `agent:*` labels that doubled as ownership; the new model just makes that structural. Cleaner mental model, fewer fields to keep in sync.

### Dependency edges in issue frontmatter

`blocks: [N, M]` and `blocked_by: [P]` live in the issue frontmatter on both endpoints. No separate edge files. Asymmetric edges are flagged by the hub for triage.

*Why frontmatter over separate files:* easy to read; one fewer artifact type; both endpoints discoverable from each issue.

### PR concept subsumed into the issue

There is no separate "PR" artifact. Implementation pushes its branch and comments on the issue with branch info. Code Review uses `git diff main..feat/<id>-<slug>` and merges directly via `git merge`. The issue thread holds review discussion.

*Why no PR:* GitHub coupling is gone; introducing a parallel PR artifact in the hub doubles the model. The issue is the unit of work.

### Maintainer override is absolute

The dep-graph scheduler computes effective priority. The maintainer can override per issue with an absolute number that bypasses computation entirely.

*Why absolute over relative:* simplest to implement and reason about. Relative ("rank above issue X") is more natural but adds a constraint solver.

### Multi-host via thin adapters

The agent roster is host-agnostic. Per-host adapters materialize the abstract definitions onto the host's primitives. v1 ships the Claude Code adapter; v3 adds opencode.

*Why host-agnostic:* avoids vendor lock-in; the agent contracts are the load-bearing part, the host integration is mechanical.

## Spec disposition

The v2 specs are ten Markdown files under `docs/specs/`:

| Spec | Concern |
|---|---|
| `dwarven.md` | Top-level: identity, invariants, architecture overview, constituent index. |
| `storage-model.md` | On-disk artifact format, directory layout, sync model. |
| `work-states.md` | State/type/blocker/priority vocabulary; transition graph. |
| `coordination-hub.md` | Hub binary: lifecycle, file watcher, SQLite ownership, config schema. |
| `dwarven-cli.md` | CLI surface: subcommands, flags, allowlist patterns, exit codes. |
| `web-api.md` | Local HTTP API consumed by the web UI. |
| `web-ui.md` | Web UI screens, flows, real-time updates. |
| `dialogue.md` | Interactive vs. detached agent-maintainer dialogue protocol. |
| `agent-roster.md` | Ten agents: trigger, I/O, allowlist, exit conditions, scope fences. |
| `host-adapter.md` | Host-agnostic contract + Claude Code adapter (v1) + opencode sketch (v3). |
| `dep-graph.md` | Dependency model and prioritization algorithm (v2). |

Cross-references use `<spec>.md#R<n>`. Top-level spec wins on conflicts.

## Working conventions

### Patterns to use in agent system prompts (carried forward from v0.1)

- **Red Flags tables** — "what you might be thinking vs. reality" anti-rationalization framing.
- **HARD-GATE / EXTREMELY-IMPORTANT framing** — explicit blocks for procedural gates that are load-bearing.
- **One question at a time + 2-3 alternatives + your lean** — dialogue discipline for Spec + Architect + other dialogue agents (`dialogue.md#R3.2`, `dialogue.md#R5.1`).
- **Self-review pass** — after writing a doc, scan for placeholders, contradictions, scope drift, ambiguity. Required for Spec, Architect, Planning, Doc.
- **Verification before completion** — every agent ends with verification.

### Terminology

- **"Your human partner"** — not "the user." Deliberate.
- **Maintainer** — the human who owns the project; singular for v1.
- **Agent** — a defined role (system prompt + tool allowlist + scope). Implemented per host as a subagent.
- **Shell** — the host's top-level session.
- **Hub** — the local coordination daemon (Rust binary).
- **Pipeline** — spec → gap → pm → plan → test → implement → review → doc.
- **Dispatch** — invoking an agent (via slash command or, in v1.1+, by hook event).
- **Gap** — disagreement between spec and docs/code, surfaced by Gap Analysis.

### Skills are behavior-shaping code, not prose

Changes to Red Flags tables, rationalization lists, and "EXTREMELY-IMPORTANT" framing need eval evidence. When folding inherited skills into agent prompts, prefer fresh writes to in-place rewrites of behavior-shaping content.

### TDD is RED-GREEN-REFACTOR

Test Dev writes failing tests; Implementation drives green then refactors. Test Dev's tool allowlist excludes source paths, so it cannot cheat by writing source code — the discipline is mechanism-enforced.

### Process before implementation

In the pipeline, Spec / Architect / Gap / PM / Planning all run upstream of Test Dev and Implementation. Don't shortcut to Implementation when an upstream gap is the actual problem.

### One problem per branch

Describe the problem, not just the change. Branches are `feat/<id>-<slug>` and are merged (not deleted) by Code Review.

## Working in this repo

- **Specs:** `docs/specs/*.md` — ten v2 specs as listed above.
- **Plans:** `docs/plans/` (currently empty; populated as implementation issues are planned).
- **Architecture:** `docs/architecture/` (currently empty; populated as design decisions are made).
- **Changelog:** `docs/CHANGELOG.md` — v2.0.0 onward.
- **Maintainer scratchpad:** `prompts/todo.local.md` (gitignored); not authoritative.

## Pointers

- `README.md` — public guiding document with the v2 architecture
- `docs/specs/dwarven.md` — top-level spec; lists constituents
- `docs/specs/agent-roster.md` — full I/O contracts for the ten agents
- `docs/specs/host-adapter.md` — Claude Code adapter (v1) materialization spec
- `docs/CHANGELOG.md` — change log (v2.0.0 forward)
