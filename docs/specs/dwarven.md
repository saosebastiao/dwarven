---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
related_architecture: (none yet)
constituent_specs:
  - coordination-hub.md
  - storage-model.md
  - dwarven-cli.md
  - web-api.md
  - web-ui.md
  - agent-roster.md
  - work-states.md
  - dialogue.md
  - dep-graph.md
  - host-adapter.md
---

# Dwarven — Top-Level Specification

This document is the architectural backbone of the Dwarven specification. It establishes identity, invariants, and the constituent concerns that other spec documents elaborate. Every constituent spec listed in the frontmatter must conform to the invariants stated here.

**Conventions in this document.**

- Each section is identified `R<n>`. Each numbered claim is identified `R<n>.<m>`. Tests, plans, and `type:spec-gap` issues cite requirements by ID. Cross-spec references use the form `<spec>.md#R<n>`.
- "Must" and "may" carry their ordinary English force. RFC 2119 keywords are not used.
- Where this spec and a constituent spec disagree, this spec is correct and the constituent spec is the bug.
- Where this spec and any documentation disagree, this spec is correct and the documentation is the bug.

---

## R1 — Identity & purpose

R1.1 — Dwarven is a system for specification-driven software development.

R1.2 — Dwarven orchestrates a fixed roster of focused agents that develop software against a written specification.

R1.3 — Dwarven targets project maintainers who use AI coding-agent hosts as their primary development environment.

R1.4 — The problem Dwarven solves: keeping intent (the spec), behavior (the docs), code, and the prioritization of work synchronized through mechanism-enforced agent decoupling, a local coordination hub, and dependency-aware scheduling.

R1.5 — Dwarven is host-agnostic. It targets multiple AI coding-agent hosts via thin per-host adapters. The pipeline, agent roster, work states, and coordination contract are defined independently of any single host. v1 ships the Claude Code adapter; v3 adds opencode (R7).

R1.6 — Dwarven owns its coordination substrate. It does not depend on GitHub, GitLab, Linear, Jira, or any external issue tracker. External integrations may exist as adapters but are never load-bearing.

---

## R2 — Core invariants

R2.1 — Specifications drive everything. A spec describes how the system *should* work; documentation describes how it *does* work; the disagreement between them is a *gap*.

R2.2 — Tests gatekeep implementation. Tests are the executable form of the spec. Implementation must not land without tests that verify it satisfies the spec.

R2.3 — Architecture is a distinct concern from specification. A spec says what the system should do; an architecture says how the system should solve it. Architecture documents live at `docs/architecture/*.md`.

R2.4 — Decoupling is mechanism-enforced, not conventional. Each agent must be implemented as a host-native subagent (or the host's nearest equivalent) with its own system prompt and tool allowlist. The per-host adapter (host-adapter.md) defines how this mechanism maps onto each supported host.

R2.5 — An agent must not change roles mid-session. Cross-agent handoff happens through coordination-hub state (R3) or through bounded subagent dispatch where the host supports it.

R2.6 — The coordination hub is the only inter-agent coordination surface. Inter-agent state lives in the hub's tracked artifacts (issues, comments, dependencies, work states, plans). There is no other shared state.

R2.7 — Monorepo workflows are preferred but not required. Dwarven assumes spec, docs, code, tests, plans, and hub-tracked artifacts live in a single workspace.

R2.8 — Source of truth for hub-tracked artifacts is the file system. Issues, dialogue, dependencies, and work-state transitions are stored as Markdown files (with frontmatter) in the repository under `.dwarven/`. The hub's database (SQLite) is a derived index for queries and the web UI; it must be reproducible from the files at any time.

R2.9 — Prioritization is dynamic and dependency-aware. The maintainer's product priorities are inputs to a scheduler that also weighs each item's downstream-unblocking value (dep-graph.md). The hub computes an effective priority; the maintainer overrides per item.

R2.10 — Agents interact with the hub exclusively through the `dwarven` CLI (R3.2). They do not call the hub's HTTP API directly. This keeps per-agent allowlists granular and host-portable.

---

## R3 — Architecture overview

The Dwarven system has four major components:

R3.1 — **The coordination hub.** A local single-binary daemon (Rust) that owns the workflow state. Reads and writes hub-tracked artifacts as files (R2.8); maintains a SQLite index for queries; exposes a local HTTP API; serves a web UI for the maintainer. Detailed in `coordination-hub.md`.

R3.2 — **The `dwarven` CLI.** A command-line interface to the hub. Agents interact with the hub exclusively through this CLI (R2.10); per-agent allowlists scope it analogously to how `gh` was scoped in v0.1 (e.g., `Bash(dwarven issue create:*)`). Detailed in `dwarven-cli.md`.

R3.3 — **The web UI.** The maintainer's primary visual surface for the hub: triage, dependency view, dialogue with agents in detached mode, scheduler overrides. Detailed in `web-ui.md`. The hub's HTTP contract is detailed in `web-api.md`.

R3.4 — **The agents and their host adapters.** A fixed roster of focused agents (`agent-roster.md`) implemented as host-native subagents via per-host adapters (`host-adapter.md`). The maintainer's *shell* is the host-specific top-level session that dispatches agents via slash commands. v1 supports Claude Code; v3 adds opencode.

Beneath these, the work-state model (`work-states.md`), the dialogue model (`dialogue.md`), and the dep-graph model (`dep-graph.md`) define the protocol layer that agents and the hub share.

---

## R4 — Constituent specifications

R4.1 — `coordination-hub.md` — The hub binary's responsibilities, file→DB sync model, daemon lifecycle, file-watching, conflict resolution, and durability guarantees.

R4.2 — `storage-model.md` — The on-disk artifact format (Markdown with frontmatter), directory layout under `.dwarven/`, file naming, and the canonical-vs-derived distinction.

R4.3 — `dwarven-cli.md` — The `dwarven` command surface, subcommand structure, output formats, exit codes, and the granular-allowlist patterns each agent uses.

R4.4 — `web-api.md` — The hub's local HTTP API surface: endpoints, request/response shapes, auth (or lack thereof, given the local-only model), and stability guarantees.

R4.5 — `web-ui.md` — The web UI's screens, flows, and interaction model for the maintainer.

R4.6 — `agent-roster.md` — The fixed agent roster: trigger, inputs, outputs, tool allowlist, exit conditions, scope fences. Carries forward the spec → gap → pm → plan → test → implement → review → doc pipeline. Replaces v0.1's GH-coupled `gh` allowlists with `dwarven`-CLI-scoped allowlists.

R4.7 — `work-states.md` — Issue/work-state model: state names, valid transitions, single-owner rule, blockers. Replaces v0.1's GH label protocol.

R4.8 — `dialogue.md` — Interactive (host-native `AskUserQuestion` equivalent) and detached (hub comment thread) dialogue mechanisms: how an agent escalates and how the maintainer responds.

R4.9 — `dep-graph.md` — The dependency model: how edges are created, by whom; the prioritization algorithm; how the maintainer overrides. v2 scope.

R4.10 — `host-adapter.md` — The host-agnostic agent contract and the per-host adapter contract. v1 ships the Claude Code adapter; v3 adds opencode.

---

## R5 — Spec versioning model

R5.1 — Constituent spec files live flat at `docs/specs/*.md`. The top-level spec (`docs/specs/dwarven.md`) lists them in its `constituent_specs` frontmatter.

R5.2 — `docs/CHANGELOG.md` tracks all changes from v2.0.0 forward. v0.1.x history is preserved in git but not migrated forward.

R5.3 — On the first breaking spec change after v2.0.0, the existing `docs/specs/*` content migrates to `docs/specs/v2/`, and the new major version goes to `docs/specs/v3/`.

R5.4 — Minor spec versions are recorded as inline annotations within spec files.

R5.5 — Patch-level changes (typically documentation reflecting code) are tracked in `docs/CHANGELOG.md` only.

---

## R6 — Scope boundaries

R6.1 — Dwarven is not a CI system. CI integration (e.g., remote dispatch from a CI provider) is an optional adapter, never load-bearing.

R6.2 — Dwarven is not a code generator. Agents write code derived from specs and plans; they do not template or scaffold project source from prompts alone.

R6.3 — Dwarven is not opinionated about test runners, build tools, languages, or frameworks. Test paths and build commands are configured per repository.

R6.4 — Dwarven is not a replacement for human judgment. The maintainer remains the source of truth for scope, priority, and acceptance. The dep-graph scheduler ranks; the maintainer chooses.

R6.5 — Dwarven does not enforce naming conventions beyond those required for agent coordination (work-state names, branch prefix, commit format). Project-internal naming is the project's concern.

R6.6 — Dwarven does not depend on any external issue tracker. Adapters to external trackers (GitHub, etc.) may exist but never as the source of truth for any hub-tracked artifact.

---

## R7 — Implementation versioning

R7.1 — **v1** — Coordination hub (Rust binary, SQLite index, file-based artifacts), `dwarven` CLI, web UI, agent roster rewired off GitHub, Claude Code host adapter. Pipeline parity with the inherited v0.1 design (spec → gap → pm → plan → test → implement → review → doc).

R7.2 — **v2** — Dependency graph and dynamic prioritization. New artifact type (dependency edges), new scheduler component in the hub, web UI surfaces ranking and maintainer overrides.

R7.3 — **v3** — opencode host adapter. Generalizes the host-adapter contract from "Claude Code + theoretical" to "Claude Code + opencode."

R7.4 — Each version is independently shippable. Implementation work for vN+1 must not begin until vN is functionally complete and the spec for vN+1 is settled. The order may be revisited only on explicit maintainer decision.
