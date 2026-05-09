---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Dwarven CLI

This document specifies the `dwarven` command-line surface — subcommands, flags, output formats, exit codes, and the per-agent allowlist patterns that scope the CLI's mutation surface for each agent.

Per `dwarven.md#R2.10`, agents interact with the hub *exclusively* through this CLI. Agents do not call the hub's HTTP API directly. The CLI's filesystem-only behavior is specified in `coordination-hub.md#R2`.

---

## R1 — Scope

R1.1 — This spec defines the `dwarven` binary's command-line surface for both the maintainer and dispatched agents.

R1.2 — Out of scope: subcommand implementation details (those are an architecture concern); the hub HTTP API (`web-api.md`); the on-disk format read or written by subcommands (`storage-model.md`); the daemon's runtime behavior (`coordination-hub.md`).

---

## R2 — Invocation conventions

R2.1 — All subcommands use the form `dwarven <noun> <verb> [args] [flags]`. Examples: `dwarven issue create`, `dwarven daemon stop`. Single-word top-level commands (`dwarven init`, `dwarven serve`, `dwarven reindex`) are permitted for setup and daemon-management ergonomics.

R2.2 — Global flags accepted by every subcommand:

| Flag | Purpose |
|---|---|
| `--actor <name>` | Attribution for any mutation; defaults to env `DWARVEN_ACTOR`, then `maintainer` (R3) |
| `--json` | Machine-parseable JSON output (R4.2) |
| `--repo <path>` | Operate on the repository rooted at `<path>`; defaults to discovery from CWD |
| `--quiet` | Suppress non-essential stdout |
| `--help` | Print subcommand help and exit |

R2.3 — Long flags use `--kebab-case`. Short flags are reserved for the most common per-noun operations (e.g., `-t` for `--title`).

R2.4 — Multi-line text input (issue bodies, comments) accepts: `--body <text>` (single-line); `--body-file <path>` (read from file); `--body-stdin` (read from stdin). The three are mutually exclusive.

---

## R3 — Actor attribution

R3.1 — Every mutation (any subcommand that writes a file) records an `author` value in the resulting comment or state-change record (`storage-model.md#R4.4`).

R3.2 — The author is determined by, in priority order: the `--actor` flag; the `DWARVEN_ACTOR` environment variable; the literal string `maintainer`.

R3.3 — Per-host adapters (`host-adapter.md`) are responsible for setting `DWARVEN_ACTOR` per agent invocation so that subagents do not need to pass `--actor` explicitly. For example, the Claude Code adapter sets `DWARVEN_ACTOR=spec` for the Spec subagent.

R3.4 — The CLI does not enforce that an agent only acts as itself. Actor attribution is structural (recorded in every comment) but not authenticated — the maintainer's adapter is trusted. This mirrors v0.1's prompt-level discipline (see `dwarven.md#R6.4` analogue) and is acceptable for a single-maintainer model.

---

## R4 — Output formats

R4.1 — **Human-readable (default).** Subcommands produce structured, terminal-formatted output: tables for lists, headers for views, color where the terminal supports it. Output is for human consumption; format may change between versions.

R4.2 — **JSON (`--json`).** Subcommands produce a single JSON document on stdout. The JSON shape is stable within a major version (v2.x). Agents that parse output must use `--json`.

R4.3 — Errors are printed to stderr in human-readable form; with `--json`, errors go to stderr as a JSON object with `error` and `code` keys, and stdout is empty.

R4.4 — Subcommands that produce no meaningful output on success (e.g., `dwarven issue transition`) print a one-line confirmation in human mode; in JSON mode they print `{"ok": true, ...}` with the resulting artifact's identifying fields.

---

## R5 — Exit codes

R5.1 — Standard exit codes:

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | User error (bad flags, invalid input, illegal transition) |
| 2 | Hub error (file-system failure, lock contention, corrupted state) |
| 3 | Not found (issue ID, comment seq, etc.) |
| 4 | Daemon error (only for daemon-management subcommands) |

R5.2 — `dwarven issue transition` rejecting an illegal transition (per `work-states.md#R6.2`) exits 1 with an error message naming the legal transitions from the current state.

---

## R6 — Subcommand catalog

### R6.1 — `dwarven init`

R6.1.1 — Initializes `.dwarven/` in the current directory. Creates `.dwarven/config.toml` (with default port, generated repo identity, initial next-id counter at 1), `.dwarven/issues/`, and `.dwarven/.gitignore`.

R6.1.2 — Idempotent. Running on a repo that already has `.dwarven/` reports the existing state and exits 0 without writes.

### R6.2 — `dwarven issue create`

R6.2.1 — Required flags: `--type <type>`, `--title <text>`. Body via R2.4.

R6.2.2 — Optional flags: `--state <state>` (defaults per `work-states.md#R6.1`); `--priority <p0|p1|p2>`; `--blocked-by <id,id,...>`; `--blocks <id,id,...>`; `--epic <slug>`.

R6.2.3 — Assigns the next issue ID atomically (`storage-model.md#R5.2`), writes `.dwarven/issues/<id>/issue.md`, creates an initial state-change comment recording creation.

R6.2.4 — On success, prints the assigned ID. With `--json`, returns the full issue object.

### R6.3 — `dwarven issue view <id>`

R6.3.1 — Reads and prints the issue's frontmatter and body, followed by all comments in chronological order.

R6.3.2 — Flags: `--no-comments` (issue only); `--state-history` (state-change comments only); `--last <n>` (most recent N comments).

### R6.4 — `dwarven issue list`

R6.4.1 — Prints a table of issues, defaulting to active states only (R5.4 of `work-states.md`).

R6.4.2 — Filter flags: `--state <s,s,...>`; `--type <t,t,...>`; `--blocker <b,b,...>`; `--priority <p,p,...>`; `--epic <slug>`; `--open` (default; excludes terminal states); `--closed` (terminal states only); `--all`.

R6.4.3 — Body-search flag: `--grep <pattern>` runs a literal-string filter against issue titles and bodies. Regex is not supported in v1.

R6.4.4 — Sort flag: `--sort <field>` where field ∈ `id`, `created`, `updated`, `priority`. Default: `updated` descending.

### R6.5 — `dwarven issue transition <id> <new-state>`

R6.5.1 — Performs a state transition per the transition graph in `work-states.md#R6.2`.

R6.5.2 — Required when present: a brief `--comment <text>` describing why (recommended; not strictly required).

R6.5.3 — `--override` flag permits a transition not in `work-states.md#R6.2`. Only valid when `--actor maintainer` (or unset → defaults to maintainer). Agents using `--override` produce a CLI error.

R6.5.4 — Records a `state-change` comment per `storage-model.md#R3.3` and `R4.4`.

### R6.6 — `dwarven issue comment <id>`

R6.6.1 — Appends a `kind: comment` comment to the issue. Body via R2.4.

R6.6.2 — Sequence number assigned atomically per `storage-model.md#R5.3`.

### R6.7 — `dwarven issue close <id>`

R6.7.1 — Terminal transition. Defaults to `done`. `--dropped` flag transitions to `dropped` instead.

R6.7.2 — `--comment <text>` (or via R2.4) is required and recorded as the closure comment.

R6.7.3 — `close` is the terminal-closure escape hatch and accepts both targets from any active state: `done` per `work-states.md#R6.2.4`, `dropped` per `work-states.md#R6.2.2`. Issues already in a terminal state are rejected (R7.4 absorbing). The per-state graph in R6.2 governs `dwarven issue transition`, not `close`.

### R6.8 — `dwarven issue blocker set <id> <blocker>` / `dwarven issue blocker clear <id>`

R6.8.1 — `set` requires a blocker value from `work-states.md#R4.2`. `--comment <text>` describes context.

R6.8.2 — `clear` removes the blocker field. `--comment <text>` describes the resolution.

R6.8.3 — Writes the issue frontmatter and emits a `blocker-set` or `blocker-cleared` comment per `storage-model.md#R4.4`.

### R6.9 — `dwarven issue priority <id> <p0|p1|p2>`

R6.9.1 — Sets the issue's `priority` frontmatter field. Maintainer-only by convention; agents must not invoke this.

### R6.10 — `dwarven issue edit <id>`

R6.10.1 — Edits low-churn frontmatter fields: `--title <text>`, `--type <type>`, `--epic <slug>`. State, priority, and blocker have dedicated subcommands and may not be set here.

R6.10.2 — Editing `type` or `title` is restricted to maintainer or Triage by convention; the CLI does not enforce.

### R6.11 — `dwarven issue dep add <from-id> blocks <to-id>` / `dwarven issue dep remove <from-id> <to-id>`

R6.11.1 — Manages dependency edges. `add` updates both endpoints atomically (writes `from-id`'s `blocks` and `to-id`'s `blocked_by`). `remove` clears the edge from both endpoints.

R6.11.2 — Flag `--rationale <text>` (or `--rationale-file`, `--rationale-stdin`) records why the edge exists; appended as a comment on the `from-id` issue.

R6.11.3 — Cycle detection: `add` rejects edges that would create a cycle in the dep graph (`dep-graph.md`). Rejection produces a CLI error naming the cycle path.

### R6.12 — `dwarven serve`

R6.12.1 — Starts the daemon (`coordination-hub.md#R3`).

R6.12.2 — Flags: `--port <n>` (default per `coordination-hub.md#R3.3`); `--background`.

### R6.13 — `dwarven daemon stop` / `dwarven daemon status` / `dwarven daemon restart`

R6.13.1 — Daemon-management subcommands. `stop` sends SIGTERM and waits for graceful shutdown. `status` reports running / not-running and the port. `restart` is `stop` then `serve`.

### R6.14 — `dwarven reindex`

R6.14.1 — Rebuilds the SQLite index from `.dwarven/` files (`storage-model.md#R8.1`). Functions whether or not the daemon is running. If the daemon is running, the daemon performs the reindex; otherwise the CLI does.

### R6.15 — `dwarven config get <key>` / `dwarven config set <key> <value>`

R6.15.1 — Reads or writes `.dwarven/config.toml`. Key paths use dotted notation (e.g., `daemon.port`).

R6.15.2 — Setting a key while the daemon is running prints a warning that the change requires a daemon restart (per `coordination-hub.md#R9.5`).

---

## R7 — Per-agent allowlist patterns

R7.1 — Each agent's tool allowlist is specified in `agent-roster.md`. The `Bash` patterns scope `dwarven` invocations to the agent's responsibilities. Pattern matching is prefix-based per the host's convention; finer enforcement (e.g., "transition only to `implement`") is prompt-level (per R3.4 rationale).

R7.2 — Examples (illustrative; canonical lists in `agent-roster.md`):

| Agent | Representative allowlist patterns |
|---|---|
| Spec | `Bash(dwarven issue view:*)`, `Bash(dwarven issue list:*)`, `Bash(dwarven issue comment:*)`, `Bash(dwarven issue close:*)` |
| Architect | Spec patterns + `Bash(dwarven issue create:*)` (for filing `type:arch` issues) |
| Gap | `Bash(dwarven issue view|list|create|comment:*)` |
| PM | `Bash(dwarven issue view|list|create|comment|edit|transition|close:*)` |
| Planning | `Bash(dwarven issue view|comment|transition:*)` |
| Test Dev | `Bash(dwarven issue view|comment|transition:*)` |
| Implementation | `Bash(dwarven issue view|comment|transition|blocker:*)` |
| Code Review | `Bash(dwarven issue view|comment|transition:*)` |
| Doc | `Bash(dwarven issue view|comment|close:*)` |
| Triage | `Bash(dwarven issue view|list|edit|comment|blocker|transition:*)` |

R7.3 — No agent's allowlist may include `dwarven serve`, `dwarven daemon *`, `dwarven init`, `dwarven config set`, `dwarven reindex`, or `dwarven issue priority`. These are maintainer-only.

R7.4 — No agent's allowlist may include `--override` patterns for `dwarven issue transition`. Override is structurally maintainer-only per R6.5.3.

---

## R8 — Stability guarantees

R8.1 — Within a major version (v2.x), the `--json` output schema for any subcommand is stable. Field additions are minor-version changes; field removals or renames are major-version changes.

R8.2 — Subcommand names, flag names, and exit codes are stable within a major version.

R8.3 — Human-readable output format (default mode) is not stable. Agents must not parse default-mode output; they must use `--json`.

---

## R9 — Out of scope for v1

R9.1 — **Search.** No full-text search subcommand. `dwarven issue list --grep` covers basic filtering. Web UI (via daemon's SQLite FTS) handles richer search.

R9.2 — **Bulk operations.** No `dwarven issue bulk transition` or similar. Maintainer scripts via shell loops if needed.

R9.3 — **Plan / spec / architecture management.** Plans, specs, and architecture docs are managed with normal file operations (`Write`, `Edit`, git). The CLI does not have `dwarven plan create`. Agents that author these documents do so directly.

R9.4 — **Cross-repo operations.** All subcommands operate on a single repository (R2.2 `--repo` flag).

R9.5 — **Templating.** No `dwarven issue create --template <name>`. Issue body templates are out of scope; maintainers/agents compose bodies directly.

R9.6 — **Custom subcommand extensibility.** No plugin/extension mechanism in v1.
