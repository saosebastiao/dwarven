use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo, read_dwarven};

#[test]
fn get_existing_default_keys() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "get", "daemon.port"])
        .assert()
        .success()
        .stdout(predicate::str::contains("7777"));
    dwarven(tmp.path())
        .args(["config", "get", "daemon.bind"])
        .assert()
        .success()
        .stdout(predicate::str::contains("127.0.0.1"));
    dwarven(tmp.path())
        .args(["config", "get", "scheduler.alpha"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.5"));
}

#[test]
fn get_unknown_key_exits_not_found() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "get", "does.not.exist"])
        .assert()
        .code(3);
}

#[test]
fn set_then_get_round_trips_int() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "set", "daemon.port", "8080"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["config", "get", "daemon.port"])
        .assert()
        .success()
        .stdout(predicate::str::contains("8080"));
}

#[test]
fn set_string_value() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "set", "daemon.bind", "0.0.0.0"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["config", "get", "daemon.bind"])
        .assert()
        .success()
        .stdout(predicate::str::contains("0.0.0.0"));
}

#[test]
fn set_creates_missing_section_and_key() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "set", "newsection.newkey", "42"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["config", "get", "newsection.newkey"])
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
fn config_comments_preserved_across_set() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "set", "daemon.port", "9999"])
        .assert()
        .success();
    let raw = read_dwarven(tmp.path(), "config.toml");
    assert!(raw.contains("# Dwarven hub configuration"));
    assert!(raw.contains("port = 9999"));
}

#[test]
fn empty_key_rejected() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "get", ""])
        .assert()
        .code(1);
    dwarven(tmp.path())
        .args(["config", "set", "", "x"])
        .assert()
        .code(1);
}

#[test]
fn malformed_dotted_key_rejected() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["config", "get", "daemon..port"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("segments must not be empty"));
}

#[test]
fn config_requires_initialized_repo() {
    let tmp = tempfile::tempdir().unwrap();
    dwarven(tmp.path())
        .args(["config", "get", "daemon.port"])
        .assert()
        .failure();
}
