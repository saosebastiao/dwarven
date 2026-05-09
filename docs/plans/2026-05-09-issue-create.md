---
slice: dwarven issue create (slice 2)
date: 2026-05-09
status: resolved (open questions answered 2026-05-09; ready to implement)
---

# Plan: `dwarven issue create`

**Scope:** Implement `dwarven issue create` end-to-end against the v2 storage model. Atomic ID assignment, issue file write, initial creation comment, and reciprocal-edge updates for `--blocks`/`--blocked-by`.

**Why this slice second:** `dwarven init` produced empty `.dwarven/issues/`. Until issues can be created, no other subcommand operates against meaningful state. Creation also exercises the load-bearing primitives (atomic counter, frontmatter writer, comment writer, multi-file edit) that all later mutation subcommands reuse.

## Spec references

- `dwarven-cli.md#R6.2` — subcommand contract: required/optional flags, output
- `dwarven-cli.md#R2.4` — body input: `--body` / `--body-file` / `--body-stdin`
- `dwarven-cli.md#R3` — actor attribution
- `dwarven-cli.md#R5` — exit codes
- `storage-model.md#R2.2` — directory layout (`issues/<id>/issue.md`, `comments/`)
- `storage-model.md#R4.3` — issue frontmatter shape
- `storage-model.md#R4.4` — comment frontmatter shape
- `storage-model.md#R5.2` — atomic ID assignment via `config.toml` counter
- `storage-model.md#R5.3` — per-issue comment seq starting at 1
- `storage-model.md#R6.4` — atomic file writes via temp + rename
- `storage-model.md#R6.5` — multi-file updates not transactional; hub heals on reindex
- `work-states.md#R6.1` — entry-point states per type
- `work-states.md#R3.1` — type vocabulary
- `work-states.md#R5.2` — priority vocabulary

## Concrete behavior

### Invocation

```
dwarven issue create --type <type> --title <text> [body-input] [optional flags]
```

Required: `--type`, `--title`.

Body input (mutually exclusive, optional): `--body <text>` | `--body-file <path>` | `--body-stdin`. Body may be empty.

Optional: `--state <state>`, `--priority <p0|p1|p2>`, `--blocked-by <id,id,...>`, `--blocks <id,id,...>`, `--epic <slug>`.

Global flags inherited: `--actor`, `--repo`, `--quiet`, `--json` (the last is best-effort here — see "Deferred").

### Validation (exit 1 on failure)

- `--type` ∈ `{spec-gap, feature, bug, arch, doc, chore}` (`work-states.md#R3.1`).
- `--state`, if present, ∈ active state set (`work-states.md#R2.3`). Terminal states forbidden at creation.
- `--priority`, if present, ∈ `{p0, p1, p2}`.
- `--title` non-empty after trim, ≤ 120 chars (`storage-model.md#R4.3`, "≤120 chars").
- `--blocked-by` / `--blocks` parse as comma-separated positive integers; each referenced issue must already exist (otherwise exit 3, not found). v1 cycle detection deferred to the dep-graph slice — see "Deferred".
- `--epic` matches `[a-z0-9]+(-[a-z0-9]+)*` (kebab-slug). Spec doesn't define the slug regex precisely; this is a reasonable default and easy to relax later.
- Mutually-exclusive body inputs: pass at most one.

### Default initial state (`work-states.md#R6.1`)

Resolve before write:

| `--type` | default `state` |
|---|---|
| `spec-gap` | `pm` |
| `feature` | `pm` |
| `bug` | `plan` |
| `arch` | `architect` |
| `chore` | `plan` |
| `doc` | `doc` |

If `--state` is supplied, it overrides the default (R6.1.1 permits maintainer override; CLI does not gate on actor for this slice — see "Open question 3").

### ID assignment (`storage-model.md#R5.2`)

1. Open `.dwarven/config.toml` with an OS advisory exclusive lock (`fs2::FileExt::lock_exclusive` or `flock(2)` directly). Hold the lock for the duration of read → increment → write.
2. Parse TOML, read `counters.next_issue_id`.
3. Assigned ID = current `next_issue_id`. New `next_issue_id = current + 1`.
4. Write config back via temp+rename (`storage-model.md#R6.4`); fsync; release lock.

Lock is needed because the daemon (future) and concurrent CLI invocations could race. Even pre-daemon, two parallel `dwarven issue create` runs would otherwise collide. Cost: one extra dependency (`fs2`) and a lock file.

Alternative considered: `O_EXCL` create of a sibling lock file. Rejected as more error-prone for cleanup.

### Filesystem writes (in order)

All writes go through the temp + rename helper (`write_atomic` from `init.rs`, lifted to a shared module).

1. `mkdir -p .dwarven/issues/<id>/comments/` (id zero-padded to 4 digits, R2.3).
2. Write `.dwarven/issues/<id>/issue.md` with frontmatter:
   ```yaml
   ---
   id: <id>
   title: <title>
   type: <type>
   state: <resolved-state>
   priority: <priority?>          # omit key if not set
   blocked_by: [<ids>?]           # omit key if empty
   blocks: [<ids>?]               # omit key if empty
   epic: <slug>?                  # omit key if not set
   created: <iso8601-utc-seconds>
   created_by: <actor>
   updated: <iso8601-utc-seconds> # equal to created at creation
   ---
   <body>
   ```
3. Write the initial comment at `.dwarven/issues/<id>/comments/001-<iso-filename>-<actor>.md`:
   - `<iso-filename>` is `YYYY-MM-DDTHHMMZ` per R4.4.2 (minute precision).
   - Frontmatter:
     ```yaml
     ---
     seq: 1
     issue: <id>
     author: <actor>
     kind: state-change
     created: <iso8601-utc-seconds>
     from: created                  # see Open question 1
     to: <resolved-state>
     ---
     ```
   - Body: `Issue created.` (one-liner placeholder; the maintainer-supplied body lives in `issue.md`, not in the creation comment).
4. For each `id` in `--blocks`: open `.dwarven/issues/<padded>/issue.md`, append the new id to its `blocked_by` array (preserving sort), bump its `updated`, write back atomically.
5. For each `id` in `--blocked-by`: same, but append to its `blocks` array.

Steps 4–5 are not transactional with steps 1–3 (R6.5). If we fail mid-way, the new issue exists with its claimed edges but the counterparty may be missing the reciprocal edge. The hub will flag asymmetric edges on reindex per `storage-model.md#R4.3.6`. Acceptable for v1.

### Output

Human (default): `Created issue #<id>: <title>` plus a one-line state hint, e.g. `state: pm`.

JSON (`--json`): full issue object per `dwarven-cli.md#R4.2` (R6.2.4). For this slice, defer full JSON shape definition to the JSON-output slice; emit a minimal `{"id": <id>, "state": "<state>", "type": "<type>"}` and a one-line stderr warning that JSON output is not yet stable. The maintainer may also defer JSON entirely.

### Exit codes

- 0 success
- 1 user error (validation failures listed above)
- 2 hub error (lock acquisition failure, write failure, TOML parse failure)
- 3 not found (referenced `--blocks`/`--blocked-by` id missing)

## Code organization

```
src/
├── main.rs                 # add `Issue { Create { ... } }` subcommand wiring
├── init.rs
├── issue/
│   ├── mod.rs              # subcommand dispatch
│   └── create.rs           # `dwarven issue create` impl
├── storage/
│   ├── mod.rs
│   ├── config.rs           # config.toml read/write + lock-protected counter increment
│   ├── issue_file.rs       # frontmatter struct + serde + read/write
│   ├── comment_file.rs     # comment frontmatter + filename composition + write
│   └── atomic.rs           # write_atomic helper (lifted from init.rs)
└── time.rs                 # iso8601 helpers (frontmatter form + filename form)
```

Lifting `write_atomic` out of `init.rs` is small and unblocks reuse. No deeper refactor of init.

New deps:
- `fs2` (≈40 KB) — cross-platform advisory file locks. Or use `nix` for a thinner crate set; `fs2` is the standard pick.
- `chrono` with `serde` + `clock` features — ISO 8601 formatting, UTC clock. Alternative: `time` crate. Either works; `chrono` is more widely understood.
- `serde_yaml` — YAML frontmatter (issue.md and comment.md). Note: `serde_yaml` is unmaintained but stable. Alternative: `yaml-rust2`. For v1 I'd take `serde_yaml` and revisit if it bites; the input surface is small.

### YAML frontmatter strategy

Frontmatter is a known fixed shape; use `#[derive(Serialize, Deserialize)]` structs. Body is read/written as a separate string. Round-tripping preserves field order via field order in the struct, not via the YAML library.

I'll skip key-omission-when-None at the YAML level (`serde_yaml` doesn't natively skip None) and instead either:
- Use `#[serde(skip_serializing_if = "Option::is_none")]` per optional field, OR
- Build the YAML frontmatter as a string assembled from field-by-field templates.

Recommendation: use `skip_serializing_if`. Less code, lets serde do the work.

## Verification

Manual end-to-end against the freshly-bootstrapped repo:

1. `cargo run -- issue create --type feature --title "Foo bar" --body "body text"` — observe issue 0001 written, comment 001 written, config.toml counter incremented to 2.
2. `cargo run -- issue create --type bug --title "Test bug"` — observe issue 0002, defaults to state `plan`.
3. `cargo run -- issue create --type feature --title "Linked" --blocks 1` — observe issue 0003 with `blocks: [1]` and issue 0001 updated with `blocked_by: [3]`.
4. `cargo run -- issue create --type feature --title ""` — exit 1, validation error.
5. `cargo run -- issue create --type feature --title "X" --blocks 9999` — exit 3.
6. Run two `issue create` invocations with `&` to verify lock serialization (smoke check; not a stress test).

No automated tests this slice — there is still no test framework in the repo. A test-framework bootstrap is its own slice and should come before slice 3 (`issue view`/`list`) so we can lock down read-side behavior with tests.

## Deferred (out of scope for this slice)

- **Cycle detection on creation.** Per `dwarven-cli.md#R6.11.3` cycle detection is specified for `dep add`. For `--blocks`/`--blocked-by` at creation it isn't specified. Punt to the dep-graph slice (`docs/specs/dep-graph.md`) which will own cycle-detection logic shared by `dep add` and `issue create`.
- **`--state` rejection for non-maintainer actors.** R6.1.1 says maintainer may override; agent override is unspecified. Default behavior in this slice: accept `--state` regardless of actor. Agent allowlists already prevent this in practice (allowlist patterns don't include `--state`), so CLI-level enforcement adds little.
- **Full `--json` output schema.** Emit a minimal stable subset; the JSON-output slice locks the shape.
- **Title length enforcement boundary.** `storage-model.md#R4.3` says ≤120 chars. Validate at the byte length of the title; UTF-8 grapheme counting is overkill.
- **Test framework.** Unit tests + a `tempfile`-based integration harness should land as a separate slice before more subcommands.

## Resolved decisions (2026-05-09)

1. **Creation comment shape.** `kind: state-change`, `from: created` (sentinel string), `to: <resolved-state>`. Body: `Issue created.` This keeps creation visible to any `kind: state-change` filter (state-history queries). **Spec amendment required:** `storage-model.md#R4.4` must note that `from` accepts the sentinel value `created` for the initial creation event. This amendment lands as part of this slice.

2. **Reciprocal-edge writes.** Creation writes both endpoints atomically (per file; not transactional across files per R6.5). Mirrors `dep add` behavior. The hub heals asymmetric edges on reindex if a multi-file write is interrupted.

3. **`--state` actor gate.** CLI accepts `--state` from any actor. Per `dwarven-cli.md#R3.4`, attribution is structural, not authenticated; per R7, allowlist patterns are the enforcement boundary.

4. **YAML library.** `serde_yaml`. Revisit only if we hit a bug.

5. **Date library.** `chrono` with `serde` + `clock` features.

6. **Test framework slice ordering.** Lands as its own slice *after* this one, *before* slice 3 (`issue view`/`list`). This slice ships with manual verification only.

## Spec amendment piggybacked on this slice

`storage-model.md#R4.4` — extend the `from` field description to permit the sentinel value `created` for the initial state-change comment that records issue creation. One-line edit; lands in the same commit as the implementation.
