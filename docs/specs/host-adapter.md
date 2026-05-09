---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Host Adapter

This document specifies the contract that any AI coding-agent host must satisfy to support Dwarven, plus the v1 Claude Code adapter and the v3 opencode adapter sketch.

The contract is what makes Dwarven host-agnostic (`dwarven.md#R1.5`). Per-host adapters translate the abstract agent definitions in `agent-roster.md` into the host's native primitives (subagents, commands, hooks, etc.) and provide the runtime glue (interactive question primitive, actor attribution, session orientation).

---

## R1 — Scope

R1.1 — This spec defines: the abstract host-adapter contract; the v1 Claude Code adapter; the v3 opencode adapter sketch; and the `dwarven init --host <h>` invocation that materializes a host's adapter into a target repository.

R1.2 — Out of scope: the agents themselves (`agent-roster.md`); the CLI surface they call (`dwarven-cli.md`); the dialogue protocol (`dialogue.md`); the on-disk artifacts they read/write (`storage-model.md`).

---

## R2 — The abstract host-adapter contract

R2.1 — A host adapter must provide all of the following capabilities. Failure to provide any one of them is grounds for rejecting the host as unsupported.

### R2.2 — Agent materialization

R2.2.1 — The adapter must take each abstract agent definition from `agent-roster.md` (system prompt + tool allowlist + scope fences) and materialize it as a host-native subagent (or the host's nearest equivalent — an isolated execution context with its own prompt and tool access).

R2.2.2 — The materialized subagent's tool allowlist must reflect the agent's spec'd allowlist 1:1, including the universal constraints (`agent-roster.md#R13`).

R2.2.3 — The materialized subagent must be invocable as a one-shot dispatch that returns a single result (the conversation does not persist across dispatches).

R2.2.4 — The materialized subagent must be isolated from its parent: the parent does not inherit the subagent's context.

### R2.3 — Slash command registration

R2.3.1 — The adapter must register a slash command per dispatch entry-point (`/spec`, `/architect`, `/gap`, `/pm`, `/plan`, `/test`, `/implement`, `/review`, `/doc`, `/triage`).

R2.3.2 — The slash command must dispatch the corresponding subagent.

R2.3.3 — If the host does not natively support slash commands, the adapter must provide an equivalent one-keystroke dispatch mechanism documented in the adapter's notes.

### R2.4 — Maintainer shell configuration

R2.4.1 — The adapter must configure the host's top-level session (the *shell*) with: read-only access to the workspace and to `dwarven` CLI read subcommands; the `Agent` (subagent dispatch) primitive; the host's interactive question primitive.

R2.4.2 — The shell must not be allowed to mutate state directly. All mutations route through dispatched agents.

### R2.5 — Interactive question primitive

R2.5.1 — The adapter must surface the host's interactive question primitive (per `dialogue.md#R3.1`) to dialogue agents only (`agent-roster.md#R2.2`).

R2.5.2 — The adapter must suppress or otherwise prevent the interactive question primitive in detached dispatch contexts (`dialogue.md#R4.4`). Mechanism is host-specific; the requirement is mechanism-enforced unavailability, not prompt-level discipline.

### R2.6 — Actor attribution

R2.6.1 — The adapter must arrange for each agent's `dwarven` CLI invocations to carry the agent's name as `--actor` (per `dwarven-cli.md#R3`).

R2.6.2 — The recommended mechanism is to bake `--actor <agent-name>` into every `dwarven` allowlist pattern for that agent (e.g., the Spec agent's allowlist contains `Bash(dwarven --actor spec issue view:*)`, never `Bash(dwarven issue view:*)`). This makes attribution structural at the allowlist boundary rather than dependent on prompt obedience.

R2.6.3 — Adapters that prefer environment-variable propagation (`DWARVEN_ACTOR`) instead of explicit flags must ensure the env var cannot be overridden by the agent.

### R2.7 — Session orientation

R2.7.1 — The adapter must orient the maintainer at the start of each shell session: surface the dwarven workflow primer (the v2 successor to the inherited `using-dwarven` skill), confirm the hub daemon is reachable (or prompt to start it), and print the web UI URL.

R2.7.2 — Orientation is one-shot per session; it must not be re-injected on every message.

### R2.8 — Universal constraint enforcement

R2.8.1 — The adapter must enforce the universal allowlist constraints in `agent-roster.md#R13` at the host's permission layer (e.g., a hook or settings rule), in addition to per-agent allowlist declarations. Defense in depth — both layers must agree.

R2.8.2 — If the host cannot enforce a particular pattern at the permission layer, the adapter must document the gap and the prompt-level discipline that compensates.

### R2.9 — Detached dispatch (v1.1+)

R2.9.1 — The adapter should provide a detached-dispatch mechanism: a way to invoke an agent without an interactive shell, triggered by external events (file change, schedule, hub event). v1 may ship without this and rely on the maintainer running slash commands manually; v1.1 adds the automation.

R2.9.2 — When detached dispatch ships, the adapter must guarantee that the interactive question primitive is unavailable to the dispatched agent (R2.5.2).

---

## R3 — Claude Code adapter (v1)

The Claude Code adapter is the v1 reference implementation.

### R3.1 — Agent materialization

R3.1.1 — Each abstract agent in `agent-roster.md` is materialized as a Claude Code subagent file at `.claude/agents/<name>.md` with frontmatter: `name`, `description`, `tools` (the allowlist), `model` (per-agent override if any).

R3.1.2 — The subagent's system prompt is the agent's spec content rendered in prose form, including: trigger expectations, inputs, outputs, exit conditions, scope fences, dialogue mode, and pointers to `dwarven.md` and the agent's section in `agent-roster.md`.

R3.1.3 — The Claude Code `Agent` tool dispatches the subagent. Subagent isolation is provided by Claude Code natively (parent does not inherit subagent context).

### R3.2 — Slash command registration

R3.2.1 — Each entry-point slash command is registered as a Claude Code slash command at `.claude/commands/<name>.md`. The command body is a thin dispatch invoking `Agent(subagent_type="<name>", prompt="...")` with the maintainer's argument forwarded.

### R3.3 — Maintainer shell configuration

R3.3.1 — The shell's tool allowlist is declared in `.claude/settings.json` and must include: `Agent`, `Read`, `AskUserQuestion`, and read-only `Bash` patterns: `Bash(git log:*)`, `Bash(git diff:*)`, `Bash(git status:*)`, `Bash(git show:*)`, `Bash(dwarven --actor maintainer issue view:*)`, `Bash(dwarven --actor maintainer issue list:*)`, `Bash(dwarven --actor maintainer daemon status:*)`.

R3.3.2 — The shell's allowlist must exclude: `Write`, `Edit`, `NotebookEdit`; any mutating `Bash(git ...)` pattern; any mutating `Bash(dwarven ...)` pattern.

### R3.4 — Interactive question primitive

R3.4.1 — `AskUserQuestion` is a Claude Code built-in tool. The adapter includes it in dialogue agents' allowlists (`agent-roster.md#R2.2`) and excludes it from discrete-work agents' allowlists (`agent-roster.md#R2.3`).

R3.4.2 — Detached dispatch suppression of `AskUserQuestion` is implemented by the adapter's detached-dispatch hook (R3.7), which strips it from the runtime allowlist if present.

### R3.5 — Actor attribution

R3.5.1 — Each agent's allowlist patterns carry the agent's name as a literal in the `--actor` position. Example for the Spec agent:

```
Bash(dwarven --actor spec issue view:*)
Bash(dwarven --actor spec issue list:*)
Bash(dwarven --actor spec issue comment:*)
Bash(dwarven --actor spec issue close:*)
```

R3.5.2 — The maintainer's shell uses `--actor maintainer` patterns (R3.3.1).

### R3.6 — Session orientation

R3.6.1 — A SessionStart hook at `.claude/hooks/session-start.json` (or registered via `.claude/settings.json` hooks block) auto-loads the v2 `using-dwarven` skill. The skill's body covers: the hub workflow, slash command catalog, web UI URL, and the dispatch model.

R3.6.2 — The SessionStart hook also checks for a running daemon (via `dwarven daemon status`) and prints a one-line note: either the web UI URL or the start command.

### R3.7 — Universal constraint enforcement

R3.7.1 — The hard-never list (`agent-roster.md#R13`) is enforced by a PreToolUse hook at `.claude/hooks/pre-tool-use.{js,sh,...}` (host-language-flexible). The hook receives subagent context where available and rejects matching tool calls.

R3.7.2 — The PreToolUse hook supplements the per-agent allowlist declarations; it is the second line of defense. Per-agent allowlists are the first.

### R3.8 — Detached dispatch (v1.1+)

R3.8.1 — Claude Code's headless invocation (`claude --print` or equivalent) is used for detached dispatch. The adapter provides a wrapper that maps a hub event (e.g., issue transitioned to `state: plan`) to a headless dispatch of the corresponding agent.

R3.8.2 — The detached-dispatch wrapper must strip `AskUserQuestion` from the runtime allowlist (R3.4.2).

R3.8.3 — Triggering events are provided by the hub's SSE stream (`web-api.md#R5`); the wrapper subscribes and dispatches matching events.

---

## R4 — opencode adapter (v3 sketch)

The opencode adapter is a v3 deliverable. This section sketches the contract; the full spec is written in v3.

R4.1 — opencode is an open-source AI coding-agent CLI with its own primitives (subagents, commands, hooks, MCP). The adapter must map Dwarven's abstract contract (R2) onto opencode's specifics.

R4.2 — Open questions to resolve in v3:

- Does opencode expose subagent isolation equivalent to Claude Code's `Agent` primitive? (If not, the adapter must approximate it via process boundaries.)
- Does opencode support per-tool allowlists with prefix-matched patterns? (Required for the granular `Bash(dwarven --actor <name> ...)` approach in R2.6.)
- Does opencode have an interactive question primitive analogous to `AskUserQuestion`? (Required for dialogue agents.)
- Does opencode support hooks (SessionStart, PreToolUse equivalents)? (Required for orientation and universal constraint enforcement.)

R4.3 — If any of R4.2's open questions resolves negatively, the adapter must document the gap and either compensate (e.g., via prompt-level discipline) or declare the affected agent unsupported under the opencode adapter.

R4.4 — opencode adapter materialization output lives at `.opencode/<adapter-files>` (exact paths TBD per opencode's conventions).

R4.5 — The two adapters share no on-disk artifacts. A repository may install both adapters simultaneously; each writes to its own host-specific directory; the maintainer chooses which host to launch.

---

## R5 — Adapter installation

R5.1 — `dwarven init --host <h>` materializes the named adapter into the current repository (`dwarven-cli.md#R6.1`). Permitted values: `claude-code` (v1), `opencode` (v3+).

R5.2 — `dwarven init` without `--host` prompts the maintainer to choose, defaulting to `claude-code` in v1.

R5.3 — `dwarven init` is idempotent within an adapter: re-running on a repo that already has the adapter installed reports current state and exits 0 without writes (`dwarven-cli.md#R6.1.2`).

R5.4 — `dwarven init` may be invoked with multiple `--host` flags to install multiple adapters at once (e.g., `dwarven init --host claude-code --host opencode`).

R5.5 — The adapter installation must be non-destructive in retrofit mode: detect existing host-specific files (e.g., a pre-existing `.claude/settings.json`) and merge non-destructively, printing a diff and requiring explicit confirmation before any write that modifies maintainer content.

---

## R6 — Out of scope for v1

R6.1 — **Adapter for hosts other than Claude Code (v1) and opencode (v3).** Cursor, Continue.dev, Aider, etc. are not targeted. The contract in R2 is published; community contributions are welcome but not maintained as part of the core project.

R6.2 — **Hot-swap of adapter at runtime.** Adapter installation is a one-shot operation; switching hosts requires a fresh `dwarven init`.

R6.3 — **Cross-host shared subagent state.** No mechanism for an agent dispatched under Claude Code to hand off to an agent running under opencode mid-task. Each host runs independently.

R6.4 — **Adapter versioning independent of spec versioning.** Adapter version tracks the spec version (an adapter built against spec v2.0 supports spec v2.x). Cross-version compatibility is not maintained.
