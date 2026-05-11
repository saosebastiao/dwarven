//! Dependency-graph scheduler.
//!
//! Computes a ranked queue of active issues by combining each issue's
//! base priority weight with the discounted score of every issue it
//! blocks:
//!
//! ```text
//! score(i) = base_priority(i) + α · Σ_{j ∈ blocks(i)} score(j)
//! ```
//!
//! `base_priority(i)` maps the `priority` field to a number via
//! [`config::PriorityWeights`] (defaults p0=4, p1=2, p2=1, unset=1).
//! `α` is `scheduler.alpha` (default 0.5). Higher score ranks higher.
//!
//! The maintainer can override the algorithmic ranking per-issue via
//! `effective_priority_override` on the issue frontmatter;
//! [`compute::ScheduledIssue::effective_priority`] uses the override
//! when present, falling back to the algorithmic `score`.
//!
//! Spec: `docs/specs/dep-graph.md`.
//! Architecture: `docs/architecture/scheduler.md`.

// The HTTP scheduler API and `dwarven schedule` CLI consume these in
// slice 21; silence dead-code spam in the meantime.
#![allow(dead_code, unused_imports)]

pub mod compute;
pub mod config;

pub use compute::{ScheduledIssue, compute};
pub use config::{PriorityWeights, SchedulerConfig, read_scheduler_config};
