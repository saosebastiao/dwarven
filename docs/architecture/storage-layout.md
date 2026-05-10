---
spec_ref: storage-model.md
date: 2026-05-10
issue: 8
---

# Storage layout

Where the hub's persistent state lives and why each artifact is shaped the way it is. This is the implementation companion to `docs/specs/storage-model.md`.

## The `.dwarven/` tree

```
.dwarven/
├── config.toml          # per-repo configuration (R10.2 schema)
├── .gitignore           # excludes derived state from git
├── .index.sqlite        # derived; daemon-owned (R8.4)
├── .daemon.pid          # daemon lifecycle marker (R3.5)
├── .config.lock         # advisory flock target for atomic counters
└── issues/
    └── <padded-id>/
        ├── issue.md             # frontmatter + body
        └── comments/
            └── <seq>-<iso>-<author>.md
```

The directory is committed to git as the canonical state. The daemon's index and lock files are gitignored — they are per-checkout, machine-local, and reproducible from the canonical files.

## Atomic writes via temp + rename

Every write to a hub-tracked file (`issue.md`, comment files, `config.toml`) goes through `crate::storage::atomic::write_atomic`. The helper writes to a sibling temporary file (`.<name>.tmp.<pid>`) in the same directory, fsyncs, then `rename(2)`s over the target. On POSIX filesystems `rename` within a directory is atomic — readers see either the old file or the complete new file, never a partial write.

Why same directory: `rename(2)` is only atomic on a single filesystem. Same-directory temp paths guarantee this regardless of how `.dwarven/` is mounted.

Why a per-pid temp suffix: two concurrent writes from the same process would otherwise collide. The process id disambiguates; within a single process the file lock (below) serializes the overall operation.

## The advisory file lock (`.config.lock`)

Multiple `dwarven` CLI invocations can race on:

- the `next_issue_id` counter in `config.toml`
- the per-issue `seq` counter scanned from `comments/`
- writes to an issue's frontmatter from concurrent mutations

`crate::storage::config::with_repo_lock` wraps an exclusive `flock(2)` on `.dwarven/.config.lock`. Every mutating CLI path takes this lock for the duration of its critical section. The lock target is a dedicated file (not `config.toml` itself) because `rename`-over-target replaces the inode, severing any lock held on the previous one.

Lock scope is coarse — one lock for the whole repo's mutations — which trades concurrency for simplicity. At the spec's "hundreds of issues" scale (per `coordination-hub.md`), contention is not the bottleneck. The same lock is reused for issue creation, comment append, state transitions, blocker / priority mutations, and dependency edge writes.

## Frontmatter shape and round-tripping

`IssueFrontmatter` (in `crate::storage::issue_file`) derives `Serialize` + `Deserialize` from serde, with two key conventions:

- `#[serde(skip_serializing_if = "Option::is_none")]` on every optional field. Missing keys on disk stay missing; setting and then clearing a field leaves no trace of the intermediate state. This keeps git diffs small and the on-disk schema "what's set" rather than "what's possible."
- `#[serde(rename = "type")]` on `issue_type` because `type` is a Rust keyword.

YAML library: `serde_yaml`. It is officially unmaintained as of 2024 but stable, and the surface we use (small fixed schemas, no exotic YAML features) is well-trodden. The cost of switching is low; we'd revisit only if a real bug surfaced.

## TOML and `toml_edit`

`config.toml` is edited in place by `dwarven config set` and by the scheduler-override / counter paths. Naïve TOML round-tripping (deserialize → mutate struct → serialize) loses comments and reorders keys. `toml_edit` preserves them. Every config write goes through it.

The startup-time validation (`crate::daemon::config::read_full_config`) does its own parse via `toml::Value` because it only needs to read, not preserve, the file. The trade-off: two parses of `config.toml` per daemon start. At ~30 lines that cost is irrelevant.

## SQLite index byte-reproducibility

`crate::index::rebuild` produces a `.index.sqlite` that is byte-identical across two consecutive rebuilds on the same canonical files, modulo SQLite's internal page state. Two engineering choices enable this:

1. **`journal_mode = DELETE`** on the indexer's connection. WAL mode produces sidecar `-wal` / `-shm` files whose contents depend on commit timing; DELETE journal mode produces no sidecar at quiescence.
2. **Deterministic insertion order**. `rebuild` walks issue IDs in ascending order, inserts rows in that order, then `VACUUM`s the database at the end. SQLite's b-tree layout is then determined by insert order alone.

The integration test `reindex_is_idempotent` asserts byte-identity by reading two consecutive rebuilds and comparing the raw bytes.

The daemon's runtime journal mode is a separate choice; see `cli-vs-daemon.md` for the WAL-vs-DELETE story there.
