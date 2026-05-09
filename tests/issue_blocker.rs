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
fn set_records_blocker_on_frontmatter_and_in_comment() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");

    dwarven(tmp.path())
        .args([
            "issue",
            "blocker",
            "set",
            "1",
            "maintainer-input",
            "--comment",
            "Need product call on edge case.",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "blocker set to maintainer-input",
        ));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("blocker: maintainer-input"));

    let comment_path = std::fs::read_dir(tmp.path().join(".dwarven/issues/0001/comments"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("002-"))
        .unwrap();
    let body = std::fs::read_to_string(&comment_path).unwrap();
    assert!(body.contains("kind: blocker-set"));
    assert!(body.contains("blocker: maintainer-input"));
    assert!(body.contains("Need product call on edge case."));
}

#[test]
fn clear_removes_blocker_and_records_prior_value() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "1", "external", "--quiet"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args([
            "issue", "blocker", "clear", "1", "--comment", "Vendor responded.",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocker cleared (was external)"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(!issue.contains("blocker:"));

    // Verify the most-recent comment is kind: blocker-cleared with the prior value.
    let mut entries: Vec<_> = std::fs::read_dir(tmp.path().join(".dwarven/issues/0001/comments"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    let last = entries.last().unwrap();
    let body = std::fs::read_to_string(last).unwrap();
    assert!(body.contains("kind: blocker-cleared"));
    assert!(body.contains("blocker: external"));
}

#[test]
fn invalid_blocker_value_rejected() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "1", "wibble"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid blocker"));
}

#[test]
fn clear_with_no_blocker_set_rejected() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "blocker", "clear", "1"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no blocker set"));
}

#[test]
fn set_on_terminal_issue_rejected() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "done"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "1", "external"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("terminal state"));
}

#[test]
fn list_blocker_filter_matches_set_value_and_unset_sentinel() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // 1: no blocker
    create(tmp.path(), "feature"); // 2: will get external

    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "2", "external", "--quiet"])
        .assert()
        .success();

    let out = dwarven(tmp.path())
        .args(["issue", "list", "--blocker", "external"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.lines().any(|l| l.starts_with("   2 ")));
    assert!(!stdout.lines().any(|l| l.starts_with("   1 ")));

    let out = dwarven(tmp.path())
        .args(["issue", "list", "--blocker", "unset"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.lines().any(|l| l.starts_with("   1 ")));
    assert!(!stdout.lines().any(|l| l.starts_with("   2 ")));
}

#[test]
fn view_renders_blocker_line_when_set() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "1", "upstream", "--quiet"])
        .assert()
        .success();

    let pred = predicate::str::is_match(r"blocker:\s+upstream").unwrap();
    dwarven(tmp.path())
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(pred);
}

#[test]
fn missing_id_exits_not_found_for_both_set_and_clear() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "blocker", "set", "999", "external"])
        .assert()
        .code(3);
    dwarven(tmp.path())
        .args(["issue", "blocker", "clear", "999"])
        .assert()
        .code(3);
}
