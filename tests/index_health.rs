//! Issue #1: probe_health for the SQLite index. Tests
//! coordination-hub.md#R7.3 (PRAGMA integrity_check) and #R7.4
//! (schema-version recorded in meta; rebuild on mismatch).
//!
//! These are end-to-end tests: spawn the daemon with various
//! pre-existing index states, capture stderr, and verify the
//! diagnosis log line + that the post-startup index is healthy.

use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use rusqlite::Connection;

mod common;
use common::{dwarven, fresh_repo};

const READY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

static NEXT_PORT: AtomicU16 = AtomicU16::new(25000);

fn allocate_port() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}

fn set_port(repo: &Path, port: u16) {
    dwarven(repo)
        .args(["config", "set", "daemon.port", &port.to_string()])
        .assert()
        .success();
}

fn index_path(repo: &Path) -> std::path::PathBuf {
    repo.join(".dwarven/.index.sqlite")
}

fn pidfile(repo: &Path) -> std::path::PathBuf {
    repo.join(".dwarven/.daemon.pid")
}

struct DaemonGuard {
    child: Child,
    port: u16,
    stderr_path: std::path::PathBuf,
}

impl DaemonGuard {
    fn spawn(repo: &Path, stderr_path: std::path::PathBuf) -> Self {
        let port = allocate_port();
        set_port(repo, port);
        let stderr_file = std::fs::File::create(&stderr_path).expect("open stderr file");
        let mut cmd = std::process::Command::cargo_bin("dwarven").unwrap();
        cmd.arg("--repo")
            .arg(repo)
            .arg("serve")
            .env_remove("DWARVEN_ACTOR")
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file));
        let child = cmd.spawn().expect("spawn daemon");
        Self { child, port, stderr_path }
    }

    fn wait_ready(&self) {
        let url = format!("http://127.0.0.1:{}/api/v1/daemon", self.port);
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if let Ok(resp) = ureq::get(&url).timeout(Duration::from_millis(200)).call() {
                if resp.status() == 200 {
                    return;
                }
            }
            std::thread::sleep(POLL);
        }
        panic!("daemon HTTP did not become ready");
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn create_one_issue(repo: &Path) {
    dwarven(repo)
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
}

fn assert_index_healthy(repo: &Path) {
    let conn = Connection::open(index_path(repo)).expect("open new index");
    let v: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .expect("schema_version row");
    assert_eq!(v, "1");
    let issues: i64 = conn
        .query_row("SELECT COUNT(*) FROM issue", [], |r| r.get(0))
        .expect("count issues");
    assert!(issues >= 0);
}

#[test]
fn missing_index_logs_no_existing_and_rebuilds() {
    let tmp = fresh_repo();
    create_one_issue(tmp.path());
    assert!(!index_path(tmp.path()).exists());

    let log_path = tmp.path().join("daemon.log");
    let guard = DaemonGuard::spawn(tmp.path(), log_path);
    guard.wait_ready();

    let log = guard.stderr();
    assert!(
        log.contains("no existing index"),
        "expected no-existing-index log line; got:\n{log}"
    );
    assert_index_healthy(tmp.path());

    dwarven(tmp.path()).args(["daemon", "stop"]).assert().code(0);
}

#[test]
fn healthy_index_logs_passed_checks_and_rebuilds() {
    let tmp = fresh_repo();
    create_one_issue(tmp.path());
    // Pre-build a healthy index via the CLI before daemon startup.
    dwarven(tmp.path()).args(["reindex"]).assert().success();
    assert!(index_path(tmp.path()).exists());

    let log_path = tmp.path().join("daemon.log");
    let guard = DaemonGuard::spawn(tmp.path(), log_path);
    guard.wait_ready();

    let log = guard.stderr();
    assert!(
        log.contains("passed integrity + version checks"),
        "expected healthy log line; got:\n{log}"
    );
    assert_index_healthy(tmp.path());

    dwarven(tmp.path()).args(["daemon", "stop"]).assert().code(0);
}

#[test]
fn corrupt_index_logs_corrupt_and_rebuilds() {
    let tmp = fresh_repo();
    create_one_issue(tmp.path());
    dwarven(tmp.path()).args(["reindex"]).assert().success();
    // Replace the index with garbage bytes.
    std::fs::write(index_path(tmp.path()), b"NOT A SQLITE DATABASE").unwrap();

    let log_path = tmp.path().join("daemon.log");
    let guard = DaemonGuard::spawn(tmp.path(), log_path);
    guard.wait_ready();

    let log = guard.stderr();
    assert!(
        log.contains("corrupt"),
        "expected corrupt log line; got:\n{log}"
    );
    assert_index_healthy(tmp.path());

    dwarven(tmp.path()).args(["daemon", "stop"]).assert().code(0);
}

#[test]
fn schema_version_mismatch_logs_mismatch_and_rebuilds() {
    let tmp = fresh_repo();
    create_one_issue(tmp.path());
    dwarven(tmp.path()).args(["reindex"]).assert().success();
    {
        let conn = Connection::open(index_path(tmp.path())).unwrap();
        conn.execute(
            "UPDATE meta SET value = ?1 WHERE key = 'schema_version'",
            ["999"],
        )
        .unwrap();
    }

    let log_path = tmp.path().join("daemon.log");
    let guard = DaemonGuard::spawn(tmp.path(), log_path);
    guard.wait_ready();

    let log = guard.stderr();
    assert!(
        log.contains("schema version mismatch"),
        "expected version-mismatch log line; got:\n{log}"
    );
    // After rebuild, version should be back to current.
    assert_index_healthy(tmp.path());

    dwarven(tmp.path()).args(["daemon", "stop"]).assert().code(0);
}

#[allow(dead_code)]
fn _suppress_unused(p: &Path) {
    let _ = pidfile(p);
}
