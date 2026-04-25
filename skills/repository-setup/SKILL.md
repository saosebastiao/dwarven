---
name: repository-setup
description: Use to scaffold a target repository for use with Dwarven, or to retrofit an existing repo. Creates the docs/ structure, label set, issue/PR templates, .claude/settings.json, and main branch protection per R8 of the Dwarven spec.
---

# Repository Setup

Scaffold a target repository to use Dwarven, or non-destructively retrofit an existing repo to match Dwarven's expected structure.

This skill is invoked by the maintainer (typically in a fresh repo or one new to Dwarven). It does not run automatically. The maintainer reviews each step's diff before any write.

## Modes

- **Init** — empty or near-empty repo. Creates everything from scratch.
- **Retrofit** — existing repo with some structure. Detects what's present, prints a diff of intended changes, and requires explicit confirmation before any write.

The skill auto-detects mode (init if `docs/specs/` does not exist; retrofit otherwise).

## What this skill scaffolds

Per R8.2 of `docs/specs/dwarven.md`:

1. **Directories:** `docs/specs/`, `docs/architecture/`, `docs/plans/`.
2. **CHANGELOG seed:** `docs/CHANGELOG.md` per Keep-a-Changelog format (R9.5).
3. **Label set:** all `agent:*`, `type:*`, `blocker:*`, `priority:*` labels (R5.1) created in the GitHub repo.
4. **Issue templates:** `.github/ISSUE_TEMPLATE/{feature,bug,arch,doc,chore}.yml` (one per filable `type:*`).
5. **PR template:** `.github/PULL_REQUEST_TEMPLATE.md`.
6. **Claude settings:** `.claude/settings.json` with the v0.1 allow + deny patterns (R6, R6.5).
7. **Branch protection:** `main` requires PR review (R5.5.2).

`type:spec-gap` is filed by the Gap Analysis agent only, not manually, so no issue template.

Hooks (SessionStart loading `using-dwarven`, PreToolUse enforcing R6.5) are installed via the plugin manifest, not by this skill.

## Process

1. **Detect mode.** Check for `docs/specs/`, `.github/ISSUE_TEMPLATE/`, the v0.1 GitHub labels. If anything exists → retrofit.
2. **Inventory current state.** List what's present and what's missing.
3. **Present diff.** For retrofit: show the maintainer exactly what will be added or changed. Files that exist with conflicting content are flagged for review (never overwritten silently).
4. **Confirm.** Wait for explicit "yes" before any write.
5. **Create directories.** `mkdir -p docs/specs docs/architecture docs/plans`.
6. **Seed CHANGELOG.** If `docs/CHANGELOG.md` does not exist, copy from `templates/CHANGELOG.md`.
7. **Create labels.** Run `${CLAUDE_PLUGIN_ROOT}/skills/repository-setup/scripts/create-labels.sh`.
8. **Install issue + PR templates.** Copy from `templates/.github/...` if not present.
9. **Write `.claude/settings.json`.** Copy from `templates/.claude/settings.json`. Note: the target repo's `.gitignore` likely needs `.claude/*` + `!.claude/settings.json` — flag if the maintainer should add this.
10. **Configure branch protection.** Run `${CLAUDE_PLUGIN_ROOT}/skills/repository-setup/scripts/setup-branch-protection.sh`. Requires admin privileges on the repo.
11. **Verify.** Re-inventory; confirm all expected items exist. Print a summary report.

## Templates

Templates live at `${CLAUDE_PLUGIN_ROOT}/skills/repository-setup/templates/`:

- `CHANGELOG.md` — initial CHANGELOG seed.
- `.github/ISSUE_TEMPLATE/feature.yml` — `type:feature` template.
- `.github/ISSUE_TEMPLATE/bug.yml` — `type:bug` template.
- `.github/ISSUE_TEMPLATE/arch.yml` — `type:arch` template.
- `.github/ISSUE_TEMPLATE/doc.yml` — `type:doc` template.
- `.github/ISSUE_TEMPLATE/chore.yml` — `type:chore` template.
- `.github/PULL_REQUEST_TEMPLATE.md`.
- `.claude/settings.json` — the v0.1 allow + deny patterns.

## Helper scripts

- `scripts/create-labels.sh` — creates the v0.1 label set via `gh label create` / `gh label edit`. Idempotent.
- `scripts/setup-branch-protection.sh` — configures `main` branch protection via `gh api`. Requires admin.

## Red Flags

| Thought | Reality |
|---|---|
| "This file already has content; I'll just overwrite it" | Never overwrite without explicit confirmation. Print the diff first. |
| "I'll customize the template content while installing" | Templates install unmodified. Maintainer customizes after. |
| "Branch protection failed; I'll skip it" | If failed, REPORT IT. Don't silently skip — the maintainer needs to know what's not set. |
| "Retrofit means update everything to current schema" | Retrofit means add what's missing. Existing content stays unless explicitly confirmed for overwrite. |
| "I'll create the labels via REST API in bash" | Use `gh label` via the helper script. Don't reinvent. |
| "I'll add hooks to the target repo's hooks.json" | Hooks are plugin-level (`${CLAUDE_PLUGIN_ROOT}/hooks/`). Target repos don't need their own. |

## Hard gate

<HARD-GATE>
For any file overwrite (existing → new content):

1. Print a diff (`diff` or `git diff --no-index`).
2. Wait for explicit "overwrite" confirmation from the maintainer.
3. If declined: skip and note in the final report.

Never overwrite without an explicit "yes." Files left as-is are reported.
</HARD-GATE>

## Verification before exit

- `ls docs/specs/ docs/architecture/ docs/plans/` — directories exist.
- `cat docs/CHANGELOG.md` — CHANGELOG seeded (or pre-existing).
- `gh label list --json name --jq '.[].name'` includes all v0.1 labels.
- `ls .github/ISSUE_TEMPLATE/` includes feature, bug, arch, doc, chore.
- `cat .github/PULL_REQUEST_TEMPLATE.md` exists.
- `cat .claude/settings.json` exists with `permissions.allow` and `permissions.deny`.
- `gh api repos/<owner>/<repo>/branches/main/protection` returns the protection config.

Report each check's result. Anything that didn't get installed (whether skipped, failed, or already present with different content) goes in the report so the maintainer knows.

## Self-dogfood note

When run against the Dwarven plugin's own repo, `repository-setup` should detect that most scaffolding already exists (specs, plans, architecture dirs, CHANGELOG, settings.json, gitignore negation pattern) and only add what's missing — typically the GitHub label set, issue/PR templates, and branch protection.
