# Dwarven — Project Context

## What this is

Dwarven is a Claude Code plugin for **specification-driven development** with **strongly decoupled agents** and **GitHub as the coordination surface**. The architecture is locked; implementation is in progress against the inherited content in `agents/`, `skills/`, `commands/`, and `hooks/`.

The full architecture is documented in `README.md`. This file is the load-bearing in-session reference: design decisions with rationale, working conventions, and pointers. Don't relitigate locked decisions without evidence of changed circumstances.

## Working state

| Area | Status |
|---|---|
| Architecture | Locked. Source of truth: `docs/specs/dwarven.md` (R1–R11). |
| Agent definitions (`agents/`) | All 10 written per R3.1–R3.10. |
| Skills (`skills/`) | 8 in place: 7 cross-cutting per R7.2 + 1 maintainer-invoked (`repository-setup`, R8). Folding work complete. |
| Slash commands (`commands/`) | All 10 in place; deprecated inherited stubs deleted. |
| Repository setup skill | Implemented at `skills/repository-setup/` — not yet run against any target repo (including this one). |
| Hooks | SessionStart + PreToolUse both registered. PreToolUse enforces R6.5 universal never-list. |
| Per-agent permission enforcement | `.claude/settings.json` is project-wide; per-agent restrictions (shell vs. agent; Implementation vs. `main`; etc.) require subagent context in hook input — targeted for follow-up. |
| CI detached dispatch | v0.2+ target. |
| Dwarven's own scaffolding | `docs/specs/dwarven.md`, `docs/CHANGELOG.md`, `docs/architecture/`, `docs/plans/` exist. GitHub-side scaffolding (label set, issue/PR templates, branch protection) lands when `repository-setup` first runs against this repo. |

## Architectural decisions (locked, with rationale)

### Agent model: everything is a subagent

Every agent role is implemented as a Claude Code subagent (`agents/<name>.md`) with its own system prompt and tool allowlist. Decoupling lives in the dispatch mechanism — an agent **cannot** mid-session become a different agent; it can only dispatch a bounded subagent that returns a single result.

*Why this over single-session-per-agent:* subagent isolation is one-directional (child fresh, parent not), but the parent here is the maintainer's thin shell which holds no work-specific context. We get the strongest mechanism-level guarantee Claude Code offers.

*Why this over a work-picker pattern:* a picker accumulates context across dispatches — exactly the drift we adopted this model to avoid.

### Maintainer shell: thin, explicit routing

The maintainer's top-level session has tools `Agent`, `Read`, and read-only `Bash` (`git log`, `git diff`, `git status`, `gh issue view`, `gh issue list`, `gh pr view`, `gh pr list`, `gh label list`). It cannot write. All actions go through explicit slash commands. Bare maintainer messages get read-based orientation only — the shell never dispatches without a slash command.

*Why explicit over inferred:* silent misrouting is the dominant failure mode of inferred routing. Explicit routing costs a few keystrokes and removes that class of drift.

### GitHub interaction: `gh` CLI, no MCP

Every agent that touches GitHub uses `gh` CLI via Bash with per-agent scoped patterns (e.g., `Bash(gh issue create:*)`). No MCP GitHub server dependency.

*Why not MCP:* MCP adds a server dependency, install friction, and an extra auth layer. `gh` works zero-setup once `gh auth login` is done. Bash pattern allowlists are sufficiently granular per agent. If MCP wins later, we can add it as a thin abstraction without touching agent definitions.

### Spec versioning: flat for v0.1, directory-per-major later

`docs/specs/*.md` flat. `docs/CHANGELOG.md` from v0.1.0. On the first breaking spec change, migrate `docs/specs/*` to `docs/specs/v1/`; new version goes to `docs/specs/v2/`. Migration is a `git mv`; no tooling needed until then. Annotations-for-minor and changelog-for-patch conventions are documented but not used until they matter.

### Dialogue when detached: GitHub state

Subagents are one-shot — they cannot converse with the maintainer across invocations. Two paths:

- **Interactive dispatch** (from the shell): subagent uses `AskUserQuestion` to clarify mid-run.
- **Detached dispatch** (by hook, v0.2+): subagent writes a structured comment + `blocker:*` + swaps the issue to `agent:maintainer` + exits. Maintainer answers in the issue; next dispatch reads the new state.

`AskUserQuestion` is allowlisted only on dialogue agents (Spec, Architect, Gap, PM, Planning). Discrete-work agents (Test Dev, Implementation, Review, Doc, Triage) escalate structurally — never synchronously.

### Repository-setup scope: standard + retrofit

The `repository-setup` skill is **standard scope** (file scaffolding + label set + issue/PR templates + `.claude/settings.json` + slash command registration + `main` branch protection + SessionStart and PreToolUse hooks) and **retrofit-capable** (detects existing structure, non-destructive merge, diff preview before any write). CI dispatch workflows are deferred to v0.2+.

## Agent roster (target)

10 agents, each defined in `agents/<name>.md`. See README "The agent roster" for the table. Full I/O contracts (trigger, reads, writes, allowlist shape, exit conditions, scope fences) live in each agent file.

**Branch convention:** `feat/<issue-number>-<slug>`. Test Dev creates, Implementation carries, Review merges (does not delete).

**Direct commits to `main`** are allowed only for Spec, Architect, and Doc (per spec).

**Hard "never" list, applies to every agent:** force pushes, `git reset --hard`, `git checkout -- .`, `git restore .`, `git clean -f*`, `rm -rf*`, `gh repo *` (mutating), `gh release delete *`, direct `git push origin main` for any agent except Spec/Architect/Doc.

## Label protocol

See README "Labels" for the full taxonomy. Working notes:

- **Single-owner rule.** Exactly one `agent:*` per open issue. Multi-owner is forbidden — if parallel work is needed, spawn sibling issues.
- **Label writes are prompt-level, not mechanism-level.** GitHub doesn't expose per-user label permissions, and Bash patterns can't distinguish `gh label add agent:foo` from `gh label add agent:bar`. Discipline is enforced by agent system prompts and post-hoc Triage audit. This asymmetry is acceptable: mislabeled issues are recoverable; misbehaved code is not.

## Skills disposition (complete)

This was the v0.1 disposition plan. All actions below have shipped; the table is retained as historical context for future contributors.

| Skill | Disposition |
|---|---|
| `brainstorming` | Fold (hybrid) → Spec + Architect; delete |
| `dispatching-parallel-agents` | Keep cross-cutting (minor update for Agent tool reference) |
| `executing-plans` | Fold (hybrid) → Implementation; delete |
| `finishing-a-development-branch` | Fold (hybrid) → Implementation + Review; delete |
| `receiving-code-review` | Fold (hybrid) → Implementation; delete |
| `requesting-code-review` | Fold (hybrid) → Implementation; delete |
| `subagent-driven-development` | Delete (the architecture IS this; rationale moves to `docs/architecture/`) |
| `systematic-debugging` | Keep cross-cutting |
| `test-driven-development` | Keep cross-cutting (invoked by Test Dev RED, Implementation GREEN/REFACTOR) |
| `using-dwarven` | Keep + major rewrite (must teach new shell + agent + slash-command model) |
| `using-git-worktrees` | Keep cross-cutting |
| `verification-before-completion` | Keep cross-cutting |
| `writing-plans` | Fold (hybrid) → Planning; delete |
| `writing-skills` | Keep cross-cutting (meta) |

**Fold style:** hybrid — verbatim copy of timeless-discipline content into agent prompts (TDD framing, verification checklists), first-principles rewrite for flow-control content tangled with the old single-session pattern.

*Rationale for delete-rather-than-rewrite:* CLAUDE.md (this file) says skill rewrites need eval evidence. We have none. Folding into a fresh agent prompt is a new write, not a rewrite of an eval-tested skill.

## Working conventions

### Patterns to use in agent system prompts

- **Red Flags tables** — "what you might be thinking vs. reality" anti-rationalization framing. Use where shortcut-thinking is a real risk for the agent.
- **HARD-GATE / EXTREMELY-IMPORTANT framing** — explicit blocks for procedural gates that are load-bearing (e.g., "do not exit before committing").
- **One question at a time + 2-3 alternatives + your lean** — dialogue discipline for Spec + Architect.
- **Self-review pass** — after writing a doc, scan for placeholders, contradictions, scope drift, ambiguity. Required for Spec, Architect, Planning, Doc.
- **Verification before completion** — every agent ends with verification.
- **Process-flow DOT diagrams** — for state machines in agent and skill docs.

### Terminology

- **"Your human partner"** — not "the user." Deliberate.
- **Maintainer** — the human who owns the project; singular for v0.1.
- **Agent** — a defined role (system prompt + tool allowlist + scope). Implemented as a Claude Code **subagent**. Use "agent" for the concept; "subagent" only when emphasizing the dispatch mechanism.
- **Shell** — the maintainer's top-level Claude Code session.
- **Pipeline** — spec → gap → PM → plan → test → implement → review → doc.
- **Dispatch** — invoking a subagent (via slash command or hook).
- **Gap** — a disagreement between spec and docs/code, surfaced by Gap Analysis.

### Skills are behavior-shaping code, not prose

Changes to Red Flags tables, rationalization lists, and "EXTREMELY_IMPORTANT" framing need eval evidence. This is why we **fold-and-delete** rather than rewrite inherited skills in place when they don't fit the new architecture — folding into a new agent prompt is a fresh write, not a rewrite of an eval-tested skill.

### TDD is RED-GREEN-REFACTOR

Test Dev writes failing tests; Implementation drives green then refactors. Test Dev's tool allowlist excludes source paths, so it cannot cheat by writing source code — the discipline is mechanism-enforced.

### Process before implementation

In the pipeline, Spec / Architect / Gap Analysis / PM / Planning all run upstream of Test Dev and Implementation. Don't shortcut to Implementation when an upstream gap is the actual problem. Use `systematic-debugging` before reaching for fixes.

### One problem per branch

Describe the problem, not just the change. Branches are `feat/<issue-number>-<slug>` and are merged (not deleted) by Review.

## Working in this repo

- Specs live under `docs/specs/` (currently empty; the first spec is the Dwarven plugin's own behavior — to be written).
- Plans live under `docs/plans/` (currently empty).
- Architecture docs live under `docs/architecture/` (currently empty).
- The `using-dwarven` skill is auto-loaded by the SessionStart hook (`hooks/hooks.json`) — currently the inherited version; rewrite pending.
- A scratchpad at `prompts/todo.local.md` (gitignored) captures the maintainer's working notes; do not cite as authoritative.

## Pointers

- `README.md` — public guiding document with the full architecture
- `agents/` — agent definitions (subagent system prompts, tool allowlists)
- `skills/` — cross-cutting skills (per disposition table above)
- `commands/` — slash command definitions
- `hooks/` — SessionStart and PreToolUse hooks
- `docs/specs/` — what the system should do (per major version)
- `docs/architecture/` — how the system should solve the spec
- `docs/plans/` — per-issue implementation plans
- `docs/CHANGELOG.md` — change log
- `.claude/settings.json` — per-agent tool/path permissions
