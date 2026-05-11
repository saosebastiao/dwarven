# CLI reference

Reference for every `dwarven` subcommand, flag, and exit code, with
realistic examples.

The CLI is filesystem-only — it does not require the daemon. Anything the
daemon exposes via HTTP is also reachable from the CLI. The CLI is the
host-portable interface that agents use; see
[`docs/specs/dwarven-cli.md`](specs/dwarven-cli.md) for the formal
contract.

## Global flags

These apply to every subcommand:

| Flag | Effect |
|---|---|
| `--repo <PATH>` | Operate on the repository rooted at `<PATH>` instead of the current directory. Resolved against the cwd. |
| `--actor <NAME>` | Attribution for any mutation. Falls back to `$DWARVEN_ACTOR`, then `maintainer`. Per-host adapter generators bake this into agent allowlist patterns so attribution is structural rather than prompt-level. |
| `--quiet` | Suppress non-essential stdout. Errors still print. |
| `--json` | Machine-parseable JSON output instead of human text. |
| `-h` / `--help` | Print help for the subcommand and exit `0`. |
| `-V` / `--version` | Print version (top-level only). |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | General error (validation, IO, missing file, illegal transition, etc.). The human-readable error is printed to stderr; with `--json`, the error envelope `{"error":{"code":"...","message":"..."}}` is printed to stdout. |
| `2` | clap argument-parsing error. |

---

## `dwarven init`

Initialize `.dwarven/` in the current directory and (optionally)
materialize one or more host adapters.

```
dwarven init [--host <HOST>]...
```

| Argument / flag | Effect |
|---|---|
| `--host <HOST>` | Host adapter to install. Accepts `claude-code` or `opencode`. May be repeated for dual install. Optional. |

Without `--host`, only the `.dwarven/` directory and `config.toml` are
created.

### Examples

```bash
# Bootstrap the storage layout only
dwarven init

# Bootstrap + install Claude Code adapter
dwarven init --host claude-code

# Bootstrap + dual install
dwarven init --host claude-code --host opencode
```

Idempotent — re-running with the same flags is a no-op. Different flags
augment but do not remove an existing adapter installation.

---

## `dwarven issue create`

Create a new issue.

```
dwarven issue create --type <TYPE> --title <TITLE>
                     [--body <BODY> | --body-file <PATH> | --body-stdin]
                     [--state <STATE>] [--priority <p0|p1|p2>] [--epic <SLUG>]
                     [--blocked-by <IDS>] [--blocks <IDS>]
```

| Flag | Effect |
|---|---|
| `--type <TYPE>` | One of `spec-gap`, `feature`, `bug`, `arch`, `doc`, `chore`. Required. |
| `--title <TITLE>` | ≤120 bytes, non-empty after trim. Required. |
| `--body <BODY>` / `--body-file <PATH>` / `--body-stdin` | Body content. Mutually exclusive; if all are omitted, body is empty. |
| `--state <STATE>` | Override the type-default initial state. See `docs/specs/work-states.md` for the routing table. |
| `--priority <p0\|p1\|p2>` | Maintainer-asserted priority. Lower number = higher priority. Omit for "unset". |
| `--epic <SLUG>` | Epic slug (kebab-case). Groups issues in the deps view. |
| `--blocked-by <IDS>` | Comma-separated IDs that block this one. |
| `--blocks <IDS>` | Comma-separated IDs this one blocks. |

### Examples

```bash
dwarven issue create --type feature --title "rename Export to Share" \
  --priority p1 --epic v2-launch --body "Marketing copy update."

# Body from a draft file
dwarven issue create --type bug --title "scheduler crashes on cycle" \
  --priority p0 --body-file /tmp/draft.md

# Linked at creation
dwarven issue create --type chore --title "tidy unused imports" \
  --blocked-by 12,13
```

Prints the assigned ID, state, and on-disk path. `--json` returns the full
created record.

---

## `dwarven issue view`

View an issue, its frontmatter, body, and comment thread.

```
dwarven issue view <ID> [--no-comments] [--state-history] [--last <N>]
```

| Flag | Effect |
|---|---|
| `--no-comments` | Print header + body only. |
| `--state-history` | Filter comments to state-change records only. |
| `--last <N>` | Show only the most recent N comments. |

### Examples

```bash
dwarven issue view 12

# Just the state-change history
dwarven issue view 12 --state-history

# Last three comments only
dwarven issue view 12 --last 3
```

---

## `dwarven issue list`

List issues, optionally filtered. Output is one issue per line, sorted
per `--sort`.

```
dwarven issue list [--state <CSV>] [--type <CSV>] [--blocker <CSV>]
                   [--priority <CSV>] [--epic <SLUG>] [--grep <STR>]
                   [--open | --closed | --all] [--sort <FIELD>]
```

| Flag | Effect |
|---|---|
| `--state <CSV>` | Comma-separated state values. |
| `--type <CSV>` | Comma-separated type values. |
| `--blocker <CSV>` | Comma-separated blocker values (`maintainer-input`, `external`, `upstream`). |
| `--priority <CSV>` | Comma-separated priorities. `unset` matches issues with no priority. |
| `--epic <SLUG>` | Restrict to a single epic. |
| `--grep <STR>` | Literal-string filter against title and body. |
| `--open` (default) | Active issues only. |
| `--closed` | `done` and `dropped` only. |
| `--all` | All issues regardless of state. |
| `--sort <FIELD>` | One of `id`, `created`, `updated` (default), `priority`. |

### Examples

```bash
# Everything currently waiting on the planner
dwarven issue list --state plan

# All P0 + P1 features
dwarven issue list --type feature --priority p0,p1

# All issues in the v2-launch epic, including closed
dwarven issue list --epic v2-launch --all

# Find by phrase
dwarven issue list --grep "scheduler"
```

---

## `dwarven issue transition`

Move an issue to a new state. Validated against the state graph in
`docs/specs/work-states.md`.

```
dwarven issue transition <ID> <NEW_STATE> [--comment <STR>] [--override]
```

| Argument / flag | Effect |
|---|---|
| `<NEW_STATE>` | Target state (see work-states.md for the legal graph). |
| `--comment <STR>` | Optional rationale; recorded on the auto-emitted state-change comment. |
| `--override` | Force a transition not in the legal graph. Maintainer-only by convention; cannot transition into terminal states (`done`, `dropped`) — use `dwarven issue close` for those. |

### Examples

```bash
dwarven issue transition 12 pm
dwarven issue transition 12 plan --comment "PM accepted scope, ready for planning."

# Recover from a wrong state (maintainer only)
dwarven issue transition 12 spec --override --comment "rolled back; spec needs more work"
```

Exits `1` with "illegal transition" if the edge is not in the graph and
`--override` is not set.

---

## `dwarven issue close`

Terminally close an issue to `done` (default) or `dropped`.

```
dwarven issue close <ID> [--dropped]
                    (--comment <STR> | --comment-file <PATH> | --comment-stdin)
```

The closure comment is required — terminal transitions must carry a
rationale.

### Examples

```bash
dwarven issue close 12 --comment "Doc landed."

# Drop with reason
dwarven issue close 12 --dropped --comment "Superseded by #14."
```

---

## `dwarven issue comment`

Add a comment to an issue.

```
dwarven issue comment <ID> (--body <STR> | --body-file <PATH> | --body-stdin)
```

### Examples

```bash
dwarven issue comment 12 --body "Spotted a regression; reopening might be needed."
dwarven issue comment 12 --body-file /tmp/long-note.md
echo "from stdin" | dwarven issue comment 12 --body-stdin
```

---

## `dwarven issue blocker set` / `clear`

Set or clear the issue's blocker field.

```
dwarven issue blocker set <ID> <BLOCKER> [--comment <STR>]
dwarven issue blocker clear <ID> [--comment <STR>]
```

`<BLOCKER>` is one of `maintainer-input`, `external`, `upstream`.

### Examples

```bash
dwarven issue blocker set 12 maintainer-input \
  --comment "Waiting on architecture decision."

dwarven issue blocker clear 12 --comment "Decision recorded in #15."
```

A non-`none` blocker pushes the issue to the maintainer's inbox.

---

## `dwarven issue priority`

Set the maintainer-asserted priority. Maintainer-only by convention.

```
dwarven issue priority <ID> <p0|p1|p2>
```

`p0` is highest. This is the *input* priority; the scheduler computes
*effective* priority by combining this with downstream-unblocking value.
See `docs/specs/dep-graph.md`.

### Examples

```bash
dwarven issue priority 12 p1
```

To clear an asserted priority, edit the frontmatter directly (the field
is omitted to indicate "unset") — there is no `--clear` on this
subcommand.

---

## `dwarven issue priority-override`

Force an absolute scheduler rank, bypassing the dep-graph computation.

```
dwarven issue priority-override <ID> <VALUE>
dwarven issue priority-override <ID> --clear
```

`<VALUE>` is any finite number; higher = ranked higher in the queue.
Negative values are accepted (place them after `--` if your shell tries
to parse them as flags).

### Examples

```bash
# Pin this issue to the top
dwarven issue priority-override 12 100

# Bury it without dropping
dwarven issue priority-override 12 -- -50

# Restore algorithmic ranking
dwarven issue priority-override 12 --clear
```

---

## `dwarven issue edit`

Edit low-churn frontmatter fields. Title, type, and epic only — body
changes go through comments or direct file edits.

```
dwarven issue edit <ID> [--title <STR>] [--type <TYPE>] [--epic <SLUG>]
```

### Examples

```bash
dwarven issue edit 12 --title "docs/getting-started.md: quickstart walkthrough"
dwarven issue edit 12 --epic user-docs
dwarven issue edit 12 --type doc
```

Changing `--type` does *not* re-route the state. Use `transition` if a
state change is also needed.

---

## `dwarven issue dep add` / `remove`

Manage dependency edges between issues.

```
dwarven issue dep add <FROM> blocks <TO>
                      [--rationale <STR> | --rationale-file <PATH> | --rationale-stdin]
dwarven issue dep remove <FROM> <TO>
```

The literal word `blocks` between IDs is required for readability:
`dep add 5 blocks 7` reads "5 blocks 7." Both endpoints' frontmatter
(`blocks:`, `blocked_by:`) are updated atomically.

### Examples

```bash
# 5 must close before 7 can start
dwarven issue dep add 5 blocks 7 \
  --rationale "7 imports the API surface 5 is defining."

dwarven issue dep remove 5 7
```

Cycle detection rejects edges that would create a cycle; the error
identifies the offending cycle.

---

## `dwarven serve`

Run the coordination hub daemon in the foreground.

```
dwarven serve
```

The daemon:
- acquires `.dwarven/daemon/pid` (exits `1` if another daemon is running),
- validates `.dwarven/config.toml`,
- probes the existing SQLite index, then unconditionally rebuilds it from files,
- starts the file watcher on `.dwarven/issues/`,
- binds the HTTP API and serves the web UI,
- handles `SIGTERM` / `SIGINT` gracefully (releases the PID lock,
  flushes the SSE ring buffer, closes the DB).

To run detached:

```bash
nohup dwarven serve >/tmp/dwarven.log 2>&1 &
```

Use `dwarven daemon stop` or `dwarven daemon restart` rather than killing
by signal directly.

---

## `dwarven daemon status` / `stop` / `restart`

Inspect or control the running daemon.

```
dwarven daemon status
dwarven daemon stop
dwarven daemon restart
```

- `status` — Prints PID, port, uptime, recent reindex outcome, SSE buffer occupancy.
  Exit `0` if running, `1` if not.
- `stop` — Sends `SIGTERM` and waits for the daemon to exit. Cleans up
  the PID lock if it was stale.
- `restart` — Stop (if running) then `serve` in the foreground.

---

## `dwarven schedule next`

Print the top N issues from the dep-graph scheduler.

```
dwarven schedule next [--count <N>] [--state <STATE>] [--actionable-only]
```

| Flag | Effect |
|---|---|
| `--count <N>` | How many rows to print. Default `1`. |
| `--state <STATE>` | Restrict to issues currently in this state (e.g. `plan` to find the next planning candidate). |
| `--actionable-only` | Only show issues whose dependencies are clear (no open `blocked_by`). |

### Examples

```bash
# What should I do next?
dwarven schedule next

# Top 5 unblocked candidates
dwarven schedule next --count 5 --actionable-only

# Next thing for the test agent
dwarven schedule next --state test
```

Output includes `id`, `state`, `title`, score, override (if any), and
effective rank.

---

## `dwarven reindex`

Drop and rebuild `.dwarven/index.sqlite` from the on-disk files. The
daemon also does this on startup; this subcommand is for when the index
and files have drifted (manual file surgery, partial restore, etc.).

```
dwarven reindex
```

If the daemon is running, the reindex happens inside it (so the file
watcher and HTTP API see a consistent view). Otherwise it's an offline
rebuild.

---

## `dwarven config get` / `set`

Read or write a value in `.dwarven/config.toml` by dotted key.

```
dwarven config get <KEY>
dwarven config set <KEY> <VALUE>
```

Values are parsed as int → float → bool → string in that order. To force
a string, quote it: `dwarven config set repo.name '"my repo"'`.

See [`docs/configuration.md`](configuration.md) for every key, its
default, and whether changing it requires a daemon restart.

### Examples

```bash
dwarven config get daemon.port
dwarven config set daemon.port 8484
dwarven config set scheduler.alpha 0.5
dwarven config get scheduler.priority_weights.p0
```

Mutations are written atomically (write-temp + rename).
