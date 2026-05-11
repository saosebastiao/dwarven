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

## R4 — opencode adapter (v3)

The opencode adapter is a v3 deliverable. opencode is an open-source AI coding-agent CLI with primary/subagent isolation, per-agent permission gating, and configuration via markdown agent files plus `opencode.json` (`https://opencode.ai/docs/`).

The posture is **best-effort with documented gaps** per `dwarven.md#R3.4` working convention: where opencode has a clean primitive we use it; where it lacks one we document the gap and compensate via prompt-level discipline or process tooling, or declare the affected agent unsupported.

This section specs the adapter contract concretely. Implementation lives under issue #5 in the project tracker.

### R4.1 — Agent materialization

R4.1.1 — Each abstract agent in `agent-roster.md` is materialized as an opencode subagent file at `.opencode/agents/<name>.md` with frontmatter: `description`, `mode: subagent`, `model` (per-agent override or inheritance), and `permission` (per-tool gating, R4.6).

R4.1.2 — Subagent files mirror Claude Code's contract one-to-one: same prompt body content, same scope fences, same exit conditions. The two adapters share no on-disk artifacts (R4.10), but the rendered prose for each agent is byte-identical between adapters modulo host-specific frontmatter.

R4.1.3 — Subagent isolation is provided natively by opencode: each invocation runs in a separate child session with its own context, prompt, and permissions. The parent does not inherit the subagent's context, satisfying `R2.2.4`.

R4.1.4 — Subagents are invocable two ways:
- Programmatic dispatch from a primary agent via opencode's `Task` tool (`@<name>` mention or `Task(<name>, prompt)`).
- Direct user invocation via the `@<name>` mention in chat.

Slash commands (R4.2) are the primary maintainer-facing dispatch surface; `Task` is the agent-to-agent dispatch surface where authorized (R4.6, mirroring `R13.5`).

### R4.2 — Slash command registration

R4.2.1 — opencode does not have a Claude-Code-equivalent project-level slash-command primitive at the time of this writing. The adapter substitutes the `@<name>` mention pattern: each entry-point dispatch is documented in the project's `AGENTS.md` (R4.5) as "type `@<agent> <topic>` to dispatch the agent."

R4.2.2 — If opencode adds a native slash-command primitive in a future release, the adapter SHOULD migrate to it (one-keystroke dispatch per `R2.3`) and the spec is amended accordingly.

R4.2.3 — Custom shell aliases or scripts that wrap `opencode run --agent <name> "<topic>"` are acceptable maintainer-side ergonomics; they are not part of the materialized adapter output.

### R4.3 — Maintainer shell configuration

R4.3.1 — The maintainer's primary agent (the top-level opencode session) uses the `Build` built-in agent or a project-specific primary defined at `.opencode/agents/build.md`. Its `permission` block declares the maintainer-shell allowlist analogous to `R3.3.1`:

```yaml
permission:
  read: allow
  edit: deny
  write: deny
  bash:
    "git status*": allow
    "git diff*": allow
    "git log*": allow
    "git show*": allow
    "git branch*": allow
    "git rev-parse*": allow
    "git merge-base*": allow
    "dwarven --actor maintainer issue view*": allow
    "dwarven --actor maintainer issue list*": allow
    "dwarven --actor maintainer daemon status*": allow
    "dwarven --actor maintainer config get*": allow
    "*": deny
  task:
    "*": allow
  question: allow
```

R4.3.2 — The shell's allowlist excludes any mutating bash pattern, any `edit`/`write` permission, and any `dwarven` pattern that mutates hub state (mutations route through dispatched agents — `R2.4.2`). The `task` permission with `*: allow` lets the shell dispatch any subagent; per-agent permissions (R4.6) constrain what each subagent can do.

### R4.4 — Interactive question primitive

R4.4.1 — opencode exposes a `question` permission key gating a question-asking primitive. The adapter includes `question: allow` in dialogue agents' permission blocks (`agent-roster.md#R2.2`) and `question: deny` in discrete-work agents' (`R2.3`).

R4.4.2 — opencode's question primitive is currently chat-based (free-text) rather than structured-options like Claude Code's `AskUserQuestion`. The dialogue protocol's "one question + 2-3 alternatives + your lean" framing (`dialogue.md#R3.2`) is achieved via prompt content: the dialogue agents render their question and alternatives as Markdown in the response body. The maintainer answers in free text.

R4.4.3 — Detached dispatch suppression of the question primitive (`R2.5.2`) is implemented by overriding `question: deny` in the runtime permission for the detached invocation (R4.8). Mechanism: the detached-dispatch wrapper writes a per-session permission override before invoking the agent.

### R4.5 — Session orientation

R4.5.1 — opencode does not have a SessionStart hook (the config docs explicitly list no hooks system). The adapter substitutes opencode's `AGENTS.md` mechanism: orientation copy is materialized at `AGENTS.md` in the repo root (or `.opencode/AGENTS.md` if the maintainer prefers a tooling-scoped location), which opencode auto-loads into the model's context at session start.

R4.5.2 — Orientation copy covers the same surface as the Claude Code SessionStart hook (`R3.6.1`): the hub workflow primer, the agent dispatch catalog (`@<name>` instead of `/<name>`), web UI URL, and daemon-status guidance. The daemon-status check at session start is NOT automatic under opencode — the orientation copy includes "run `dwarven daemon status` to check the hub" as a manual step.

R4.5.3 — The trade-off is that opencode's orientation is content-only, not behavioral. Claude Code can run `dwarven daemon status` automatically and inject the result; opencode cannot. The maintainer who wants the daemon-status-injection ergonomics picks Claude Code.

### R4.6 — Actor attribution

R4.6.1 — Each agent's bash permission patterns carry the agent's name as a literal in the `--actor` position. Example for the Spec agent's `.opencode/agents/spec.md`:

```yaml
permission:
  bash:
    "dwarven --actor spec issue view*": allow
    "dwarven --actor spec issue list*": allow
    "dwarven --actor spec issue comment*": allow
    "dwarven --actor spec issue close*": allow
    "git status*": allow
    "git diff*": allow
    "git log*": allow
    "git add*": allow
    "git commit*": allow
    "git push origin main*": allow
    "*": deny
```

R4.6.2 — opencode's pattern syntax is simpler than Claude Code's: `*` matches any string, `?` matches one character, literal otherwise. The semantics are equivalent for our prefix-pattern use case. The trailing `"*": deny` is mandatory in every agent's bash permission block — without it, opencode's default-permissive behavior would allow unlisted commands.

R4.6.3 — Agent attribution is structural at the allowlist boundary (parallel to `R2.6.2` for Claude Code). The agent cannot invoke `dwarven` without `--actor <agent-name>` because no other pattern is allowed; the agent cannot impersonate another agent because the patterns embed its own name.

### R4.7 — Per-agent permission examples

R4.7.1 — A full agent file under opencode looks like:

```markdown
---
description: Edit docs/specs/*.md and the changelog in dialogue with the maintainer.
mode: subagent
model: inherit
permission:
  read: allow
  edit:
    "docs/specs/**": allow
    "docs/CHANGELOG.md": allow
    "*": deny
  write:
    "docs/specs/**": allow
    "docs/CHANGELOG.md": allow
    "*": deny
  task: deny
  question: allow
  bash:
    "git status*": allow
    "git diff*": allow
    "git log*": allow
    "git add*": allow
    "git commit*": allow
    "git push origin main*": allow
    "dwarven --actor spec issue view*": allow
    "dwarven --actor spec issue list*": allow
    "dwarven --actor spec issue comment*": allow
    "dwarven --actor spec issue close*": allow
    "*": deny
---

# spec agent

[body identical to the Claude Code adapter's spec.md, minus the
`tools:` frontmatter which lives in the permission block above]
```

R4.7.2 — opencode's `edit` and `write` permissions support glob patterns (`docs/specs/**`). This is finer-grained than Claude Code's per-tool-name allowlist; under opencode the scope-fence paths are mechanism-enforced rather than prompt-level discipline. This is a *strict improvement* over the Claude Code adapter for path-scoped agents (Spec, Architect, Planning, Test Dev, Implementation, Doc).

### R4.8 — Universal constraint enforcement

R4.8.1 — `agent-roster.md#R13` ("never list") is enforced via the global `permission` block in `opencode.json` at the project root, materialized by the adapter:

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
  }
}
```

R4.8.2 — opencode's permission semantics are "last match wins" merged with agent-specific overrides taking precedence. The `agent-roster.md#R13` deny patterns at global scope are the floor; per-agent allow patterns are subject to the deny floor in opencode's resolution. Net effect: an agent's allowlist cannot weaken the universal deny set.

R4.8.3 — **Gap**: Claude Code's adapter uses a `PreToolUse` hook (`R3.7.1`) as a second line of defense beyond permission lists. opencode has no hooks system. The adapter has only one layer of enforcement (the permission block). If a per-agent permission file is misconfigured to allow a universally-forbidden pattern, opencode's "last match wins" semantics may favor the agent rule over the global deny depending on rule ordering. The maintainer mitigation: validate materialized `.opencode/agents/*.md` against the R13 deny list at `dwarven init --host opencode` time (the adapter must check that no agent file shadows a global deny).

R4.8.4 — `agent-roster.md#R13.5` ("Agent tool restricted to maintainer / Spec / Architect"): enforced via per-agent `permission.task` settings. Spec and Architect get `task: allow` (or specific subagent allowlist); every other agent gets `task: deny`. The maintainer's primary agent (R4.3) is `task: "*": allow`.

### R4.9 — Detached dispatch (v1.1+ under opencode = v3+)

R4.9.1 — opencode's CLI supports `opencode run --agent <name> "<prompt>"` for non-interactive invocation. The detached-dispatch wrapper uses this against a per-event prompt template.

R4.9.2 — The wrapper writes a per-session permission override (R4.4.3) before invoking, stripping `question` from the runtime permission so detached-dispatched agents cannot block on a user response (`R2.5.2`).

R4.9.3 — Triggering events come from the hub's SSE stream (`web-api.md#R5`); the wrapper subscribes and dispatches matching events into `opencode run`. Same pattern as the Claude Code adapter (`R3.8.3`).

### R4.10 — Coexistence with the Claude Code adapter

R4.10.1 — The two adapters share no on-disk artifacts. Claude Code writes to `.claude/`; opencode writes to `.opencode/`. A repository may install both adapters simultaneously (`dwarven init --host claude-code --host opencode`); each writes to its own host-specific directory; the maintainer chooses which host to launch.

R4.10.2 — Per-agent content (the prompt body) is rendered from one source of truth in `src/adapter/<host>/agents.rs`. Both adapters share the same agent contracts from `agent-roster.md` and produce byte-identical prompt bodies modulo host-specific frontmatter.

### R4.11 — Supported agents under opencode

R4.11.1 — All ten agents in `agent-roster.md` are supported under opencode with the following caveats:

| Agent | Status | Notes |
|---|---|---|
| Spec | Supported | `question` is free-text, not structured options (R4.4.2). |
| Architect | Supported | Same caveat as Spec. |
| Gap | Supported | Same caveat as Spec; detached path uses R4.9. |
| PM | Supported | Same caveat as Spec; detached path uses R4.9. |
| Planning | Supported | Same caveat as Spec; detached path uses R4.9. |
| Test Dev | Supported | Discrete-work agent; no question primitive needed. |
| Implementation | Supported | Discrete-work agent; same as Test Dev. |
| Code Review | Supported | Discrete-work agent; same as Test Dev. |
| Doc | Supported | Discrete-work agent; same as Test Dev. |
| Triage | Supported | Discrete-work agent; scheduled detached via R4.9. |

R4.11.2 — The single substantive degradation under opencode vs Claude Code is the dialogue agents' question-asking ergonomics: free-text instead of structured options. The maintainer who prefers the structured-options affordance picks Claude Code.

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
