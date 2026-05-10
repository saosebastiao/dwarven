use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::daemon::pidfile::{is_alive, read_pid, send_term};
use crate::storage::config::{RepoPaths, require_initialized};

const STOP_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub struct StatusArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
}

pub struct StopArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
}

pub struct RestartArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
}

pub enum Status {
    Running(i32),
    Stale(i32),
    Stopped,
}

pub fn current_status(paths: &RepoPaths) -> Result<Status> {
    let pidfile = paths.dwarven_dir().join(".daemon.pid");
    match read_pid(&pidfile)? {
        None => Ok(Status::Stopped),
        Some(pid) if is_alive(pid) => Ok(Status::Running(pid)),
        Some(pid) => Ok(Status::Stale(pid)),
    }
}

pub fn run_status(args: StatusArgs) -> Result<i32> {
    let paths = RepoPaths::new(args.repo_root.clone());
    require_initialized(&paths)?;
    let status = current_status(&paths)?;

    if args.json {
        let body = match &status {
            Status::Running(pid) => format!("{{\"state\":\"running\",\"pid\":{pid}}}"),
            Status::Stale(pid) => format!("{{\"state\":\"stale\",\"pid\":{pid}}}"),
            Status::Stopped => "{\"state\":\"stopped\"}".to_string(),
        };
        println!("{body}");
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        match &status {
            Status::Running(pid) => println!("dwarven daemon: running (PID {pid})"),
            Status::Stale(pid) => println!(
                "dwarven daemon: stopped (stale PID file recorded {pid})"
            ),
            Status::Stopped => println!("dwarven daemon: stopped"),
        }
    }

    Ok(match status {
        Status::Running(_) => 0,
        Status::Stale(_) | Status::Stopped => 4,
    })
}

pub fn run_stop(args: StopArgs) -> Result<i32> {
    let paths = RepoPaths::new(args.repo_root.clone());
    require_initialized(&paths)?;
    let pidfile = paths.dwarven_dir().join(".daemon.pid");

    let pid = match read_pid(&pidfile)? {
        Some(p) => p,
        None => {
            if !args.quiet && !args.json {
                println!("dwarven daemon: not running");
            }
            return Ok(0);
        }
    };

    if !is_alive(pid) {
        // Stale PID file — remove it.
        let _ = std::fs::remove_file(&pidfile);
        if !args.quiet && !args.json {
            println!("dwarven daemon: stale PID {pid} cleared");
        }
        return Ok(0);
    }

    send_term(pid)?;

    // Poll for pidfile absence (the daemon removes it on graceful shutdown).
    // We deliberately don't poll `is_alive(pid)` — `kill -0` returns true
    // for zombie processes whose parent hasn't reaped them, so under
    // `cargo test` (where the parent of the daemon is the test binary,
    // which doesn't wait()) we'd report a false "still alive" indefinitely.
    let deadline = Instant::now() + STOP_TIMEOUT;
    while Instant::now() < deadline {
        if !pidfile.exists() {
            if !args.quiet && !args.json {
                println!("dwarven daemon: stopped (PID {pid})");
            }
            return Ok(0);
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    if !args.quiet {
        eprintln!(
            "dwarven daemon: PID {pid} did not exit within {:?}",
            STOP_TIMEOUT
        );
    }
    Ok(4)
}

pub fn run_restart(args: RestartArgs) -> Result<i32> {
    // Stop first; ignore "not running" (exit 0). Propagate true failures.
    let stop_code = run_stop(StopArgs {
        repo_root: args.repo_root.clone(),
        quiet: args.quiet,
        json: false,
    })?;
    if stop_code != 0 {
        return Ok(stop_code);
    }

    // Then exec into serve in the foreground. Returns whatever serve returns.
    crate::daemon::serve::run(crate::daemon::serve::ServeArgs {
        repo_root: args.repo_root,
        quiet: args.quiet,
    })?;
    Ok(0)
}
