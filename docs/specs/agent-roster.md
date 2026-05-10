---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Agent Roster

This document specifies the fixed roster of ten agents: trigger, inputs, outputs, tool allowlist, exit conditions, scope fences, and supported dialogue mode for each.

The pipeline is `spec → gap → pm → plan → test → implement → review → doc`. Triage runs orthogonally to audit the queue. The maintainer's shell dispatches each agent via slash command (`host-adapter.md`).

This spec specifies the agent contracts in host-agnostic form. The per-host adapter (`host-adapter.md`) maps these contracts to host primitives (e.g., Claude Code subagent definitions).

---

## R1 — Scope

R1.1 — This spec defines: the ten agent roles, their I/O contracts, their tool allowlists, and the universal allowlist constraints that apply to all agents.

R1.2 — Out of scope: per-host implementation of agents (`host-adapter.md`); the on-disk artifact format (`storage-model.md`); the CLI command surface (`dwarven-cli.md`); the dialogue protocol (`dialogue.md`).

R1.3 — The roster is fixed at ten in v1. Adding an eleventh role is a spec change.

---

## R2 — Shared conventions

R2.1 — Each agent supports detached dispatch (`dialogue.md#R8.1`). A subset additionally supports interactive dispatch and the host's interactive question primitive (R2.2).

R2.2 — **Dialogue agents**: Spec, Architect, Gap, PM, Planning. Their tool allowlist may include the host's interactive question primitive (e.g., Claude Code's `AskUserQuestion`).

R2.3 — **Discrete-work agents**: Test Dev, Implementation, Code Review, Doc, Triage. Their tool allowlist must exclude the host's interactive question primitive.

R2.4 — Allowlists below are written as `Bash(<pattern>:*)` style. Per-host adapters translate to the host's pattern syntax. Patterns are prefix-matched.

R2.5 — Every agent reads from `.dwarven/issues/` directly (with `Read`) and may also use `dwarven issue view` / `dwarven issue list` for index-friendly queries. Both paths are equally supported (`storage-model.md#R7.1`).

R2.6 — Every agent ends its work by transitioning the issue to the next state (or terminating it) via `dwarven issue transition` or `dwarven issue close`. Exiting without a transition leaves the issue in an inconsistent state and is forbidden.

R2.7 — Branches use the `feat/<id>-<slug>` convention. Test Dev creates the branch; Implementation carries it; Code Review merges (and does not delete) on approval.

R2.8 — Commit messages follow Conventional Commits and reference the owning issue ID (e.g., `feat(auth): add MFA (#42)`).

---

## R3 — Spec (`/spec`)

R3.1 — **Trigger.** Interactive only. The maintainer invokes `/spec [topic]` from the shell. The agent must never be subject to detached dispatch — spec changes require live dialogue with the maintainer.

R3.2 — **Inputs.** `docs/specs/*.md`; `docs/CHANGELOG.md`; open issues with `type: spec-gap`; `docs/architecture/*.md` (to avoid conflicting with locked design).

R3.3 — **Outputs.** Edits to `docs/specs/*.md`; appended entries in `docs/CHANGELOG.md`; comments on or closure of `type: spec-gap` issues that the spec change resolves.

R3.4 — **Tool allowlist must include.** `Read`, `Write`, `Edit`, `AskUserQuestion`, `Agent`, `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git add:*)`, `Bash(git commit:*)`, `Bash(git push origin main:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue list:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue close:*)`.

R3.5 — **Tool allowlist must exclude.** Any `Edit` or `Write` pattern outside `docs/specs/` and `docs/CHANGELOG.md`; `Bash(git push origin feat/*:*)`; any `dwarven issue create`, `dwarven issue transition`, `dwarven issue dep`, `dwarven issue blocker`, `dwarven issue priority`, `dwarven issue edit` patterns.

R3.6 — **Exit conditions.** A spec change has been committed and pushed to `main`, `docs/CHANGELOG.md` has been updated, and any resolved `type: spec-gap` issues have been closed; or the maintainer indicates the session is complete with no spec change.

R3.7 — **Scope fences.** Must not write outside `docs/specs/` and `docs/CHANGELOG.md`. Must not write code or tests. Must not author architecture (surface design questions as `type: arch` issues for the Architect). Must not decompose specifications into implementation work.

---

## R4 — Architect (`/architect`)

R4.1 — **Trigger.** Interactive only. The maintainer invokes `/architect [topic]`.

R4.2 — **Inputs.** `docs/specs/*.md`; `docs/architecture/*.md`; open issues with `type: arch`.

R4.3 — **Outputs.** Edits to `docs/architecture/*.md`; comments on or closure of `type: arch` issues; new `type: arch` or `type: spec-gap` issues to surface design or spec ambiguity.

R4.4 — **Tool allowlist must include.** `Read`, `Write`, `Edit`, `AskUserQuestion`, `Agent`, `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git add:*)`, `Bash(git commit:*)`, `Bash(git push origin main:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue list:*)`, `Bash(dwarven issue create:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue close:*)`.

R4.5 — **Tool allowlist must exclude.** Writes outside `docs/architecture/`; `Bash(git push origin feat/*:*)`; `dwarven issue transition`, `dwarven issue edit`, `dwarven issue dep`, `dwarven issue blocker`, `dwarven issue priority`.

R4.6 — **Exit conditions.** An architecture document has been committed and pushed to `main`; or the maintainer ends the session.

R4.7 — **Scope fences.** Must not write code, tests, specs, plans, or product docs. Must not modify a specification — surface ambiguity as a `type: spec-gap` issue.

---

## R5 — Gap (`/gap`)

R5.1 — **Trigger.** Interactive (`/gap [scope]`) or detached (on spec/code change; v1 supports detached dispatch via the hub's protocol — see `dialogue.md#R2.4`).

R5.2 — **Inputs.** `docs/specs/*.md`; documentation (`docs/*.md` outside `specs/`, `architecture/`, `plans/`); source code; existing open issues (for deduplication).

R5.3 — **Outputs.** New issues with `type: spec-gap` and default initial state per `work-states.md#R6.1` (`pm`); comments on existing gap issues when evidence changes.

R5.4 — **Tool allowlist must include.** `Read`, `AskUserQuestion` (interactive path only — gated by host adapter when detached), `Bash(git log:*)`, `Bash(git diff:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue list:*)`, `Bash(dwarven issue create:*)`, `Bash(dwarven issue comment:*)`.

R5.5 — **Tool allowlist must exclude.** `Write`, `Edit`; `dwarven issue transition`, `dwarven issue close`, `dwarven issue edit`, `dwarven issue dep`, `dwarven issue blocker`, `dwarven issue priority`; any `git add`, `git commit`, `git push`.

R5.6 — **Exit conditions.** Requested scope has been scanned and a summary has been posted (interactive) or the new-issue count has been logged via the dispatch context (detached).

R5.7 — **Scope fences.** Must not modify any file. Must not decompose issues. Must not implement.

---

## R6 — PM (`/pm`)

R6.1 — **Trigger.** Interactive (`/pm [issue-id]`) or detached (on issues entering `state: pm`).

R6.2 — **Inputs.** The parent issue (currently `state: pm`); related specs and architecture; existing epic groupings.

R6.3 — **Outputs.** Child issues with `type: feature` (or `bug`/`chore` as appropriate) and default initial state `plan`, optionally tagged with the parent's `epic`; comments on the parent updating with the child list; the parent transitioned to `state: done` (children carry the work) per `work-states.md#R6.2`.

R6.4 — **Tool allowlist must include.** `Read`, `AskUserQuestion` (interactive path only), `Bash(git log:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue list:*)`, `Bash(dwarven issue create:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue edit:*)`, `Bash(dwarven issue transition:*)`, `Bash(dwarven issue close:*)`, `Bash(dwarven issue dep:*)`.

R6.5 — **Tool allowlist must exclude.** `Write`, `Edit`; `dwarven issue priority` (priority is maintainer-only); `dwarven issue blocker`; `git add`, `git commit`, `git push`.

R6.6 — **Exit conditions.** Parent decomposed and children created with dependency edges from each child to the parent; parent transitioned to `state: done`. Or parent escalated via `dialogue.md#R4` if decomposition requires maintainer input.

R6.7 — **Scope fences.** Must not write files. Must not plan implementation details (Planning's job). Must not implement. Must not touch issues outside its decomposition tree.

---

## R7 — Planning (`/plan`)

R7.1 — **Trigger.** Interactive (`/plan [issue-id]`) or detached.

R7.2 — **Inputs.** The owning issue (currently `state: plan`); related specs and architecture; codebase.

R7.3 — **Outputs.** `docs/plans/YYYY-MM-DD-<slug>.md` committed and pushed to `main`; a comment on the issue with the plan path; transition to `state: test`.

R7.4 — **Tool allowlist must include.** `Read`, `Write` and `Edit` (restricted to `docs/plans/`), `AskUserQuestion`, `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git add:*)`, `Bash(git commit:*)`, `Bash(git push origin main:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue transition:*)`.

R7.5 — **Tool allowlist must exclude.** Writes outside `docs/plans/`; `Bash(git push origin feat/*:*)`; `dwarven issue create`, `dwarven issue edit`, `dwarven issue close`, `dwarven issue dep`, `dwarven issue blocker`, `dwarven issue priority`.

R7.6 — **Exit conditions.** A plan has been committed and pushed to `main` and the issue has been transitioned to `state: test`.

R7.7 — **Scope fences.** Must not write code or tests. Must not modify specs or architecture. Must not file new issues (escalate via `dialogue.md#R4` if needed).

---

## R8 — Test Dev (`/test`)

R8.1 — **Trigger.** Interactive (`/test [issue-id]`) or detached.

R8.2 — **Inputs.** The owning issue (currently `state: test`); the plan referenced by the issue; related specs; existing tests.

R8.3 — **Outputs.** Test files only (paths matching the project's test-path patterns, codified in repository configuration; defaults: `tests/**`, `**/*.test.*`, `**/*_test.*`, `**/*.spec.*`); failing tests committed on branch `feat/<id>-<slug>`; transition to `state: implement`.

R8.4 — **Tool allowlist must include.** `Read`, `Write` and `Edit` (restricted to test paths), `Bash(<test-runner>:*)` (per-project), `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git branch:*)`, `Bash(git checkout:*)`, `Bash(git add:*)`, `Bash(git commit:*)`, `Bash(git push origin feat/*:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue transition:*)`.

R8.5 — **Tool allowlist must exclude.** Writes to non-test paths; `Bash(git push origin main:*)`; `AskUserQuestion`; `Agent`; `dwarven issue create`, `dwarven issue close`, `dwarven issue edit`, `dwarven issue dep`, `dwarven issue priority`.

R8.6 — **Exit conditions.** Failing tests have been committed; the branch has been pushed; the issue has been transitioned to `state: implement`. The committed test run must fail before exit (verifying RED).

R8.7 — **Scope fences.** Must not write source code. Must not make tests pass. Must not modify the plan, specs, or docs. Must not create issues — structural escalation only via `dialogue.md#R4`.

---

## R9 — Implementation (`/implement`)

R9.1 — **Trigger.** Interactive (`/implement [issue-id]`) or detached.

R9.2 — **Inputs.** The owning issue (currently `state: implement`); the plan; the failing tests on the branch; related specs; codebase.

R9.3 — **Outputs.** Source code changes (paths excluding `docs/specs/`, `docs/architecture/`, `docs/plans/`, `docs/CHANGELOG.md`); test extensions allowed only to strengthen coverage (existing failing tests must still fail until implementation makes them pass); a comment on the issue with the branch name and a summary of the change; transition to `state: review`.

R9.4 — **Tool allowlist must include.** `Read`, `Write` and `Edit` (restricted to source + test paths), `Bash(<build|test>:*)` (per-project), `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git checkout:*)`, `Bash(git add:*)`, `Bash(git commit:*)`, `Bash(git push origin feat/*:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue transition:*)`, `Bash(dwarven issue blocker:*)`.

R9.5 — **Tool allowlist must exclude.** Writes to spec, architecture, plan, or changelog paths; `Bash(git push origin main:*)`; `AskUserQuestion`; `Agent`; `dwarven issue create`, `dwarven issue close`, `dwarven issue edit`, `dwarven issue dep`, `dwarven issue priority`.

R9.6 — **Exit conditions.** Tests pass (RED → GREEN → REFACTOR complete); the branch is pushed; a summary comment has been posted with the branch name; the issue has been transitioned to `state: review`.

R9.7 — **Scope fences.** Must not modify specs, architecture, or plans. Must not delete existing failing tests. Must not weaken existing tests. Must not file new issues — escalate structurally via `dialogue.md#R4` (set blocker, transition to `state: maintainer`).

---

## R10 — Code Review (`/review`)

R10.1 — **Trigger.** Interactive (`/review [issue-id]`) or detached.

R10.2 — **Inputs.** The branch's diff against `main` (`git diff main..feat/<id>-<slug>`); the owning issue; the plan; relevant specs; tests; source history.

R10.3 — **Outputs.** Review comments posted to the issue. On approve: merge the branch into `main` (`git merge --ff-only` preferred; `git merge --no-ff` if non-fast-forward), push `main`, transition issue to `state: doc`. On changes-requested: comment with feedback, transition issue back to `state: implement` (or to `state: plan` if the plan itself is flawed).

R10.4 — **Tool allowlist must include.** `Read`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git checkout:*)`, `Bash(git merge:*)`, `Bash(git push origin main:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue transition:*)`.

R10.5 — **Tool allowlist must exclude.** `Write`, `Edit`; `Bash(git push origin feat/*:*)` (Code Review does not push to feature branches); `Bash(git branch -d:*)` and `Bash(git push origin --delete:*)` (branches are not deleted on merge — `R2.7`); `AskUserQuestion`; `Agent`; `dwarven issue create`, `dwarven issue close`, `dwarven issue edit`, `dwarven issue dep`, `dwarven issue blocker`, `dwarven issue priority`.

R10.6 — **Exit conditions.** A review has been posted, leaving the issue in a routed state — approved with branch merged into `main` and `state: doc`, or changes-requested with `state: implement` (or `state: plan`).

R10.7 — **Scope fences.** Must not write code, tests, specs, plans, or docs. Must not approve a change whose author is itself. Must not delete branches. Must not push to feature branches.

---

## R11 — Doc (`/doc`)

R11.1 — **Trigger.** Interactive (`/doc [issue-id | scope]`) or detached (on merged-PR-equivalent: an issue transitioning into `state: doc`).

R11.2 — **Inputs.** The merged change's diff (read via git); the owning issue; relevant specs; current docs.

R11.3 — **Outputs.** Edits to `docs/*.md` excluding `docs/specs/`, `docs/architecture/`, `docs/plans/`, and `docs/CHANGELOG.md` (but including an appended entry in `docs/CHANGELOG.md` for the patch-level change); closure of the originating issue (`dwarven issue close`).

R11.4 — **Tool allowlist must include.** `Read`, `Write` and `Edit` (restricted to allowed docs paths), `Bash(git status:*)`, `Bash(git diff:*)`, `Bash(git log:*)`, `Bash(git add:*)`, `Bash(git commit:*)`, `Bash(git push origin main:*)`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue close:*)`, `Bash(dwarven issue comment:*)`.

R11.5 — **Tool allowlist must exclude.** Writes to specs, architecture, or plans; source code modification; `Bash(git push origin feat/*:*)`; `AskUserQuestion`; `Agent`; `dwarven issue create`, `dwarven issue transition` (Doc only closes), `dwarven issue edit`, `dwarven issue dep`, `dwarven issue blocker`, `dwarven issue priority`.

R11.6 — **Exit conditions.** Docs have been committed and pushed to `main` (including the appended CHANGELOG entry); the originating issue has been closed.

R11.7 — **Scope fences.** Must not modify specs, architecture, or plans. Must not modify code. Must not re-open closed issues.

---

## R12 — Triage (`/triage`)

R12.1 — **Trigger.** Interactive (`/triage`) or scheduled (configurable cadence).

R12.2 — **Inputs.** All open issues; the dependency graph; recent state-change history; the integrity report from the hub (asymmetric edges, blocker-without-state-maintainer, etc.).

R12.3 — **Outputs.** Edit fixes on issues with malformed metadata (missing `type`, missing `state`, `state: maintainer` without a `blocker`); transitions of stale issues to `state: maintainer` with `blocker: maintainer-input` (cadence: configurable; default 14 days without state change); a `type: chore` triage report comment on a tracking issue (or a new `type: chore` issue if no tracking issue exists).

R12.4 — **Tool allowlist must include.** `Read`, `Bash(dwarven issue view:*)`, `Bash(dwarven issue list:*)`, `Bash(dwarven issue edit:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue blocker:*)`, `Bash(dwarven issue transition:*)`, `Bash(dwarven issue create:*)` (for the triage report only).

R12.5 — **Tool allowlist must exclude.** `Write`, `Edit`; `dwarven issue close` (Triage may flag for closure but cannot close); `dwarven issue priority`; `dwarven issue dep`; `AskUserQuestion`; `Agent`; any `git` mutation.

R12.6 — **Exit conditions.** Queue audited; triage report posted (as a comment on the tracking issue or a new `type: chore` issue).

R12.7 — **Scope fences.** Must not write files. Must not create substantive issues (only the triage report). Must not reassign work across agents without maintainer approval — Triage can only escalate to `state: maintainer`, never directly to another active agent's queue.

---

## R13 — Universal allowlist constraints

The following patterns must never appear in any agent's allowlist (host adapters enforce this in addition to per-agent declarations):

R13.1 — **Destructive git operations.**
- `git push --force`, `git push -f`, `git push --force-with-lease`
- `git reset --hard`
- `git checkout -- .`, `git restore .`
- `git clean -f*`
- `git branch -D *`, `git push origin --delete *`

R13.2 — **Destructive filesystem operations.**
- `rm -rf*`
- `rm -f*` against `.dwarven/`, `docs/specs/`, `docs/architecture/`, `docs/plans/`, or any source path

R13.3 — **Hub administrative operations.**
- `dwarven serve`, `dwarven daemon *`
- `dwarven init`
- `dwarven config set:*`
- `dwarven reindex`
- `dwarven issue priority` (maintainer-only — `dwarven-cli.md#R6.9`)
- `dwarven issue priority-override` (maintainer-only — `dep-graph.md#R4`)
- `dwarven issue transition * --override` (maintainer-only — `dwarven-cli.md#R6.5.3`)

R13.4 — **Direct push to `main`** is restricted to: Spec (R3), Architect (R4), Planning (R7), Code Review (R10, only as part of merge), and Doc (R11). All other agents must commit only to `feat/*` branches.

R13.5 — **`Agent` (subagent-dispatch) tool** is restricted to: the maintainer's shell (per `host-adapter.md`), Spec, and Architect. No other agent may dispatch sub-subagents.

R13.6 — **`AskUserQuestion`** is restricted per `dialogue.md#R8`: dialogue agents only.

---

## R14 — Out of scope for v1

R14.1 — **Custom agent roles per project.** The roster is fixed at ten. Project-specific agents are out of scope.

R14.2 — **Multi-agent collaboration on a single issue.** One agent owns an issue at a time per `work-states.md#R2.5`.

R14.3 — **Automatic agent re-dispatch on state change.** Per `dialogue.md#R7.3`, re-dispatch is manual in v1.

R14.4 — **Per-agent learning / memory.** Agents are stateless beyond the host's session context. There is no persistent agent-side memory store.

R14.5 — **Cross-repo agent activity.** Each agent operates within one repository (`dwarven-cli.md#R9.4`).
