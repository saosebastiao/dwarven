use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo, read_dwarven};

fn create(repo: &std::path::Path, ty: &str) {
    dwarven(repo)
        .args(["issue", "create", "--type", ty, "--title", "x"])
        .assert()
        .success();
}

#[test]
fn close_default_targets_done_from_pm() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // pm

    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "Resolved upstream."])
        .assert()
        .success()
        .stdout(predicate::str::contains("pm → done"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: done"));
}

#[test]
fn close_done_works_from_implement_state() {
    // Per work-states.md#R6.2.4 close is the universal terminal escape hatch
    // even though the R6.2 graph does not list `implement → done`.
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    for next in ["plan", "test", "implement"] {
        dwarven(tmp.path())
            .args(["issue", "transition", "1", next])
            .assert()
            .success();
    }

    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "Shipped."])
        .assert()
        .success();

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: done"));
}

#[test]
fn close_dropped_targets_dropped() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");

    dwarven(tmp.path())
        .args([
            "issue", "close", "1", "--dropped", "--comment", "Won't be done.",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("pm → dropped"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: dropped"));
}

#[test]
fn close_records_state_change_with_comment_body() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");

    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "Closure rationale here."])
        .assert()
        .success();

    let comment_path = std::fs::read_dir(tmp.path().join(".dwarven/issues/0001/comments"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("002-"))
        .unwrap();
    let body = std::fs::read_to_string(&comment_path).unwrap();
    assert!(body.contains("kind: state-change"));
    assert!(body.contains("from: pm"));
    assert!(body.contains("to: done"));
    assert!(body.contains("Closure rationale here."));
}

#[test]
fn close_requires_comment() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "close", "1"])
        .assert()
        .failure();
}

#[test]
fn close_already_terminal_rejected() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "first close"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args([
            "issue", "close", "1", "--comment", "second close attempt",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("terminal state 'done'"));
}

#[test]
fn close_missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "close", "999", "--comment", "x"])
        .assert()
        .code(3);
}

#[test]
fn closed_issue_hidden_from_default_list() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    create(tmp.path(), "feature");

    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "done"])
        .assert()
        .success();

    let out = dwarven(tmp.path())
        .args(["issue", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    // Issue 1 is closed → hidden from default --open scope.
    assert!(
        !stdout.lines().any(|l| l.starts_with("   1 ")),
        "issue 1 should not appear in default list:\n{stdout}"
    );

    let out = dwarven(tmp.path())
        .args(["issue", "list", "--closed"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.lines().any(|l| l.starts_with("   1 ")),
        "issue 1 should appear in --closed list:\n{stdout}"
    );
}
