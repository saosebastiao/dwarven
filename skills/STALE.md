# STALE (mostly) — v0.1 inherited content

The skills in this directory were curated for the **v0.1 architecture** (Claude-Code-only, GitHub-coupled). The disposition under v2 is:

| Skill | v2 disposition |
|---|---|
| `repository-setup` | **Stale.** Replaced by `dwarven init --host <h>` per `docs/specs/host-adapter.md#R5`. |
| `using-dwarven` | **Stale.** v0.1 content describes the GH-coupled workflow. A v2 successor will be authored as part of v1 implementation (see `docs/specs/host-adapter.md#R3.6.1`). |
| `dispatching-parallel-agents` | Likely keep cross-cutting, possibly with minor updates for the v2 dispatch model. Defer evaluation until v1 implementation. |
| `systematic-debugging` | Keep cross-cutting. Independent of architecture. |
| `test-driven-development` | Keep cross-cutting. Carries the RED-GREEN-REFACTOR discipline used by Test Dev and Implementation. |
| `using-git-worktrees` | Keep cross-cutting. Independent of architecture. |
| `verification-before-completion` | Keep cross-cutting. Independent of architecture. |
| `writing-skills` | Keep cross-cutting (meta). Independent of architecture. |

**Do not refactor stale skills in place.** v1 implementation will replace them. The "keep cross-cutting" skills may receive minor updates as the v1 adapter materializes; substantive rewrites of behavior-shaping content require eval evidence (per `CLAUDE.md` "Skills are behavior-shaping code, not prose").

The project pivoted on 2026-05-09. See `CLAUDE.md` and `docs/specs/dwarven.md` for the v2 architecture.
