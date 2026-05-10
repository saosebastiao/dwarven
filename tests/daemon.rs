use std::path::Path;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use predicates::prelude::*;

mod common;
use common::{dwarven, fresh_repo};

const READY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

/// Spawn `dwarven --repo <path> serve` in the background. The returned guard
/// kills the child on drop so tests don't leak daemons on panic.
struct DaemonGuard {
    child: Child,
}

impl DaemonGuard {
    fn spawn(repo: &Path) -> Self {
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

fn wait_until_pidfile_absent(repo: &Path) {
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if !pidfile(repo).exists() {
            return;
        }
        std::thread::sleep(POLL);
    }
    panic!("PID file still present after stop");
}

#[test]
fn status_stopped_on_fresh_repo() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["daemon", "status"])
        .assert()
        .code(4)
        .stdout(predicate::str::contains("stopped"));
}

#[test]
fn lifecycle_serve_status_stop() {
    let tmp = fresh_repo();
    let _guard = DaemonGuard::spawn(tmp.path());
    wait_until_pidfile_exists(tmp.path());

    let pid_str = std::fs::read_to_string(pidfile(tmp.path())).unwrap();
    let pid: i32 = pid_str.trim().parse().unwrap();

    dwarven(tmp.path())
        .args(["daemon", "status"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("running"))
        .stdout(predicate::str::contains(pid.to_string()));

    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("stopped"));

    wait_until_pidfile_absent(tmp.path());

    dwarven(tmp.path())
        .args(["daemon", "status"])
        .assert()
        .code(4);
}

#[test]
fn second_daemon_rejected_while_first_alive() {
    let tmp = fresh_repo();
    let _guard = DaemonGuard::spawn(tmp.path());
    wait_until_pidfile_exists(tmp.path());

    dwarven(tmp.path())
        .args(["serve"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already running"));
}

#[test]
fn stop_when_not_running_succeeds() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("not running"));
}

#[test]
fn stale_pidfile_cleared_on_stop() {
    let tmp = fresh_repo();
    // PID 1 is init / launchd; we cannot signal it. Use a PID that won't
    // exist as a child of this user — pick a very high integer so kill(0)
    // returns ESRCH.
    let fake_pid = 0x7fff_fff0_i32;
    std::fs::write(pidfile(tmp.path()), format!("{fake_pid}\n")).unwrap();

    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("stale PID"));

    assert!(!pidfile(tmp.path()).exists(), "stale pidfile not removed");
}

#[test]
fn stale_pidfile_status_reports_stale() {
    let tmp = fresh_repo();
    let fake_pid = 0x7fff_fff0_i32;
    std::fs::write(pidfile(tmp.path()), format!("{fake_pid}\n")).unwrap();

    dwarven(tmp.path())
        .args(["daemon", "status"])
        .assert()
        .code(4)
        .stdout(predicate::str::contains("stale"));
}

#[test]
fn stale_pidfile_reclaimed_on_serve() {
    let tmp = fresh_repo();
    let fake_pid = 0x7fff_fff0_i32;
    std::fs::write(pidfile(tmp.path()), format!("{fake_pid}\n")).unwrap();

    let _guard = DaemonGuard::spawn(tmp.path());
    // Wait for the daemon to overwrite the stale PID, not just for the
    // file to exist (it already exists).
    let deadline = Instant::now() + READY_TIMEOUT;
    let mut pid = fake_pid;
    while Instant::now() < deadline {
        if let Ok(s) = std::fs::read_to_string(pidfile(tmp.path())) {
            if let Ok(p) = s.trim().parse::<i32>() {
                if p != fake_pid {
                    pid = p;
                    break;
                }
            }
        }
        std::thread::sleep(POLL);
    }
    assert_ne!(pid, fake_pid, "stale PID was not overwritten within timeout");

    dwarven(tmp.path())
        .args(["daemon", "stop"])
        .assert()
        .code(0);
}

#[test]
fn serve_requires_initialized_repo() {
    let tmp = tempfile::tempdir().unwrap();
    dwarven(tmp.path())
        .args(["serve"])
        .assert()
        .failure();
}

#[test]
fn status_json_emits_pid_when_running() {
    let tmp = fresh_repo();
    let _guard = DaemonGuard::spawn(tmp.path());
    wait_until_pidfile_exists(tmp.path());

    let out = dwarven(tmp.path())
        .args(["--json", "daemon", "status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("\"state\":\"running\""));
    assert!(stdout.contains("\"pid\":"));

    dwarven(tmp.path()).args(["daemon", "stop"]).assert().code(0);
}
