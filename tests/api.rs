use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;
use serde_json::Value;

mod common;
use common::{dwarven, fresh_repo};

const READY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

// Each test gets a unique port to avoid contention when run in parallel.
static NEXT_PORT: AtomicU16 = AtomicU16::new(17800);

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
    fn spawn(repo: &Path, port: u16) -> Self {
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
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn url(port: u16, path: &str) -> String {
    format!("http://127.0.0.1:{port}{path}")
}

#[test]
fn daemon_endpoint_reports_status() {
    let tmp = fresh_repo();
    let port = allocate_port();
    set_port(tmp.path(), port);
    let guard = DaemonGuard::spawn(tmp.path(), port);
    guard.wait_ready();

    let resp: Value = ureq::get(&url(port, "/api/v1/daemon"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(resp["state"], "running");
    assert_eq!(resp["port"].as_u64().unwrap(), port as u64);
    assert!(resp["pid"].as_u64().unwrap() > 0);
    assert_eq!(resp["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn list_issues_returns_active_by_default() {
    let tmp = fresh_repo();
    let port = allocate_port();
    set_port(tmp.path(), port);

    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "Open one"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "Closed one"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "close", "2", "--comment", "shipped"])
        .assert()
        .success();

    let guard = DaemonGuard::spawn(tmp.path(), port);
    guard.wait_ready();

    let body: Value = ureq::get(&url(port, "/api/v1/issues"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1, "default scope should hide closed: {arr:?}");
    assert_eq!(arr[0]["id"].as_u64().unwrap(), 1);
    assert_eq!(arr[0]["title"], "Open one");

    let body: Value = ureq::get(&url(port, "/api/v1/issues?closed=true"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_u64().unwrap(), 2);

    let body: Value = ureq::get(&url(port, "/api/v1/issues?all=true"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[test]
fn view_issue_returns_full_object() {
    let tmp = fresh_repo();
    let port = allocate_port();
    set_port(tmp.path(), port);

    dwarven(tmp.path())
        .args([
            "issue",
            "create",
            "--type",
            "feature",
            "--title",
            "Hello",
            "--body",
            "world",
            "--priority",
            "p1",
        ])
        .assert()
        .success();

    let guard = DaemonGuard::spawn(tmp.path(), port);
    guard.wait_ready();

    let issue: Value = ureq::get(&url(port, "/api/v1/issues/1"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(issue["id"].as_u64().unwrap(), 1);
    assert_eq!(issue["title"], "Hello");
    assert_eq!(issue["type"], "feature");
    assert_eq!(issue["priority"], "p1");
    assert!(issue["body"].as_str().unwrap().contains("world"));
}

#[test]
fn view_missing_issue_returns_404_with_error_shape() {
    let tmp = fresh_repo();
    let port = allocate_port();
    set_port(tmp.path(), port);
    let guard = DaemonGuard::spawn(tmp.path(), port);
    guard.wait_ready();

    let err = ureq::get(&url(port, "/api/v1/issues/999"))
        .call()
        .unwrap_err();
    let response = match err {
        ureq::Error::Status(code, resp) => {
            assert_eq!(code, 404);
            resp
        }
        other => panic!("expected status error, got {other:?}"),
    };
    let body: Value = response.into_json().unwrap();
    assert_eq!(body["error"], "not_found");
    assert!(body["message"].as_str().unwrap().contains("999"));
}

#[test]
fn list_comments_filters_by_kind() {
    let tmp = fresh_repo();
    let port = allocate_port();
    set_port(tmp.path(), port);

    dwarven(tmp.path())
        .args(["issue", "create", "--type", "feature", "--title", "x"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "comment", "1", "--body", "a user comment"])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "transition", "1", "plan"])
        .assert()
        .success();

    let guard = DaemonGuard::spawn(tmp.path(), port);
    guard.wait_ready();

    let all: Value = ureq::get(&url(port, "/api/v1/issues/1/comments"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(all.as_array().unwrap().len(), 3);

    let history: Value = ureq::get(&url(port, "/api/v1/issues/1/comments?kind=state-change"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let arr = history.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr.iter().all(|c| c["kind"] == "state-change"));
}

#[test]
fn list_filters_state_priority_grep() {
    let tmp = fresh_repo();
    let port = allocate_port();
    set_port(tmp.path(), port);

    dwarven(tmp.path())
        .args([
            "issue", "create", "--type", "feature", "--title", "needle alpha", "--priority", "p0",
        ])
        .assert()
        .success();
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "ordinary"])
        .assert()
        .success();

    let guard = DaemonGuard::spawn(tmp.path(), port);
    guard.wait_ready();

    let by_priority: Value = ureq::get(&url(port, "/api/v1/issues?priority=p0"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(by_priority.as_array().unwrap().len(), 1);
    assert_eq!(by_priority[0]["title"], "needle alpha");

    let by_grep: Value = ureq::get(&url(port, "/api/v1/issues?grep=needle"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(by_grep.as_array().unwrap().len(), 1);

    let by_state: Value = ureq::get(&url(port, "/api/v1/issues?state=plan"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(by_state.as_array().unwrap().len(), 1);
    assert_eq!(by_state[0]["state"], "plan");
}
