---
issue: 5
date: 2026-05-10
---

# Plan: opencode adapter implementation

**Issue:** #5
**Spec:** `host-adapter.md#R4` (full 11-rule expansion shipped under #4)

## Goal

`dwarven init --host opencode` materializes the full `.opencode/` and root-level surface per `host-adapter.md#R4`. Parallel structure to the Claude Code adapter (slice 18), with opencode-specific frontmatter and permission shapes per R4.6 / R4.7.

## Materialization surface

```
opencode.json                       # R4.8: global permission floor (R13 deny list)
.opencode/AGENTS.md                 # R4.5: orientation copy auto-loaded at session start
.opencode/agents/<name>.md          # R4.1: 10 subagent files (one per dispatched agent)
.opencode/agents/build.md           # R4.3: maintainer primary agent
```

11 files materialized total: 10 agents + 1 maintainer + global config + AGENTS.md = 13 files actually. (Counted: 10 subagent .md + 1 build.md + 1 AGENTS.md + 1 opencode.json = 13.)

## Refactor: shared agent registry

The Claude Code adapter has `AgentDef` + `ROSTER` baked into `src/adapter/claude_code/agents.rs`. The two adapters should share that registry — only the render function differs.

Plan:
- Move `AgentDef` + `Mode` + `ROSTER` + `roster()` to `src/adapter/registry.rs`.
- `src/adapter/claude_code/agents.rs` keeps only `render(&AgentDef)` and the Claude Code-specific frontmatter format; imports `AgentDef`/`roster` from the shared module.
- `src/adapter/opencode/agents.rs` defines its own `render(&AgentDef)` returning opencode-formatted frontmatter; imports the same registry.

This keeps spec-driven content (each agent's description, scope fences, bash patterns) in one place.

## Module layout

```
src/adapter/
├── mod.rs                      # dispatch by host name (existing)
├── registry.rs                 # NEW: AgentDef + Mode + ROSTER
├── claude_code/
│   ├── mod.rs                  # (existing)
│   ├── agents.rs               # uses registry; Claude Code render
│   ├── commands.rs             # (existing)
│   ├── settings.rs             # (existing)
│   └── hooks.rs                # (existing)
└── opencode/                   # NEW
    ├── mod.rs                  # orchestrator: mkdirs + write per file
    ├── agents.rs               # opencode render (subagent frontmatter)
    ├── maintainer.rs           # build.md (primary agent)
    ├── opencode_json.rs        # global permission floor
    └── agents_md.rs            # .opencode/AGENTS.md orientation
```

## Permission patterns (R4.6, R4.7, R4.8)

opencode's permission syntax is `allow` / `ask` / `deny` with wildcards `*` / `?`. Per-agent permission block sits in the agent's frontmatter:

```yaml
permission:
  read: allow
  edit:
    "docs/specs/**": allow
    "docs/CHANGELOG.md": allow
    "*": deny
  write: <same as edit>
  task: deny
  question: allow
  bash:
    "dwarven --actor spec issue view*": allow
    "dwarven --actor spec issue list*": allow
    ...
    "git status*": allow
    ...
    "*": deny
```

Each agent's bash patterns mirror its current Claude Code allowlist (already encoded in `AgentDef.bash_patterns`), but rewritten from Claude Code's `Bash(<pattern>:*)` shape to opencode's `"<pattern>*": allow` shape. The translation is mechanical: strip `Bash(...)`/`:*` and append `*`.

Dialogue agents (Spec/Architect/Gap/PM/Planning) get `question: allow`. Discrete-work agents get `question: deny`. Map from `Mode` field in `AgentDef`.

Spec and Architect get `task: { ...: allow }` for the agents they may dispatch (per R13.5); every other agent gets `task: deny`. Maintainer primary (build.md) gets `task: "*": allow`.

## R13 universal-deny at global scope (R4.8.1)

`opencode.json` at project root:

```json
{
  "permission": {
    "bash": {
      "git push --force*": "deny",
      "git push --force-with-lease*": "deny",
      "git push -f*": "deny",
      "git reset --hard*": "deny",
      "git checkout -- .*": "deny",
      "git restore .*": "deny",
      "git clean -f*": "deny",
      "rm -rf*": "deny",
      "rm -f*": "deny",
      "dwarven serve*": "deny",
      "dwarven daemon*": "deny",
      "dwarven init*": "deny",
      "dwarven reindex*": "deny",
      "dwarven config set*": "deny",
      "dwarven issue priority*": "deny",
      "dwarven issue priority-override*": "deny",
      "dwarven issue transition * --override*": "deny"
    }
  },
  "instructions": [".opencode/AGENTS.md"]
}
```

The `instructions` reference loads `.opencode/AGENTS.md` into the session-start context per R4.5.

## Materialize-time validation (R4.8.3)

The opencode adapter has no runtime PreToolUse hook to enforce R13 as a second line of defense (R4.8.3 gap). The adapter compensates at materialize time: before writing the per-agent files, scan each agent's bash allow patterns against the R13 deny list. If any pattern would weaken the global deny (e.g., an `allow` pattern that subsumes a `deny` pattern), abort the init with a clear error.

Specifically: for each per-agent `bash` allow pattern `P_allow` and each global deny pattern `P_deny`, check that `P_allow` does NOT match any string that `P_deny` matches. Implementation: take a fixture string from `P_deny` (the literal prefix before `*`) and test it against `P_allow` glob; if `P_allow` matches, error.

This is a defensive check, not a comprehensive proof — but it catches the common misconfigurations.

## Tests

`tests/adapter_opencode.rs` (new), parallel to `tests/adapter_claude_code.rs`:

1. `adapter_creates_all_agent_files` — `.opencode/agents/<n>.md` for each of 10 agents.
2. `adapter_creates_maintainer_build` — `.opencode/agents/build.md` exists.
3. `adapter_creates_root_opencode_json` — `opencode.json` at repo root with R13 deny patterns.
4. `adapter_creates_agents_md` — `.opencode/AGENTS.md` with orientation copy.
5. `adapter_agent_files_have_correct_frontmatter` — `mode: subagent`, `model:`, `permission:` block.
6. `adapter_per_agent_actor_baked_into_bash_patterns` — each agent's bash allow patterns contain `--actor <agent-name>`.
7. `adapter_dialogue_agents_allow_question` — spec/architect/gap/pm/plan have `question: allow`.
8. `adapter_discrete_agents_deny_question` — test/implement/review/doc/triage have `question: deny`.
9. `adapter_idempotent_second_run` — two consecutive runs produce byte-identical output.
10. `adapter_r13_validation_blocks_shadow` — if a per-agent pattern would weaken R13, materialize fails. (Test by constructing a forced override in a fixture rather than by adding such a pattern to the production ROSTER.)
11. `adapter_dual_install_no_conflict` — `dwarven init --host claude-code --host opencode` produces both surfaces without overlap.

Plus update existing claude_code tests if the registry refactor changes any observable behavior (it shouldn't; pure relocation).

## Branch + commits

`feat/5-opencode-adapter`. Commits:

1. Registry extraction — move `AgentDef`/`Mode`/`ROSTER` to `src/adapter/registry.rs`; update `claude_code/agents.rs` imports. Tests still pass.
2. opencode skeleton — `src/adapter/opencode/{mod, agents, maintainer, opencode_json, agents_md}.rs` empty stubs + skeleton tests RED.
3. opencode render + write — implement each piece; tests GREEN.
4. Validation — materialize-time R13 shadow check + its test.
5. Dual-install test confirming coexistence.

## Branch fitness

The Claude Code adapter shipped in slice 18 is 887 LoC. opencode at parity is roughly the same. ~5 commits, ~1000 LoC total including the registry refactor and tests.
