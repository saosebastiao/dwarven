---
spec_ref: host-adapter.md, agent-roster.md
date: 2026-05-11
issue: 19
---

# Host adapter

Implementation of `docs/specs/host-adapter.md`. How the ten host-agnostic
agent definitions are rendered into per-host configuration when
`dwarven init --host <host>` runs, and the safety mechanisms that keep
the universal-deny floor enforced across hosts whose enforcement
primitives differ.

## The registry

`src/adapter/registry.rs` is the single source of truth for the agent
roster:

```rust
pub struct AgentDef {
    pub name: &'static str,
    pub spec_section: &'static str,
    pub description: &'static str,
    pub mode: Mode,                            // Dialogue or Discrete
    pub trigger: &'static str,
    pub inputs: &'static str,
    pub outputs: &'static str,
    pub exit_conditions: &'static str,
    pub scope_fences: &'static str,
    pub non_bash_tools: &'static [&'static str],
    pub bash_patterns: &'static [&'static str],
}

const ROSTER: [AgentDef; 10] = [ ... ];
```

The ten fields capture every part of an agent's contract per
`agent-roster.md` R3-R12. The roster is `const`, so it is baked into the
binary; no runtime lookup, no plugin loading. Adding an agent is a code
change.

**Why not data-driven (YAML / TOML for the roster)?** Two reasons:

1. The roster is small (10 agents) and stable. The cost of YAML loading,
   error handling, and runtime indirection outweighs the editing
   benefit.
2. The compiler enforces field completeness. A new field added to
   `AgentDef` is a compile error in every roster row that hasn't been
   updated; YAML would silently default.

**Bash pattern conventions.** Patterns in `bash_patterns` use a
single-pattern dialect — `:*` is the host-agnostic "any suffix"
wildcard. Per-host renderers translate to the host's actual glob
dialect:

- Claude Code uses `Bash(<pattern>)` in `settings.json`; the `:*` form
  is preserved as-is.
- opencode uses glob-style patterns in `permission.bash` blocks;
  `to_opencode_pattern()` rewrites `:*` to `*`.

Each agent's allowed `dwarven` commands carry `--actor <name>` literally
(`dwarven --actor spec issue view:*`). This is structural attribution —
an agent cannot fake another agent's actor name through the shell.

## Per-host modules

```
src/adapter/
├── mod.rs            # install(host, root) dispatch
├── registry.rs       # AgentDef, ROSTER, roster()
├── claude_code/
│   ├── mod.rs        # install() → 23 files
│   ├── agents.rs     # render each AgentDef → .claude/agents/<name>.md
│   ├── commands.rs   # render each AgentDef → .claude/commands/<name>.md
│   ├── settings.rs   # generate .claude/settings.json
│   └── hooks.rs      # session-start.sh + pre-tool-use.sh
└── opencode/
    ├── mod.rs        # install() → 13 files, validates first
    ├── agents.rs     # render each AgentDef → .opencode/agents/<name>.md
    ├── agents_md.rs  # .opencode/AGENTS.md (orientation)
    ├── maintainer.rs # .opencode/agents/build.md (maintainer primary)
    ├── opencode_json.rs # opencode.json (root-level, R13 floor)
    └── validation.rs # check_no_shadowing — materialize-time gate
```

Each host module exposes `pub fn install(repo_root: &Path) -> Result<ChangeSummary>`
and that's it. `mod.rs::install(host, root)` is a small dispatch:

```rust
pub fn install(host: &str, repo_root: &Path) -> Result<ChangeSummary> {
    match host {
        "claude-code" => claude_code::install(repo_root),
        "opencode" => opencode::install(repo_root),
        other => Err(anyhow!("unknown host adapter '{other}'; ...")),
    }
}
```

`dwarven init --host <h>` calls this once per `--host` flag, so dual
install is just two sequential calls. The hosts touch disjoint
directories (`.claude/` vs `.opencode/`), and opencode also writes
`opencode.json` at repo root, which Claude Code never touches.

## Rendering: Claude Code

For each agent in `ROSTER`, the Claude Code adapter materializes:

- **`.claude/agents/<name>.md`** — system prompt (assembled from
  `description` + `inputs` + `outputs` + `exit_conditions` +
  `scope_fences`) with a YAML frontmatter block declaring the tools
  allowlist:

  ```yaml
  ---
  name: spec
  description: ...
  tools:
    - Read
    - Write
    - Edit
    - AskUserQuestion
    - Agent
    - Bash(git status:*)
    - Bash(dwarven --actor spec issue view:*)
    - ...
  ---
  ```

- **`.claude/commands/<name>.md`** — slash command that dispatches the
  subagent. Lets the maintainer type `/spec [topic]` in a Claude Code
  session.

- **`.claude/settings.json`** — project-wide allow patterns (a
  superset of per-agent patterns plus the maintainer-shell allowlist),
  the R13 universal-deny list, and the hook registration.

- **`.claude/hooks/session-start.sh`** — runs on every session start.
  Emits orientation copy: "Here are the agents and how to dispatch
  them."

- **`.claude/hooks/pre-tool-use.sh`** — defense-in-depth on R13. Inspects
  each tool call before execution and refuses any call that matches a
  never-list pattern. This is the runtime second line of defense; if
  the settings allowlist somehow lets through a forbidden command, the
  hook still blocks it.

23 files total: 10 agents + 10 commands + `settings.json` + 2 hooks.

## Rendering: opencode

opencode's primitives differ from Claude Code's:

- Subagents are declared as `.md` files with `mode: subagent` in
  frontmatter. Per-subagent `permission:` blocks gate tools.
- The maintainer's shell is the "primary" agent (`mode: primary`),
  which has its own `permission:` block.
- Project-wide config lives in `opencode.json` at the repo root, not
  inside `.opencode/`.
- An auto-loaded `.opencode/AGENTS.md` injects orientation context at
  session start. opencode reads this file (and any other paths listed
  under `instructions:` in `opencode.json`) automatically.
- **There is no PreToolUse hook primitive.** opencode has no runtime
  second line of defense against an allowlist that's too permissive.

So the opencode adapter produces:

- **`opencode.json`** — repo-root config with the global
  `permission.bash` block (R13 deny patterns) and an
  `instructions: [".opencode/AGENTS.md"]` ref so the orientation file
  loads at session start.
- **`.opencode/AGENTS.md`** — orientation copy describing the `@<name>`
  dispatch mechanic and the available agents.
- **`.opencode/agents/<name>.md`** — one per agent. `mode: subagent`,
  `model: inherit`, a per-agent `permission:` block where every shell
  pattern outside the agent's allowlist is `deny` (a `"*": deny` catch-all
  closes the residue), and `question: allow` for dialogue agents,
  `question: deny` for discrete agents (opencode's `AskUserQuestion`
  equivalent is the `question` capability).
- **`.opencode/agents/build.md`** — the maintainer primary. Read-only
  shell (`edit: deny`, `write: deny`); allowed to `task: <any>` (dispatch
  any subagent).

13 files total.

## Materialize-time shadow validation (opencode only)

Because opencode lacks a runtime PreToolUse hook, the adapter cannot
rely on hook-time enforcement to catch an allowlist that subsumes a
global deny. The check has to happen at materialization time, before
the files are written.

`src/adapter/opencode/validation.rs::check_no_shadowing`:

```rust
pub fn check_no_shadowing(
    agents: &[AgentDef],
    deny_patterns: Vec<&str>,
) -> Result<()> {
    for agent in agents {
        let allows: Vec<String> = agent.bash_patterns
            .iter().map(|p| to_opencode_pattern(p)).collect();
        for deny in &deny_patterns {
            // Take the literal prefix of the deny (text before the first `*`).
            let prefix = deny.split('*').next().unwrap_or("").trim();
            if prefix.is_empty() { continue; }
            for allow in &allows {
                let matcher = Glob::new(allow)?.compile_matcher();
                if matcher.is_match(prefix) {
                    return Err(anyhow!("agent '{}' bash pattern '{}' \
                                        shadows global deny '{}' ..."));
                }
            }
        }
    }
    Ok(())
}
```

The algorithm: for each global deny pattern, take its literal prefix
(the part before the first `*`) and test it against every per-agent
allow pattern. If any allow matches the deny prefix, the agent's
allowlist would subsume the deny — which under opencode's per-resource
permission resolution would override the global deny.

This runs at the top of `opencode::install`, **before** any file is
written. A shadow detection short-circuits the install with a clear
error naming the offending agent, the allow pattern, and the deny it
subsumes.

A unit test (`production_roster_does_not_shadow_r13`) holds the live
roster and the live deny list against this check, so any future
loosening of a roster pattern that introduces a shadow fails the test
suite before it can ship.

### Why the production registry had to be tightened

When the opencode adapter was first wired up, two patterns from the
v0.1 registry tripped this check:

- `Bash(git checkout:*)` would match the global deny on
  `git checkout main` (the maintainer wanted to allow only specific
  branches).
- `Bash(git branch:*)` would match denies on dangerous `git branch -D`
  forms.

The fix was narrowing in the registry itself:

```rust
// Test agent's bash_patterns, after narrowing:
"git checkout -b *:*",    // can create branches
"git checkout feat/*:*",  // can checkout feature branches
"git checkout main:*",    // can checkout main
// No bare "git checkout:*" — the gate refused that pattern.
```

This narrowing happens once in `registry.rs` and benefits both adapters:
Claude Code's allowlist becomes more specific (defense in depth even
though the hook would catch a violation), and opencode's materialization
succeeds.

## Idempotent materialization

Both adapters use the same `write_if_changed` helper:

```rust
fn write_if_changed(path: &Path, content: &str, summary: &mut ChangeSummary) -> Result<()> {
    if read_if_exists(path)?.as_deref() == Some(content) {
        summary.unchanged += 1;
        return Ok(());
    }
    write_atomic(path, content.as_bytes())?;
    summary.written += 1;
    Ok(())
}
```

Re-running `dwarven init --host <h>` against an already-installed
adapter produces zero file changes (`unchanged == total`). The
adapter integration tests' "second run is byte-identical" assertion
holds across both adapters.

## Dual install

```bash
dwarven init --host claude-code --host opencode
```

The two adapters touch disjoint directories:

- Claude Code: `.claude/`
- opencode: `.opencode/` + `opencode.json` (root)

There is no shared file, so neither adapter overwrites the other's
output. The maintainer can have both hosts configured and switch
between them per-session.

## Extending: a third adapter

To add a third host (call it `kestrel`):

1. **Add `src/adapter/kestrel/mod.rs`** with a `pub fn install(root: &Path) -> Result<ChangeSummary>`.
2. **Decide the surface.** What files in what directories realize the
   host's primitives for: per-agent prompts + tool gating, maintainer
   primary, orientation copy, global deny floor.
3. **If the host has no runtime hook,** port the shadow-validation gate
   (the `check_no_shadowing` function is generic; just provide the
   host's deny prefix list and the host's pattern translator).
4. **Wire `mod.rs`** to recognize `"kestrel"`.
5. **Update `src/main.rs`'s help text** for `--host` to list the new
   adapter.
6. **Add integration tests** under `tests/adapter_kestrel.rs` parallel
   to the existing two.

The agent-side contract (`AgentDef`, the ten roster entries) does not
change. The per-host adapter is the only thing that touches host
specifics.

## What the adapter does *not* do

- **Does not run during normal operation.** The adapter is invoked once
  at `dwarven init --host <h>` and never again unless the maintainer
  re-runs init.
- **Does not modify the host's tools at runtime.** Allowlist enforcement
  is the host's job (Claude Code's settings + hooks, opencode's
  permission resolution).
- **Does not synthesize substantive agent prompts** (Red Flags tables,
  rationalization lists, etc.). Per CLAUDE.md "Skills are
  behavior-shaping code, not prose" — those need eval evidence per
  agent. Issue #6 tracks the substantive prompt work.
