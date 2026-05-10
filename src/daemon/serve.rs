use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use anyhow::{Context, Result, anyhow};
use fs2::FileExt;

use crate::daemon::pidfile::read_pid;
use crate::issue::create::UserError;
use crate::storage::config::{RepoPaths, require_initialized};

pub struct ServeArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
}

pub fn run(args: ServeArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    require_initialized(&paths)?;

    let pidfile_path = paths.dwarven_dir().join(".daemon.pid");

    // Open the pidfile and try to take an exclusive lock without blocking.
    // Per `coordination-hub.md#R3.3` step 2, this is the daemon's enforcement
    // of single-instance-per-repo (R4.4). If the lock is held, another
    // daemon is alive — fail. If the file is stale (no live PID), we
    // reclaim it (R3.5).
    // Validate the full config (R10.6) before any side effects. This
    // surfaces malformed daemon.port / scheduler.alpha / etc. as a clean
    // exit-1 instead of a runtime crash later in startup.
    let _full_config = crate::daemon::config::read_full_config(&paths)
        .map_err(|e| UserError(format!("config validation failed: {e:#}")))?;

    let pid_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&pidfile_path)
        .with_context(|| format!("opening {}", pidfile_path.display()))?;

    if let Err(_) = pid_file.try_lock_exclusive() {
        // Another process holds the lock. Read the recorded PID for the error.
        let other = read_pid(&pidfile_path)?.unwrap_or(0);
        return Err(UserError(format!(
            "another daemon is already running for this repo (PID {other}); \
             stop it with `dwarven daemon stop` first"
        ))
        .into());
    }

    // Register signal handlers BEFORE publishing the PID. Once the PID file
    // is visible, a `dwarven daemon stop` may immediately send SIGTERM; if
    // signals haven't been wired yet, the default disposition would
    // terminate the process without our cleanup running.
    //
    // We use signal-hook's iterator pattern (a dedicated signal-handling
    // thread that consumes SIGINT/SIGTERM/SIGHUP via a synchronous wait)
    // rather than the flag-only pattern. The iterator approach is robust
    // against inherited signal masks (e.g., from `cargo test`'s harness)
    // that the flag-only pattern is not.
    let term_flag = Arc::new(AtomicBool::new(false));
    spawn_signal_thread(Arc::clone(&term_flag))?;

    // We hold the lock. Write our PID, truncating any stale content (R3.5).
    write_pid(&pid_file, std::process::id() as i32)?;

    let port = read_port(&paths).unwrap_or(7777);
    let bind: std::net::SocketAddr = format!("127.0.0.1:{port}").parse()?;

    // Spawn the HTTP server first so a bind failure (e.g., port in use)
    // surfaces before we publish anything else. The HTTP thread terminates
    // when term_flag flips.
    let events = crate::api::events::EventBus::new();
    let http_handle = crate::api::server::spawn(
        paths.clone(),
        bind,
        Arc::clone(&term_flag),
        events.clone(),
    )
    .with_context(|| "starting HTTP server")?;

    if !args.quiet {
        println!("dwarven daemon started (PID {})", std::process::id());
        println!("http://127.0.0.1:{port}");
        println!("press Ctrl-C or send SIGTERM to stop");
    }

    // Run the file watcher + reindex loop on this thread. Returns once
    // term_flag is set (signal received).
    crate::daemon::watcher::run(&paths, Arc::clone(&term_flag), events)?;

    // Wait for the HTTP server thread to drain.
    let _ = http_handle.join();

    if !args.quiet {
        println!("dwarven daemon stopping...");
    }

    // Best-effort cleanup. Lock is released when `pid_file` drops; remove
    // the visible PID file so `dwarven daemon status` reports stopped.
    let _ = FileExt::unlock(&pid_file);
    let _ = fs::remove_file(&pidfile_path);
    Ok(())
}

fn write_pid(mut file: &std::fs::File, pid: i32) -> Result<()> {
    use std::io::Seek;
    file.set_len(0)?;
    file.seek(std::io::SeekFrom::Start(0))?;
    writeln!(file, "{pid}")?;
    file.sync_all()?;
    Ok(())
}

fn spawn_signal_thread(flag: Arc<AtomicBool>) -> Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;

    // Unblock our termination signals at the process level. cargo test's
    // harness masks SIGTERM in spawned children; without this the iterator
    // would never see the signal because the kernel never delivers it.
    unblock_signals(&[SIGTERM, SIGINT, SIGHUP])?;

    let mut signals = Signals::new([SIGTERM, SIGINT, SIGHUP])
        .with_context(|| "registering signal iterator")?;
    std::thread::Builder::new()
        .name("dwarven-signal".into())
        .spawn(move || {
            for _ in &mut signals {
                flag.store(true, Ordering::Relaxed);
                break;
            }
        })
        .with_context(|| "spawning signal-handling thread")?;
    Ok(())
}

fn unblock_signals(signals: &[i32]) -> Result<()> {
    use nix::sys::signal::{SigSet, SigmaskHow, Signal, sigprocmask};
    let mut set = SigSet::empty();
    for s in signals {
        let sig = Signal::try_from(*s)
            .map_err(|e| anyhow!("invalid signal {s}: {e}"))?;
        set.add(sig);
    }
    sigprocmask(SigmaskHow::SIG_UNBLOCK, Some(&set), None)
        .with_context(|| "unblocking termination signals")?;
    Ok(())
}

fn read_port(paths: &RepoPaths) -> Result<u16> {
    let raw = std::fs::read_to_string(paths.config_path())?;
    let doc: toml_edit::DocumentMut = raw.parse()?;
    let port = doc
        .get("daemon")
        .and_then(|t| t.as_table())
        .and_then(|t| t.get("port"))
        .and_then(|i| i.as_integer())
        .ok_or_else(|| anyhow!("daemon.port missing or not integer"))?;
    if !(1..=65535).contains(&port) {
        return Err(anyhow!("daemon.port {port} out of range"));
    }
    Ok(port as u16)
}

