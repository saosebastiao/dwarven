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

static NEXT_PORT: AtomicU16 = AtomicU16::new(20000);

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

fn post_json<S: AsRef<str>>(url: S, body: Value) -> ureq::Response {
    ureq::post(url.as_ref())
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .unwrap()
}

fn patch_json<S: AsRef<str>>(url: S, body: Value) -> ureq::Response {
    ureq::request("PATCH", url.as_ref())
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .unwrap()
}

fn put_json<S: AsRef<str>>(url: S, body: Value) -> ureq::Response {
    ureq::request("PUT", url.as_ref())
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .unwrap()
}

fn delete<S: AsRef<str>>(url: S) -> ureq::Response {
    ureq::request("DELETE", url.as_ref()).call().unwrap()
}

fn delete_with_body<S: AsRef<str>>(url: S, body: Value) -> ureq::Response {
    ureq::request("DELETE", url.as_ref())
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .unwrap()
}

#[test]
fn create_issue_returns_201_and_object() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = post_json(
        guard.url("/api/v1/issues"),
        json!({
            "type": "feature",
            "title": "Built via HTTP",
            "body": "from the network",
            "priority": "p1"
        }),
    );
    assert_eq!(resp.status(), 201);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["id"], 1);
    assert_eq!(issue["title"], "Built via HTTP");
    assert_eq!(issue["priority"], "p1");
    assert!(issue["body"].as_str().unwrap().contains("from the network"));
}

#[test]
fn create_with_blocker_field_sets_blocker() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x", "blocker": "external"}),
    );
    assert_eq!(resp.status(), 201);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["blocker"], "external");
}

#[test]
fn create_with_invalid_type_returns_400() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let err = ureq::post(&guard.url("/api/v1/issues"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"type": "wibble", "title": "x"}).to_string())
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
    assert!(body["message"].as_str().unwrap().contains("invalid type"));
}

#[test]
fn patch_edits_title_type_epic() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "before"}),
    );
    let resp = patch_json(
        guard.url("/api/v1/issues/1"),
        json!({"title": "after", "type": "bug", "epic": "alpha-epic"}),
    );
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["title"], "after");
    assert_eq!(issue["type"], "bug");
    assert_eq!(issue["epic"], "alpha-epic");
}

#[test]
fn append_comment_returns_full_thread() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x"}),
    );
    let resp = post_json(
        guard.url("/api/v1/issues/1/comments"),
        json!({"body": "Hello via HTTP"}),
    );
    assert_eq!(resp.status(), 201);
    let comments: Value = resp.into_json().unwrap();
    let arr = comments.as_array().unwrap();
    assert_eq!(arr.len(), 2); // creation + the new one
    assert_eq!(arr[1]["kind"], "comment");
    assert!(arr[1]["body"].as_str().unwrap().contains("Hello via HTTP"));
}

#[test]
fn transition_legal_state_change() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x"}),
    );
    let resp = post_json(
        guard.url("/api/v1/issues/1/transitions"),
        json!({"to": "plan", "comment": "decomposed"}),
    );
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["state"], "plan");
}

#[test]
fn transition_to_done_routes_through_close() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x"}),
    );
    // Move through pm → plan → test → implement.
    for next in ["plan", "test", "implement"] {
        post_json(
            guard.url("/api/v1/issues/1/transitions"),
            json!({"to": next}),
        );
    }
    // implement → done is illegal in the R6.2 graph but allowed by close.
    let resp = post_json(
        guard.url("/api/v1/issues/1/transitions"),
        json!({"to": "done", "comment": "shipped"}),
    );
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["state"], "done");
}

#[test]
fn illegal_transition_returns_409_conflict() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x"}),
    );
    let err = ureq::post(&guard.url("/api/v1/issues/1/transitions"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"to": "implement"}).to_string())
        .unwrap_err();
    let resp = match err {
        ureq::Error::Status(code, r) => {
            assert_eq!(code, 409);
            r
        }
        other => panic!("expected status, got {other:?}"),
    };
    let body: Value = resp.into_json().unwrap();
    assert_eq!(body["error"], "conflict");
    assert!(body["message"].as_str().unwrap().contains("illegal transition"));
}

#[test]
fn blocker_set_and_clear() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x"}),
    );
    let resp = put_json(
        guard.url("/api/v1/issues/1/blocker"),
        json!({"blocker": "upstream", "comment": "waiting on lib"}),
    );
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["blocker"], "upstream");

    let resp = delete_with_body(
        guard.url("/api/v1/issues/1/blocker"),
        json!({"comment": "lib released"}),
    );
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert!(issue["blocker"].is_null());
}

#[test]
fn priority_set_and_clear() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    post_json(
        guard.url("/api/v1/issues"),
        json!({"type": "feature", "title": "x"}),
    );
    let resp = put_json(
        guard.url("/api/v1/issues/1/priority"),
        json!({"priority": "p0"}),
    );
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert_eq!(issue["priority"], "p0");

    let resp = delete(guard.url("/api/v1/issues/1/priority"));
    assert_eq!(resp.status(), 200);
    let issue: Value = resp.into_json().unwrap();
    assert!(issue["priority"].is_null());
}

#[test]
fn dep_add_remove_and_list() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    for _ in 0..2 {
        post_json(
            guard.url("/api/v1/issues"),
            json!({"type": "feature", "title": "x"}),
        );
    }

    let resp = post_json(
        guard.url("/api/v1/dependencies"),
        json!({"from": 1, "to": 2}),
    );
    assert_eq!(resp.status(), 201);

    let resp = ureq::get(&guard.url("/api/v1/dependencies"))
        .call()
        .unwrap();
    let edges: Value = resp.into_json().unwrap();
    let arr = edges.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["from"], 1);
    assert_eq!(arr[0]["to"], 2);

    let resp = ureq::get(&guard.url("/api/v1/issues/1/dependencies"))
        .call()
        .unwrap();
    let body: Value = resp.into_json().unwrap();
    assert_eq!(body["blocks"][0], 2);

    let resp = delete(guard.url("/api/v1/dependencies/1/2"));
    assert_eq!(resp.status(), 204);

    let resp = ureq::get(&guard.url("/api/v1/dependencies"))
        .call()
        .unwrap();
    let edges: Value = resp.into_json().unwrap();
    assert!(edges.as_array().unwrap().is_empty());
}

#[test]
fn dep_cycle_returns_409_conflict() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    for _ in 0..2 {
        post_json(
            guard.url("/api/v1/issues"),
            json!({"type": "feature", "title": "x"}),
        );
    }
    post_json(
        guard.url("/api/v1/dependencies"),
        json!({"from": 1, "to": 2}),
    );

    let err = ureq::post(&guard.url("/api/v1/dependencies"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"from": 2, "to": 1}).to_string())
        .unwrap_err();
    let resp = match err {
        ureq::Error::Status(code, r) => {
            assert_eq!(code, 409);
            r
        }
        other => panic!("expected status, got {other:?}"),
    };
    let body: Value = resp.into_json().unwrap();
    assert_eq!(body["error"], "conflict");
    assert!(body["message"].as_str().unwrap().contains("cycle"));
}
