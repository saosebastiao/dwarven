# Dwarven — Project Context

## What This Is

Dwarven is a Claude Code skills plugin being retargeted toward:

- **Specification-driven development** — specs (how the system *should* work) drive tests; tests gatekeep implementation
- **Strongly decoupled agent workflows** — each agent type performs only its allocated work; no role-switching mid-session
- **Tight GitHub integration** — issues, wiki, releases, and CI/CD are first-class surfaces for planning, backlog, and handoff
- **Native Claude Code features** — the plugin embraces Claude Code primitives (Task tool, subagents, hooks, skills); no cross-harness abstraction

The initial direction sketch lives in `prompts/todo.md`. The codebase is inherited content at the starting point — redesign is in progress.

## Current State

- Target: Claude Code only. Cross-harness adapters (OpenCode, Cursor, Codex, Gemini, Copilot) and upstream-specific publishing/sync scripts have been removed.
- `skills/`, `agents/`, `commands/`, and `hooks/` contain the surviving inherited content.
- Release automation (version bump, registry publish) is not yet in place and will be re-introduced if/when the redesign calls for it.

## Conventions (keep unless there's evidence to change)

- **"Your human partner"** is the preferred term, not "the user". It's deliberate.
- **Skills are behavior-shaping code**, not prose. Changes to Red Flags tables, rationalization lists, and "EXTREMELY_IMPORTANT" framing need eval evidence.
- **TDD is RED-GREEN-REFACTOR.** Write the failing test first.
- **Process skills run before implementation skills.** Brainstorming before design, debugging before fixes.

## Working In This Repo

- Check `skills/` for existing workflows before reinventing them
- Plans and specs live under `docs/` (currently empty after the pre-fork plan purge — new ones go in `docs/plans/` and `docs/specs/`)
- The `using-dwarven` skill is auto-loaded by the SessionStart hook and is the entry point that introduces the skills system
- One problem per branch; describe the problem, not just the change
