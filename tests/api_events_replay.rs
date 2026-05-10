//! Issue #3: Last-Event-ID replay on SSE reconnect.
//! Tests web-api.md#R5.6 + R5.7.

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

static NEXT_PORT: AtomicU16 = AtomicU16::new(26000);

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
        panic!("daemon HTTP did not become ready");
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

#[derive(Debug)]
struct SseEvent {
    id: Option<String>,
    name: String,
    data: Value,
}

/// Read SSE events from `url` (with optional Last-Event-ID header), stopping
/// after `target` events or `timeout`. Returns events in arrival order.
fn read_sse(url: &str, last_id: Option<&str>, target: usize, timeout: Duration) -> Vec<SseEvent> {
    let mut req = ureq::get(url)
        .set("Accept", "text/event-stream")
        .timeout(timeout);
    if let Some(id) = last_id {
        req = req.set("Last-Event-ID", id);
    }
    let resp = req.call().expect("SSE connect");
    let reader = BufReader::new(resp.into_reader());
    let deadline = Instant::now() + timeout;

    let mut events = Vec::new();
    let mut cur_id: Option<String> = None;
    let mut cur_event: Option<String> = None;

    for line in reader.lines() {
        if Instant::now() > deadline {
            break;
        }
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(rest) = line.strip_prefix("id: ") {
            cur_id = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("event: ") {
            cur_event = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("data: ") {
            if let (Some(name), Ok(data)) =
                (cur_event.take(), serde_json::from_str::<Value>(rest))
            {
                events.push(SseEvent {
                    id: cur_id.take(),
                    name,
                    data,
                });
                if events.len() >= target {
                    return events;
                }
            }
        }
    }
    events
}

fn create_issue(guard: &DaemonGuard, title: &str) {
    ureq::post(&guard.url("/api/v1/issues"))
        .set("Content-Type", "application/json")
        .send_string(&json!({"type": "feature", "title": title}).to_string())
        .unwrap();
}

#[test]
fn events_carry_monotonic_id() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    let url = guard.url("/api/v1/events");
    let handle = std::thread::spawn(move || read_sse(&url, None, 2, Duration::from_secs(5)));
    std::thread::sleep(Duration::from_millis(200));

    create_issue(&guard, "first");
    create_issue(&guard, "second");

    let events = handle.join().unwrap();
    assert!(events.len() >= 2, "got {} events", events.len());
    let ids: Vec<u64> = events
        .iter()
        .filter_map(|e| e.id.as_deref().and_then(|s| s.parse().ok()))
        .collect();
    assert_eq!(
        ids.len(),
        events.len(),
        "every event must carry an id: line; got {} ids for {} events ({events:?})",
        ids.len(),
        events.len()
    );
    assert!(
        ids.windows(2).all(|w| w[1] > w[0]),
        "ids must be monotonically increasing: {ids:?}"
    );
}

#[test]
fn replay_emits_missed_events() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    // Connect, capture the first event's id, then disconnect.
    let url = guard.url("/api/v1/events");
    let url_clone = url.clone();
    let handle = std::thread::spawn(move || read_sse(&url_clone, None, 1, Duration::from_secs(3)));
    std::thread::sleep(Duration::from_millis(200));
    create_issue(&guard, "before-disconnect");
    let initial = handle.join().unwrap();
    let last_id = initial[0].id.clone().expect("event has id");

    // Two more events while we're not subscribed.
    create_issue(&guard, "while-disconnected-1");
    create_issue(&guard, "while-disconnected-2");

    // Reconnect with Last-Event-ID set; expect the two missed events.
    let url_clone = url.clone();
    let last_id_clone = last_id.clone();
    let replay = std::thread::spawn(move || {
        read_sse(&url_clone, Some(&last_id_clone), 2, Duration::from_secs(3))
    })
    .join()
    .unwrap();

    assert!(
        replay.len() >= 2,
        "expected >=2 events, got {}: {:?}",
        replay.len(),
        replay
    );
    let initial_id: u64 = last_id.parse().unwrap();
    for ev in &replay[..2] {
        let ev_id: u64 = ev.id.as_deref().unwrap().parse().unwrap();
        assert!(
            ev_id > initial_id,
            "replay event id {} should be > initial {}",
            ev_id,
            initial_id
        );
    }
}

#[test]
fn replay_refresh_required_when_id_too_old() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    // Emit way more events than the ring capacity (256). Each issue create
    // emits at least 2 (issue.created + daemon.reindexed via watcher), so
    // 200 issues → ~400 events well past the buffer.
    for i in 0..200 {
        create_issue(&guard, &format!("evict-{i}"));
    }

    // Reconnect with Last-Event-ID = 1 (way older than buffer's oldest).
    let url = guard.url("/api/v1/events");
    let events = read_sse(&url, Some("1"), 1, Duration::from_secs(3));
    assert!(
        !events.is_empty(),
        "expected at least one event after reconnect"
    );
    assert_eq!(
        events[0].name, "stream.refresh-required",
        "first event should signal refresh required, got: {:?}",
        events[0]
    );
}

#[test]
fn no_last_event_id_streams_live_only() {
    let tmp = fresh_repo();
    let guard = DaemonGuard::spawn(tmp.path());
    guard.wait_ready();

    // Emit before subscribing.
    create_issue(&guard, "before-connect");

    // Connect without Last-Event-ID. The pre-existing event should NOT be
    // replayed; only events emitted after we connect should arrive.
    let url = guard.url("/api/v1/events");
    let url_clone = url.clone();
    let handle = std::thread::spawn(move || read_sse(&url_clone, None, 1, Duration::from_secs(3)));
    std::thread::sleep(Duration::from_millis(300));

    create_issue(&guard, "after-connect");

    let events = handle.join().unwrap();
    assert!(events.len() >= 1, "got no events");
    let first = &events[0];
    let payload = first.data.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
    // The first issue had id=1; after-connect issue has id=2. The fresh
    // subscriber should receive only id=2 events.
    assert!(
        payload != 1 || first.name != "issue.created",
        "should not have replayed the before-connect issue.created (got {first:?})"
    );
}
