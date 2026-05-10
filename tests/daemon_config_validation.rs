//! Issue #2: validate config.toml at daemon startup; fatal on invalid.
//! Tests coordination-hub.md#R10.6.

use std::path::Path;

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo};

fn write_config(repo: &Path, contents: &str) {
    std::fs::write(repo.join(".dwarven/config.toml"), contents).unwrap();
}

const HEADER: &str = "
[repo]
id = \"00000000-0000-0000-0000-000000000000\"
name = \"x\"

[counters]
next_issue_id = 1
";

fn serve(repo: &Path) -> std::process::Command {
    let mut cmd = std::process::Command::cargo_bin("dwarven").unwrap();
    cmd.arg("--repo").arg(repo).arg("serve");
    cmd
}

#[test]
fn rejects_port_out_of_range() {
    let tmp = fresh_repo();
    write_config(
        tmp.path(),
        &format!("{HEADER}\n[daemon]\nport = 70000\nbind = \"127.0.0.1\"\n"),
    );
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("daemon.port"));
}

#[test]
fn rejects_alpha_out_of_range() {
    let tmp = fresh_repo();
    write_config(
        tmp.path(),
        &format!("{HEADER}\n[scheduler]\nalpha = 2.0\n"),
    );
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("scheduler.alpha"));
}

#[test]
fn rejects_priority_weights_out_of_order() {
    let tmp = fresh_repo();
    write_config(
        tmp.path(),
        &format!(
            "{HEADER}\n[scheduler]\nalpha = 0.5\n[scheduler.priority_weights]\np0 = 1\np1 = 5\np2 = 1\nunset = 1\n"
        ),
    );
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("p0 >= p1 >= p2"));
}

#[test]
fn rejects_zero_reconciliation_interval() {
    let tmp = fresh_repo();
    write_config(
        tmp.path(),
        &format!("{HEADER}\n[daemon]\nreconciliation_interval_seconds = 0\n"),
    );
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("reconciliation_interval_seconds"));
}

#[test]
fn rejects_zero_stale_threshold() {
    let tmp = fresh_repo();
    write_config(
        tmp.path(),
        &format!("{HEADER}\n[triage]\nstale_threshold_days = 0\n"),
    );
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("stale_threshold_days"));
}

#[test]
fn rejects_negative_priority_weight() {
    let tmp = fresh_repo();
    write_config(
        tmp.path(),
        &format!(
            "{HEADER}\n[scheduler]\nalpha = 0.5\n[scheduler.priority_weights]\np0 = -1\np1 = 1\np2 = 1\nunset = 1\n"
        ),
    );
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("priority_weights"));
}

#[test]
fn config_set_via_cli_also_rejects_invalid() {
    // The `dwarven config set` path should validate too — already implemented
    // for scheduler.* via slice 20; this confirms the same rejection path
    // is in place for daemon.* / triage.* after issue #2.
    let tmp = fresh_repo();
    // Bad port via the CLI — config set itself doesn't validate, but the
    // next serve will fail. This test documents the at-startup-only model
    // for daemon.* keys: invalid values can be written, but the daemon
    // refuses to start.
    dwarven(tmp.path())
        .args(["config", "set", "daemon.port", "70000"])
        .assert()
        .success();
    serve(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("daemon.port"));
}
