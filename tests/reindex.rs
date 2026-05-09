use std::path::Path;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use rusqlite::Connection;

mod common;
use common::{dwarven, fresh_repo};

fn run_reindex(repo: &Path) {
    dwarven(repo)
        .args(["reindex"])
        .assert()
        .success();
}

fn open_index(repo: &Path) -> Connection {
    Connection::open(repo.join(".dwarven/.index.sqlite")).unwrap()
}

#[test]
fn reindex_on_empty_repo_creates_schema() {
    let tmp = fresh_repo();
    run_reindex(tmp.path());

    let conn = open_index(tmp.path());
    let issue_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue", [], |r| r.get(0))
        .unwrap();
    assert_eq!(issue_count, 0);

    let schema_version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(schema_version, "1");
}

#[test]
fn reindex_populates_issues_comments_and_edges() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "Foo", "--priority", "p0",
        ])
        .assert()
        .success();
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "bug", "--title", "Bar", "--blocks", "1",
        ])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "comment", "1", "--body", "user comment"])
        .assert()
        .success();

    run_reindex(tmp.path());

    let conn = open_index(tmp.path());

    let (id, title, ty, state, priority): (i64, String, String, String, Option<String>) = conn
        .query_row(
            "SELECT id, title, type, state, priority FROM issue WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(id, 1);
    assert_eq!(title, "Foo");
    assert_eq!(ty, "feature");
    assert_eq!(state, "pm");
    assert_eq!(priority.as_deref(), Some("p0"));

    let comment_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM comment WHERE issue_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    // creation comment + user comment = 2.
    assert_eq!(comment_count, 2);

    // 2 blocks 1.
    let edge_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM issue_blocks WHERE blocker_id = 2 AND blocked_id = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(edge_count, 1);
}

#[test]
fn reindex_is_idempotent() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();

    run_reindex(tmp.path());
    let first = std::fs::read(tmp.path().join(".dwarven/.index.sqlite")).unwrap();

    run_reindex(tmp.path());
    let second = std::fs::read(tmp.path().join(".dwarven/.index.sqlite")).unwrap();

    assert_eq!(
        first, second,
        "two reindex runs over identical files should produce byte-identical index files"
    );
}

#[test]
fn reindex_overwrites_stale_index() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "first"])
        .assert()
        .success();
    run_reindex(tmp.path());

    // Now add another issue without reindexing.
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "second"])
        .assert()
        .success();

    // Stale index sees only the first issue.
    let conn = open_index(tmp.path());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    drop(conn);

    // Reindex picks up both.
    run_reindex(tmp.path());
    let conn = open_index(tmp.path());
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn reindex_state_change_comments_have_from_to() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .success();

    run_reindex(tmp.path());
    let conn = open_index(tmp.path());

    let mut stmt = conn
        .prepare(
            "SELECT seq, kind, from_state, to_state FROM comment WHERE issue_id = 1 ORDER BY seq",
        )
        .unwrap();
    let rows: Vec<(i64, String, Option<String>, Option<String>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, "state-change");
    assert_eq!(rows[0].2.as_deref(), Some("created"));
    assert_eq!(rows[0].3.as_deref(), Some("pm"));
    assert_eq!(rows[1].1, "state-change");
    assert_eq!(rows[1].2.as_deref(), Some("pm"));
    assert_eq!(rows[1].3.as_deref(), Some("plan"));
}

#[test]
fn reindex_blocker_comments_have_blocker_value() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "1", "external"])
        .assert()
        .success();

    run_reindex(tmp.path());
    let conn = open_index(tmp.path());

    let blocker: Option<String> = conn
        .query_row(
            "SELECT blocker_value FROM comment WHERE issue_id = 1 AND kind = 'blocker-set'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(blocker.as_deref(), Some("external"));
}

#[test]
fn reindex_requires_initialized_repo() {
    let tmp = tempfile::tempdir().unwrap();
    dwarven(tmp.path())
        .args(["reindex"])
        .assert()
        .failure();
}

#[test]
fn reindex_clears_index_on_empty_repo_after_close() {
    // Start with content, close everything, reindex, and verify the row
    // is still there with state=done (closed issues remain indexed).
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "shipped"])
        .assert()
        .success();

    run_reindex(tmp.path());
    let conn = open_index(tmp.path());
    let state: String = conn
        .query_row("SELECT state FROM issue WHERE id = 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "done");
}

#[test]
fn reindex_output_reports_counts() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "y", "--blocks", "1",
        ])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["reindex"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2 issues"))
        .stdout(predicate::str::contains("1 edges"));
}
