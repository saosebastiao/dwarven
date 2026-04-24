# Dwarven

Dwarven is a Claude Code skills plugin being retargeted toward specification-driven development with tight GitHub integration and strongly decoupled implementation agents.

> **Status:** under redesign. The codebase inherits a large set of skills that predate the redesign; the direction sketch for the new architecture lives in `prompts/todo.md`.

## Inherited Workflow

The inherited skills define a multi-stage agent workflow that triggers automatically during a coding session:

1. **brainstorming** — activates before writing code. Refines rough ideas through questions, explores alternatives, presents design in sections for validation. Saves a design document.
2. **using-git-worktrees** — activates after design approval. Creates an isolated workspace on a new branch, runs project setup, verifies a clean test baseline.
3. **writing-plans** — breaks work into bite-sized tasks (2–5 minutes each) with exact file paths, complete code, and verification steps.
4. **subagent-driven-development** or **executing-plans** — dispatches a fresh subagent per task with two-stage review (spec compliance, then code quality), or executes in batches with human checkpoints.
5. **test-driven-development** — enforces RED-GREEN-REFACTOR during implementation.
6. **requesting-code-review** — reviews against the plan and reports issues by severity between tasks.
7. **finishing-a-development-branch** — verifies tests, presents merge/PR/keep/discard options, cleans up the worktree.

## Skills Library

**Testing**
- **test-driven-development** — RED-GREEN-REFACTOR cycle (includes testing anti-patterns reference)

**Debugging**
- **systematic-debugging** — 4-phase root cause process
- **verification-before-completion** — ensure it's actually fixed

**Collaboration**
- **brainstorming** — Socratic design refinement
- **writing-plans** — detailed implementation plans
- **executing-plans** — batch execution with checkpoints
- **dispatching-parallel-agents** — concurrent subagent workflows
- **requesting-code-review** — pre-review checklist
- **receiving-code-review** — responding to feedback
- **using-git-worktrees** — parallel development branches
- **finishing-a-development-branch** — merge/PR decision workflow
- **subagent-driven-development** — fast iteration with two-stage review

**Meta**
- **writing-skills** — create new skills following best practices
- **using-dwarven** — introduction to the skills system (loaded at session start)

## Philosophy

- **Specification-driven** — specs describe how the system *should* work; documentation describes how it *does*. Specs drive tests; tests gatekeep implementation.
- **Test-Driven Development** — write tests first, always.
- **Systematic over ad-hoc** — process over guessing.
- **Complexity reduction** — simplicity as a primary goal.
- **Evidence over claims** — verify before declaring success.

## Installation

Dwarven is **not yet published**. Target is Claude Code only. To try it locally, clone this repo and register it as a local plugin in your Claude Code setup.

## License

MIT — see `LICENSE`.
