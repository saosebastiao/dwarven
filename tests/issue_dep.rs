use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo, read_dwarven};

fn create_n(repo: &std::path::Path, n: usize) {
    for _ in 0..n {
        dwarven(repo)
            .args(["issue", "create", "--type", "feature", "--title", "x"])
            .assert()
            .success();
    }
}

#[test]
fn add_writes_both_endpoints() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);

    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added edge: #1 blocks #2"));

    let one = read_dwarven(tmp.path(), "issues/0001/issue.md");
    let two = read_dwarven(tmp.path(), "issues/0002/issue.md");
    assert!(one.contains("blocks:") && one.contains("- 2"));
    assert!(two.contains("blocked_by:") && two.contains("- 1"));
}

#[test]
fn add_rejects_self_edge() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 1);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "1"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("self-edge"));
}

#[test]
fn add_rejects_direct_cycle() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "2", "blocks", "1"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("would create a cycle"));
}

#[test]
fn add_rejects_transitive_cycle() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 3);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "2", "blocks", "3"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "3", "blocks", "1"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("would create a cycle"));
}

#[test]
fn add_idempotent_duplicate_rejected() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already present"));
}

#[test]
fn add_rationale_lands_as_comment_on_from() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args([
            "issue",
            "dep",
            "add",
            "1",
            "blocks",
            "2",
            "--rationale",
            "Both depend on the same migration.",
        ])
        .assert()
        .success();

    let comments_dir = tmp.path().join(".dwarven/issues/0001/comments");
    let entries: Vec<_> = std::fs::read_dir(&comments_dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let last = entries
        .iter()
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("002-"))
        .unwrap();
    let body = std::fs::read_to_string(last).unwrap();
    assert!(body.contains("kind: comment"));
    assert!(body.contains("Dep added: #1 blocks #2."));
    assert!(body.contains("Both depend on the same migration."));
}

#[test]
fn remove_drops_edge_from_both_endpoints() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "dep", "remove", "1", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed edge"));

    let one = read_dwarven(tmp.path(), "issues/0001/issue.md");
    let two = read_dwarven(tmp.path(), "issues/0002/issue.md");
    assert!(!one.contains("blocks:"));
    assert!(!two.contains("blocked_by:"));
}

#[test]
fn remove_nonexistent_edge_rejected() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args(["issue", "dep", "remove", "1", "2"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no edge"));
}

#[test]
fn missing_id_exits_not_found_for_add_and_remove() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 1);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "999"])
        .assert()
        .code(3);
    dwarven(tmp.path())
        .args(["issue", "dep", "remove", "1", "999"])
        .assert()
        .code(3);
}

#[test]
fn add_requires_blocks_keyword_literally() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "wibble", "2"])
        .assert()
        .failure();
}

#[test]
fn view_renders_edges_after_add() {
    let tmp = fresh_repo();
    create_n(tmp.path(), 2);
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocks:").and(predicate::str::contains("#2")));
    dwarven(tmp.path())
        .args(["issue", "view", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocked_by:").and(predicate::str::contains("#1")));
}
