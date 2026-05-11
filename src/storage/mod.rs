//! Filesystem-side IO for hub-tracked artifacts.
//!
//! Submodules:
//!
//! - [`atomic`]: write-via-temp+rename helper. Every hub-tracked
//!   write goes through [`write_atomic`]; the file watcher's
//!   correctness depends on it.
//! - [`config`]: [`config::RepoPaths`] (the resolved-paths struct
//!   every other module threads through), the repo-wide advisory
//!   [`config::with_repo_lock`], and counter allocation.
//! - [`issue_file`]: `issue.md` frontmatter + body serialization.
//! - [`comment_file`]: comment file naming + frontmatter.
//!
//! Architecture: [`docs/architecture/storage-layout.md`](../../../docs/architecture/storage-layout.md).
//! Spec: [`docs/specs/storage-model.md`](../../../docs/specs/storage-model.md).

pub mod atomic;
pub mod comment_file;
pub mod config;
pub mod issue_file;

pub use atomic::write_atomic;
