use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use serde_json::{Value, json};

mod common;
use common::{dwarven, fresh_repo};

const READY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

static NEXT_PORT: AtomicU16 = AtomicU16::new(21000);

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
    port: u16,
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
        Self { child, port }
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
        panic!("daemon HTTP did not become ready within {READY_TIMEOUT:?}");
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
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

#[test]
fn config_get_returns_full_toml_as_json() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp: Value = ureq::get(&guard.url("/api/v1/config"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert!(resp.get("repo").is_some());
    assert!(resp.get("counters").is_some());
    assert!(resp["scheduler"]["alpha"].as_f64().is_some());
}

#[test]
fn config_patch_updates_dotted_keys_and_signals_restart() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::request("PATCH", &guard.url("/api/v1/config"))
        .set("Content-Type", "application/json")
        .send_string(
            &json!({"daemon": {"port": 22222}, "triage": {"stale_threshold_days": 7}}).to_string(),
        )
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.into_json().unwrap();
    assert!(body["requires_restart"].as_bool().unwrap());
    assert_eq!(body["config"]["daemon"]["port"], 22222);
    assert_eq!(body["config"]["triage"]["stale_threshold_days"], 7);
}

#[test]
fn config_patch_no_restart_for_non_listen_keys() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::request("PATCH", &guard.url("/api/v1/config"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"scheduler": {"alpha": 0.7}}).to_string())
        .unwrap();
    let body: Value = resp.into_json().unwrap();
    assert!(!body["requires_restart"].as_bool().unwrap());
}

#[test]
fn config_patch_empty_body_returns_400() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let err = ureq::request("PATCH", &guard.url("/api/v1/config"))
        .set("Content-Type", "application/json")
        .send_string("{}")
        .unwrap_err();
    let resp = match err {
        ureq::Error::Status(code, r) => {
            assert_eq!(code, 400);
            r
        }
        other => panic!("expected status, got {other:?}"),
    };
    let body: Value = resp.into_json().unwrap();
    assert_eq!(body["error"], "bad_request");
}

#[test]
fn daemon_reindex_returns_counts() {
    let tmp = fresh_repo();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::post(&guard.url("/api/v1/daemon/reindex"))
        .set("Content-Type", "application/json")
        .send_string("")
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.into_json().unwrap();
    assert_eq!(body["issues"], 1);
    assert!(body["comments"].as_u64().unwrap() >= 1);
}

#[test]
fn daemon_shutdown_terminates_process() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::post(&guard.url("/api/v1/daemon/shutdown"))
        .set("Content-Type", "application/json")
        .send_string("")
        .unwrap();
    assert_eq!(resp.status(), 202);

    // Daemon's clean shutdown removes the PID file.
    let deadline = Instant::now() + READY_TIMEOUT;
    while Instant::now() < deadline {
        if !pidfile(tmp.path()).exists() {
            return;
        }
        std::thread::sleep(POLL);
    }
    panic!("daemon did not shut down within timeout");
}

// Removed: scheduler endpoints were 501 stubs in slice 16; slice 21 made
// them real. See tests/api_scheduler.rs for the current coverage.
