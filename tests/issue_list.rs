use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo};

fn create(repo: &std::path::Path, ty: &str, title: &str, extras: &[&str]) {
    let mut cmd = dwarven(repo);
    cmd.args(["issue", "create", "--type", ty, "--title", title]);
    cmd.args(extras);
    cmd.assert().success();
}

#[test]
fn list_empty_repo_says_no_issues() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No issues match."));
}

#[test]
fn list_default_shows_active_issues() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature", "Foo", &[]);
    create(tmp.path(), "bug", "Bar", &[]);

    dwarven(tmp.path())
        .args(["issue", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ID"))
        .stdout(predicate::str::contains("STATE"))
        .stdout(predicate::str::contains("Foo"))
        .stdout(predicate::str::contains("Bar"));
}

#[test]
fn list_filter_by_state() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature", "Feature one", &[]); // state=pm
    create(tmp.path(), "bug", "Bug one", &[]); // state=plan

    dwarven(tmp.path())
        .args(["issue", "list", "--state", "plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bug one"))
        .stdout(predicate::str::contains("Feature one").not());
}

#[test]
fn list_filter_by_type() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature", "Feature one", &[]);
    create(tmp.path(), "bug", "Bug one", &[]);

    dwarven(tmp.path())
        .args(["issue", "list", "--type", "bug"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Bug one"))
        .stdout(predicate::str::contains("Feature one").not());
}

#[test]
fn list_filter_by_priority_including_unset() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature", "P0 thing", &["--priority", "p0"]);
    create(tmp.path(), "bug", "Unset thing", &[]);

    dwarven(tmp.path())
        .args(["issue", "list", "--priority", "p0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("P0 thing"))
        .stdout(predicate::str::contains("Unset thing").not());

    dwarven(tmp.path())
        .args(["issue", "list", "--priority", "unset"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unset thing"))
        .stdout(predicate::str::contains("P0 thing").not());
}

#[test]
fn list_filter_by_epic() {
    let tmp = fresh_repo();
    create(
        tmp.path(),
        "feature",
        "In epic",
        &["--epic", "cli-foundation"],
    );
    create(tmp.path(), "feature", "No epic", &[]);

    dwarven(tmp.path())
        .args(["issue", "list", "--epic", "cli-foundation"])
        .assert()
        .success()
        .stdout(predicate::str::contains("In epic"))
        .stdout(predicate::str::contains("No epic").not());
}

#[test]
fn list_grep_matches_title_and_body() {
    let tmp = fresh_repo();
    create(
        tmp.path(),
        "feature",
        "Match-in-title needle",
        &["--body", "ordinary body"],
    );
    create(
        tmp.path(),
        "bug",
        "Different title",
        &["--body", "needle in body"],
    );
    create(tmp.path(), "bug", "Unrelated", &[]);

    dwarven(tmp.path())
        .args(["issue", "list", "--grep", "needle"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Match-in-title"))
        .stdout(predicate::str::contains("Different title"))
        .stdout(predicate::str::contains("Unrelated").not());
}

#[test]
fn list_sort_id_ascending() {
    let tmp = fresh_repo();
    create(tmp.path(), "feature", "alpha", &[]);
    create(tmp.path(), "bug", "beta", &[]);
    create(tmp.path(), "chore", "gamma", &[]);

    let out = dwarven(tmp.path())
        .args(["issue", "list", "--sort", "id"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let alpha = stdout.find("alpha").unwrap();
    let beta = stdout.find("beta").unwrap();
    let gamma = stdout.find("gamma").unwrap();
    assert!(alpha < beta && beta < gamma, "id sort order broken:\n{stdout}");
}

#[test]
fn list_invalid_sort_exits_user_error() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "list", "--sort", "wibble"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid --sort"));
}

#[test]
fn list_open_closed_all_are_mutually_exclusive() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "list", "--open", "--closed"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("mutually exclusive"));
}
