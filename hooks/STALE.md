# STALE — v0.1 inherited content

The hooks in this directory implement the **v0.1 architecture**:

- `session-start` — auto-loads the v0.1 `using-dwarven` skill.
- `pre-tool-use` — enforces the v0.1 R6.5 universal never-list (per the v0.1 spec).
- `hooks.json` — registers the above with Claude Code.

The project pivoted on 2026-05-09. See `CLAUDE.md` and `docs/specs/dwarven.md` for the v2 architecture. The universal allowlist constraints have moved to `docs/specs/agent-roster.md#R13`; session orientation has moved to `docs/specs/host-adapter.md#R2.7`.

**Do not refactor in place.** v1 implementation will materialize fresh hooks via the Claude Code adapter (`docs/specs/host-adapter.md#R3.6, #R3.7`). The hooks here will be removed when the v1 adapter ships.
