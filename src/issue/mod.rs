//! Per-CLI-verb business logic for issue mutations.
//!
//! One module per `dwarven issue <verb>` subcommand. Each module
//! exposes:
//!
//! - An `Args` struct (or several) capturing the validated inputs.
//! - A `run(args)` function that performs the mutation.
//!
//! These same functions are called from the HTTP API handlers in
//! `src/api/mutations.rs`; the HTTP layer is extractors-translate-call,
//! not a re-implementation. This is the spec invariant that "the CLI
//! and the web API can never drift" — they share code.
//!
//! Almost every mutation takes the repo-wide advisory lock via
//! [`crate::storage::config::with_repo_lock`]. Writes go through
//! [`crate::storage::atomic::write_atomic`].
//!
//! Architecture: [`docs/architecture/cli-vs-daemon.md`](../../../docs/architecture/cli-vs-daemon.md).
//! CLI flag reference: [`docs/cli-reference.md`](../../../docs/cli-reference.md).

pub mod blocker;
pub mod close;
pub mod comment;
pub mod create;
pub mod dep;
pub mod edit;
pub mod list;
pub mod priority;
pub mod priority_override;
pub mod transition;
pub mod view;
