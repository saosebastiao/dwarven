use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{BodyInput, NotFoundError, UserError, read_body};
use crate::storage::comment_file::{
    CommentFile, CommentFrontmatter, comment_path, next_comment_seq, write_comment,
};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_filename, iso_frontmatter, now_utc};

pub struct AddArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,
    pub from_id: u64,
    pub to_id: u64,
    pub rationale: BodyInput,
}

pub struct RemoveArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub from_id: u64,
    pub to_id: u64,
}

pub fn run_add(args: AddArgs) -> Result<()> {
    if args.from_id == args.to_id {
        return Err(UserError(format!(
            "self-edge not permitted: issue #{} cannot block itself",
            args.from_id
        ))
        .into());
    }

    let paths = RepoPaths::new(args.repo_root.clone());
    require_exists(&paths, args.from_id)?;
    require_exists(&paths, args.to_id)?;

    let rationale = if matches!(args.rationale, BodyInput::None) {
        None
    } else {
        Some(read_body(&args.rationale)?)
    };

    let now = now_utc();
    let now_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);

    with_repo_lock(&paths, || {
        // Cycle check: would adding from → to create a cycle? Yes iff there
        // is already a path to → ... → from in the blocks graph.
        if path_exists(&paths, args.to_id, args.from_id)? {
            return Err(UserError(format!(
                "adding edge #{} blocks #{} would create a cycle in the dep graph",
                args.from_id, args.to_id
            ))
            .into());
        }

        let mut from = read_issue(&paths.issue_md(args.from_id))?;
        let mut to = read_issue(&paths.issue_md(args.to_id))?;

        let added_from = insert_sorted_unique(&mut from.frontmatter.blocks, args.to_id);
        let added_to = insert_sorted_unique(&mut to.frontmatter.blocked_by, args.from_id);
        if !added_from && !added_to {
            return Err(UserError(format!(
                "edge #{} blocks #{} already present",
                args.from_id, args.to_id
            ))
            .into());
        }

        from.frontmatter.updated = now_iso.clone();
        to.frontmatter.updated = now_iso.clone();
        write_issue(&paths.issue_md(args.from_id), &from)?;
        write_issue(&paths.issue_md(args.to_id), &to)?;

        if let Some(body) = &rationale {
            let comments_dir = paths.comments_dir(args.from_id);
            let seq = next_comment_seq(&comments_dir)?;
            let fm = CommentFrontmatter {
                seq,
                issue: args.from_id,
                author: args.actor.clone(),
                kind: "comment".to_string(),
                created: now_iso.clone(),
                from: None,
                to: None,
                blocker: None,
            };
            let bound = format!("Dep added: #{} blocks #{}.\n\n{body}", args.from_id, args.to_id);
            write_comment(
                &comment_path(&comments_dir, seq, &filename_iso, &args.actor),
                &CommentFile {
                    frontmatter: fm,
                    body: bound,
                },
            )?;
        }

        Ok(())
    })?;

    if args.json {
        println!(
            "{{\"from\":{},\"to\":{},\"ok\":true}}",
            args.from_id, args.to_id
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Added edge: #{} blocks #{}", args.from_id, args.to_id);
    }
    Ok(())
}

pub fn run_remove(args: RemoveArgs) -> Result<()> {
    if args.from_id == args.to_id {
        return Err(UserError("self-edge cannot exist".into()).into());
    }

    let paths = RepoPaths::new(args.repo_root.clone());
    require_exists(&paths, args.from_id)?;
    require_exists(&paths, args.to_id)?;

    let now_iso = iso_frontmatter(&now_utc());

    with_repo_lock(&paths, || {
        let mut from = read_issue(&paths.issue_md(args.from_id))?;
        let mut to = read_issue(&paths.issue_md(args.to_id))?;

        let removed_from = remove_value(&mut from.frontmatter.blocks, args.to_id);
        let removed_to = remove_value(&mut to.frontmatter.blocked_by, args.from_id);
        if !removed_from && !removed_to {
            return Err(UserError(format!(
                "no edge #{} blocks #{} to remove",
                args.from_id, args.to_id
            ))
            .into());
        }

        from.frontmatter.updated = now_iso.clone();
        to.frontmatter.updated = now_iso.clone();
        write_issue(&paths.issue_md(args.from_id), &from)?;
        write_issue(&paths.issue_md(args.to_id), &to)?;
        Ok(())
    })?;

    if args.json {
        println!(
            "{{\"from\":{},\"to\":{},\"ok\":true}}",
            args.from_id, args.to_id
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Removed edge: #{} blocks #{}", args.from_id, args.to_id);
    }
    Ok(())
}

fn require_exists(paths: &RepoPaths, id: u64) -> Result<()> {
    if !paths.issue_md(id).exists() {
        return Err(NotFoundError(format!("issue #{id} not found")).into());
    }
    Ok(())
}

fn insert_sorted_unique(v: &mut Vec<u64>, x: u64) -> bool {
    if v.contains(&x) {
        return false;
    }
    v.push(x);
    v.sort_unstable();
    true
}

fn remove_value(v: &mut Vec<u64>, x: u64) -> bool {
    let before = v.len();
    v.retain(|y| *y != x);
    before != v.len()
}

/// DFS in the `blocks` graph from `start`, looking for `target`. Returns
/// `Ok(true)` if `target` is reachable from `start`. Tolerates orphan edges
/// (an issue that lists a blocks/blocked_by id that doesn't exist on disk).
fn path_exists(paths: &RepoPaths, start: u64, target: u64) -> Result<bool> {
    if start == target {
        return Ok(true);
    }
    let mut visited: HashSet<u64> = HashSet::new();
    let mut stack: Vec<u64> = vec![start];
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        let path = paths.issue_md(id);
        if !path.exists() {
            continue;
        }
        let issue = read_issue(&path)?;
        for next in issue.frontmatter.blocks {
            if next == target {
                return Ok(true);
            }
            if !visited.contains(&next) {
                stack.push(next);
            }
        }
    }
    Ok(false)
}
