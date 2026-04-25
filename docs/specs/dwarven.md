---
spec_version: 0.1.0
last_updated: 2026-04-24
related_architecture: (none yet)
---

# Dwarven — Specification

This document is the formal specification for the Dwarven Claude Code plugin. It describes how Dwarven *should* work. The implementation may lag this specification; the gap between the two is recorded in `docs/CHANGELOG.md` and tracked as `type:spec-gap` issues.

This spec is itself written in the format Dwarven prescribes for projects using it: a single Markdown file under `docs/specs/`, with numbered requirements and a versioned frontmatter. Eat-our-own-dogfood.

**Conventions in this document.**

- Each section is identified `R<n>`. Each numbered claim is identified `R<n>.<m>`. Tests, plans, and `type:spec-gap` issues cite requirements by ID.
- "Must" and "may" carry their ordinary English force. RFC 2119 keywords are not used.
- Every requirement is independently verifiable from the codebase or from GitHub state. Where verification requires external state (e.g., GitHub branch protection settings), the verifier is named.
- Where this spec and the README disagree, this spec is correct and the README is the bug.

---

## R1 — Identity & purpose

R1.1 — Dwarven is a Claude Code plugin for specification-driven software development.

R1.2 — Dwarven orchestrates a fixed roster of focused agents that develop software against a written specification.

R1.3 — Dwarven targets project maintainers using Claude Code as their primary development environment.

R1.4 — The problem Dwarven solves: keeping intent (the spec), behavior (the docs), and code synchronized through mechanism-enforced agent decoupling and GitHub-state coordination.

R1.5 — Dwarven targets Claude Code only. There is no cross-harness abstraction.

---

## R2 — Core invariants

R2.1 — Specifications drive everything. A spec describes how the system *should* work; documentation describes how it *does* work; the disagreement between them is a *gap*.

R2.2 — Tests gatekeep implementation. Tests are the executable form of the spec. Implementation must not land without tests that verify it satisfies the spec.

R2.3 — Architecture is a distinct concern from specification. A spec says what the system should do; an architecture says how the system should solve it.

R2.4 — Decoupling is mechanism-enforced, not conventional. Each agent must be implemented as a Claude Code subagent with its own system prompt and tool allowlist.

R2.5 — An agent must not change roles mid-session. Cross-agent handoff happens through GitHub state or through bounded subagent dispatch (`Agent` tool).

R2.6 — GitHub is the only inter-agent coordination surface. Inter-agent state lives in issues, labels, comments, PRs, and releases. There is no other shared state.

R2.7 — Monorepo workflows are preferred but not required. Dwarven assumes spec, docs, code, and tests live in a single workspace.

---

## R3 — The agent roster

The v0.1 roster has ten agents, each defined as a Claude Code subagent under `agents/<name>.md`. The maintainer dispatches each agent via its slash command from the shell (R4) or, in v0.2+, by detached dispatch (R11.1).

### R3.1 — System Specification (`/spec`)

R3.1.1 — Trigger: interactive only. The agent must never be subject to detached dispatch (per R11.1). The maintainer invokes `/spec [topic]` from the shell.

R3.1.2 — Inputs (informational): `docs/specs/*.md`; `docs/CHANGELOG.md`; open issues with `type:spec-gap`; `docs/architecture/*.md` (to avoid conflicting with design).

R3.1.3 — Outputs (informational): edits to `docs/specs/*.md`; appended entries in `docs/CHANGELOG.md`; comments on / closures of `type:spec-gap` issues that the spec change resolves.

R3.1.4 — Tool allowlist must include: `Read`, `Write`, `Edit`, `AskUserQuestion`, `Agent`, scoped `Bash(git status|diff|log|add|commit:*)`, scoped `Bash(git push origin main:*)`, scoped `Bash(gh issue view|list|comment|close:*)`.

R3.1.5 — Tool allowlist must exclude: any `Edit`/`Write` pattern matching paths outside `docs/specs/` and `docs/CHANGELOG.md`; PR creation, mutation, or merge; label management beyond own issues; `Bash(git push origin feat/*)`.

R3.1.6 — Exit conditions: a spec change has been committed and pushed to `main` and `docs/CHANGELOG.md` has been updated; or the maintainer indicates the session is complete.

R3.1.7 — Scope fences: must not write outside `docs/specs/` and `docs/CHANGELOG.md`; must not write code or tests; must not author architecture (must surface design questions as `type:arch` issues for the Architect); must not decompose specifications into implementation work.

### R3.2 — Architect (`/architect`)

R3.2.1 — Trigger: interactive only. The maintainer invokes `/architect [topic]` from the shell.

R3.2.2 — Inputs (informational): `docs/specs/*.md`; `docs/architecture/*.md`; open issues with `type:arch`.

R3.2.3 — Outputs (informational): edits to `docs/architecture/*.md`; comments on / closures of `type:arch` issues; new `type:arch` or `type:spec-gap` issues to surface design or spec ambiguity.

R3.2.4 — Tool allowlist must include: `Read`, `Write`, `Edit`, `AskUserQuestion`, `Agent`, scoped `Bash(git status|diff|log|add|commit:*)`, scoped `Bash(git push origin main:*)`, scoped `Bash(gh issue view|list|create|comment|close:*)`.

R3.2.5 — Tool allowlist must exclude: writes outside `docs/architecture/`; PR mutation; label management beyond own issues.

R3.2.6 — Exit conditions: an architecture document has been committed and pushed to `main`; or the maintainer ends the session.

R3.2.7 — Scope fences: must not write code, tests, specs, plans, or product docs; must not modify a specification — instead surface ambiguity as a `type:spec-gap` issue for the System Specification agent.

### R3.3 — Specification Gap Analysis (`/gap`)

R3.3.1 — Trigger: interactive (`/gap [scope]`) or detached (per R11.1, on spec change; v0.2+).

R3.3.2 — Inputs (informational): `docs/specs/*.md`; `docs/*.md`; source code; `docs/architecture/*.md`; existing open issues (for deduplication).

R3.3.3 — Outputs (informational): new issues with labels `type:spec-gap` and `agent:pm`; comments on existing gap issues when evidence changes.

R3.3.4 — Tool allowlist must include: `Read`, `AskUserQuestion` (interactive path only), scoped `Bash(git log|diff:*)`, scoped `Bash(gh issue list|view|create|comment|search:*)`.

R3.3.5 — Tool allowlist must exclude: `Write`, `Edit`; any `gh` mutation beyond issue create / comment; PR operations; label management beyond own issues.

R3.3.6 — Exit conditions: requested scope has been scanned and a summary has been reported (interactive) or the new-issue count has been logged (detached).

R3.3.7 — Scope fences: must not modify any file; must not decompose issues; must not implement.

### R3.4 — Project Management (`/pm`)

R3.4.1 — Trigger: interactive (`/pm [issue-number]`) or detached (per R11.1, on new `agent:pm` issue; v0.2+).

R3.4.2 — Inputs (informational): the parent issue (with `agent:pm`); related specs and architecture; existing epic groupings.

R3.4.3 — Outputs (informational): child issues with `type:feature` + `agent:plan` + optional `epic:<slug>`; comments on parent updating with the child list; label swaps on the parent.

R3.4.4 — Tool allowlist must include: `Read`, `AskUserQuestion` (interactive path only), scoped `Bash(git log:*)`, scoped `Bash(gh issue view|list|create|edit|comment|close:*)`.

R3.4.5 — Tool allowlist must exclude: `Write`, `Edit`; any `gh` mutation beyond issue ops.

R3.4.6 — Exit conditions: parent decomposed and children created; or parent escalated to `agent:maintainer` with `blocker:maintainer-input`.

R3.4.7 — Scope fences: must not write files; must not plan implementation details; must not implement; must not touch issues outside its own queue.

### R3.5 — Implementation Planning (`/plan`)

R3.5.1 — Trigger: interactive (`/plan [issue-number]`) or detached (per R11.1, on `agent:plan`; v0.2+).

R3.5.2 — Inputs (informational): the owning issue; related specs and architecture; codebase.

R3.5.3 — Outputs (informational): `docs/plans/YYYY-MM-DD-<slug>.md` committed and pushed to `main`; a comment on the issue with the plan path; label swap to `agent:test`.

R3.5.4 — Tool allowlist must include: `Read`, `Write`, `Edit` (restricted to `docs/plans/`), `AskUserQuestion`, scoped `Bash(git status|diff|log|add|commit:*)`, scoped `Bash(git push origin main:*)`, scoped `Bash(gh issue view|comment|edit:*)`.

R3.5.5 — Tool allowlist must exclude: writes outside `docs/plans/`; PR operations; issue creation.

R3.5.6 — Exit conditions: a plan has been committed and pushed to `main` and the issue label has been swapped to `agent:test`.

R3.5.7 — Scope fences: must not write code or tests; must not modify specs or architecture; must not file new issues (escalate via `agent:maintainer` if needed).

### R3.6 — Test Development (`/test`)

R3.6.1 — Trigger: interactive (`/test [issue-number]`) or detached (per R11.1, on `agent:test`; v0.2+).

R3.6.2 — Inputs (informational): the owning issue; the plan (`docs/plans/...md`); related specs; existing tests.

R3.6.3 — Outputs (informational): test files only (paths matching the project's test-path patterns, codified in repo configuration; defaults: `tests/**`, `**/*.test.*`, `**/*_test.*`, `**/*.spec.*`); failing tests committed on branch `feat/<issue-number>-<slug>`; label swap to `agent:implement`.

R3.6.4 — Tool allowlist must include: `Read`, `Write`, `Edit` (restricted to test paths), scoped `Bash(<test-runner>:*)`, scoped `Bash(git status|diff|log|add|commit|push origin feat/*|branch|checkout:*)`, scoped `Bash(gh issue edit|comment:*)`.

R3.6.5 — Tool allowlist must exclude: writes to non-test paths; PR creation or mutation; `git push origin main`; `AskUserQuestion`; `Agent`.

R3.6.6 — Exit conditions: failing tests have been committed; the branch has been pushed; the issue label has been swapped to `agent:implement`. The committed test run must fail before exit (verifying RED).

R3.6.7 — Scope fences: must not write source code; must not make tests pass; must not modify plan, specs, or docs; must not create issues.

### R3.7 — Implementation (`/implement`)

R3.7.1 — Trigger: interactive (`/implement [issue-number]`) or detached (per R11.1, on `agent:implement`; v0.2+).

R3.7.2 — Inputs (informational): the owning issue; the plan; the failing tests on the branch; related specs; codebase.

R3.7.3 — Outputs (informational): source code changes (paths excluding `docs/specs/`, `docs/architecture/`, `docs/plans/`, `docs/CHANGELOG.md`); test extensions allowed only to strengthen coverage (existing failing tests must still fail until implementation makes them pass); a PR opened against `main` and linked to the issue (`Closes #N`); label swap to `agent:review`.

R3.7.4 — Tool allowlist must include: `Read`, `Write`, `Edit` (restricted to source + test paths), scoped `Bash(<build|test>:*)`, scoped `Bash(git status|diff|log|add|commit|push origin feat/*|checkout:*)`, scoped `Bash(gh pr create|view:*)`, scoped `Bash(gh issue edit|comment:*)`.

R3.7.5 — Tool allowlist must exclude: writes to spec, architecture, plan, or changelog paths; `git push origin main`; PR merge; issue creation; `AskUserQuestion`; `Agent`.

R3.7.6 — Exit conditions: tests pass (RED → GREEN → REFACTOR complete); a PR has been opened; the issue label has been swapped to `agent:review`.

R3.7.7 — Scope fences: must not modify specs, architecture, or plans; must not delete existing failing tests; must not weaken existing tests; must not file new issues except by escalation (park as `agent:maintainer` + `blocker:*` and exit).

### R3.8 — Code Review (`/review`)

R3.8.1 — Trigger: interactive (`/review [pr-number]`) or detached (per R11.1, on PR opened by Implementation; v0.2+).

R3.8.2 — Inputs (informational): PR diff; the owning issue; the plan; relevant specs; tests; source history.

R3.8.3 — Outputs (informational): PR review comments via `gh pr review` or `gh pr comment`; on approve → PR merged + issue label swapped to `agent:doc`; on changes-requested → issue label swapped back to `agent:implement` (or to `agent:plan` if the plan itself is flawed).

R3.8.4 — Tool allowlist must include: `Read`, scoped `Bash(git diff|log:*)`, scoped `Bash(gh pr view|review|comment|merge|checks:*)`, scoped `Bash(gh issue edit|comment:*)`.

R3.8.5 — Tool allowlist must exclude: `Write`, `Edit`; any `gh` mutation beyond PR + issue ops; `AskUserQuestion`; `Agent`.

R3.8.6 — Exit conditions: a review has been posted, leaving the issue in a routed state (approved + merged with `agent:doc`, or changes-requested with `agent:implement` / `agent:plan`).

R3.8.7 — Scope fences: must not write code, tests, specs, plans, or docs; must not approve a PR whose author is itself.

### R3.9 — System Documentation (`/doc`)

R3.9.1 — Trigger: interactive (`/doc [issue-number | scope]`) or detached (per R11.1, on merged PR whose owning issue has `agent:doc`; v0.2+).

R3.9.2 — Inputs (informational): the merged PR diff; the owning issue; relevant specs; current docs.

R3.9.3 — Outputs (informational): edits to `docs/*.md` excluding `docs/specs/`, `docs/architecture/`, `docs/plans/`, and `docs/CHANGELOG.md`; an appended entry in `docs/CHANGELOG.md` (patch level); closure of the originating issue.

R3.9.4 — Tool allowlist must include: `Read`, `Write`, `Edit` (restricted to allowed docs paths), scoped `Bash(git status|diff|log|add|commit:*)`, scoped `Bash(git push origin main:*)`, scoped `Bash(gh issue view|close|comment:*)`.

R3.9.5 — Tool allowlist must exclude: writes to specs, architecture, or plans; source code modification; issue reopening; `AskUserQuestion`; `Agent`.

R3.9.6 — Exit conditions: docs have been committed and pushed to `main` (including the appended CHANGELOG entry); the originating issue has been closed.

R3.9.7 — Scope fences: must not modify specs, architecture, or plans; must not modify code; must not re-open closed issues.

### R3.10 — Triage (`/triage`)

R3.10.1 — Trigger: interactive (`/triage`) or scheduled (per R11.1, configurable cadence; v0.2+).

R3.10.2 — Inputs (informational): all open issues; all labels; recent PR activity.

R3.10.3 — Outputs (informational): label fixes on mislabeled issues (missing `agent:*`, missing `type:*`, multi-`agent:` violations); `agent:maintainer` + `blocker:*` swaps on stale issues (no `agent:*` change for N configured days); a `type:chore` triage report comment or issue.

R3.10.4 — Tool allowlist must include: `Read`, scoped `Bash(gh issue list|view|edit|comment:*)`, scoped `Bash(gh pr list:*)`.

R3.10.5 — Tool allowlist must exclude: `Write`, `Edit`; substantive issue creation (only the triage-report chore is permitted); auto-close (except an explicitly-configured stale policy); `AskUserQuestion`; `Agent`.

R3.10.6 — Exit conditions: queue audited; triage report posted.

R3.10.7 — Scope fences: must not write files; must not create substantive issues; must not reassign work across agents without maintainer approval.

---

## R4 — The maintainer shell

The shell is the top-level Claude Code session the maintainer dispatches from. It holds broad orientation context but no work-specific state.

R4.1 — The shell is a top-level Claude Code session, not a subagent.

R4.2 — The shell's tool allowlist must include: `Agent`, `Read`, `AskUserQuestion`, scoped read-only `Bash(git log|diff|status|show:*)`, scoped read-only `Bash(gh issue list|view:*)`, scoped read-only `Bash(gh pr list|view:*)`, scoped read-only `Bash(gh label list:*)`.

R4.3 — The shell's tool allowlist must exclude: `Write`, `Edit`, `NotebookEdit`; any mutating `Bash(git ...)` pattern; any mutating `Bash(gh ...)` pattern.

R4.4 — All actions that mutate state are dispatched to an agent via the `Agent` tool. The shell itself never mutates state.

R4.5 — Dispatch is via explicit slash commands. Bare maintainer messages without a slash command produce read-based orientation only — the shell must not dispatch without an explicit slash command.

R4.6 — The shell does not hold multi-turn dialogue about substantive work. Such dialogue happens inside dispatched agents.

---

## R5 — Coordination protocol

### R5.1 — Labels

R5.1.1 — Every open issue must carry exactly one `agent:*` label. Permitted values: `agent:spec`, `agent:architect`, `agent:gap`, `agent:pm`, `agent:plan`, `agent:test`, `agent:implement`, `agent:review`, `agent:doc`, `agent:triage`, `agent:maintainer`.

R5.1.2 — Every open issue must carry exactly one `type:*` label. Permitted values: `type:spec-gap`, `type:feature`, `type:bug`, `type:arch`, `type:doc`, `type:chore`.

R5.1.3 — Optional labels: `blocker:*` (`maintainer-input`, `external`, `upstream`); `priority:*` (`p0`, `p1`, `p2`); `epic:<slug>` for grouping.

R5.1.4 — Single-owner rule: an issue must have at most one `agent:*` label at any time.

R5.1.5 — Label-write enforcement is prompt-level, not mechanism-level. Discipline relies on agent system prompts and post-hoc Triage audit. This asymmetry is acceptable: mislabeled issues are recoverable; misbehaved code is not.

### R5.2 — Branches

R5.2.1 — Implementation branches must be named `feat/<issue-number>-<slug>`.

R5.2.2 — Test Development creates the branch; Implementation carries it; Code Review merges (and does not delete) on approval.

R5.2.3 — Each issue has at most one corresponding branch.

### R5.3 — Commits

R5.3.1 — Commit messages follow Conventional Commits formatting.

R5.3.2 — Commit messages include the owning issue number reference (e.g., `feat(auth): add MFA (#42)`).

### R5.4 — Pull requests

R5.4.1 — PR titles reference the owning issue number.

R5.4.2 — PR bodies link to the plan path.

R5.4.3 — PR bodies use `Closes #N` to auto-link the originating issue.

### R5.5 — Direct-to-`main` rules

R5.5.1 — Direct commits to `main` are permitted for the System Specification, Architect, Implementation Planning, and System Documentation agents only.

R5.5.2 — All other agents must commit to a `feat/*` branch and submit changes via PR through the Code Review agent.

### R5.6 — GitHub interaction

R5.6.1 — All GitHub interaction uses the `gh` CLI exclusively. No MCP GitHub server dependency.

R5.6.2 — Each agent's `Bash` allowlist scopes `gh` patterns to the specific operations the agent requires.

---

## R6 — Permission model

R6.1 — Tool allowlists are declared in subagent frontmatter (`agents/<name>.md`) and reinforced in `.claude/settings.json` patterns.

R6.2 — Path scoping for `Write` and `Edit` is enforced via tool patterns where Claude Code supports it; system-prompt rules and PreToolUse hooks fill any gaps.

R6.3 — `AskUserQuestion` may appear in the allowlist of: the maintainer shell; System Specification; Architect; Specification Gap Analysis; Project Management; Implementation Planning. It must not appear in the allowlist of: Test Development; Implementation; Code Review; System Documentation; Triage.

R6.4 — The `Agent` tool may appear in the allowlist of: the maintainer shell; System Specification; Architect. It must not appear in the allowlist of any other agent.

R6.5 — The hard "never" list applies to every agent. The following `Bash` patterns must never appear in any agent's allowlist:

- `git push --force`, `git push -f`, `git push --force-with-lease`
- `git reset --hard`
- `git checkout -- .`, `git restore .`
- `git clean -f*`
- `rm -rf*`
- `gh repo *` (any mutating repo-level configuration)
- `gh release delete *`
- `git push origin main` (except for System Specification, Architect, Implementation Planning, and System Documentation per R5.5)

---

## R7 — Skills model

R7.1 — Cross-cutting skills are defined under `skills/<name>/SKILL.md`. They are invoked by an agent (or the shell) via the `Skill` tool when the skill description matches the task.

R7.2 — The v0.1 cross-cutting skill set is: `dispatching-parallel-agents`; `systematic-debugging`; `test-driven-development`; `using-dwarven`; `using-git-worktrees`; `verification-before-completion`; `writing-skills`.

R7.3 — `using-dwarven` is auto-loaded by the SessionStart hook and is the entry point for any Dwarven session.

R7.4 — Inherited skills incompatible with the v0.1 architecture are folded into the appropriate agent's system prompt and deleted as standalone skills, rather than rewritten in place. Rationale: in-place rewrites of behavior-shaping content require eval evidence that does not exist.

R7.5 — Behavior-shaping content (Red Flags tables, rationalization lists, EXTREMELY-IMPORTANT framing) is treated as code: changes require evidence, not opinion.

---

## R8 — Repository setup

R8.1 — The `repository-setup` skill scaffolds a target repository to use Dwarven.

R8.2 — In init mode, `repository-setup` creates: `docs/specs/`, `docs/architecture/`, `docs/plans/`, `docs/CHANGELOG.md`; the v0.1 label set per R5.1; `.github/ISSUE_TEMPLATE/` (one per filable `type:*`); `.github/PULL_REQUEST_TEMPLATE.md`; `.claude/settings.json` with per-agent permissions per R6; slash command registrations per R3; `main` branch protection (require PR review); SessionStart and PreToolUse hooks.

R8.3 — In retrofit mode, `repository-setup` detects existing structure and merges non-destructively. The skill must print a diff of intended changes and require explicit confirmation before any write.

R8.4 — `repository-setup` must not delete or overwrite existing maintainer content without explicit confirmation.

---

## R9 — Spec versioning model

R9.1 — For v0.1, specifications live flat at `docs/specs/*.md`.

R9.2 — `docs/CHANGELOG.md` tracks all changes from v0.1.0 forward.

R9.3 — On the first breaking spec change, the existing `docs/specs/*` content migrates to `docs/specs/v1/`, and the new major version goes to `docs/specs/v2/`.

R9.4 — Minor spec versions are recorded as inline annotations within the spec files.

R9.5 — Patch-level changes (typically documentation reflecting code) are tracked in `docs/CHANGELOG.md` only.

R9.6 — The migration from flat to versioned directories is a `git mv`. No automated tooling is required until v0.2 actually exists.

---

## R10 — Scope boundaries

R10.1 — Dwarven is not a CI system. CI workflow files are an output of `repository-setup` in v0.2+; the plugin does not run CI internally.

R10.2 — Dwarven is not a code generator. Agents write code derived from specs and plans; they do not template or scaffold project source from prompts alone.

R10.3 — Dwarven is not opinionated about test runners, build tools, languages, or frameworks. Test paths and build commands are configured per repository.

R10.4 — Dwarven is not a replacement for human judgment. The maintainer remains the source of truth for scope, priority, and acceptance.

R10.5 — Dwarven does not enforce naming conventions beyond those required for agent coordination (label names per R5.1, branch prefix per R5.2.1, commit format per R5.3). Project-internal naming is the project's concern.

---

## R11 — v0.2+ markers

R11.1 — *Detached dispatch* — automated agent invocation via GitHub Actions workflows on repository events (`issues.labeled`, `pull_request.opened`, and a Triage schedule) — is targeted for v0.2. References to "detached" triggers in R3 presume this mechanism.

R11.2 — A `claude-md-management` skill, helping projects keep agent-aware per-repo `CLAUDE.md` fragments, is targeted for v0.2.

R11.3 — Release automation (version bump, registry publish) will be reintroduced when the agent roster is stable.
