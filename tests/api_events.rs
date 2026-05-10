use std::io::{BufRead, BufReader};
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

static NEXT_PORT: AtomicU16 = AtomicU16::new(22000);

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

/// Open the SSE stream and read until we collect at least `target` event lines
/// or the timeout elapses. Returns parsed (event_name, JSON) pairs.
fn collect_events(url: &str, target: usize, timeout: Duration) -> Vec<(String, Value)> {
    let resp = ureq::get(url)
        .set("Accept", "text/event-stream")
        .timeout(timeout)
        .call()
        .expect("SSE connect");
    let reader = BufReader::new(resp.into_reader());
    let deadline = Instant::now() + timeout;
    let mut events = Vec::new();
    let mut current_event: Option<String> = None;

    for line in reader.lines() {
        if Instant::now() > deadline {
            break;
        }
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(rest) = line.strip_prefix("event: ") {
            current_event = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("data: ") {
            if let Some(name) = current_event.take() {
                if let Ok(v) = serde_json::from_str::<Value>(rest) {
                    events.push((name, v));
                    if events.len() >= target {
                        return events;
                    }
                }
            }
        }
    }
    events
}

#[test]
fn sse_emits_issue_created_and_changed() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let url = guard.url("/api/v1/events");
    // Open SSE on a worker thread so we can drive mutations from this one.
    let handle = std::thread::spawn(move || collect_events(&url, 2, Duration::from_secs(5)));

    // Give the SSE connection a beat to subscribe before we send events.
    std::thread::sleep(Duration::from_millis(200));

    ureq::post(&guard.url("/api/v1/issues"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"type":"feature","title":"x"}).to_string())
        .unwrap();
    ureq::post(&guard.url("/api/v1/issues/1/transitions"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"to":"plan"}).to_string())
        .unwrap();

    let events = handle.join().expect("collector thread");
    let names: Vec<&str> = events.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.iter().any(|n| *n == "issue.created"),
        "expected issue.created in {names:?}"
    );
    assert!(
        names.iter().any(|n| *n == "issue.changed"),
        "expected issue.changed in {names:?}"
    );

    let created = events
        .iter()
        .find(|(n, _)| n == "issue.created")
        .unwrap();
    assert_eq!(created.1["id"], 1);
}

#[test]
fn sse_emits_issue_closed_via_transitions_done() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let url = guard.url("/api/v1/events");
    let handle = std::thread::spawn(move || collect_events(&url, 2, Duration::from_secs(5)));
    std::thread::sleep(Duration::from_millis(200));

    ureq::post(&guard.url("/api/v1/issues"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"type":"feature","title":"x"}).to_string())
        .unwrap();
    ureq::post(&guard.url("/api/v1/issues/1/transitions"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"to":"done","comment":"ship"}).to_string())
        .unwrap();

    let events = handle.join().expect("collector thread");
    assert!(
        events.iter().any(|(n, _)| n == "issue.closed"),
        "expected issue.closed in {:?}",
        events.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
}

#[test]
fn sse_emits_dependency_added_and_removed() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    // Create two issues before subscribing so we don't crowd the channel
    // with issue.created events and miss our targets.
    for _ in 0..2 {
        ureq::post(&guard.url("/api/v1/issues"))
            .set("Content-Type", "application/json")
            .send_string(&json!({"type":"feature","title":"x"}).to_string())
            .unwrap();
    }

    let url = guard.url("/api/v1/events");
    let handle = std::thread::spawn(move || collect_events(&url, 2, Duration::from_secs(5)));
    std::thread::sleep(Duration::from_millis(200));

    ureq::post(&guard.url("/api/v1/dependencies"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"from":1,"to":2}).to_string())
        .unwrap();
    ureq::request("DELETE", &guard.url("/api/v1/dependencies/1/2"))
        .call()
        .unwrap();

    let events = handle.join().expect("collector thread");
    let names: Vec<&str> = events.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"dependency.added"),
        "expected dependency.added in {names:?}"
    );
    assert!(
        names.contains(&"dependency.removed"),
        "expected dependency.removed in {names:?}"
    );
}
