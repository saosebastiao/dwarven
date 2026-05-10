use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo, read_dwarven};

fn create(repo: &std::path::Path) {
    dwarven(repo)
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
}

#[test]
fn set_writes_field_to_frontmatter() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "42.5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("priority-override set to 42.5"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("effective_priority_override: 42.5"));
}

#[test]
fn set_then_clear_round_trip() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "10"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "--clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("priority-override cleared"));
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(!issue.contains("effective_priority_override"));
}

#[test]
fn clear_with_no_override_set_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "--clear"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no priority-override set"));
}

#[test]
fn must_pass_value_or_clear() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1"])
        .assert()
        .failure();
}

#[test]
fn set_on_terminal_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "done"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "10"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("terminal state"));
}

#[test]
fn missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "priority-override", "999", "5"])
        .assert()
        .code(3);
}

#[test]
fn override_survives_round_trip_through_view_api() {
    // The override should be preserved when read_issue → write_issue is
    // exercised via a transition or other mutation.
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "7.25"])
        .assert()
        .success();
    // Trigger a separate write that round-trips the frontmatter.
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("effective_priority_override: 7.25"));
}

#[test]
fn negative_and_zero_override_values_accepted() {
    // Spec says "absolute number"; we treat it as f64 and don't constrain
    // sign. Higher = ranked higher.
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "-99"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("effective_priority_override: -99"));

    dwarven(tmp.path())
        .args(["issue", "priority-override", "1", "0"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("effective_priority_override: 0"));
}
