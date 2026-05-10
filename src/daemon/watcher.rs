use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::{Event, EventKind as FsEventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::api::events::{EventKind, EventTx, emit};
use crate::index;
use crate::index::IndexHealth;
use crate::storage::config::RepoPaths;

/// How long after an event burst we wait before reindexing. Coalesces
/// rapid bursts (e.g., a `dwarven issue create` writes issue.md +
/// comments/<seq>.md back-to-back).
const DEBOUNCE: Duration = Duration::from_millis(150);

/// Tick cadence for the run loop. Bounds latency between events / signals
/// and the daemon noticing them.
const TICK: Duration = Duration::from_millis(100);

/// Default periodic reconciliation cadence (seconds). Read from
/// `daemon.reconciliation_interval_seconds` in `config.toml`; falls back
/// here per `coordination-hub.md#R6.4`.
const DEFAULT_RECONCILE_SECS: u64 = 60;

/// Run the daemon's watch + reindex loop. Returns once `term_flag` is set.
/// Performs an initial reindex on entry per `coordination-hub.md#R3.3` step 5.
pub fn run(paths: &RepoPaths, term_flag: Arc<AtomicBool>, events: EventTx) -> Result<()> {
    let (tx, rx) = channel::<()>();
    let watcher = spawn_watcher(paths, tx.clone())?;
    let _watcher = watcher; // keep alive; drop on return tears it down

    let reconcile_interval = read_reconcile_interval(paths);

    // Probe the existing index before we overwrite it (R7.3 + R7.4). The
    // unconditional rebuild that follows is preserved per R3.3 step 5;
    // the probe just makes the failure modes legible.
    match index::probe_health(&paths.index_path()) {
        Ok(IndexHealth::Missing) => {
            eprintln!("[daemon] no existing index; building fresh");
        }
        Ok(IndexHealth::Ok) => {
            eprintln!(
                "[daemon] existing index passed integrity + version checks; rebuilding per R3.3 step 5"
            );
        }
        Ok(IndexHealth::Corrupt(reason)) => {
            eprintln!("[daemon] existing index corrupt ({reason}); rebuilding");
        }
        Ok(IndexHealth::VersionMismatch { found, expected }) => {
            eprintln!(
                "[daemon] existing index schema version mismatch (found={found}, expected={expected}); rebuilding"
            );
        }
        Err(e) => {
            eprintln!("[daemon] index health probe failed: {e:#}; rebuilding anyway");
        }
    }

    // Initial reindex (R3.3 step 5).
    let stats = index::rebuild(paths)?;
    eprintln!(
        "[daemon] initial reindex: {} issues, {} comments, {} edges",
        stats.issues, stats.comments, stats.edges
    );
    emit(
        &events,
        EventKind::DaemonReindexed,
        serde_json::json!({
            "reason": "startup",
            "issues": stats.issues,
            "comments": stats.comments,
            "edges": stats.edges,
        }),
    );

    let mut last_reindex = Instant::now();
    let mut pending_since: Option<Instant> = None;

    while !term_flag.load(Ordering::Relaxed) {
        // Drain any pending events without blocking past TICK.
        match rx.recv_timeout(TICK) {
            Ok(()) => {
                pending_since = Some(Instant::now());
                while rx.try_recv().is_ok() {}
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                // Watcher thread died; degrade to pure reconciliation.
                eprintln!("[daemon] watcher channel disconnected; running on reconciliation only");
                drain_disconnected_until_term(
                    &term_flag,
                    &mut last_reindex,
                    paths,
                    reconcile_interval,
                    &events,
                );
                return Ok(());
            }
        }

        // Debounced reindex from event bursts.
        if let Some(t) = pending_since {
            if t.elapsed() >= DEBOUNCE {
                reindex_log(paths, &mut last_reindex, "events", &events);
                pending_since = None;
            }
        }

        // Periodic reconciliation (R6.4) acts as a safety net for missed events.
        if last_reindex.elapsed() >= reconcile_interval {
            reindex_log(paths, &mut last_reindex, "reconcile", &events);
        }
    }

    Ok(())
}

fn spawn_watcher(
    paths: &RepoPaths,
    tx: std::sync::mpsc::Sender<()>,
) -> Result<RecommendedWatcher> {
    let issues_dir = paths.issues_dir();
    let config_path = paths.config_path();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let Ok(event) = res else { return };
        if !is_relevant(&event) {
            return;
        }
        let _ = tx.send(());
    })
    .with_context(|| "creating filesystem watcher")?;

    watcher
        .watch(&issues_dir, RecursiveMode::Recursive)
        .with_context(|| format!("watching {}", issues_dir.display()))?;

    // Also watch config.toml so changes to scheduler weights / triage
    // thresholds prompt a reconciliation; don't watch the whole .dwarven/
    // root because .index.sqlite (we own) and .config.lock would generate
    // noise.
    if config_path.exists() {
        watcher
            .watch(&config_path, RecursiveMode::NonRecursive)
            .with_context(|| format!("watching {}", config_path.display()))?;
    }

    Ok(watcher)
}

fn is_relevant(event: &Event) -> bool {
    // Skip events from our own internal artifacts.
    if event
        .paths
        .iter()
        .any(|p| is_internal_artifact(p))
    {
        return false;
    }
    matches!(
        event.kind,
        FsEventKind::Create(_) | FsEventKind::Modify(_) | FsEventKind::Remove(_)
    )
}

fn is_internal_artifact(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with(".index.sqlite")
        || name == ".daemon.pid"
        || name == ".config.lock"
        || name.starts_with('.') && name.contains(".tmp.")
}

fn reindex_log(paths: &RepoPaths, last_reindex: &mut Instant, reason: &str, events: &EventTx) {
    match index::rebuild(paths) {
        Ok(stats) => {
            *last_reindex = Instant::now();
            eprintln!(
                "[daemon] reindex ({reason}): {} issues, {} comments, {} edges",
                stats.issues, stats.comments, stats.edges
            );
            emit(
                events,
                EventKind::DaemonReindexed,
                serde_json::json!({
                    "reason": reason,
                    "issues": stats.issues,
                    "comments": stats.comments,
                    "edges": stats.edges,
                }),
            );
        }
        Err(e) => {
            eprintln!("[daemon] reindex failed ({reason}): {e:#}");
        }
    }
}

fn read_reconcile_interval(paths: &RepoPaths) -> Duration {
    let raw = match std::fs::read_to_string(paths.config_path()) {
        Ok(r) => r,
        Err(_) => return Duration::from_secs(DEFAULT_RECONCILE_SECS),
    };
    let doc: toml_edit::DocumentMut = match raw.parse() {
        Ok(d) => d,
        Err(_) => return Duration::from_secs(DEFAULT_RECONCILE_SECS),
    };
    let secs = doc
        .get("daemon")
        .and_then(|t| t.as_table())
        .and_then(|t| t.get("reconciliation_interval_seconds"))
        .and_then(|i| i.as_integer())
        .unwrap_or(DEFAULT_RECONCILE_SECS as i64);
    if secs <= 0 {
        return Duration::from_secs(DEFAULT_RECONCILE_SECS);
    }
    Duration::from_secs(secs as u64)
}

fn drain_disconnected_until_term(
    term_flag: &AtomicBool,
    last_reindex: &mut Instant,
    paths: &RepoPaths,
    reconcile_interval: Duration,
    events: &EventTx,
) {
    while !term_flag.load(Ordering::Relaxed) {
        std::thread::sleep(TICK);
        if last_reindex.elapsed() >= reconcile_interval {
            reindex_log(paths, last_reindex, "reconcile-fallback", events);
        }
    }
}

