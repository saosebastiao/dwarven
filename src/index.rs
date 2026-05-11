//! SQLite-backed derived index of `.dwarven/issues/`.
//!
//! The index is derived from files: deleting `.index.sqlite` and
//! running [`rebuild`] reconstructs it. The daemon does this
//! unconditionally at startup (`coordination-hub.md#R3.3`) and the
//! `dwarven reindex` CLI does it on demand. The CLI never writes the
//! index outside of `reindex`.
//!
//! ## Byte-reproducibility
//!
//! The index uses `journal_mode = DELETE` and inserts in id-ascending
//! order so two consecutive rebuilds on the same canonical files
//! produce byte-identical files (modulo SQLite-internal allocation).
//! A `VACUUM` at the end of [`rebuild`] further normalizes the b-tree
//! layout. The test `tests/reindex.rs::reindex_is_idempotent` asserts
//! byte-identity.
//!
//! ## Health probe
//!
//! [`probe_health`] inspects the existing index file at daemon startup
//! to report what state it was in (Missing / Corrupt / VersionMismatch
//! / Ok). The result is logged but does not gate the rebuild: the
//! daemon always rebuilds at startup, per spec.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags, params};

use crate::storage::comment_file::list_comments;
use crate::storage::config::{RepoPaths, require_initialized};
use crate::storage::issue_file::{enumerate_issue_ids, read_issue};

/// Schema version. Bump on any breaking change to the table layout.
pub const SCHEMA_VERSION: i64 = 1;

/// Result of inspecting an existing `.index.sqlite` file before the daemon
/// performs its startup reindex. `coordination-hub.md#R7.3` (PRAGMA
/// integrity_check) and `#R7.4` (schema-version mismatch).
#[derive(Debug)]
pub enum IndexHealth {
    /// No file at the index path.
    Missing,
    /// File exists but failed to open or `PRAGMA integrity_check` did not
    /// return `ok`. The string carries the underlying error / result.
    Corrupt(String),
    /// File opens cleanly, integrity check passes, but `meta.schema_version`
    /// does not match `SCHEMA_VERSION`.
    VersionMismatch { found: i64, expected: i64 },
    /// File opens, passes integrity, schema version matches.
    Ok,
}

/// Inspect the index at `path` and report its health. Errors here are
/// treated as `Corrupt` with the error string — i.e., any unhandled failure
/// to open or query is grounds for rebuilding.
pub fn probe_health(path: &Path) -> Result<IndexHealth> {
    if !path.exists() {
        return Ok(IndexHealth::Missing);
    }
    let conn = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => c,
        Err(e) => return Ok(IndexHealth::Corrupt(format!("open: {e}"))),
    };
    match check_integrity(&conn) {
        Ok(true) => {}
        Ok(false) => return Ok(IndexHealth::Corrupt("integrity_check: not ok".into())),
        Err(e) => return Ok(IndexHealth::Corrupt(format!("integrity_check: {e}"))),
    };
    match read_schema_version(&conn) {
        Ok(Some(v)) if v == SCHEMA_VERSION => Ok(IndexHealth::Ok),
        Ok(Some(v)) => Ok(IndexHealth::VersionMismatch {
            found: v,
            expected: SCHEMA_VERSION,
        }),
        Ok(None) => Ok(IndexHealth::Corrupt("missing meta.schema_version row".into())),
        Err(e) => Ok(IndexHealth::Corrupt(format!("read schema_version: {e}"))),
    }
}

/// Run `PRAGMA integrity_check` and return `Ok(true)` only when SQLite
/// reports the literal string `ok`.
pub fn check_integrity(conn: &Connection) -> rusqlite::Result<bool> {
    let result: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    Ok(result == "ok")
}

/// Read the `value` column of the `meta` row with `key = 'schema_version'`.
/// Returns `Ok(None)` if the row is absent (treated as corruption by
/// [`probe_health`]).
pub fn read_schema_version(conn: &Connection) -> rusqlite::Result<Option<i64>> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .map(Some)
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    Ok(raw.and_then(|s| s.parse::<i64>().ok()))
}

const SCHEMA_DDL: &[&str] = &[
    "CREATE TABLE meta (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL
     )",
    "CREATE TABLE issue (
        id INTEGER PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        type TEXT NOT NULL,
        state TEXT NOT NULL,
        priority TEXT,
        blocker TEXT,
        epic TEXT,
        created TEXT NOT NULL,
        created_by TEXT NOT NULL,
        updated TEXT NOT NULL,
        body TEXT NOT NULL
     )",
    "CREATE INDEX idx_issue_state ON issue(state)",
    "CREATE INDEX idx_issue_type ON issue(type)",
    "CREATE INDEX idx_issue_priority ON issue(priority)",
    "CREATE INDEX idx_issue_blocker ON issue(blocker)",
    "CREATE INDEX idx_issue_epic ON issue(epic)",
    "CREATE INDEX idx_issue_updated ON issue(updated)",
    "CREATE TABLE issue_blocks (
        blocker_id INTEGER NOT NULL,
        blocked_id INTEGER NOT NULL,
        PRIMARY KEY (blocker_id, blocked_id),
        FOREIGN KEY (blocker_id) REFERENCES issue(id),
        FOREIGN KEY (blocked_id) REFERENCES issue(id)
     )",
    "CREATE INDEX idx_blocks_blocked ON issue_blocks(blocked_id)",
    "CREATE TABLE comment (
        issue_id INTEGER NOT NULL,
        seq INTEGER NOT NULL,
        author TEXT NOT NULL,
        kind TEXT NOT NULL,
        created TEXT NOT NULL,
        from_state TEXT,
        to_state TEXT,
        blocker_value TEXT,
        body TEXT NOT NULL,
        PRIMARY KEY (issue_id, seq),
        FOREIGN KEY (issue_id) REFERENCES issue(id)
     )",
    "CREATE INDEX idx_comment_kind ON comment(kind)",
];

/// Args for the `dwarven reindex` CLI subcommand.
pub struct ReindexArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
}

/// Per-table totals from the most recent rebuild.
pub struct ReindexStats {
    pub issues: usize,
    pub comments: usize,
    pub edges: usize,
}

/// `dwarven reindex` entry point. Calls [`rebuild`] and prints stats.
pub fn run(args: ReindexArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    require_initialized(&paths)?;

    let stats = rebuild(&paths)?;

    if args.json {
        println!(
            "{{\"issues\":{},\"comments\":{},\"edges\":{}}}",
            stats.issues, stats.comments, stats.edges
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!(
            "Reindexed: {} issues, {} comments, {} edges",
            stats.issues, stats.comments, stats.edges
        );
        println!("  index: {}", paths.index_path().display());
    }
    Ok(())
}

/// Drop the existing `.index.sqlite` (if any) and rebuild it from scratch
/// by walking `.dwarven/issues/`. Inserts in id-ascending order so a healthy
/// repo's index is reproducible byte-for-byte modulo SQLite internals.
pub fn rebuild(paths: &RepoPaths) -> Result<ReindexStats> {
    let index_path = paths.index_path();
    if index_path.exists() {
        fs::remove_file(&index_path)
            .with_context(|| format!("removing {}", index_path.display()))?;
    }
    // Also clear -wal / -shm sidecars from prior runs if present.
    for suffix in &["-wal", "-shm", "-journal"] {
        let mut p = index_path.clone();
        let mut name = p.file_name().unwrap().to_owned();
        name.push(suffix);
        p.set_file_name(name);
        if p.exists() {
            let _ = fs::remove_file(&p);
        }
    }

    let mut conn = Connection::open(&index_path)
        .with_context(|| format!("opening {}", index_path.display()))?;
    conn.pragma_update(None, "page_size", 4096)?;
    conn.pragma_update(None, "journal_mode", "DELETE")?;

    let tx = conn.transaction()?;
    for ddl in SCHEMA_DDL {
        tx.execute(ddl, [])
            .with_context(|| format!("executing DDL: {ddl}"))?;
    }
    tx.execute(
        "INSERT INTO meta (key, value) VALUES (?, ?)",
        params!["schema_version", SCHEMA_VERSION.to_string()],
    )?;

    let mut issues = 0_usize;
    let mut comments = 0_usize;
    let mut edges = 0_usize;

    let ids = enumerate_issue_ids(&paths.issues_dir())?;
    for id in ids {
        let issue = read_issue(&paths.issue_md(id))?;
        let fm = &issue.frontmatter;

        tx.execute(
            "INSERT INTO issue (id, title, type, state, priority, blocker, epic, created, created_by, updated, body)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)",
            params![
                fm.id as i64,
                fm.title,
                fm.issue_type,
                fm.state,
                fm.priority,
                fm.blocker,
                fm.epic,
                fm.created,
                fm.created_by,
                fm.updated,
                issue.body,
            ],
        )?;
        issues += 1;

        let mut blocks_sorted = fm.blocks.clone();
        blocks_sorted.sort_unstable();
        for blocked in &blocks_sorted {
            tx.execute(
                "INSERT OR IGNORE INTO issue_blocks (blocker_id, blocked_id) VALUES (?, ?)",
                params![fm.id as i64, *blocked as i64],
            )?;
            edges += 1;
        }

        let comment_files = list_comments(&paths.comments_dir(id))?;
        for c in &comment_files {
            tx.execute(
                "INSERT INTO comment (issue_id, seq, author, kind, created, from_state, to_state, blocker_value, body)
                 VALUES (?,?,?,?,?,?,?,?,?)",
                params![
                    c.frontmatter.issue as i64,
                    c.frontmatter.seq as i64,
                    c.frontmatter.author,
                    c.frontmatter.kind,
                    c.frontmatter.created,
                    c.frontmatter.from,
                    c.frontmatter.to,
                    c.frontmatter.blocker,
                    c.body,
                ],
            )?;
            comments += 1;
        }
    }

    tx.commit()?;

    // VACUUM compacts the file. Must run outside a transaction.
    conn.execute("VACUUM", [])?;

    Ok(ReindexStats {
        issues,
        comments,
        edges,
    })
}
