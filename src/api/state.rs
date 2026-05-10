use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::api::events::EventBus;
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
    /// In-process bus of change events; mutation handlers and the watcher
    /// emit events, the SSE handler subscribes per-connection.
    pub events: EventBus,
}

impl AppState {
    pub fn new(paths: RepoPaths, term_flag: Arc<AtomicBool>, events: EventBus) -> Self {
        AppState(Arc::new(AppStateInner {
            paths,
            started_at: Instant::now(),
            term_flag,
            events,
        }))
    }
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
