---
id: 19
title: 'docs/architecture/host-adapter.md: registry, materialization, shadow validation'
type: doc
state: done
priority: p2
epic: code-docs
created: 2026-05-11T15:49:34Z
created_by: maintainer
updated: 2026-05-11T15:52:17Z
---
The host-agnostic agent contract (host-adapter.md spec) is realized in src/adapter/ via a shared registry plus two host modules (claude_code, opencode). How this is structured and why is not yet captured in architecture/.

Scope:
- The host-agnostic AgentDef + Mode + ROSTER in src/adapter/registry.rs: what fields each agent carries, who reads them.
- Per-host module structure: shared AgentDef → host-specific renderers (Claude Code: agent .md + slash command + settings.json + hooks; opencode: agent .md + opencode.json + AGENTS.md).
- Materialize-time validation (opencode): why opencode adapter rejects allow patterns that shadow a global deny (no PreToolUse hook as second line). Contrast with Claude Code's defense-in-depth hook.
- Pattern-narrowing decisions (e.g., why `git checkout -b *` instead of `git checkout *`).
- Idempotent materialization: repeated init produces byte-identical output.
- Dual-install: how the two adapters coexist (separate directories + non-overlapping files at repo root).
- Extension surface: what's host-portable in the registry vs what's host-specific.

Audience: someone considering adding a third host adapter or modifying the registry.
