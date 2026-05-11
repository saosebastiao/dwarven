//! Scheduler configuration: `α` and per-priority base weights.
//!
//! Loaded from `[scheduler]` in `.dwarven/config.toml`. Missing keys
//! fall back to documented defaults per `coordination-hub.md#R10.3`.
//! All values are re-read per scheduler query, so changes are live
//! (no daemon restart required).

use std::fs;

use anyhow::{Context, Result, anyhow};

use crate::storage::config::RepoPaths;

const DEFAULT_ALPHA: f64 = 0.5;

/// Per-priority base weights for the dep-graph scheduler.
///
/// Invariant (enforced by [`PriorityWeights::validate`]): all weights
/// are strictly positive, and `p0 >= p1 >= p2`. `unset` is unconstrained
/// relative to `p2`.
#[derive(Debug, Clone, Copy)]
pub struct PriorityWeights {
    pub p0: f64,
    pub p1: f64,
    pub p2: f64,
    pub unset: f64,
}

impl PriorityWeights {
    /// Documented defaults: `p0=4, p1=2, p2=1, unset=1`.
    pub fn defaults() -> Self {
        Self {
            p0: 4.0,
            p1: 2.0,
            p2: 1.0,
            unset: 1.0,
        }
    }

    /// Look up the base weight for a priority string. Unknown values
    /// (including `None`) fall through to `unset`.
    pub fn for_priority(&self, p: Option<&str>) -> f64 {
        match p {
            Some("p0") => self.p0,
            Some("p1") => self.p1,
            Some("p2") => self.p2,
            _ => self.unset,
        }
    }

    /// Validate per `coordination-hub.md#R10.6`: weights positive,
    /// `p0 >= p1 >= p2`. (`unset` is unconstrained relative to p2.)
    pub fn validate(&self) -> Result<()> {
        if !(self.p0 > 0.0 && self.p1 > 0.0 && self.p2 > 0.0 && self.unset > 0.0) {
            return Err(anyhow!(
                "scheduler.priority_weights must all be positive (got p0={}, p1={}, p2={}, unset={})",
                self.p0, self.p1, self.p2, self.unset
            ));
        }
        if !(self.p0 >= self.p1 && self.p1 >= self.p2) {
            return Err(anyhow!(
                "scheduler.priority_weights must satisfy p0 >= p1 >= p2 (got p0={}, p1={}, p2={})",
                self.p0, self.p1, self.p2
            ));
        }
        Ok(())
    }
}

/// Full `[scheduler]` config bundle: `α` plus per-priority weights.
#[derive(Debug, Clone, Copy)]
pub struct SchedulerConfig {
    pub alpha: f64,
    pub priority_weights: PriorityWeights,
}

impl SchedulerConfig {
    /// Documented defaults: `alpha = 0.5`, weights from
    /// [`PriorityWeights::defaults`].
    pub fn defaults() -> Self {
        Self {
            alpha: DEFAULT_ALPHA,
            priority_weights: PriorityWeights::defaults(),
        }
    }

    /// Enforce `alpha ∈ [0.0, 1.0]` and the [`PriorityWeights`] invariants.
    /// Called both at daemon startup (single validation gate) and per
    /// scheduler query (so a hot config edit is caught at the next request).
    pub fn validate(&self) -> Result<()> {
        if !(self.alpha >= 0.0 && self.alpha <= 1.0 && self.alpha.is_finite()) {
            return Err(anyhow!(
                "scheduler.alpha must be in [0.0, 1.0] (got {})",
                self.alpha
            ));
        }
        self.priority_weights.validate()?;
        Ok(())
    }
}

/// Read `[scheduler]` from `.dwarven/config.toml`. Missing keys fall back to
/// documented defaults per `coordination-hub.md#R10.3`. The returned
/// config is validated via [`SchedulerConfig::validate`] before return;
/// callers can assume invariants hold.
///
/// Re-read per scheduler query (live config). At config-file size (~30
/// lines) the extra parse cost is negligible.
pub fn read_scheduler_config(paths: &RepoPaths) -> Result<SchedulerConfig> {
    let raw = fs::read_to_string(paths.config_path())
        .with_context(|| format!("reading {}", paths.config_path().display()))?;
    let parsed: toml::Value = raw
        .parse()
        .with_context(|| format!("parsing {}", paths.config_path().display()))?;

    let mut config = SchedulerConfig::defaults();
    let scheduler = parsed.get("scheduler");

    if let Some(s) = scheduler.and_then(|t| t.get("alpha")) {
        config.alpha = match s {
            toml::Value::Float(f) => *f,
            toml::Value::Integer(i) => *i as f64,
            other => {
                return Err(anyhow!(
                    "scheduler.alpha must be a number (got {other})"
                ));
            }
        };
    }

    if let Some(weights) = scheduler.and_then(|t| t.get("priority_weights")) {
        if let Some(w) = read_weight(weights, "p0")? {
            config.priority_weights.p0 = w;
        }
        if let Some(w) = read_weight(weights, "p1")? {
            config.priority_weights.p1 = w;
        }
        if let Some(w) = read_weight(weights, "p2")? {
            config.priority_weights.p2 = w;
        }
        if let Some(w) = read_weight(weights, "unset")? {
            config.priority_weights.unset = w;
        }
    }

    config.validate()?;
    Ok(config)
}

fn read_weight(table: &toml::Value, key: &str) -> Result<Option<f64>> {
    let v = match table.get(key) {
        Some(v) => v,
        None => return Ok(None),
    };
    match v {
        toml::Value::Integer(i) => Ok(Some(*i as f64)),
        toml::Value::Float(f) => Ok(Some(*f)),
        other => Err(anyhow!(
            "scheduler.priority_weights.{key} must be a number (got {other})"
        )),
    }
}
