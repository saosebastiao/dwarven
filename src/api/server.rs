use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use axum::Json;
use axum::extract::State;
use axum::routing::get;

use crate::api::error::ApiError;
use crate::api::issues;
use crate::api::state::AppState;
use crate::api::types::DaemonStatus;
use crate::storage::config::RepoPaths;

const SHUTDOWN_TICK: Duration = Duration::from_millis(200);

/// Spawn the HTTP server on a dedicated OS thread with its own tokio
/// runtime. The thread terminates when `term_flag` flips to true.
///
/// Binding happens synchronously on the spawned thread before returning;
/// errors during bind propagate via the returned channel.
pub fn spawn(
    paths: RepoPaths,
    bind: SocketAddr,
    term_flag: Arc<AtomicBool>,
) -> Result<std::thread::JoinHandle<()>> {
    let app_state = AppState::new(paths.clone());

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<()>>();

    let handle = std::thread::Builder::new()
        .name("dwarven-http".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(2)
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = ready_tx.send(Err(anyhow!("building tokio runtime: {e}")));
                    return;
                }
            };
            runtime.block_on(async move {
                let listener = match tokio::net::TcpListener::bind(bind).await {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = ready_tx.send(Err(anyhow!(
                            "binding HTTP listener on {bind}: {e}"
                        )));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));

                let app = router(app_state);
                let term_flag = term_flag.clone();
                let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                    while !term_flag.load(Ordering::Relaxed) {
                        tokio::time::sleep(SHUTDOWN_TICK).await;
                    }
                });

                if let Err(e) = server.await {
                    eprintln!("[daemon] HTTP server error: {e}");
                }
            });
        })
        .with_context(|| "spawning HTTP server thread")?;

    // Block until bind completes (success or error).
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow!("HTTP server thread exited before binding: {e}")),
    }
}

fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route("/api/v1/daemon", get(daemon_status))
        .route("/api/v1/issues", get(issues::list))
        .route("/api/v1/issues/:id", get(issues::view))
        .route(
            "/api/v1/issues/:id/comments",
            get(issues::list_comments_for_issue),
        )
        .with_state(state)
}

async fn daemon_status(State(app): State<AppState>) -> Result<Json<DaemonStatus>, ApiError> {
    let paths = app.paths.clone();
    let port = tokio::task::spawn_blocking(move || crate::api::server::read_port(&paths))
        .await
        .map_err(|e| ApiError::hub_error(format!("task panic: {e}")))?
        .unwrap_or(7777);
    Ok(Json(DaemonStatus {
        state: "running".to_string(),
        pid: std::process::id(),
        port,
        uptime_seconds: app.started_at.elapsed().as_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

pub fn read_port(paths: &RepoPaths) -> Result<u16> {
    let raw = std::fs::read_to_string(paths.config_path())?;
    let doc: toml_edit::DocumentMut = raw.parse()?;
    let port = doc
        .get("daemon")
        .and_then(|t| t.as_table())
        .and_then(|t| t.get("port"))
        .and_then(|i| i.as_integer())
        .ok_or_else(|| anyhow!("daemon.port missing or not integer"))?;
    Ok(port as u16)
}
