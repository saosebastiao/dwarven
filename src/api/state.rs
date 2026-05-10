use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::storage::config::RepoPaths;

/// Shared state injected into every HTTP handler via `axum::extract::State`.
#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub paths: RepoPaths,
    pub started_at: Instant,
    /// Flips to true when the daemon should shut down. Owned by the
    /// daemon's lifecycle code in `serve.rs`; HTTP handlers may set it
    /// (POST /daemon/shutdown) but never clear.
    pub term_flag: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(paths: RepoPaths, term_flag: Arc<AtomicBool>) -> Self {
        AppState(Arc::new(AppStateInner {
            paths,
            started_at: Instant::now(),
            term_flag,
        }))
    }
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
