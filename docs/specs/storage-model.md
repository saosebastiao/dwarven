---
spec_version: 2.0.0-draft
last_updated: 2026-05-09
parent_spec: dwarven.md
related_architecture: (none yet)
---

# Storage Model

This document specifies how Dwarven stores hub-tracked artifacts on disk. Per `dwarven.md#R2.8`, the file system is the source of truth; the hub's SQLite index is derived and must be reproducible from files at any time.

This spec covers issues, comments (including state changes), and dependency edges. Specs (`docs/specs/`), architecture (`docs/architecture/`), plans (`docs/plans/`), and the changelog are out of scope for this document — they are referenced by hub artifacts but stored under their own conventions defined in `dwarven.md`.

---

## R1 — Scope

R1.1 — This spec defines the on-disk format for hub-tracked artifacts only: issues, comments, state-change records, and dependency edges.

R1.2 — Out of scope: spec/architecture/plan documents (governed by `dwarven.md`); source code (project-specific); attachments and binary blobs (deferred to a later spec).

---

## R2 — Directory layout

R2.1 — All hub-tracked artifacts live under `.dwarven/` at the repository root.

R2.2 — The required directory layout is:

```
.dwarven/
├── config.toml                  # repo-level hub configuration
└── issues/
    └── <id>/
        ├── issue.md             # issue body + frontmatter
        └── comments/
            ├── 001-<iso>-<author>.md
            ├── 002-<iso>-<author>.md
            └── ...
```

R2.3 — `<id>` is the issue's integer ID, zero-padded to four digits (e.g., `0042`). The four-digit minimum exists for natural sort; IDs may exceed four digits without padding adjustment.

R2.4 — Each issue is a directory, not a single file. The directory contains exactly one `issue.md` and a `comments/` subdirectory.

R2.5 — `.dwarven/config.toml` holds repo-level configuration: the next-issue-ID counter, repository identity, default labels, and per-host adapter settings.

R2.6 — The hub creates `.dwarven/.gitignore` containing patterns for any derived artifacts that should not be committed. The SQLite index file is one such artifact (see R8.4).

---

## R3 — Artifact types

R3.1 — **Issue.** A unit of trackable work. Stored at `.dwarven/issues/<id>/issue.md`. Has exactly one current state, exactly one current type, exactly one current assigned agent role.

R3.2 — **Comment.** A free-form message attached to an issue. Stored at `.dwarven/issues/<id>/comments/<seq>-<iso>-<author>.md`. Comments are append-only in normal operation; edits are permitted but discouraged (history is in git).

R3.3 — **State-change record.** A specialized comment that records a work-state transition. Distinguished by `kind: state-change` in its frontmatter (R4.4). State-change records are produced by agents and the hub on every transition; they constitute the audit trail.

R3.4 — **Dependency edge.** An "A blocks B" or "A blocked-by B" relationship. Stored *in the issue frontmatter of both endpoints* (R4.3.5–R4.3.6). There is no separate edge file.

R3.5 — There are no other hub-tracked artifact types in v1. Adding new types is a spec change.

---

## R4 — File format

R4.1 — Every hub-tracked file is UTF-8 encoded with LF line endings. The hub normalizes line endings on write.

R4.2 — Every file consists of a YAML frontmatter block delimited by `---` lines, followed by a CommonMark Markdown body. The frontmatter is required; the body may be empty.

R4.3 — **Issue frontmatter** (required keys unless marked optional):

```yaml
---
id: 42                              # integer, repo-unique
title: "Short imperative summary"   # string, ≤120 chars
type: feature                       # one of work-states.md type set
state: agent:plan                   # one of work-states.md state set
priority: p1                        # optional; one of {p0, p1, p2}
blocked_by: [12, 17]                # optional; integer issue IDs
blocks: [50]                        # optional; integer issue IDs
epic: cli-foundation                # optional; slug
created: 2026-05-09T10:30:00Z       # ISO 8601, UTC
created_by: maintainer              # "maintainer" or an agent name
updated: 2026-05-09T14:22:00Z       # ISO 8601, UTC; refreshed on any edit
---
```

R4.3.1 — `id` must match the integer in the directory name (`<id>` in R2.3).

R4.3.2 — `title`, `type`, `state`, `created`, `created_by`, and `updated` are required. All others are optional.

R4.3.3 — `type` and `state` values are constrained by `work-states.md`. The hub rejects writes with values outside those sets.

R4.3.4 — `priority` is the *maintainer-asserted* product priority. The dep-graph scheduler (`dep-graph.md`) computes an *effective* priority that combines this with downstream-unblocking value; the effective priority is not stored in the file.

R4.3.5 — `blocked_by: [N, M]` declares that this issue cannot proceed until issues N and M are resolved.

R4.3.6 — `blocks: [N]` declares that issue N cannot proceed until this issue is resolved. Conventional discipline is to keep both endpoints consistent; the hub flags asymmetric edges as `type:chore` issues for triage.

R4.4 — **Comment frontmatter** (required keys unless marked optional):

```yaml
---
seq: 1                              # per-issue monotonic integer
issue: 42                           # parent issue ID
author: spec                        # agent name or "maintainer"
kind: comment                       # one of {comment, state-change, blocker-set, blocker-cleared}
created: 2026-05-09T10:30:00Z       # ISO 8601, UTC
# state-change-only fields:
from: agent:review                  # required iff kind == state-change
to: agent:doc                       # required iff kind == state-change
# blocker-only fields:
blocker: maintainer-input           # required iff kind ∈ {blocker-set, blocker-cleared}
---
```

R4.4.1 — `seq` must match the leading integer in the comment's filename (R2.2).

R4.4.2 — Comment filenames are `<seq>-<iso>-<author>.md` where `<seq>` is the three-digit zero-padded sequence, `<iso>` is `YYYY-MM-DDTHHMMZ` (no colons, for filename safety), and `<author>` is the same value as the frontmatter `author` field.

R4.4.3 — `kind` defaults to `comment`. State changes and blocker events use the dedicated kinds so the hub can index them efficiently.

---

## R5 — ID assignment

R5.1 — Issue IDs are monotonically increasing repo-wide integers starting at 1.

R5.2 — Issue IDs are assigned by the hub at issue-creation time. The next-ID counter is stored in `.dwarven/config.toml` and incremented atomically.

R5.3 — Comment sequence numbers are monotonically increasing per-issue integers starting at 1. Sequence assignment is performed by the hub at comment-creation time, scoped to the parent issue.

R5.4 — IDs and sequences are never reused, even after deletion. Deletion is recorded in git history; the file is removed but the next-issue-ID counter is not decremented.

---

## R6 — Write protocol

R6.1 — Per `dwarven.md#R2.10`, agents must mutate hub-tracked artifacts exclusively through the `dwarven` CLI (`dwarven-cli.md`). Agents must not have `.dwarven/` in their `Write` or `Edit` allowlists. The hub validates inputs and rejects illegal transitions.

R6.2 — The maintainer may edit `.dwarven/` files directly in their editor; the hub picks up changes via file-watching (R9). This is supported by design — issue files are human-readable Markdown and direct editing is the escape hatch when the CLI is insufficient.

R6.3 — The web UI mutates files via the hub's HTTP API (`web-api.md`); the hub writes files first, then updates the index.

R6.4 — Every file write is atomic at the file level: the hub writes to a temporary file in the same directory and renames over the target. Partial writes must never be observable.

R6.5 — Multi-file updates (e.g., creating an issue with an initial comment, or recording a transition that touches both endpoints of a dependency edge) are not transactional across files. If interrupted, the on-disk state may be partial. The hub heals on the next reindex (R8.3).

R6.6 — `updated` in issue frontmatter is refreshed by the hub on any write to that issue's `issue.md` or `comments/`. Manual edits by the maintainer should also bump `updated`; the hub corrects stale `updated` values during reindex.

---

## R7 — Read protocol

R7.1 — Agents read hub-tracked artifacts either directly from the file system (with `Read` permitted on `.dwarven/`) or via `dwarven` CLI read subcommands. Both paths are equally supported; neither has primacy.

R7.2 — The web UI reads exclusively via the hub's HTTP API.

R7.3 — Direct reads from the file system see the on-disk state, which may briefly be ahead of or behind the SQLite index during sync (R9). For workflow correctness, direct file reads are sufficient — the hub's index is an optimization for query performance, not for consistency.

---

## R8 — Index reproducibility

R8.1 — The hub's SQLite index must be fully reproducible from `.dwarven/` files. Deleting the index file and running `dwarven reindex` (specified in `dwarven-cli.md`) must produce a byte-identical index, modulo any non-deterministic columns explicitly marked as such.

R8.2 — The hub treats the file system as authoritative on conflict. If the index disagrees with the files (because of manual edits, partial writes, or external git operations), the index is rebuilt from the files. The reverse — using the index to "correct" files — is forbidden.

R8.3 — `dwarven reindex` is idempotent. Running it on a healthy hub must produce no observable changes.

R8.4 — The SQLite index file lives at `.dwarven/.index.sqlite` (or similar; exact path defined in `coordination-hub.md`) and is gitignored. It is per-checkout, not per-repo.

---

## R9 — Sync model

R9.1 — The hub watches `.dwarven/` recursively using a filesystem-event API (e.g., FSEvents on macOS, inotify on Linux). On any create, modify, or delete event affecting a tracked path, the hub re-reads the affected file(s) and updates the index.

R9.2 — Sync is best-effort and eventually consistent. The hub does not guarantee that an index update follows a file write within any specific deadline; consumers requiring strong consistency must read from files directly (R7.3).

R9.3 — On hub startup, the hub performs a full reindex (R8.1) before serving requests. Subsequent live updates use the file-watcher path.

R9.4 — If the file watcher fails or drops events (which can happen under heavy load on some platforms), the hub must detect the inconsistency on the next CLI or HTTP request that triggers a read of the affected artifact and trigger a reindex of just that artifact's directory.

---

## R10 — Out of scope for v1

R10.1 — **Attachments.** Binary blobs (images, PDFs, logs) are not specified in v1. Maintainers may reference external URLs in comment bodies. A future `attachments-spec.md` will define the on-disk format.

R10.2 — **Cross-repo references.** All issue/comment IDs are repo-local. Cross-repo dependency or reference is not supported in v1.

R10.3 — **Schema migration.** The frontmatter schema in R4 is v2.0.0. Schema changes between minor versions are additive (new optional fields). Schema changes between major versions follow the `dwarven.md#R5.3` migration rule.

R10.4 — **Encryption at rest.** All `.dwarven/` files are plaintext. Sensitive content (credentials, tokens) must not be stored in hub-tracked artifacts.
