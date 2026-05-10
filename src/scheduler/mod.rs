// The HTTP scheduler API and `dwarven schedule` CLI consume these in
// slice 21; silence dead-code spam in the meantime.
#![allow(dead_code, unused_imports)]

pub mod compute;
pub mod config;

pub use compute::{ScheduledIssue, compute};
pub use config::{PriorityWeights, SchedulerConfig, read_scheduler_config};
