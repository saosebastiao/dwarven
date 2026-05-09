use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use fs2::FileExt;
use toml_edit::{DocumentMut, value};

use super::atomic::write_atomic;

pub struct RepoPaths {
    pub root: PathBuf,
}

impl RepoPaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn dwarven_dir(&self) -> PathBuf {
        self.root.join(".dwarven")
    }

    pub fn config_path(&self) -> PathBuf {
        self.dwarven_dir().join("config.toml")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.dwarven_dir().join(".config.lock")
    }

    pub fn issues_dir(&self) -> PathBuf {
        self.dwarven_dir().join("issues")
    }

    pub fn issue_dir(&self, id: u64) -> PathBuf {
        self.issues_dir().join(format_id(id))
    }

    pub fn issue_md(&self, id: u64) -> PathBuf {
        self.issue_dir(id).join("issue.md")
    }

    pub fn comments_dir(&self, id: u64) -> PathBuf {
        self.issue_dir(id).join("comments")
    }
}

pub fn format_id(id: u64) -> String {
    format!("{id:04}")
}

pub fn require_initialized(paths: &RepoPaths) -> Result<()> {
    if !paths.config_path().exists() {
        return Err(anyhow!(
            "no .dwarven/ found at {}; run `dwarven init` first",
            paths.root.display()
        ));
    }
    Ok(())
}

/// Read–increment–write `counters.next_issue_id` under an OS advisory exclusive
/// lock. Preserves comments and formatting via `toml_edit`.
pub fn allocate_next_issue_id(paths: &RepoPaths) -> Result<u64> {
    require_initialized(paths)?;

    let lock_path = paths.lock_path();
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .with_context(|| format!("opening lock file {}", lock_path.display()))?;
    lock_file
        .lock_exclusive()
        .with_context(|| format!("locking {}", lock_path.display()))?;

    let result = with_locked_counter(paths.config_path().as_path());

    // Lock is released when `lock_file` drops, but unlock explicitly so any
    // error in the result propagates without an awkward order-of-operations.
    let _ = FileExt::unlock(&lock_file);
    result
}

fn with_locked_counter(config_path: &Path) -> Result<u64> {
    let raw = fs::read_to_string(config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let mut doc: DocumentMut = raw
        .parse()
        .with_context(|| format!("parsing {}", config_path.display()))?;

    let counters = doc
        .get_mut("counters")
        .ok_or_else(|| anyhow!("config.toml missing [counters] section"))?
        .as_table_mut()
        .ok_or_else(|| anyhow!("config.toml [counters] is not a table"))?;
    let next = counters
        .get("next_issue_id")
        .and_then(|v| v.as_integer())
        .ok_or_else(|| anyhow!("config.toml [counters].next_issue_id missing or not integer"))?;
    if next < 1 {
        return Err(anyhow!(
            "config.toml [counters].next_issue_id is {next}; must be >= 1"
        ));
    }
    let id = next as u64;
    counters["next_issue_id"] = value(next + 1);

    write_atomic(config_path, doc.to_string().as_bytes())?;
    Ok(id)
}
