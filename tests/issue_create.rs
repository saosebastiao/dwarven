use std::collections::HashSet;
use std::thread;

use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo, read_dwarven};

#[test]
fn create_writes_issue_and_creation_comment() {
    let tmp = fresh_repo();

    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "Foo bar", "--body",
            "Body text.",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"))
        .stdout(predicate::str::contains("state: pm"));

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.starts_with("---\n"), "missing frontmatter delimiter");
    assert!(issue.contains("id: 1"));
    assert!(issue.contains("title: Foo bar"));
    assert!(issue.contains("type: feature"));
    assert!(issue.contains("state: pm"));
    assert!(issue.contains("created_by: maintainer"));
    assert!(issue.ends_with("Body text.\n"));

    // Creation comment uses the `from: created` sentinel.
    let comments = std::fs::read_dir(tmp.path().join(".dwarven/issues/0001/comments"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(comments.len(), 1);
    let comment_body = std::fs::read_to_string(&comments[0]).unwrap();
    assert!(comment_body.contains("kind: state-change"));
    assert!(comment_body.contains("from: created"));
    assert!(comment_body.contains("to: pm"));
    assert!(comment_body.contains("seq: 1"));
    assert!(comment_body.contains("issue: 1"));
}

#[test]
fn type_default_initial_states() {
    let tmp = fresh_repo();

    let cases = [
        ("feature", "pm"),
        ("bug", "plan"),
        ("arch", "architect"),
        ("doc", "doc"),
        ("chore", "plan"),
        ("spec-gap", "pm"),
    ];
    for (i, (ty, expected_state)) in cases.iter().enumerate() {
        let id = i + 1;
        dwarven(tmp.path())
            .args(["issue", "create", "--type", ty, "--title", "x"])
            .assert()
            .success();
        let issue = read_dwarven(tmp.path(), &format!("issues/{id:04}/issue.md"));
        assert!(
            issue.contains(&format!("state: {expected_state}")),
            "type={ty} expected state={expected_state}, got:\n{issue}"
        );
    }
}

#[test]
fn empty_title_exits_user_error() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", ""])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("title must not be empty"));
}

#[test]
fn invalid_type_exits_user_error() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "wibble", "--title", "x"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("invalid type"));
}

#[test]
fn missing_blocks_referent_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "x", "--blocks", "9999",
        ])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("9999"));
}

#[test]
fn blocks_writes_reciprocal_blocked_by() {
    let tmp = fresh_repo();

    // Create issue 1 (target of the dependency).
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "Target"])
        .assert()
        .success();

    // Create issue 2 that blocks issue 1.
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "Blocker", "--blocks", "1",
        ])
        .assert()
        .success();

    let target = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(
        target.contains("blocked_by:") && target.contains("- 2"),
        "issue 1 missing reciprocal blocked_by edge:\n{target}"
    );

    let blocker = read_dwarven(tmp.path(), "issues/0002/issue.md");
    assert!(
        blocker.contains("blocks:") && blocker.contains("- 1"),
        "issue 2 missing blocks edge:\n{blocker}"
    );
}

#[test]
fn parallel_creates_get_distinct_ids() {
    let tmp = fresh_repo();
    let path = tmp.path().to_path_buf();

    let n = 5;
    let handles: Vec<_> = (0..n)
        .map(|i| {
            let path = path.clone();
            thread::spawn(move || {
                dwarven(&path)
                    .args([
                        "issue", "create", "--type", "chore", "--title",
                        &format!("p{i}"), "--quiet",
                    ])
                    .assert()
                    .success();
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }

    let dirs: HashSet<String> = std::fs::read_dir(tmp.path().join(".dwarven/issues"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        dirs.len(),
        n,
        "expected {n} distinct issue dirs, got {dirs:?}"
    );

    let config = read_dwarven(tmp.path(), "config.toml");
    assert!(
        config.contains(&format!("next_issue_id = {}", n + 1)),
        "expected counter at {}, config:\n{config}",
        n + 1
    );
    // Verify config preserved its top-of-file comment across the writes.
    assert!(config.contains("# Dwarven hub configuration"));
}

#[test]
fn body_file_is_read_into_issue_body() {
    let tmp = fresh_repo();
    let body_path = tmp.path().join("body.md");
    std::fs::write(&body_path, "Body from file.\n").unwrap();

    dwarven(tmp.path())
        .args([
            "issue",
            "create",
            "--type",
            "feature",
            "--title",
            "x",
            "--body-file",
        ])
        .arg(&body_path)
        .assert()
        .success();

    let issue = read_dwarven(tmp.path(), "issues/0001/issue.md");
    assert!(issue.contains("Body from file."));
}

#[test]
fn body_input_flags_are_mutually_exclusive() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "x", "--body", "a",
            "--body-stdin",
        ])
        .assert()
        .failure();
}
