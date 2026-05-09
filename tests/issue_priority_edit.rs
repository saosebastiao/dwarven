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

// ---------- priority ----------

#[test]
fn priority_sets_field_on_frontmatter() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority", "1", "p0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("priority set to p0"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("priority: p0"));
}

#[test]
fn priority_invalid_value_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority", "1", "p9"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid priority"));
}

#[test]
fn priority_overwrite() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority", "1", "p0"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "priority", "1", "p2"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("priority: p2"));
    assert!(!issue.contains("priority: p0"));
}

#[test]
fn priority_on_terminal_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "done"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "priority", "1", "p0"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("terminal state"));
}

#[test]
fn priority_missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "priority", "999", "p0"])
        .assert()
        .code(3);
}

#[test]
fn list_priority_filter_after_set() {
    let tmp = fresh_repo();
    create(tmp.path());
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "priority", "1", "p0"])
        .assert()
        .success();
    let out = dwarven(tmp.path())
        .args(["issue", "list", "--priority", "p0"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.lines().any(|l| l.starts_with("   1 ")));
    assert!(!stdout.lines().any(|l| l.starts_with("   2 ")));
}

// ---------- edit ----------

#[test]
fn edit_title_updates_frontmatter() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--title", "Renamed thing"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("title: Renamed thing"));
    assert!(!issue.contains("title: x"));
}

#[test]
fn edit_type_updates_frontmatter() {
    let tmp = fresh_repo();
    create(tmp.path()); // type=feature
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--type", "bug"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("type: bug"));
}

#[test]
fn edit_epic_updates_frontmatter() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--epic", "cli-foundation"])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("epic: cli-foundation"));
}

#[test]
fn edit_no_flags_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "edit", "1"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("at least one"));
}

#[test]
fn edit_invalid_type_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--type", "wibble"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid type"));
}

#[test]
fn edit_invalid_epic_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--epic", "BadSlug"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("kebab slug"));
}

#[test]
fn edit_empty_title_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--title", ""])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("title"));
}

#[test]
fn edit_terminal_rejected() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "close", "1", "--comment", "done"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "edit", "1", "--title", "Late rename"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("terminal"));
}

#[test]
fn edit_missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "edit", "999", "--title", "x"])
        .assert()
        .code(3);
}

#[test]
fn edit_combined_flags() {
    let tmp = fresh_repo();
    create(tmp.path());
    dwarven(tmp.path())
        .args([
            "issue", "edit", "1", "--title", "Combined", "--type", "bug",
            "--epic", "alpha-epic",
        ])
        .assert()
        .success();
    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("title: Combined"));
    assert!(issue.contains("type: bug"));
    assert!(issue.contains("epic: alpha-epic"));
}
