---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Work States

This document specifies the work-state model: the vocabulary of issue types, work states, blockers, and priorities, plus the state machine governing transitions. It replaces v0.1's GitHub label protocol.

The work state of an issue is the *next action that needs to happen on it*. Equivalently: who currently owns the issue. State and owner are one concept, not two.

---

## R1 — Scope

R1.1 — This spec defines the value sets for the `type`, `state`, `blocker`, and `priority` fields in issue frontmatter (`storage-model.md#R4.3`), and the rules governing transitions between states.

R1.2 — This spec does not define how transitions are invoked (that is `dwarven-cli.md`) or how transitions are recorded on disk (that is `storage-model.md#R3.3`).

---

## R2 — State vocabulary

R2.1 — The `state` field of an issue takes exactly one of the following values:

| State | Owner | Meaning |
|---|---|---|
| `spec` | Spec agent | Spec change pending |
| `architect` | Architect agent | Architecture decision pending |
| `pm` | PM agent | Decomposition pending |
| `plan` | Planning agent | Implementation plan pending |
| `test` | Test Dev agent | Failing tests pending |
| `implement` | Implementation agent | Code change pending |
| `review` | Code Review agent | PR review pending |
| `doc` | Doc agent | User-facing doc update pending |
| `maintainer` | The maintainer (human) | Blocked on human input |
| `done` | — | Terminal: successfully resolved |
| `dropped` | — | Terminal: won't be done |

R2.2 — `done` and `dropped` are *terminal* states. An issue in a terminal state must not be transitioned to any other state. Reopening is not a transition; it is creating a new issue that may reference the closed one.

R2.3 — `spec`, `architect`, `pm`, `plan`, `test`, `implement`, `review`, `doc`, and `maintainer` are *active* states. Active issues are eligible for the scheduler (`dep-graph.md`) and the maintainer's open queue.

R2.4 — There is no `gap` or `triage` state. The Gap and Triage agents are scanners that act on existing issues; they do not own a queue of their own. Gap creates new issues at the appropriate entry-point state (R6.1); Triage transitions other issues' states.

R2.5 — `state` is a single-valued field. The single-owner invariant is structural: a value cannot represent two simultaneous owners.

---

## R3 — Type vocabulary

R3.1 — The `type` field of an issue takes exactly one of the following values:

| Type | Meaning |
|---|---|
| `spec-gap` | A disagreement between spec and code/docs/architecture |
| `feature` | A new capability to be built |
| `bug` | A defect in existing behavior |
| `arch` | An architectural decision or design question |
| `doc` | A documentation deficiency |
| `chore` | Operational work (triage reports, dep-graph cleanup, etc.) |

R3.2 — `type` is fixed at issue creation and may only be changed by the Triage agent or the maintainer with an explicit reason recorded as a comment.

---

## R4 — Blocker vocabulary

R4.1 — The `blocker` field is *optional*. It is set when an issue's state is `maintainer` (R2.1) to indicate why human input is required. It may also be set on other states to signal that the active agent is paused on an external dependency.

R4.2 — Permitted values:

| Blocker | Meaning |
|---|---|
| `maintainer-input` | Awaiting a decision or clarification from the maintainer |
| `external` | Awaiting an external party (third-party service, vendor, etc.) |
| `upstream` | Awaiting an upstream code change (dependency, related PR, etc.) |

R4.3 — Setting or clearing a blocker is recorded as a `blocker-set` or `blocker-cleared` comment (`storage-model.md#R4.4`).

R4.4 — Multiple blockers on a single issue are not supported in v1. Use the most-load-bearing one and explain nuance in a comment.

---

## R5 — Priority vocabulary

R5.1 — The `priority` field is *optional*. It is the maintainer's product-priority assertion.

R5.2 — Permitted values: `p0` (must do now), `p1` (should do soon), `p2` (would be nice). Issues without a priority are treated as `p2` by the scheduler.

R5.3 — `priority` is an input to the scheduler (`dep-graph.md`); the scheduler computes an *effective* priority that incorporates downstream-unblocking value. The effective priority is not stored in the issue file (`storage-model.md#R4.3.4`).

---

## R6 — State machine

### R6.1 — Entry points (state at issue creation)

| Type | Default initial state | Created by |
|---|---|---|
| `spec-gap` | `pm` | Gap agent, or maintainer |
| `feature` | `pm` | Maintainer (large feature) or `plan` if pre-decomposed |
| `bug` | `plan` | Maintainer, Gap, Triage |
| `arch` | `architect` | Spec, Gap, or maintainer |
| `chore` | `plan` | Triage, or maintainer |
| `doc` | `doc` | Maintainer, or implementation drift detection |

R6.1.1 — The maintainer may override the default initial state when creating an issue (e.g., a pre-scoped feature can start at `plan`).

### R6.2 — Permitted transitions

The transition graph for active states:

| From | Permitted to |
|---|---|
| `spec` | `done`, `maintainer` |
| `architect` | `done`, `maintainer` |
| `pm` | `plan`, `done` (when decomposition produces children that supersede the parent), `maintainer`, `dropped` |
| `plan` | `test`, `maintainer`, `dropped` |
| `test` | `implement`, `maintainer`, `dropped` |
| `implement` | `review`, `maintainer`, `dropped` |
| `review` | `doc` (on approve+merge), `implement` (changes requested), `plan` (plan flawed), `maintainer`, `dropped` |
| `doc` | `done`, `maintainer`, `dropped` |
| `maintainer` | any active state |

R6.2.1 — Transitions not listed in R6.2 are forbidden. The hub rejects them with an explanatory error.

R6.2.2 — `dropped` is reachable from any active state — abandonment is always permitted and is recorded with a closure comment explaining why.

R6.2.3 — Every transition produces a `state-change` comment (`storage-model.md#R3.3`) recording `from`, `to`, and the actor.

R6.2.4 — `done` is reachable from any active state via the dedicated close path (`dwarven-cli.md#R6.7`). The per-state graph in R6.2 governs `dwarven issue transition`; closing an issue is a separate, terminal-only escape hatch that mirrors R6.2.2's universal `dropped` reachability. The path constraint (close vs. transition) is the structural enforcement: agents that hold `transition:*` allowlists but not `close:*` cannot bypass an in-graph step (e.g., `implement` cannot jump straight to `done`); agents and the maintainer that hold `close:*` may close from anywhere.

### R6.3 — Maintainer override

R6.3.1 — The maintainer may force any transition between active states, including ones not in R6.2, by invoking the CLI with an explicit override flag (`dwarven-cli.md`). The override is recorded with a comment explaining why.

R6.3.2 — The maintainer may not transition into a terminal state via override; closing an issue uses the dedicated close path (which performs sanity checks and records a closure comment).

R6.3.3 — Agents may not use override. Forcing transitions is a maintainer-only escape hatch.

---

## R7 — Invariants

R7.1 — Every issue has exactly one `state` value at any time (R2.5).

R7.2 — Every issue has exactly one `type` value at any time (R3.1).

R7.3 — Every state transition produces a `state-change` comment (R6.2.3). The comment thread of an issue, filtered to `kind: state-change`, is the complete state history.

R7.4 — Terminal states are absorbing (R2.2). The hub rejects transitions out of `done` or `dropped`.

R7.5 — The `blocker` field, if set, must be one of the values in R4.2. The hub rejects writes with other values.

R7.6 — When `state = maintainer`, `blocker` should be set. The hub flags issues with `state = maintainer` and no `blocker` as `type:chore` items for triage.

---

## R8 — Out of scope for v1

R8.1 — **State-machine extensions.** Custom states per project are not supported. The vocabulary in R2.1 is fixed.

R8.2 — **Multi-blocker.** Only one `blocker` value at a time (R4.4).

R8.3 — **Per-state SLAs.** No automatic escalation based on time-in-state. Triage may surface stale items (per its scope) but the staleness threshold is project-configured, not part of the state machine.

R8.4 — **Workflow customization.** The transition graph in R6.2 is fixed for v1. Projects with different pipelines must wait for a future spec extension.
