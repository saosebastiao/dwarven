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

static NEXT_PORT: AtomicU16 = AtomicU16::new(24000);

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

fn create_with_priority(repo: &Path, title: &str, priority: &str, blocks: Option<&str>) {
    let mut cmd = dwarven(repo);
    cmd.args([
        "issue", "create", "--type", "feature", "--title", title, "--priority", priority,
    ]);
    if let Some(b) = blocks {
        cmd.args(["--blocks", b]);
    }
    cmd.assert().success();
}

#[test]
fn queue_returns_ranked_list() {
    // The motivating example: issue blocking 5 p0s outranks an isolated p1.
    let tmp = fresh_repo();
    for i in 1..=5 {
        create_with_priority(tmp.path(), &format!("p0 #{i}"), "p0", None);
    }
    create_with_priority(tmp.path(), "blocker p1", "p1", Some("1,2,3,4,5"));
    create_with_priority(tmp.path(), "isolated p1", "p1", None);

    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let rows: Value = ureq::get(&guard.url("/api/v1/scheduler/queue"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 7);

    // First row should be the p1 blocker (#6) with effective_priority 12.
    assert_eq!(arr[0]["id"], 6);
    assert!((arr[0]["effective_priority"].as_f64().unwrap() - 12.0).abs() < 1e-9);
    assert_eq!(arr[0]["actionable"], true);

    // Last row should be the isolated p1 (#7), score 2.
    let last = &arr[arr.len() - 1];
    assert_eq!(last["id"], 7);
    assert!((last["effective_priority"].as_f64().unwrap() - 2.0).abs() < 1e-9);
}

#[test]
fn queue_state_filter() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "feature one", "p1", None); // pm
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "bug one"])
        .assert()
        .success(); // plan

    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let rows: Value = ureq::get(&guard.url("/api/v1/scheduler/queue?state=plan"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["state"], "plan");
}

#[test]
fn queue_actionable_filter() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "upstream", "p1", None);
    create_with_priority(tmp.path(), "downstream", "p1", None);
    // Add an edge so #2 is blocked_by #1.
    ureq::post(&format!(
        "http://127.0.0.1:{}/api/v1/dependencies",
        9999  // dummy; we reach the daemon below
    ))
    .timeout(Duration::from_millis(1))
    .send_string("ignored")
    .ok();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    ureq::post(&guard.url("/api/v1/dependencies"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"from": 1, "to": 2}).to_string())
        .unwrap();

    let all: Value = ureq::get(&guard.url("/api/v1/scheduler/queue"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    assert_eq!(all.as_array().unwrap().len(), 2);

    let actionable: Value = ureq::get(&guard.url("/api/v1/scheduler/queue?actionable=true"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let arr = actionable.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], 1, "only upstream should be actionable");
}

#[test]
fn override_via_api_appears_in_queue() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "low", "p2", None);
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::post(&guard.url("/api/v1/scheduler/override"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"issue": 1, "value": 99.5}).to_string())
        .unwrap();
    assert_eq!(resp.status(), 200);

    let rows: Value = ureq::get(&guard.url("/api/v1/scheduler/queue"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let row = &rows.as_array().unwrap()[0];
    assert_eq!(row["override"], 99.5);
    assert!((row["effective_priority"].as_f64().unwrap() - 99.5).abs() < 1e-9);
    // base score remains the algorithmic value.
    assert!((row["score"].as_f64().unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn override_clear_via_api() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "x", "p1", None);
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    ureq::post(&guard.url("/api/v1/scheduler/override"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"issue": 1, "value": 50.0}).to_string())
        .unwrap();
    let resp = ureq::post(&guard.url("/api/v1/scheduler/override"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"issue": 1, "clear": true}).to_string())
        .unwrap();
    assert_eq!(resp.status(), 200);

    let rows: Value = ureq::get(&guard.url("/api/v1/scheduler/queue"))
        .call()
        .unwrap()
        .into_json()
        .unwrap();
    let row = &rows.as_array().unwrap()[0];
    assert!(row["override"].is_null());
    assert!((row["effective_priority"].as_f64().unwrap() - 2.0).abs() < 1e-9);
}

#[test]
fn override_body_must_have_value_xor_clear() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "x", "p1", None);
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let err = ureq::post(&guard.url("/api/v1/scheduler/override"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"issue": 1}).to_string())
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
fn override_missing_issue_returns_404() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let err = ureq::post(&guard.url("/api/v1/scheduler/override"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"issue": 999, "value": 5.0}).to_string())
        .unwrap_err();
    let resp = match err {
        ureq::Error::Status(code, r) => {
            assert_eq!(code, 404);
            r
        }
        other => panic!("expected status, got {other:?}"),
    };
    let body: Value = resp.into_json().unwrap();
    assert_eq!(body["error"], "not_found");
}

#[test]
fn cli_schedule_next_default_returns_top_one() {
    let tmp = fresh_repo();
    for _ in 0..3 {
        create_with_priority(tmp.path(), "x", "p1", None);
    }
    let out = dwarven(tmp.path())
        .args(["schedule", "next"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let row_lines: Vec<_> = stdout.lines().filter(|l| l.starts_with("   1")).collect();
    assert_eq!(row_lines.len(), 1);
}

#[test]
fn cli_schedule_next_count_n() {
    let tmp = fresh_repo();
    for _ in 0..3 {
        create_with_priority(tmp.path(), "x", "p1", None);
    }
    let out = dwarven(tmp.path())
        .args(["schedule", "next", "--count", "3"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let count = stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("RANK"))
        .count();
    assert_eq!(count, 3);
}

#[test]
fn cli_schedule_next_state_filter() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "feature", "p1", None);
    dwarven(tmp.path())
        .args(["issue", "create", "--type", "bug", "--title", "bug"])
        .assert()
        .success();

    let out = dwarven(tmp.path())
        .args(["schedule", "next", "--state", "plan", "--count", "5"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("plan"));
    assert!(!stdout.contains("feature"));
}

#[test]
fn cli_schedule_next_actionable_only() {
    let tmp = fresh_repo();
    create_with_priority(tmp.path(), "actionable", "p1", None);
    create_with_priority(tmp.path(), "blocked-by-it", "p0", None);
    // Add edge: issue 1 blocks issue 2 → issue 2 not actionable.
    dwarven(tmp.path())
        .args(["issue", "dep", "add", "1", "blocks", "2"])
        .assert()
        .success();

    let out = dwarven(tmp.path())
        .args(["schedule", "next", "--count", "5", "--actionable-only"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let body_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("RANK"))
        .collect();
    assert_eq!(body_lines.len(), 1);
    assert!(body_lines[0].contains("actionable"));
}
