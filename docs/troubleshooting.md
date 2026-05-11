# Troubleshooting

Common failures and how to recover from them. If you don't find your
issue here, see [where to file a bug](#where-to-file-a-bug) at the
bottom.

---

## Daemon

### `dwarven serve` exits with "another daemon is already running"

**Symptom.** The daemon refuses to start; the message names the PID
recorded in `.dwarven/daemon/pid`.

**Cause.** One of:
1. Another `dwarven serve` is genuinely running.
2. A previous daemon exited without releasing the lock (crash, kill -9,
   abrupt OS shutdown). The PID file is stale.

**Fix.** Check which case you're in:

```bash
dwarven daemon status
```

If `status` reports the recorded PID is alive, that's the live daemon —
stop it explicitly with `dwarven daemon stop` rather than starting a
second one.

If `status` reports the PID is stale, `dwarven daemon stop` will clean
up the lock; then `dwarven serve` will succeed.

If `daemon status` itself is confused, manually inspect:

```bash
cat .dwarven/daemon/pid
ps -p <pid>
```

If the process is genuinely gone, `rm .dwarven/daemon/pid` reclaims the
lock. Only do this when you have verified the process is dead.

### Daemon won't start: port in use

**Symptom.** `Address already in use (os error 48)` (or similar) during
bind.

**Cause.** Another process is bound to `daemon.port` (default `7777`).

**Fix.**

```bash
# Pick a free port
dwarven config set daemon.port 8484

# Or identify what's using it
lsof -iTCP:7777 -sTCP:LISTEN
```

Then `dwarven serve`. The port is read once at startup, so changing it
does not affect a running daemon.

### Web UI shows "connecting…" indefinitely

**Symptom.** The browser loads the page but the connection status pill
never goes "live"; mutations from the UI hang or error.

**Cause.** Usually one of:
1. The daemon is not running.
2. The daemon is on a different port than the UI is hitting.
3. A browser extension or firewall is blocking the SSE stream.

**Fix.**

```bash
# 1. Confirm daemon is up
dwarven daemon status

# 2. Confirm port matches
dwarven config get daemon.port

# 3. Test SSE from the terminal
curl -N http://127.0.0.1:7777/api/v1/events
```

If `curl -N` streams events but the browser doesn't, the cause is in
the browser (extension, content-blocker, or aggressive cache). Try a
private window.

---

## CLI

### `illegal transition '<from>' → '<to>'`

**Symptom.** `dwarven issue transition` exits with code 1 and this
message.

**Cause.** The edge is not in the graph defined by
[`docs/specs/work-states.md`](specs/work-states.md). Most-common case:
trying to go `test → review` (the graph routes through `implement`).

**Fix.** Walk the legal path:

```bash
dwarven issue transition 12 implement
dwarven issue transition 12 review
```

Or, if you genuinely need to skip — e.g. rolling back a state — use
`--override`. The override flag is maintainer-only by convention and
cannot target terminal states (`done`, `dropped`); use `dwarven issue close`
for those.

### `would create a cycle` on `dwarven issue dep add`

**Symptom.** `dep add A blocks B` fails with this error; the message
identifies the offending cycle (e.g. `B → … → A → B`).

**Cause.** Adding the edge would form a cycle in the dependency graph.

**Fix.** Decide which edge in the cycle is wrong and remove it before
adding the new one. Read the chain in the error message; usually one
edge is the "real" dependency and the others were added speculatively.

```bash
# Inspect the affected issues' dependency lists
dwarven issue view A
dwarven issue view B

# Remove the misplaced edge
dwarven issue dep remove <from> <to>
```

### "Index out of sync" — CLI shows different state than web UI

**Symptom.** A field you changed on disk (or via the CLI on a separate
process) doesn't appear in the web UI; or the scheduler ranks an issue
based on stale data.

**Cause.** The SQLite index is derived from the on-disk files. The
daemon's file watcher rebuilds index rows on file events, but events
can be missed (e.g., editor wrote in a way `inotify` didn't see).

**Fix.**

```bash
# If the daemon is running, this is handled in-process:
dwarven reindex

# Or via the daemon endpoint:
curl -X POST http://127.0.0.1:7777/api/v1/daemon/reindex
```

The reconciliation pass also runs periodically (default every 60
seconds, see `daemon.reconciliation_interval_seconds` in
[`docs/configuration.md`](configuration.md)), so untouched divergence
will self-heal eventually.

### Config validation fails at startup

**Symptom.** `dwarven serve` exits immediately with a message like
`scheduler.alpha must be in [0.0, 1.0]`.

**Cause.** A value in `.dwarven/config.toml` is out of range or
otherwise invalid.

**Fix.** The error names the key and the constraint. Look it up in
[`docs/configuration.md`](configuration.md), pick a valid value, and:

```bash
dwarven config set <key> <value>
dwarven serve
```

Validation is the same on `config set` as on `serve` startup, so the
fix is durable.

### Adapter materialization fails: "shadows global deny"

**Symptom.** `dwarven init --host opencode` exits with an error about
a per-agent allow pattern shadowing a global deny.

**Cause.** A pattern in the agent registry would, under opencode's
permission resolution, override an R13 universal-deny. opencode has no
runtime PreToolUse hook as a second line, so the materializer refuses
to write a configuration that would weaken the deny floor.

**Fix.** If you are seeing this from a shipped Dwarven release, file a
bug — the production registry should already be clean. If you are
modifying the registry yourself, narrow the offending allow pattern.
For example, replace `git checkout:*` (would shadow `git checkout
main:deny`) with `git checkout -b *`, `git checkout feat/*`, etc.

---

## Eval runner

### `ANTHROPIC_API_KEY missing or invalid`

**Symptom.** `cargo run --example eval-runner` exits with this error
before running any scenarios.

**Cause.** The eval framework refuses to run without a valid live API
key — it makes real Claude calls and we don't want surprise costs.

**Fix.** Set the environment variable:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
cargo run --example eval-runner evals/spec/refuses-out-of-scope-edits.yaml
```

If the key is set but rejected by the API, the error from Anthropic
flows through; verify the key is active and has API access.

---

## SSE stream

### Reconnect dropped events — am I missing state?

**Symptom.** Network blip dropped your SSE connection; on reconnect you
got a `stream.refresh-required` event.

**Cause.** Your `Last-Event-ID` is older than the oldest entry in the
256-slot ring buffer. The daemon can't replay events that have aged
out, so it emits `stream.refresh-required` to tell you "refetch state
from scratch."

**Fix.** Refetch whatever you care about (e.g. `GET /api/v1/issues`)
and resume the stream. The web UI does this automatically.

If you're seeing `stream.refresh-required` frequently with a short
disconnect, your event rate is high enough to overflow the buffer in
under your reconnect window. For v1 the buffer size is fixed; in v2
this may become configurable.

---

## Storage

### Issue file in `.dwarven/issues/<id>/` looks corrupted

**Symptom.** `dwarven issue view <id>` fails with a frontmatter parse
error, or fields show as wrong types in the UI.

**Cause.** A direct file edit produced invalid YAML or violated a
constraint (e.g., non-list `blocks:`).

**Fix.** Open `.dwarven/issues/<id>/issue.md` in an editor. Compare the
frontmatter against a known-good issue in the same repo. Fix and save.
The file watcher will pick up the change, or run `dwarven reindex` to
force a rebuild.

The on-disk format is intentionally human-editable. The CLI and web UI
are the safer paths, but file edits are a legitimate escape hatch.

### Two clients wrote to the same issue at the same time

**Symptom.** Atomic writes are guarded by advisory locks under
`.dwarven/issues/<id>/.lock`. If two clients race, one wins; the other
exits with a lock-acquisition error.

**Fix.** Retry the losing operation. The error is transient by design;
no manual cleanup is needed.

---

## Where to file a bug

1. **Check the spec** at `docs/specs/<area>.md` first — sometimes the
   reported "bug" is intentional behavior.
2. **Re-read this troubleshooting guide** in case the case matches a
   known issue.
3. **Try `dwarven daemon status` + `dwarven daemon restart`** for
   intermittent issues. Many transient errors are cured by a clean
   restart.
4. **File against the repository** with:
   - Output of `dwarven --version`.
   - The exact command you ran.
   - Full error text (stderr).
   - Daemon log if applicable (`/tmp/dwarven.log` if you ran detached,
     else the terminal output of `dwarven serve`).
   - Relevant `.dwarven/config.toml` excerpt (redact `repo.id` if
     you'd rather not share it).

For questions about behavior (not bugs), see the relevant spec under
[`docs/specs/`](specs/dwarven.md).
