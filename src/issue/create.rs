use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::storage::comment_file::{CommentFile, CommentFrontmatter, comment_path, write_comment};
use crate::storage::config::{RepoPaths, allocate_next_issue_id};
use crate::storage::issue_file::{IssueFile, IssueFrontmatter, read_issue, write_issue};
use crate::time::{iso_filename, iso_frontmatter, now_utc};

const TYPES: &[&str] = &["spec-gap", "feature", "bug", "arch", "doc", "chore"];

const ACTIVE_STATES: &[&str] = &[
    "spec",
    "architect",
    "pm",
    "plan",
    "test",
    "implement",
    "review",
    "doc",
    "maintainer",
];

const PRIORITIES: &[&str] = &["p0", "p1", "p2"];

const TITLE_MAX: usize = 120;

pub struct CreateArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,

    pub issue_type: String,
    pub title: String,
    pub body: BodyInput,
    pub state: Option<String>,
    pub priority: Option<String>,
    pub blocked_by: Vec<u64>,
    pub blocks: Vec<u64>,
    pub epic: Option<String>,
}

pub enum BodyInput {
    None,
    Inline(String),
    File(PathBuf),
    Stdin,
}

/// Top-level entry. Returns `(exit_code, ())` via Result; user errors exit 1,
/// not-found exits 3, hub errors exit 2. Errors are mapped in main.rs.
pub fn run(args: CreateArgs) -> Result<()> {
    validate_value("type", &args.issue_type, TYPES)?;
    if let Some(s) = &args.state {
        validate_value("state", s, ACTIVE_STATES)?;
    }
    if let Some(p) = &args.priority {
        validate_value("priority", p, PRIORITIES)?;
    }
    validate_title(&args.title)?;
    if let Some(epic) = &args.epic {
        validate_epic(epic)?;
    }

    let body = read_body(&args.body)?;

    let paths = RepoPaths::new(args.repo_root.clone());

    // Verify referenced ids exist before allocating our own; on missing,
    // exit 3 (not found) without consuming an id.
    verify_existing(&paths, &args.blocked_by, "blocked-by")?;
    verify_existing(&paths, &args.blocks, "blocks")?;

    let final_state = args
        .state
        .clone()
        .unwrap_or_else(|| default_state_for_type(&args.issue_type).to_string());

    let id = allocate_next_issue_id(&paths)?;

    let now = now_utc();
    let created_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);

    let comments_dir = paths.comments_dir(id);
    fs::create_dir_all(&comments_dir)
        .with_context(|| format!("creating {}", comments_dir.display()))?;

    let mut blocked_by_sorted = args.blocked_by.clone();
    blocked_by_sorted.sort_unstable();
    let mut blocks_sorted = args.blocks.clone();
    blocks_sorted.sort_unstable();

    let issue_fm = IssueFrontmatter {
        id,
        title: args.title.clone(),
        issue_type: args.issue_type.clone(),
        state: final_state.clone(),
        priority: args.priority.clone(),
        blocked_by: blocked_by_sorted.clone(),
        blocks: blocks_sorted.clone(),
        epic: args.epic.clone(),
        created: created_iso.clone(),
        created_by: args.actor.clone(),
        updated: created_iso.clone(),
    };
    write_issue(
        &paths.issue_md(id),
        &IssueFile {
            frontmatter: issue_fm,
            body,
        },
    )?;

    let comment_fm = CommentFrontmatter {
        seq: 1,
        issue: id,
        author: args.actor.clone(),
        kind: "state-change".to_string(),
        created: created_iso.clone(),
        from: Some("created".to_string()),
        to: Some(final_state.clone()),
        blocker: None,
    };
    let comment_p = comment_path(&comments_dir, 1, &filename_iso, &args.actor);
    write_comment(
        &comment_p,
        &CommentFile {
            frontmatter: comment_fm,
            body: "Issue created.".to_string(),
        },
    )?;

    // Reciprocal-edge writes. Per R6.5 these are not transactional with the
    // primary writes above; the hub heals asymmetric edges on reindex.
    for other_id in &blocks_sorted {
        update_reciprocal_edge(&paths, *other_id, id, EdgeKind::AddBlockedBy, &created_iso)?;
    }
    for other_id in &blocked_by_sorted {
        update_reciprocal_edge(&paths, *other_id, id, EdgeKind::AddBlocks, &created_iso)?;
    }

    if args.json {
        println!(
            "{{\"id\":{id},\"state\":\"{final_state}\",\"type\":\"{ty}\"}}",
            ty = args.issue_type
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Created issue #{id}: {title}", id = id, title = args.title);
        println!("  state: {final_state}");
        println!("  path:  {}", paths.issue_md(id).display());
    }

    Ok(())
}

fn validate_value(field: &str, got: &str, allowed: &[&str]) -> Result<()> {
    if allowed.iter().any(|a| *a == got) {
        Ok(())
    } else {
        Err(UserError(format!(
            "invalid {field} '{got}'; expected one of: {}",
            allowed.join(", ")
        ))
        .into())
    }
}

fn validate_title(title: &str) -> Result<()> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(UserError("title must not be empty".into()).into());
    }
    if title.len() > TITLE_MAX {
        return Err(UserError(format!(
            "title length {} exceeds {TITLE_MAX} bytes",
            title.len()
        ))
        .into());
    }
    Ok(())
}

fn validate_epic(epic: &str) -> Result<()> {
    if epic.is_empty() {
        return Err(UserError("--epic must not be empty".into()).into());
    }
    let valid = epic
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    let no_edge_dash = !epic.starts_with('-') && !epic.ends_with('-');
    let no_double_dash = !epic.contains("--");
    if !(valid && no_edge_dash && no_double_dash) {
        return Err(UserError(format!(
            "--epic '{epic}' must be a kebab slug: [a-z0-9]+(-[a-z0-9]+)*"
        ))
        .into());
    }
    Ok(())
}

pub fn read_body(input: &BodyInput) -> Result<String> {
    match input {
        BodyInput::None => Ok(String::new()),
        BodyInput::Inline(s) => Ok(s.clone()),
        BodyInput::File(p) => fs::read_to_string(p)
            .with_context(|| format!("reading body from {}", p.display())),
        BodyInput::Stdin => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .context("reading body from stdin")?;
            Ok(buf)
        }
    }
}

fn verify_existing(paths: &RepoPaths, ids: &[u64], flag: &str) -> Result<()> {
    for id in ids {
        let p = paths.issue_md(*id);
        if !p.exists() {
            return Err(NotFoundError(format!(
                "--{flag} references issue {id} which does not exist at {}",
                p.display()
            ))
            .into());
        }
    }
    Ok(())
}

fn default_state_for_type(ty: &str) -> &'static str {
    // work-states.md#R6.1
    match ty {
        "spec-gap" => "pm",
        "feature" => "pm",
        "bug" => "plan",
        "arch" => "architect",
        "chore" => "plan",
        "doc" => "doc",
        _ => unreachable!("type validated upstream"),
    }
}

enum EdgeKind {
    AddBlockedBy,
    AddBlocks,
}

fn update_reciprocal_edge(
    paths: &RepoPaths,
    other_id: u64,
    new_id: u64,
    kind: EdgeKind,
    updated_iso: &str,
) -> Result<()> {
    let path = paths.issue_md(other_id);
    let mut file = read_issue(&path)?;
    match kind {
        EdgeKind::AddBlockedBy => insert_sorted_unique(&mut file.frontmatter.blocked_by, new_id),
        EdgeKind::AddBlocks => insert_sorted_unique(&mut file.frontmatter.blocks, new_id),
    }
    file.frontmatter.updated = updated_iso.to_string();
    write_issue(&path, &file)
}

fn insert_sorted_unique(v: &mut Vec<u64>, x: u64) {
    if !v.contains(&x) {
        v.push(x);
        v.sort_unstable();
    }
}

// ---- Error types for exit-code mapping ----

#[derive(Debug)]
pub struct UserError(pub String);
impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for UserError {}

#[derive(Debug)]
pub struct NotFoundError(pub String);
impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for NotFoundError {}

pub fn classify_exit_code(err: &anyhow::Error) -> i32 {
    if err.downcast_ref::<UserError>().is_some() {
        1
    } else if err.downcast_ref::<NotFoundError>().is_some() {
        3
    } else {
        2
    }
}

