//! Full daemon-side validation of `.dwarven/config.toml` per
//! `coordination-hub.md#R10.6`. Read at `dwarven serve` startup;
//! invalid values are fatal before HTTP bind / watcher start.
//!
//! `[scheduler]` validation already lives in `crate::scheduler::config`
//! (used per-request by the queue endpoint); we re-use that here so the
//! check runs once at startup AND on each query.

use std::fs;

use anyhow::{Context, Result, anyhow};

use crate::scheduler::config::SchedulerConfig;
use crate::storage::config::RepoPaths;

#[derive(Debug, Clone)]
pub struct DaemonSection {
    pub port: u16,
    pub bind: String,
    pub reconciliation_interval_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct TriageSection {
    pub stale_threshold_days: u64,
}

#[derive(Debug, Clone)]
pub struct FullConfig {
    pub daemon: DaemonSection,
    pub scheduler: SchedulerConfig,
    pub triage: TriageSection,
}

const DEFAULT_PORT: u16 = 7777;
const DEFAULT_BIND: &str = "127.0.0.1";
const DEFAULT_RECONCILE_SECS: u64 = 60;
const DEFAULT_STALE_DAYS: u64 = 14;

/// Read + validate the entire config. Missing optional keys fall back to
/// documented defaults (R10.3).
pub fn read_full_config(paths: &RepoPaths) -> Result<FullConfig> {
    let raw = fs::read_to_string(paths.config_path())
        .with_context(|| format!("reading {}", paths.config_path().display()))?;
    let parsed: toml::Value = raw
        .parse()
        .with_context(|| format!("parsing {}", paths.config_path().display()))?;

    let daemon = read_daemon_section(parsed.get("daemon"))?;
    let triage = read_triage_section(parsed.get("triage"))?;
    // SchedulerConfig::validate is invoked by read_scheduler_config too,
    // but that path goes through a separate file read. Call validate
    // directly here so we surface scheduler errors at startup before
    // the queue endpoint is ever exercised.
    let scheduler = crate::scheduler::config::read_scheduler_config(paths)?;

    Ok(FullConfig {
        daemon,
        scheduler,
        triage,
    })
}

fn read_daemon_section(table: Option<&toml::Value>) -> Result<DaemonSection> {
    let mut s = DaemonSection {
        port: DEFAULT_PORT,
        bind: DEFAULT_BIND.to_string(),
        reconciliation_interval_seconds: DEFAULT_RECONCILE_SECS,
    };

    let Some(t) = table else {
        return Ok(s);
    };

    if let Some(p) = t.get("port") {
        let port = p
            .as_integer()
            .ok_or_else(|| anyhow!("daemon.port must be an integer (got {p})"))?;
        if !(1..=65535).contains(&port) {
            return Err(anyhow!(
                "daemon.port {port} out of range [1, 65535]"
            ));
        }
        s.port = port as u16;
    }

    if let Some(b) = t.get("bind") {
        s.bind = b
            .as_str()
            .ok_or_else(|| anyhow!("daemon.bind must be a string (got {b})"))?
            .to_string();
    }

    if let Some(r) = t.get("reconciliation_interval_seconds") {
        let secs = r
            .as_integer()
            .ok_or_else(|| anyhow!("daemon.reconciliation_interval_seconds must be an integer (got {r})"))?;
        if secs <= 0 {
            return Err(anyhow!(
                "daemon.reconciliation_interval_seconds must be a positive integer (got {secs})"
            ));
        }
        s.reconciliation_interval_seconds = secs as u64;
    }

    Ok(s)
}

fn read_triage_section(table: Option<&toml::Value>) -> Result<TriageSection> {
    let mut s = TriageSection {
        stale_threshold_days: DEFAULT_STALE_DAYS,
    };

    let Some(t) = table else {
        return Ok(s);
    };

    if let Some(d) = t.get("stale_threshold_days") {
        let days = d
            .as_integer()
            .ok_or_else(|| anyhow!("triage.stale_threshold_days must be an integer (got {d})"))?;
        if days <= 0 {
            return Err(anyhow!(
                "triage.stale_threshold_days must be a positive integer (got {days})"
            ));
        }
        s.stale_threshold_days = days as u64;
    }

    Ok(s)
}
