use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo};

fn create_one(repo: &std::path::Path) {
    dwarven(repo)
        .args(["issue", "create", "--type", "feature", "--title", "Target"])
        .assert()
        .success();
}

#[test]
fn comment_appends_seq_2_after_creation_comment() {
    let tmp = fresh_repo();
    create_one(tmp.path());

    dwarven(tmp.path())
        .args([
            "issue", "comment", "1", "--body", "First user comment.",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added comment #002 on issue #1"));

    let comments_dir = tmp.path().join(".dwarven/issues/0001/comments");
    let entries: Vec<_> = std::fs::read_dir(&comments_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries.len(), 2);
    let has_002 = entries.iter().any(|n| n.starts_with("002-"));
    assert!(has_002, "expected 002- prefix, got: {entries:?}");
}

#[test]
fn comment_bumps_issue_updated_timestamp() {
    let tmp = fresh_repo();
    create_one(tmp.path());

    let before = std::fs::read_to_string(tmp.path().join(".dwarven/issues/0001/issue.md"))
        .unwrap();

    // Sleep a second so updated changes (frontmatter has second precision).
    std::thread::sleep(std::time::Duration::from_millis(1100));

    dwarven(tmp.path())
        .args(["issue", "comment", "1", "--body", "x"])
        .assert()
        .success();

    let after = std::fs::read_to_string(tmp.path().join(".dwarven/issues/0001/issue.md"))
        .unwrap();
    assert_ne!(
        extract_field(&before, "updated"),
        extract_field(&after, "updated"),
        "updated should differ; before:\n{before}\nafter:\n{after}"
    );
}

#[test]
fn comment_requires_body_input() {
    let tmp = fresh_repo();
    create_one(tmp.path());
    // clap's required ArgGroup catches this before our handler runs.
    dwarven(tmp.path())
        .args(["issue", "comment", "1"])
        .assert()
        .failure();
}

#[test]
fn comment_missing_id_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "comment", "999", "--body", "x"])
        .assert()
        .code(3);
}

#[test]
fn multiple_comments_get_distinct_seq() {
    let tmp = fresh_repo();
    create_one(tmp.path());

    for i in 0..5 {
        dwarven(tmp.path())
            .args(["issue", "comment", "1", "--body", &format!("c{i}"), "--quiet"])
            .assert()
            .success();
    }

    // Creation comment + 5 user comments = 6 files.
    let entries: Vec<_> = std::fs::read_dir(tmp.path().join(".dwarven/issues/0001/comments"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(entries.len(), 6);
    let prefixes: Vec<&str> = entries
        .iter()
        .filter_map(|n| n.split('-').next())
        .collect();
    let mut sorted = prefixes.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["001", "002", "003", "004", "005", "006"],
        "expected 001..006 distinct seqs, got {sorted:?}"
    );
}

#[test]
fn comment_appears_in_view_output() {
    let tmp = fresh_repo();
    create_one(tmp.path());
    dwarven(tmp.path())
        .args(["issue", "comment", "1", "--body", "Hello from the test."])
        .assert()
        .success();

    dwarven(tmp.path())
        .args(["issue", "view", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--- comments (2) ---"))
        .stdout(predicate::str::contains("Hello from the test."));
}

fn extract_field<'a>(content: &'a str, key: &str) -> &'a str {
    content
        .lines()
        .find_map(|l| l.strip_prefix(&format!("{key}: ")))
        .unwrap_or("")
}
