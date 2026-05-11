---
spec_ref: —
date: 2026-05-11
issue: 20
---

# Testing

The test surface and the patterns it expects. A contributor adding a
feature should be able to find the right place to write a test from
this doc alone.

## Layout

```
src/<module>/<file>.rs       # `#[cfg(test)] mod tests { ... }` — unit tests
tests/<area>.rs              # one integration crate per concern
tests/common/mod.rs          # shared harness (dwarven() builder, fresh_repo, etc.)
evals/<agent>/<scenario>.yaml # agent-prompt eval scenarios
```

**Unit tests** live in the same file as the code they exercise, gated
on `#[cfg(test)]`. They test pure functions and module-internal logic
that doesn't touch the filesystem or spawn subprocesses. There are
~35 unit tests across the codebase.

**Integration tests** live under `tests/`. Each file is its own crate,
compiled and linked separately by cargo. They invoke the `dwarven`
binary via `assert_cmd` and assert on stdout, stderr, exit code, and
filesystem state. There are ~26 integration files comprising the bulk
of the ~270-test suite.

**Eval scenarios** live under `evals/` as YAML files. They are
consumed by the eval runner (`cargo run --example eval-runner`), not
by `cargo test`. The runner makes real Anthropic API calls and is
gated on `ANTHROPIC_API_KEY`. See
[`agent-eval.md`](agent-eval.md).

## The `common` harness

`tests/common/mod.rs` exposes the patterns every integration test
uses:

```rust
pub fn dwarven(repo: &Path) -> Command {
    let mut cmd = Command::cargo_bin("dwarven").expect("dwarven binary built");
    cmd.arg("--repo").arg(repo);
    cmd.env_remove("DWARVEN_ACTOR");
    cmd
}

pub fn fresh_repo() -> TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    dwarven(tmp.path()).args(["init"]).output().expect("init runs");
    tmp
}

pub fn read_dwarven(repo: &Path, rel: &str) -> String { ... }
```

Each integration test gets its own tempdir-rooted repo. There is no
shared state across tests; cargo's parallel test runner can shred the
suite across cores safely.

The `DWARVEN_ACTOR` env-var scrub matters: if the test runner inherits
`DWARVEN_ACTOR=spec` from the surrounding shell, every test mutation
would attribute to "spec" and break assertions on actor names. Always
construct `dwarven` invocations via `common::dwarven()`.

The `dead_code` allow at the top of `common/mod.rs` is necessary
because each `tests/*.rs` is its own crate; helpers used by some files
but not others would otherwise warn in the unused crates.

## Determinism rules

Tests rely on several deterministic properties of the implementation;
breaking these will manifest as flaky tests.

### Byte-reproducible reindex

`crate::index::rebuild` produces a `.index.sqlite` whose raw bytes are
identical across two consecutive rebuilds on the same canonical files.
The mechanisms (DELETE journal mode, deterministic insertion order,
VACUUM at the end) are documented in
[`storage-layout.md`](storage-layout.md). `tests/reindex.rs::reindex_is_idempotent`
asserts byte-identity via `std::fs::read` + `assert_eq!` on the raw
bytes — if you make the index non-deterministic, this is the test
that fails.

### Atomic file writes

Every hub-tracked file write goes through `crate::storage::atomic::write_atomic`.
Tests that observe a file mid-write (e.g., watcher-debounce tests) rely
on never seeing a half-written file. If you introduce a write path that
bypasses `write_atomic`, that's a regression — file watchers may see
inconsistent intermediate state and emit spurious reindex events.

### Frontmatter field ordering

`IssueFrontmatter` derives `Serialize` with a stable field order. Tests
that snapshot issue.md content (`tests/issue_create.rs::created_file_shape`)
match the serialized order exactly. Reordering the struct breaks them.
Same applies to `Comment` and to `config.toml` (via `toml_edit`,
which preserves authored key order).

## Daemon tests

`tests/daemon.rs` and `tests/api_*.rs` spawn the daemon as a child
process. Two harness pieces handle the moving parts:

### Port allocation

```rust
static NEXT_PORT: AtomicU16 = AtomicU16::new(18000);

fn allocate_port() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}
```

The daemon's default port is 7777. Under cargo's parallel test runner,
N daemons would collide on it. Each daemon test reserves a unique port
starting from 18000 via the atomic counter.

This works *within* a single test crate. Cargo runs each `tests/*.rs`
crate as its own process, and the atomic resets per crate — so two
crates could in principle pick the same port. In practice the
collision window is small enough that we haven't hit it; if it ever
becomes flaky, switch to OS-assigned ports (bind to `:0`, read back).

### Ready polling

After `dwarven serve` is spawned, the test polls until the daemon's
HTTP endpoint is reachable, with a 5-second timeout:

```rust
const READY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);
```

Spawning is async-unfriendly (no PID inheritance signal). Polling is
the pragmatic answer at this scale. Don't sleep blindly — the polling
loop completes in tens of milliseconds in practice.

### Cleanup

Test teardown sends `SIGTERM` and waits for the pidfile to disappear
(the "pidfile-absence-as-stop-signal" trick documented in
[`cli-vs-daemon.md`](cli-vs-daemon.md)). Don't `kill -9` — that
leaves a zombie under cargo test's process model and the next test in
the same crate spawning on the same repo path will fail to acquire the
lock.

## What we don't mock

### SQLite

Tests open and write real `.index.sqlite` files. Rationale: SQLite's
schema-evolution surface (PRAGMA `integrity_check`, `user_version`,
DELETE journal mode) is exactly the surface we care about. A mock
would miss the engine-specific quirks the design relies on. The bundled
SQLite (rusqlite's `bundled` feature) keeps test runs hermetic.

Cost: ~6s test runtime end-to-end. Acceptable.

### File IO

Tests write real files and observe real `inotify` / `FSEvents` events
(via `notify`). Rationale: atomic-write semantics, advisory `flock`
behavior, and rename-then-fsync ordering are the entire reason
`storage/atomic.rs` exists. Mocking these out tests a different
implementation.

Tempdirs (`tempfile::tempdir()`) isolate each test's filesystem
footprint. Drop semantics clean up after the test exits.

### Anthropic API

The eval runner makes real API calls. There is no mock-Anthropic
fixture, by design — the eval framework's value is asserting on the
actual model's behavior. Tests requiring a mock would be testing the
runner, not the agent.

`cargo test` (the unit + integration suite) does NOT invoke the API.
The eval runner is a separate `cargo run --example eval-runner`
invocation. The unit tests under `src/eval/` test the pure pieces
(scenario parsing, matcher logic, mock tool implementations) — no
network.

## Where to put a new test

| Concern | Test home |
|---|---|
| Pure function in `src/<m>/<f>.rs` | `#[cfg(test)] mod tests` at the bottom of that file. |
| A new CLI subcommand | `tests/<verb>.rs` (one file per verb area, e.g., `tests/issue_create.rs`). |
| HTTP API endpoint | `tests/api_<resource>.rs` (e.g., `tests/api_mutations.rs`). |
| SSE / events behavior | `tests/api_events*.rs`. |
| Daemon lifecycle | `tests/daemon.rs` (spawn-and-stop patterns), `tests/daemon_*.rs` for sub-behaviors. |
| Adapter materialization output | `tests/adapter_<host>.rs` (parallel layout per host). |
| Web UI rendering | `tests/api_web.rs` — currently only asserts that key renderer names and endpoint paths are present in the embedded JS bundle; behavioral correctness rides on the underlying API tests. There is no headless-browser harness. |
| Agent prompt behavior | `evals/<agent>/<scenario>.yaml`. Run with `cargo run --example eval-runner`. |
| Config schema or scheduler algorithm | unit-test in `src/<config\|scheduler>/<f>.rs::tests`. |
| Storage layout / atomic write | unit-test in `src/storage/<f>.rs::tests`. |

When you're not sure: integration tests are the default. Prefer adding
a test that runs the binary end-to-end over a tighter unit test that
only exercises a function in isolation. The integration surface is
where the spec contracts are observable.

## Running tests

```bash
cargo test                              # everything except evals
cargo test --test api_mutations         # one integration crate
cargo test -- --nocapture               # see println output
cargo run --example eval-runner evals/spec/refuses-out-of-scope-edits.yaml
```

`cargo test` runs the suite in ~6s on a recent laptop. The bottleneck
is daemon spawn-and-wait latency (each daemon test pays ~100-200ms).

If `cargo test --doc` ever becomes a thing (it does not today, because
the rustdoc-doctest harness adds compile time without proportional
value at our scale), it would go via the same `cargo test` invocation.

## Coverage shape

Current rough numbers as of `b6fcd3e` (recent main):

- ~35 unit tests across `src/`.
- ~235 integration tests across 26 files.
- 2 reference eval scenarios under `evals/` (spec + test agent).

This is the contract-level surface. Numerous edge cases and error
shapes have one-test coverage each; no concerted attempt at coverage
percentages has been made because the goal is "every spec invariant
has at least one test" — measuring against that is qualitative.

## A few patterns worth absorbing

### Asserting on the issue file shape

```rust
let content = read_dwarven(&repo, "issues/0001/issue.md");
assert!(content.starts_with("---\n"));
assert!(content.contains("type: feature"));
assert!(content.contains("priority: p1"));
```

Use string `contains` rather than re-parsing the YAML. The point of
the test is "the file looks like this on disk"; re-parsing would
launder the format and mask serialization bugs.

### Asserting on JSON via `--json`

```rust
let out = dwarven(&repo).args(["--json", "issue", "view", "1"])
    .output().unwrap().stdout;
let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
assert_eq!(v["state"], "spec");
```

Every CLI subcommand supports `--json`. Use it when the test cares
about a specific field; use the human format when the test cares about
the rendered text.

### Daemon-with-port pattern

```rust
let repo = fresh_repo();
let port = allocate_port();
set_port(repo.path(), port);
let mut child = spawn_serve(repo.path());
wait_until_ready(port);
// ... assertions ...
stop_daemon(repo.path(), &mut child);
```

The five lines are mechanical. If you find yourself writing them in a
new test file, lift them into `common` first.
