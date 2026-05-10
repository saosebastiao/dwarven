use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::{Duration, Instant};

use assert_cmd::cargo::CommandCargoExt;
use assert_cmd::prelude::*;

mod common;
use common::{dwarven, fresh_repo};

const READY_TIMEOUT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

static NEXT_PORT: AtomicU16 = AtomicU16::new(23000);

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

#[test]
fn root_serves_index_html_with_correct_content_type() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::get(&guard.url("/")).call().unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp.header("content-type").unwrap();
    assert!(ct.starts_with("text/html"), "content-type was {ct:?}");
    let body = resp.into_string().unwrap();
    assert!(body.contains("<title>Dwarven</title>"));
    assert!(body.contains("<script src=\"/app.js\">"));
}

#[test]
fn app_js_is_served() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::get(&guard.url("/app.js")).call().unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp.header("content-type").unwrap();
    assert!(ct.starts_with("application/javascript"));
    let body = resp.into_string().unwrap();
    assert!(body.contains("EventSource"));
    assert!(body.contains("connectSSE"));
}

#[test]
fn app_css_is_served() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let resp = ureq::get(&guard.url("/app.css")).call().unwrap();
    assert_eq!(resp.status(), 200);
    let ct = resp.header("content-type").unwrap();
    assert!(ct.starts_with("text/css"));
}

#[test]
fn spa_fallback_serves_html_for_deep_links() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    for path in &["/issues", "/issues/42", "/daemon", "/anything-else"] {
        let resp = ureq::get(&guard.url(path)).call().unwrap();
        assert_eq!(resp.status(), 200, "fallback failed for {path}");
        let ct = resp.header("content-type").unwrap();
        assert!(
            ct.starts_with("text/html"),
            "{path} returned content-type {ct:?}"
        );
    }
}

#[test]
fn unknown_api_path_returns_json_404() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let err = ureq::get(&guard.url("/api/v1/no-such-endpoint"))
        .call()
        .unwrap_err();
    let resp = match err {
        ureq::Error::Status(code, r) => {
            assert_eq!(code, 404);
            r
        }
        other => panic!("expected status, got {other:?}"),
    };
    let ct = resp.header("content-type").unwrap();
    assert!(ct.starts_with("application/json"));
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["error"], "not_found");
}
