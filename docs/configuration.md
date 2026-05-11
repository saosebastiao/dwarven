# Configuration

Reference for `.dwarven/config.toml` — every section, every key, its
default, what changing it does, and whether the daemon must be restarted
to pick up the change.

The formal schema lives at [`docs/specs/coordination-hub.md`](specs/coordination-hub.md)
section R10; this doc is the user-facing reference.

## Where it lives

`.dwarven/config.toml`, alongside the issues directory. It is checked into
git. The daemon validates it at startup (`coordination-hub.md#R10.6`); an
invalid value causes `dwarven serve` to exit with code 1 before any side
effects (no PID lock, no HTTP bind).

## Default config

`dwarven init` writes:

```toml
# Dwarven hub configuration. Schema: coordination-hub.md#R10.

[repo]
id = "<uuid>"
name = "<derived from directory>"

[counters]
next_issue_id = 1

[daemon]
port = 7777
bind = "127.0.0.1"
reconciliation_interval_seconds = 60

[scheduler]
alpha = 0.5

[scheduler.priority_weights]
p0 = 4
p1 = 2
p2 = 1
unset = 1

[triage]
stale_threshold_days = 14
```

## Reading and writing

```bash
dwarven config get <KEY>
dwarven config set <KEY> <VALUE>
```

Keys are dotted paths into the TOML tree:
`daemon.port`, `scheduler.priority_weights.p0`. `set` parses the value
as int → float → bool → string in that order; quote with embedded
quotes to force a string. Writes are atomic.

You can also edit the file directly. Both paths go through the same
validation when the daemon next reads.

---

## `[repo]`

Immutable identity for the repository. Set once at `init`; treat as
read-only thereafter.

| Key | Type | Default | Restart? | Notes |
|---|---|---|---|---|
| `id` | UUID string | generated at init | n/a (do not change) | Used in cross-repo references and (eventually) federation. Changing it after issues exist will break references. |
| `name` | string | derived from directory | live | Cosmetic; used in UI titles. Safe to change. |

---

## `[counters]`

Internal counters managed by the CLI. Do not edit by hand.

| Key | Type | Default | Restart? | Notes |
|---|---|---|---|---|
| `next_issue_id` | int | `1` | n/a | Bumped atomically each time `dwarven issue create` runs. |

---

## `[daemon]`

Runtime parameters for `dwarven serve`.

| Key | Type | Default | Restart? | Notes |
|---|---|---|---|---|
| `port` | int (1–65535) | `7777` | **yes** | TCP port for the HTTP API + web UI. Read once at startup; changing requires `dwarven daemon restart`. If the port is in use, the daemon exits with a clear error. |
| `bind` | string | `"127.0.0.1"` | **yes** | Intended to control the bind address. **Known limitation:** v1 daemon hardcodes the bind to `127.0.0.1` and reads this field for validation only. Don't rely on it. |
| `reconciliation_interval_seconds` | int (≥1) | `60` | live | How often the file watcher's reconciliation pass runs (a defense against missed inotify events). Re-read each loop iteration; lowering it increases CPU overhead, raising it increases the worst-case staleness window. |

### Example

```bash
# Move to a different port
dwarven config set daemon.port 8484
dwarven daemon restart

# Tighten reconciliation cadence (live)
dwarven config set daemon.reconciliation_interval_seconds 30
```

---

## `[scheduler]`

Inputs to the dep-graph scheduler ([`docs/specs/dep-graph.md`](specs/dep-graph.md)).
All values are re-read per scheduler query, so changes are live.

| Key | Type | Default | Restart? | Notes |
|---|---|---|---|---|
| `alpha` | float (0.0–1.0) | `0.5` | live | Weight on downstream-unblocking value relative to the issue's own priority. `0.0` ignores downstream and ranks by raw priority; `1.0` ranks almost entirely by how much each issue unblocks. The default `0.5` is a balanced starting point. |

### `[scheduler.priority_weights]`

Per-priority base scores. The score the scheduler uses for an issue is
its weight, plus `alpha` times the (recursively discounted) sum of its
direct dependents' scores.

| Key | Type | Default | Restart? | Notes |
|---|---|---|---|---|
| `p0` | float (>0) | `4` | live | Weight for `p0`-priority issues. |
| `p1` | float (>0) | `2` | live | Weight for `p1`. |
| `p2` | float (>0) | `1` | live | Weight for `p2`. |
| `unset` | float (>0) | `1` | live | Weight when no priority is set. |

**Invariants** (validated at startup *and* on each scheduler query):

- All four weights must be strictly positive.
- `p0 ≥ p1 ≥ p2`. `unset` is unconstrained relative to `p2`.

### Tuning

- **Want more "unblock the chain" behavior?** Raise `alpha`.
- **Want strict priority ordering?** Set `alpha = 0.0`.
- **Want bigger gaps between priority tiers?** Spread the weights
  (e.g. `p0 = 10, p1 = 3, p2 = 1`). The ratios matter, not the
  absolute scale.

The maintainer can also override the algorithmic ranking on a per-issue
basis with `dwarven issue priority-override <id> <value>`; the scheduler
treats override values as absolute ranks. See
[`docs/cli-reference.md`](cli-reference.md#dwarven-issue-priority-override).

### Example

```bash
dwarven config set scheduler.alpha 0.7
dwarven config set scheduler.priority_weights.p0 10
```

---

## `[triage]`

Parameters for triage helpers (state-staleness flags, future inbox
heuristics).

| Key | Type | Default | Restart? | Notes |
|---|---|---|---|---|
| `stale_threshold_days` | int (≥1) | `14` | live | Number of days an issue can sit in a non-terminal state before triage flags it as stale. **Known limitation:** v1 validates the key but the triage surface that consumes it is not yet wired up. The value is reserved for forward compatibility. |

---

## Validation errors

If `dwarven serve` exits with a configuration error, the message names
the bad key and the constraint it violated. Common cases:

| Error | Cause | Fix |
|---|---|---|
| `daemon.port N out of range [1, 65535]` | Set a value outside the valid port range. | `dwarven config set daemon.port <valid>` |
| `scheduler.alpha must be in [0.0, 1.0]` | Float out of range. | Set to a value in `[0.0, 1.0]`. |
| `scheduler.priority_weights must satisfy p0 >= p1 >= p2` | Reordered weights. | Restore the invariant. |
| `triage.stale_threshold_days must be a positive integer` | Set to `0` or negative. | Set to a value ≥ 1. |
| `daemon.reconciliation_interval_seconds must be a positive integer` | Set to `0` or negative. | Set to a value ≥ 1. |

See [`docs/troubleshooting.md`](troubleshooting.md) for recovery
procedures when the daemon won't start.
