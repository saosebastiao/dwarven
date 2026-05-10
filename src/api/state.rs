use std::sync::Arc;
use std::time::Instant;

use crate::storage::config::RepoPaths;

/// Shared state injected into every HTTP handler via `axum::extract::State`.
#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub paths: RepoPaths,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(paths: RepoPaths) -> Self {
        AppState(Arc::new(AppStateInner {
            paths,
            started_at: Instant::now(),
        }))
    }
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
