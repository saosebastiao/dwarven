# STALE — v0.1 inherited content

The agent definitions in this directory implement the **v0.1 architecture**: a Claude Code plugin coupled to GitHub for coordination (`gh` CLI patterns, `agent:*` labels, GH issues/PRs).

The project pivoted on 2026-05-09 to a host-agnostic, hub-coordinated v2 architecture. See:

- `README.md` — public v2 overview
- `CLAUDE.md` — in-session reference, including "The pivot"
- `docs/specs/dwarven.md` — v2 top-level specification
- `docs/specs/agent-roster.md` — v2 agent contracts (host-agnostic, `dwarven` CLI not `gh`)
- `docs/specs/host-adapter.md#R3` — Claude Code adapter that will replace these files

**Do not refactor in place.** v1 implementation will materialize fresh agent definitions via the Claude Code adapter (`docs/specs/host-adapter.md#R3.1`) and these files will be removed. Editing them now is wasted effort.

If you need historical context for what the v0.1 design intended, read these files. For everything else, defer to `docs/specs/`.
