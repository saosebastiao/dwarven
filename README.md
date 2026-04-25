# Dwarven

A Claude Code plugin for **specification-driven software development** with **strongly decoupled agents** and **GitHub as the coordination surface**.

> **Status:** Pre-release (v0.1, design phase complete; implementation in progress). The architecture documented here is the target. The current `agents/`, `skills/`, and `commands/` directories contain inherited content under active redesign — see [Status](#status) below for the gap between designed and shipped, and `CLAUDE.md` for in-session implementation guidance.

## What Dwarven Is

Dwarven orchestrates a fixed roster of focused agents to develop software against a written specification. Each agent has one responsibility, a strict tool allowlist, and a defined input/output contract. Agents cannot change roles mid-session — each agent is a separate Claude Code subagent dispatch with its own system prompt and tools. Coordination happens through GitHub issue state.

The plugin embraces Claude Code primitives end-to-end: agents are subagents (Agent tool), dispatched from a thin shell via slash commands or by hooks. There is no cross-harness abstraction.

### Core ideas

**Specifications drive everything.** A spec describes how the system *should* work; documentation describes how it *does* work. The disagreement between them is a **gap**, surfaced as an issue, broken into work, planned, tested, implemented, reviewed, and folded back into docs. This follows POSIWID — *the purpose of a system is what it does* — by making the gap between intent and behavior a first-class artifact that drives all development work.

**Tests gatekeep implementation.** Tests are the executable form of the spec. Implementation cannot land without tests that verify it satisfies the spec.

**Architecture is its own concern.** A specification says what the system should do; an architecture says how the system should solve it — what components exist, how they connect, what constraints apply. Architecture is owned by a distinct agent and lives in its own subtree.

**Decoupling is mechanism-enforced, not conventional.** Every agent is a Claude Code subagent with its own system prompt and tool allowlist. An agent cannot mid-session decide to do another agent's job — it can only dispatch a bounded subagent that returns a single result, or hand off via GitHub state. Decoupling lives in the dispatch mechanism, not in agent willpower.

**GitHub is the coordination surface.** Issues, labels, comments, PRs, and releases are how agents talk to each other and to the maintainer. There is no other shared state.

**Monorepos are preferred (soft).** A monorepo gives the architecture agent and the gap-analysis agent visibility into every component and every boundary in one workspace. Dwarven does not require monorepos, but its workflows assume the spec, docs, code, and tests live together.

## The agent roster

| Agent | Slash command | Scope |
|---|---|---|
| **System Specification** | `/spec` | Evolves `docs/specs/*.md`. What the system should do. |
| **Architect** | `/architect` | Designs `docs/architecture/*.md`. How the system should solve the spec. |
| **Specification Gap Analysis** | `/gap` | Compares spec vs. docs/code; files gap issues. |
| **Project Management** | `/pm` | Decomposes top-level gap issues into implementable child issues. |
| **Implementation Planning** | `/plan` | Produces `docs/plans/YYYY-MM-DD-<slug>.md` for one issue. |
| **Test Development** | `/test` | Writes failing tests from plan + spec. Test paths only. |
| **Implementation** | `/implement` | Drives failing tests to green. RED-GREEN-REFACTOR. |
| **Code Review** | `/review` | Reviews PRs against plan + spec; merges or routes back. |
| **System Documentation** | `/doc` | Updates `docs/*.md` to reflect actual code behavior. |
| **Triage** | `/triage` | Audits issue labels; surfaces stuck work. |

Each agent's full I/O contract — trigger, reads, writes, tool allowlist, exit conditions, and scope fences — lives in `agents/<name>.md`.

## The pipeline

A specification gap becomes shipped code through a fixed sequence:

```
Spec change committed
    ↓
Gap Analysis     ─→ creates issue (type:spec-gap, agent:pm)
    ↓
PM               ─→ child issues (type:feature, agent:plan, epic:<slug>)
    ↓
Plan             ─→ commits docs/plans/...md, swaps to agent:test
    ↓
Test Dev         ─→ failing tests on feat/<n>-<slug>, swaps to agent:implement
    ↓
Implementation   ─→ green + PR, swaps to agent:review
    ↓
Review           ─→ merges PR, swaps to agent:doc
    ↓
Documentation    ─→ updates docs/*.md, appends CHANGELOG, closes issue
```

At any step, an agent that hits an unresolvable ambiguity escalates: structured comment + `blocker:*` + label swap to `agent:maintainer` + exit. The maintainer responds in the issue; the next dispatch picks up from there.

## The maintainer shell

The maintainer interacts with Dwarven through a top-level Claude Code session called the **shell**. The shell is intentionally thin:

- **It can read.** Files, `git log`, `gh issue list`, `gh pr view` — anything to orient.
- **It cannot write.** Every action that changes state is dispatched to an agent via the Agent tool.
- **It dispatches via slash commands.** `/spec`, `/architect`, `/plan #42`, etc. There is no inferred routing — the maintainer always names the agent.
- **It does not converse multi-turn about work.** Dialogue happens inside dispatched agents (which can use `AskUserQuestion`), not in the shell.

The shell holds broad context but no specific work. Putting writes in the shell would erode the per-agent decoupling guarantee.

## Dialogue when an agent needs help

Subagents are one-shot — they cannot converse with the maintainer across invocations. Two communication paths:

- **Interactive dispatch** (from the shell): the subagent uses `AskUserQuestion` to clarify mid-run, then completes.
- **Detached dispatch** (by hook, v0.2+): the subagent posts a structured comment + `blocker:*` + swaps the issue to `agent:maintainer` + exits. The maintainer answers in the issue; the next dispatch reads the new state.

`AskUserQuestion` is allowlisted only on dialogue agents (Spec, Architect, Gap, PM, Planning). Discrete-work agents (Test Dev, Implementation, Review, Doc, Triage) escalate structurally — never synchronously.

## GitHub as the coordination surface

### Labels

Every open issue carries:

- **`agent:*`** (exactly one) — who owns the next action. Values: `spec`, `architect`, `gap`, `pm`, `plan`, `test`, `implement`, `review`, `doc`, `triage`, `maintainer`.
- **`type:*`** (exactly one) — what kind of work. Values: `spec-gap`, `feature`, `bug`, `arch`, `doc`, `chore`.

Optional:
- **`blocker:*`** — paired with `agent:maintainer`. Values: `maintainer-input`, `external`, `upstream`.
- **`priority:*`** — `p0`, `p1`, `p2`. Unlabeled = medium default.
- **`epic:<slug>`** — groups child issues under a named epic.

**Single-owner rule:** exactly one `agent:*` per open issue. Routing is unambiguous; parallelism comes from sibling issues, not shared ownership.

### Branches and PRs

- Branches: `feat/<issue-number>-<slug>`. Created by Test Dev, carried by Implementation, merged (not deleted) by Review.
- Commits: Conventional Commits + issue reference (`feat(auth): add MFA (#42)`).
- PRs: title references issue; body links to plan path; auto-links via `Closes #N`.
- Direct commits to `main` are allowed for **Spec, Architect, and Documentation only**. Implementation always goes via PR through Review.

### Interaction surface

Agents use the `gh` CLI exclusively for GitHub interaction (no MCP server dependency). Each agent's Bash allowlist is scoped to specific `gh` patterns it needs.

## Spec versioning

For v0.1, specs live flat at `docs/specs/*.md`. `docs/CHANGELOG.md` tracks changes from v0.1.0 forward. The forward plan, when the first breaking spec change ships:

- Migrate `docs/specs/*` → `docs/specs/v1/*`
- New version → `docs/specs/v2/*`
- Annotations on individual specs mark minor version changes
- `docs/CHANGELOG.md` continues to track patches

Major versions get a clean directory split, minor versions an inline annotation convention, patches a single timeline. The migration is a `git mv`; tooling for it is deferred until a v2 actually exists.

## Repository setup

The `repository-setup` skill scaffolds a repo to use Dwarven:

- Creates `docs/specs/`, `docs/architecture/`, `docs/plans/`
- Seeds `docs/CHANGELOG.md`
- Installs the label set and `.github/ISSUE_TEMPLATE/` (one per filable `type:*`)
- Writes `.claude/settings.json` with per-agent permissions
- Registers slash commands
- Sets `main` branch protection (require PR review)
- Installs SessionStart and PreToolUse hooks

**Modes:**
- *Init* — empty/new repos.
- *Retrofit* — existing repos; non-destructive merge with diff preview before any write.

CI-driven detached dispatch (GitHub Actions that fire agents on `issues.labeled`, `pull_request.opened`, and a Triage schedule) is targeted for v0.2+. Until then, agents are dispatched interactively from the shell via slash commands.

## Installation

Dwarven is **not yet published**. To try it locally:

```bash
git clone <this-repo> ~/path/to/dwarven
# Register as a local plugin in your Claude Code setup
```

Once installed, run `repository-setup` against a target project to scaffold it.

## Status

| Area | State |
|---|---|
| Architecture | Designed; documented in this README and `CLAUDE.md`. Spec for the plugin's own behavior is forthcoming under `docs/specs/`. |
| Agent definitions (`agents/`) | Inherited content; redesign in progress. `code-reviewer.md` is closest to its target form (becomes Code Review); the other 9 agents need to be written. |
| Skills (`skills/`) | 14 inherited. Disposition: 6 keep cross-cutting, 1 keep + major rewrite (`using-dwarven`), 6 fold into agent prompts and delete, 1 delete entirely. See `CLAUDE.md` for the table. |
| Slash commands (`commands/`) | 3 inherited; full set of 10 (one per agent) targeted. |
| Repository setup skill | Targeted; not yet implemented. |
| CI-driven detached dispatch | Targeted for v0.2+. |
| Release automation | Will be re-introduced once redesign is far enough along. |
| Future: CLAUDE.md management skill | Helper for keeping per-repo CLAUDE.md files agent-aware. v0.2+. |

Implementation status of each agent and skill is tracked in GitHub issues.

## Philosophy

- **Specification-driven.** Specs describe how the system *should* work; documentation describes how it *does*. Specs drive tests; tests gatekeep implementation.
- **Decoupling at the mechanism.** Each agent is a Claude Code subagent with enforced tool allowlist and path scope.
- **GitHub-native coordination.** State lives in issues; transitions happen via labels.
- **One problem per branch.** Describe the problem, not just the change.
- **Process before implementation.** Spec, architecture, and gap analysis run upstream of test development and implementation. Don't shortcut to coding when an upstream gap is the actual problem.
- **Skills are behavior-shaping code, not prose.** Changes to discipline framing need eval evidence.

## License

MIT — see `LICENSE`.
