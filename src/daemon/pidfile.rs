//! PID file + advisory-lock primitives used by [`crate::daemon::serve`]
//! and [`crate::daemon::control`].
//!
//! The PID file lives at `.dwarven/.daemon.pid`. Its presence is the
//! "a daemon owns this repo" signal; its absence (after clean shutdown)
//! is the "no daemon is running" signal. Distinguishing "pidfile
//! present but process gone" (a crashed daemon) from "pidfile present
//! and live" is the job of [`is_pid_alive`] / [`probe`].

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

/// Read the recorded PID from `.dwarven/.daemon.pid`.
/// Returns `Ok(None)` if the file is absent or unparseable.
pub fn read_pid(pidfile: &Path) -> Result<Option<i32>> {
    let raw = match fs::read_to_string(pidfile) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {}", pidfile.display())),
    };
    Ok(raw.trim().parse::<i32>().ok())
}

/// Returns true if the given PID corresponds to a live process. Uses
/// `kill(pid, 0)` which is harmless on Unix.
pub fn is_alive(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }
    matches!(kill(Pid::from_raw(pid), None), Ok(()))
}

/// Send SIGTERM to the given PID. Returns Ok regardless of whether the
/// process was alive — the caller is expected to poll for actual exit.
pub fn send_term(pid: i32) -> Result<()> {
    if pid <= 0 {
        return Ok(());
    }
    let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
    Ok(())
}
