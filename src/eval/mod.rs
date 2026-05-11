//! Agent-prompt eval framework. Spec: `docs/architecture/agent-eval.md`.
//!
//! This module is consumed by `examples/eval_runner.rs`. It is not used
//! by the production daemon; nothing here ships in the `dwarven` binary
//! beyond what `cargo build` chooses to include based on reachability.

#![allow(dead_code, unused_imports)]

pub mod matcher;
pub mod mock_tools;
pub mod scenario;

pub use matcher::{ToolInvocation, forbidden_call_violations, pattern_matches, required_call_satisfied};
pub use scenario::{JudgeRubric, Scenario, ToolPattern};
