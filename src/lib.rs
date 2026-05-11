//! Library facade for the `dwarven` binary's modules. Allows
//! `examples/`, integration tests, and external tooling to import the
//! same modules the bin uses. Module declarations are duplicated
//! between this file and `src/main.rs`; cargo compiles them under
//! both lib and bin targets — that's a known cost of the no-refactor
//! library extraction.

pub mod adapter;
pub mod api;
pub mod config;
pub mod daemon;
pub mod eval;
pub mod index;
pub mod init;
pub mod issue;
pub mod schedule_cli;
pub mod scheduler;
pub mod storage;
pub mod time;
