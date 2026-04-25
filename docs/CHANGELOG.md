# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Per Dwarven's spec versioning model: major spec versions migrate to versioned directories (`docs/specs/v1/`, `docs/specs/v2/`); minor spec versions are annotated inline; patch-level changes are tracked here.

## [Unreleased]

### Added

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
