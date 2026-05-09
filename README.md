# Dwarven

A host-agnostic system for **specification-driven software development** with **strongly decoupled agents** and a **local coordination hub**.

> **Status:** Pre-release, mid-pivot (2026-05). The architecture documented here is **v2**, a substantial rework of the inherited v0.1 design (Claude-Code-only, GitHub-coupled). v2 specifications are drafted under `docs/specs/`; implementation is pending. The current `agents/`, `skills/`, `commands/`, and `hooks/` directories implement v0.1 and are flagged stale — see [Status](#status).

## What Dwarven is

Dwarven orchestrates a fixed roster of focused agents to develop software against a written specification. Each agent has one responsibility, a strict tool allowlist, and a defined input/output contract. Agents cannot change roles mid-session — each is a host-native subagent dispatch with its own system prompt and tools. Coordination happens through a **local coordination hub** — a Rust binary that owns the workflow state on disk.

Four pillars:

- **Mechanism-enforced decoupling.** Every agent is a host-native subagent with isolated context and a granular tool allowlist. Drift between roles is prevented at the mechanism layer, not by prompt discipline.
- **Local coordination hub.** A single-binary daemon owns the workflow state. Issues, comments, dependencies, and state transitions live as Markdown files in your repo (`.dwarven/`); a SQLite index serves a local web UI. No GitHub dependency.
- **Dependency-aware scheduling (v2).** Priorities reflect both your product priorities *and* each issue's downstream-unblocking value. Working on a `p1` that unblocks five `p0`s outranks a `p1` that unblocks nothing.
- **Multi-host.** A thin per-host adapter materializes the agent roster onto each supported AI coding-agent CLI. v1 ships the Claude Code adapter; v3 adds opencode.

## Core ideas

**Specifications drive everything.** A spec describes how the system *should* work; documentation describes how it *does* work. The disagreement between them is a **gap**, surfaced as an issue, broken into work, planned, tested, implemented, reviewed, and folded back into docs.

**Tests gatekeep implementation.** Tests are the executable form of the spec. Implementation cannot land without tests that verify it satisfies the spec.

**Architecture is its own concern.** A specification says what the system should do; an architecture says how the system should solve it. Architecture is owned by a distinct agent and lives in its own subtree (`docs/architecture/`).

**Decoupling is mechanism-enforced.** Every agent is a host-native subagent with its own system prompt and tool allowlist. An agent cannot mid-session decide to do another agent's job — it can only dispatch a bounded subagent that returns a single result, or hand off via hub state.

**The hub is the coordination surface.** Issues, comments, dependencies, and state transitions live in `.dwarven/` and are mediated by the `dwarven` CLI (for agents) and a local web UI (for the maintainer). There is no other shared inter-agent state.

**Files are the source of truth.** Hub-tracked artifacts live as Markdown files under `.dwarven/`, committed alongside code. The SQLite index is derived and reproducible at any time. You can edit issue files directly in your editor as an escape hatch.

**Prioritization is dynamic and dep-aware.** You assert product priorities; the hub computes effective priority by combining base priority with downstream-unblocking value across the dep graph.

**Monorepos are preferred (soft).** Architecture, gap-analysis, and triage agents see further when the spec, docs, code, tests, and hub artifacts share a workspace.

## The agent roster

| Agent | Slash command | Scope |
|---|---|---|
| **Spec** | `/spec` | Evolves `docs/specs/*.md`. What the system should do. |
| **Architect** | `/architect` | Designs `docs/architecture/*.md`. How the system should solve the spec. |
| **Gap** | `/gap` | Compares spec vs. docs/code; files gap issues. |
| **PM** | `/pm` | Decomposes top-level gap issues into implementable child issues. |
| **Planning** | `/plan` | Produces `docs/plans/YYYY-MM-DD-<slug>.md` for one issue. |
| **Test Dev** | `/test` | Writes failing tests from plan + spec. Test paths only. |
| **Implementation** | `/implement` | Drives failing tests to green. RED-GREEN-REFACTOR. |
| **Code Review** | `/review` | Reviews diff against plan + spec; merges or routes back. |
| **Doc** | `/doc` | Updates `docs/*.md` to reflect actual code behavior. |
| **Triage** | `/triage` | Audits issue queue; surfaces stuck work. |

Each agent's full I/O contract — trigger, inputs, outputs, tool allowlist, exit conditions, and scope fences — is specified in [`docs/specs/agent-roster.md`](docs/specs/agent-roster.md).

## The pipeline

A specification gap becomes shipped code through a fixed sequence:

```
Spec change committed
    ↓
Gap         → creates issue (type:spec-gap, state:pm)
    ↓
PM          → child issues (type:feature, state:plan, epic:<slug>)
    ↓
Planning    → commits docs/plans/...md, transitions issue to state:test
    ↓
Test Dev    → failing tests on feat/<id>-<slug>, transitions to state:implement
    ↓
Implementation → green code, pushes branch, transitions to state:review
    ↓
Code Review → merges branch into main, transitions to state:doc
    ↓
Doc         → updates docs/*.md, appends CHANGELOG, closes issue
```

At any step, an agent that hits an unresolvable ambiguity escalates: structured comment + `blocker:maintainer-input` + transition to `state:maintainer` + exit. The maintainer responds via the web UI; the next dispatch picks up from there.

## The maintainer shell

You interact with Dwarven through a top-level session in your AI coding-agent host (Claude Code in v1) called the **shell**. The shell is intentionally thin:

- **It can read.** Files, `git log`, `dwarven issue view`, `dwarven issue list` — anything to orient.
- **It cannot write.** Every action that changes state is dispatched to an agent.
- **It dispatches via slash commands.** `/spec`, `/architect`, `/plan 42`, etc. There is no inferred routing — you always name the agent.
- **It does not converse multi-turn about work.** Dialogue happens inside dispatched agents, not in the shell.

The shell holds broad context but no specific work. Putting writes in the shell would erode the per-agent decoupling guarantee.

## Dialogue when an agent needs help

Subagents are one-shot — they cannot converse with you across invocations. Two communication paths:

- **Interactive dispatch** (from the shell): the subagent uses the host's interactive question primitive (Claude Code's `AskUserQuestion`) to clarify mid-run, then completes.
- **Detached dispatch**: the subagent posts a structured comment to the hub, sets `blocker:maintainer-input`, transitions to `state:maintainer`, and exits. You answer via the web UI; the next dispatch reads the new state.

The interactive primitive is allowlisted only on dialogue agents (Spec, Architect, Gap, PM, Planning). Discrete-work agents (Test Dev, Implementation, Code Review, Doc, Triage) escalate structurally — never synchronously.

## The coordination hub

The hub is a single-binary Rust daemon that owns the workflow state. It has two execution modes:

- **CLI mode** — `dwarven <subcommand>`. One-shot operations: create an issue, post a comment, transition state, manage dependencies. CLI mode reads and writes `.dwarven/` files directly; no daemon required.
- **Daemon mode** — `dwarven serve`. Runs an HTTP server (default `127.0.0.1:7777`) that backs the local web UI, plus a file watcher that keeps the SQLite index in sync. Required only for the web UI.

Hub artifacts (issues, comments, state transitions, dependency edges) live as Markdown files with frontmatter under `.dwarven/`, committed alongside code. The SQLite index is derived and reproducible from the files at any time. You can hand-edit any artifact in your editor as an escape hatch.

Agents talk to the hub *exclusively* through the `dwarven` CLI. Per-agent allowlists scope CLI invocations granularly (e.g., the Spec agent has `Bash(dwarven --actor spec issue close:*)` but not `dwarven issue create:*`).

## Dependency-aware prioritization

(v2 deliverable.) The hub stores dependency edges between issues — "A blocks B" relationships that form a DAG. The scheduler computes an *effective priority* for each active issue:

```
score(i) = base_priority(i) + α · Σ score(j) for j blocked by i
```

Higher `α` makes downstream-unblocking value matter more; lower `α` makes immediate base priority dominate. Default `α = 0.5`.

You can override the computed score per issue (the "I know better than the algorithm" escape hatch). Edge creation is restricted to upstream agents (Architect, PM, Planning) and you; downstream agents escalate to record edges they discover.

Full algorithm and tunables: [`docs/specs/dep-graph.md`](docs/specs/dep-graph.md).

## Hosts

Dwarven targets multiple AI coding-agent hosts via thin per-host adapters. Each adapter materializes the abstract agent roster ([`docs/specs/agent-roster.md`](docs/specs/agent-roster.md)) into the host's native primitives.

- **Claude Code** — v1 reference adapter. Maps agents to `.claude/agents/`, slash commands to `.claude/commands/`, hooks to `.claude/hooks/`, settings to `.claude/settings.json`.
- **opencode** — v3 deliverable. Adapter contract is sketched in [`docs/specs/host-adapter.md`](docs/specs/host-adapter.md); open questions to resolve in v3.

A repository may install multiple adapters simultaneously; each writes to its own host-specific directory and you choose which host to launch.

## Spec versioning

Specs live flat at `docs/specs/*.md`. The top-level spec ([`docs/specs/dwarven.md`](docs/specs/dwarven.md)) lists its constituent specs in frontmatter; each constituent owns one concern (storage, CLI, web API, web UI, work states, dialogue, agent roster, host adapter, coordination hub, dep graph).

`docs/CHANGELOG.md` tracks all changes from v2.0.0 forward. On the first breaking spec change after v2.0.0, existing specs migrate to `docs/specs/v2/` and the new major version goes to `docs/specs/v3/`. The migration is a `git mv`; tooling is deferred until v3 actually exists.

## Repository setup

`dwarven init [--host <h>]` scaffolds a repository to use Dwarven:

- Creates `.dwarven/` (`config.toml`, `issues/`, gitignore for the SQLite index)
- Creates `docs/specs/`, `docs/architecture/`, `docs/plans/`, `docs/CHANGELOG.md`
- Materializes the chosen host adapter (e.g., `--host claude-code` writes `.claude/agents/`, `.claude/commands/`, hooks, settings)

Idempotent within an adapter. Retrofit-safe: detects existing host-specific files, merges non-destructively, and prints a diff requiring confirmation before any write that modifies maintainer content.

Multi-host installs are supported (`dwarven init --host claude-code --host opencode` once both adapters exist).

## Installation

Dwarven is **not yet published**. The Rust binary is in development. To follow along:

```bash
git clone <this-repo> ~/path/to/dwarven
# Once the binary is buildable: `cargo install --path .` (or similar)
# Then in your project: `dwarven init --host claude-code`
```

## Status

| Area | State |
|---|---|
| v2 specs (`docs/specs/*.md`) | All 10 constituent specs drafted (top-level + storage + work-states + coordination-hub + CLI + web-api + web-ui + dialogue + agent-roster + host-adapter + dep-graph). |
| `dwarven` Rust binary | Not started. v1 deliverable. |
| Web UI | Not started. v1 deliverable. |
| Claude Code adapter | Not started. v1 deliverable. |
| opencode adapter | v3. |
| Dep-graph scheduler | v2. |
| Inherited `agents/`, `skills/`, `commands/`, `hooks/` | Implements v0.1 (GH-coupled, CC-only). Flagged stale; will be replaced as v1 implementation lands. |

## Philosophy

- **Specification-driven.** Specs describe how the system *should* work; documentation describes how it *does*. Specs drive tests; tests gatekeep implementation.
- **Decoupling at the mechanism.** Each agent is a host-native subagent with enforced tool allowlist and path scope.
- **Local coordination, files first.** State lives in your repo, in human-readable files, mediated by a single local binary. No external service dependencies.
- **One problem per branch.** Describe the problem, not just the change.
- **Process before implementation.** Spec, architecture, and gap analysis run upstream of test development and implementation. Don't shortcut to coding when an upstream gap is the actual problem.
- **Skills are behavior-shaping code, not prose.** Changes to discipline framing need eval evidence.

## License

MIT — see `LICENSE`.
