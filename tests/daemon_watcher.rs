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
const INDEX_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(50);

static NEXT_PORT: AtomicU16 = AtomicU16::new(19000);

fn allocate_port() -> u16 {
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}

fn set_port(repo: &Path, port: u16) {
    dwarven(repo)
        .args(["config", "set", "daemon.port", &port.to_string()])
        .assert()
        .success();
}

struct DaemonGuard {
    child: Child,
}

impl DaemonGuard {
    fn spawn(repo: &Path) -> Self {
        let port = allocate_port();
        set_port(repo, port);
        let mut cmd = std::process::Command::cargo_bin("dwarven").unwrap();
        cmd.arg("--repo")
            .arg(repo)
            .arg("serve")
            .env_remove("DWARVEN_ACTOR")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = cmd.spawn().expect("spawn daemon");
        Self { child }
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn pidfile(repo: &Path) -> std::path::PathBuf {
    repo.join(".dwarven/.daemon.pid")
}

fn wait_until_pidfile_exists(repo: &Path) {
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if pidfile(repo).exists() {
            return;
        }
        std::thread::sleep(POLL);
    }
    panic!("daemon did not produce PID file within {READY_TIMEOUT:?}");
}

fn wait_for_index(repo: &Path) {
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if repo.join(".dwarven/.index.sqlite").exists() {
            return;
        }
        std::thread::sleep(POLL);
    }
    panic!("daemon did not create index within {READY_TIMEOUT:?}");
}

fn count_issues(repo: &Path) -> Option<i64> {
    let path = repo.join(".dwarven/.index.sqlite");
    if !path.exists() {
        return None;
    }
    let conn = Connection::open(&path).ok()?;
    conn.query_row("SELECT COUNT(*) FROM issue", [], |r| r.get(0))
        .ok()
}

fn wait_until_issues(repo: &Path, expected: i64) {
    let deadline = Instant::now() + INDEX_TIMEOUT;
    while Instant::now() < deadline {
        if let Some(n) = count_issues(repo) {
            if n == expected {
                return;
            }
        }
        std::thread::sleep(POLL);
    }
    panic!(
        "index did not converge to {expected} issues within {INDEX_TIMEOUT:?} (last: {:?})",
        count_issues(repo)
    );
}

#[test]
fn daemon_initial_reindex_on_startup() {
    let tmp = fresh_repo();
    // Pre-seed two issues before starting the daemon.
    for _ in 0..2 {
        dwarven(tmp.path())
            .args(["issue", "create", "--type", "feature", "--title", "x"])
            .assert()
            .success();
    }

    let _guard = DaemonGuard::spawn(tmp.path());
    wait_until_pidfile_exists(tmp.path());
    wait_for_index(tmp.path());
    wait_until_issues(tmp.path(), 2);

    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0);
}

#[test]
fn daemon_picks_up_newly_created_issue() {
    let tmp = fresh_repo();
    let _guard = DaemonGuard::spawn(tmp.path());
    wait_until_pidfile_exists(tmp.path());
    wait_for_index(tmp.path());
    wait_until_issues(tmp.path(), 0);

    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "Live"])
        .assert()
        .success();
    wait_until_issues(tmp.path(), 1);

    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "Live2"])
        .assert()
        .success();
    wait_until_issues(tmp.path(), 2);

    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0);
}

#[test]
fn daemon_picks_up_state_transition_via_index() {
    let tmp = fresh_repo();
    let _guard = DaemonGuard::spawn(tmp.path());
    wait_until_pidfile_exists(tmp.path());
    wait_for_index(tmp.path());

    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    wait_until_issues(tmp.path(), 1);

    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .success();

    // Wait for the daemon to reindex the new state.
    let deadline = Instant::now() + INDEX_TIMEOUT;
    let mut state = String::new();
    while Instant::now() < deadline {
        let conn = Connection::open(tmp.path().join(".dwarven/.index.sqlite")).unwrap();
        state = conn
            .query_row("SELECT state FROM issue WHERE id = 1", [], |r| r.get(0))
            .unwrap_or_default();
        if state == "plan" {
            break;
        }
        std::thread::sleep(POLL);
    }
    assert_eq!(state, "plan", "daemon did not pick up transition in time");

    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0);
}
