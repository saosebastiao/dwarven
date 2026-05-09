use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo};

#[test]
fn view_prints_header_body_and_creation_comment() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args([
            "issue",
            "create",
            "--type",
            "feature",
            "--title",
            "Walking skeleton",
            "--body",
            "First body line.",
        ])
        .assert()
        .success();

    let pred = predicate::str::is_match(r"type:\s+feature").unwrap()
        .and(predicate::str::is_match(r"state:\s+pm").unwrap());
    dwarven(tmp.path())
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Issue #1: Walking skeleton"))
        .stdout(pred)
        .stdout(predicate::str::contains("created_by:").not()) // header uses "created:"
        .stdout(predicate::str::contains("by maintainer"))
        .stdout(predicate::str::contains("First body line."))
        .stdout(predicate::str::contains("--- comments (1) ---"))
        .stdout(predicate::str::contains("state-change: created → pm"))
        .stdout(predicate::str::contains("Issue created."));
}

#[test]
fn view_no_comments_suppresses_comment_section() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "X"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1", "--no-comments"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--- comments").not())
        .stdout(predicate::str::contains("Issue created.").not());
}

#[test]
fn view_state_history_keeps_state_change_kind_only() {
    // The creation comment is kind=state-change, so it survives the filter.
    // Once `dwarven issue comment` lands, plain comments will be filtered out.
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "X"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1", "--state-history"])
        .assert()
        .success()
        .stdout(predicate::str::contains("state-change: created → plan"));
}

#[test]
fn view_last_n_keeps_most_recent_comments() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "X"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1", "--last", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--- comments (1) ---"));
}

#[test]
fn view_missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "view", "999"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn view_renders_dependency_edges() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "Target"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "Blocker", "--blocks", "1",
        ])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocked_by:").and(predicate::str::contains("#2")));

    dwarven(tmp.path())
        .args(["issue", "view", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("blocks:").and(predicate::str::contains("#1")));
}
