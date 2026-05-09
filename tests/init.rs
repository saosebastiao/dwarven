use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, read_dwarven};

#[test]
fn init_creates_canonical_layout() {
    let tmp = tempfile::tempdir().unwrap();

    dwarven(tmp.path())
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized dwarven"));

    let dwarven_dir = tmp.path().join(".dwarven");
    assert!(dwarven_dir.is_dir());
    assert!(dwarven_dir.join("config.toml").is_file());
    assert!(dwarven_dir.join(".gitignore").is_file());
    assert!(dwarven_dir.join("issues").is_dir());

    let config = read_dwarven(tmp.path(), "config.toml");
    assert!(config.contains("next_issue_id = 1"));
    assert!(config.contains("[repo]"));
    assert!(config.contains("[counters]"));

    let gitignore = read_dwarven(tmp.path(), ".gitignore");
    assert!(gitignore.contains(".index.sqlite"));
    assert!(gitignore.contains(".config.lock"));
}

#[test]
fn init_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();

    dwarven(tmp.path()).args(["init"]).assert().success();
    let first = read_dwarven(tmp.path(), "config.toml");

    dwarven(tmp.path())
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already initialized"));

    let second = read_dwarven(tmp.path(), "config.toml");
    assert_eq!(first, second, "config.toml mutated by re-running init");
}

#[test]
fn init_rejects_partial_directory() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".dwarven")).unwrap();
    // Leave config.toml absent → partially-initialized.

    dwarven(tmp.path())
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("partially-initialized"));
}
