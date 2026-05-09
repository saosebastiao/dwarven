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
fn legal_transition_pm_to_plan_on_feature() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // state=pm

    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pm → plan"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: plan"));
}

#[test]
fn transition_records_state_change_comment() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args([
            "issue",
            "transition",
            "1",
            "plan",
            "--comment",
            "Decomposed into single concrete plan.",
        ])
        .assert()
        .success();

    let dir = tmp.path().join(".dwarven/issues/0001/comments");
    let comment_path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("002-"))
        .unwrap();
    let body = std::fs::read_to_string(&comment_path).unwrap();
    assert!(body.contains("kind: state-change"));
    assert!(body.contains("from: pm"));
    assert!(body.contains("to: plan"));
    assert!(body.contains("Decomposed into single concrete plan."));
}

#[test]
fn illegal_transition_rejected_with_allowed_list() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // pm
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "implement"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("illegal transition 'pm' → 'implement'"))
        .stderr(predicate::str::contains("permitted:"));
}

#[test]
fn dropped_reachable_from_any_active_state() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // pm

    dwarven(tmp.path())
        .args(["issue", "transition", "1", "dropped"])
        .assert()
        .success();

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: dropped"));
}

#[test]
fn transition_out_of_terminal_rejected() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "dropped"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("terminal"));
}

#[test]
fn no_op_transition_rejected() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // pm
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "pm"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already in state 'pm'"));
}

#[test]
fn override_requires_maintainer_actor() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // pm

    dwarven(tmp.path())
        .args([
            "--actor", "spec", "issue", "transition", "1", "test", "--override",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--override is restricted"));
}

#[test]
fn override_cannot_target_terminal() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args([
            "issue",
            "transition",
            "1",
            "done",
            "--override",
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--override cannot transition into terminal"));
}

#[test]
fn override_allows_jump_between_active_states_for_maintainer() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature"); // pm
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "implement", "--override"])
        .assert()
        .success();

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: implement"));
}

#[test]
fn maintainer_state_can_route_to_any_active() {
    let tmp = fresh_repo();
    // Create an issue and use --override to move it to 'maintainer'.
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "maintainer", "--override"])
        .assert()
        .success();

    // From maintainer: any active state should be reachable without override.
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "test"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("state: test"));
}

#[test]
fn missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "transition", "999", "plan"])
        .assert()
        .code(3);
}

#[test]
fn transition_appears_as_state_change_in_view_state_history() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature");
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1", "--state-history"])
        .assert()
        .success()
        .stdout(predicate::str::contains("created → pm"))
        .stdout(predicate::str::contains("pm → plan"));
}
