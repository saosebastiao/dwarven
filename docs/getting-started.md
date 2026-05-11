# Getting started

A walkthrough from zero to a first agent dispatch in about ten minutes.

This guide assumes you have used a terminal and a git repository before, but
it does not assume you have used Claude Code, opencode, or any other AI coding
agent host.

## What you'll have at the end

- A repository with `.dwarven/` initialized.
- One host adapter installed (either Claude Code or opencode).
- The local coordination hub running.
- The web UI open at `http://127.0.0.1:8474`.
- One issue filed and dispatched to its first agent.

## Prerequisites

- **Rust toolchain**, 1.74 or newer. Install via [rustup](https://rustup.rs)
  if you don't have it.
- **git**, any reasonably recent version.
- **One host**: either
  - [Claude Code](https://docs.claude.com/en/docs/agents-and-tools/claude-code/overview)
    (CLI, desktop, or IDE extension), or
  - [opencode](https://opencode.ai).

  You can install both if you want; Dwarven coexists with both adapters in a
  single repository. Pick one for this walkthrough.

## 1. Build and install `dwarven`

Clone the Dwarven repository and install the binary:

```bash
git clone https://github.com/<you>/dwarven.git
cd dwarven
cargo install --path .
```

`cargo install` writes the `dwarven` binary to `~/.cargo/bin/`. If that
directory is not already on your `PATH`, add it.

Verify:

```bash
dwarven --version
```

## 2. Initialize a repository

Switch to the repository you want Dwarven to manage. It can be the Dwarven
repository itself or any other git repository.

```bash
cd ~/code/my-project
dwarven init
```

This creates a `.dwarven/` directory with:

- `config.toml` — repo identity, daemon settings, scheduler weights.
- `issues/` — empty; each issue is one directory under here.
- `index.sqlite` — derived query index (reproducible from files).
- `daemon/` — runtime state (pidfile, log).

Commit it:

```bash
git add .dwarven/
git commit -m "chore: dwarven init"
```

> `.dwarven/index.sqlite`, `.dwarven/daemon/`, and the WAL files are
> ignored by git on Dwarven's behalf — `dwarven init` writes a
> `.dwarven/.gitignore`. The files-of-record (issues, config) are tracked.

## 3. Install a host adapter

The host adapter materializes the ten Dwarven agents onto your host's
primitives. Pick one:

### Claude Code

```bash
dwarven init --host claude-code
```

Creates:

- `.claude/agents/<name>.md` — one per agent, with the system prompt,
  the tool allowlist (including `Bash(dwarven --actor <name> ...)`
  patterns for actor attribution), and the model.
- `.claude/commands/<name>.md` — slash commands (`/spec`, `/architect`,
  …) that dispatch each agent.
- `.claude/hooks/pre-tool-use.sh` — defense-in-depth on the global
  never-list.
- `.claude/settings.json` — project-wide allow + deny patterns and the
  hook registration.

### opencode

```bash
dwarven init --host opencode
```

Creates:

- `.opencode/agents/<name>.md` — one per agent with `mode: subagent` and
  per-agent `permission:` blocks.
- `.opencode/agents/build.md` — maintainer primary; can `task:` any subagent.
- `.opencode/AGENTS.md` — orientation copy that opencode auto-loads at
  session start. Lists the available `@spec`, `@architect`, ... mentions.
- `opencode.json` — repo-root config with the universal-deny floor.

The Claude Code adapter relies on a `PreToolUse` hook as a runtime
second line of defense. opencode has no equivalent hook primitive, so the
opencode adapter validates pattern shadowing at materialization time
instead — `dwarven init --host opencode` will refuse to write files if
any per-agent allow pattern would shadow a global deny.

Commit the host directory:

```bash
git add .claude/ .opencode/ opencode.json 2>/dev/null
git commit -m "chore: install host adapter"
```

## 4. Start the coordination hub

The hub is the local daemon that owns the SQLite index, the HTTP API, the
web UI, and the file watcher. The CLI works fully without it; the daemon
is required only for the web UI.

Run it in the foreground in its own terminal:

```bash
dwarven serve
```

You should see something like:

```
daemon listening on http://127.0.0.1:8474
index: healthy (12 issues, 4 dependencies)
file watcher: watching .dwarven/issues/
```

Leave this terminal open. `Ctrl-C` shuts the daemon down cleanly.

To run detached:

```bash
nohup dwarven serve >/tmp/dwarven.log 2>&1 &
dwarven daemon status
```

`dwarven daemon stop` shuts it down; `dwarven daemon restart` cycles.

## 5. Open the web UI

In a browser, visit:

```
http://127.0.0.1:8474
```

You'll land on the **Inbox** — issues that currently need your attention
(maintainer states, dialogue replies, blocker triage). With a fresh repo
this is empty.

The left nav has:

- **Inbox** — what needs you next.
- **Issues** — full list with filters.
- **Deps** — the dependency DAG, grouped by epic.
- **Schedule** — the dep-graph scheduler's ranked queue.
- **Daemon** — status, reindex, shutdown.
- **Config** — `config.toml` reader/editor.

Updates are real-time over SSE. Mutating from CLI updates the UI within
a tick.

## 6. File your first issue

Either via the web UI ("New issue" on the Issues screen) or the CLI:

```bash
dwarven issue create \
  --type feature \
  --title "rename the export button to 'Share'" \
  --priority p1 \
  --body "Marketing prefers 'Share' over 'Export' for the v2 launch copy."
```

The output shows the assigned ID and the initial state. For `type: feature`
that's `spec` (the Spec agent owns it next); for `type: doc` it's `doc`
(routes straight to the Doc agent); see `docs/specs/work-states.md` for
the full type → initial-state routing.

View it:

```bash
dwarven issue view 1
```

## 7. Dispatch your first agent

Open your host shell from the same repository directory.

### Claude Code

In a Claude Code session in this repo, run the slash command for the
agent that owns the issue. For a fresh `feature` issue:

```
/spec 1
```

The Spec agent will open a dialogue with you to elaborate the issue into
a specification. It asks one question at a time, presents 2–3
alternatives with its lean, and only writes once you've converged.

### opencode

In an opencode session:

```
@spec please pick up issue 1
```

opencode's `@<name>` mention dispatches the named subagent.

In both cases, the agent transitions the issue (e.g. `spec` → `gap` or
`spec` → `pm`) as it completes its scope, and a state-change comment
records who did what when.

## 8. Walk the pipeline once

The pipeline is:

```
spec → gap → pm → plan → test → implement → review → doc → done
```

Each transition is a state change owned by the corresponding agent. You
can drive the whole sequence yourself for a tiny issue:

```bash
# After Spec has produced the spec body
dwarven issue transition 1 --to pm

# After PM
dwarven issue transition 1 --to plan

# ...and so on; or use the agents.
```

In practice you dispatch the next agent and it transitions when done.

The terminal states are `done` and `dropped`. Use the close convenience:

```bash
dwarven issue close 1                    # → done
dwarven issue close 1 --to dropped       # → dropped, with a reason in --body
```

## What to read next

- **CLI** — every subcommand + flag + example: [`docs/cli-reference.md`](cli-reference.md).
- **Configuration** — every `config.toml` key, what changing it does,
  whether a daemon restart is required: [`docs/configuration.md`](configuration.md).
- **HTTP API** — the local daemon's API surface, for scripts, custom UIs,
  or CI integration: [`docs/http-api-reference.md`](http-api-reference.md).
- **Web UI** — screen-by-screen walkthrough: [`docs/web-ui.md`](web-ui.md).
- **Troubleshooting** — common issues and recovery: [`docs/troubleshooting.md`](troubleshooting.md).
- **Specs** — the source of truth for the contracts:
  [`docs/specs/dwarven.md`](specs/dwarven.md) is the top-level entry
  point.
