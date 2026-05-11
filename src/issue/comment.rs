//! `dwarven issue comment` — append a comment.
//!
//! Allocates the next comment `seq` under the repo lock by scanning
//! existing `comments/*.md` filenames, then atomic-writes the new
//! comment file. Bumps `issue.updated`.

use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::{BodyInput, NotFoundError, UserError, read_body};
use crate::storage::comment_file::{
    CommentFile, CommentFrontmatter, comment_path, next_comment_seq, write_comment,
};
use crate::storage::config::{RepoPaths, with_repo_lock};
use crate::storage::issue_file::{read_issue, write_issue};
use crate::time::{iso_filename, iso_frontmatter, now_utc};

pub struct CommentArgs {
    pub repo_root: PathBuf,
    pub actor: String,
    pub quiet: bool,
    pub json: bool,
    pub id: u64,
    pub body: BodyInput,
}

pub fn run(args: CommentArgs) -> Result<()> {
    if matches!(args.body, BodyInput::None) {
        return Err(UserError(
            "comment requires a body via --body, --body-file, or --body-stdin".into(),
        )
        .into());
    }
    let body = read_body(&args.body)?;

    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }

    let now = now_utc();
    let created_iso = iso_frontmatter(&now);
    let filename_iso = iso_filename(&now);
    let comments_dir = paths.comments_dir(args.id);

    let seq = with_repo_lock(&paths, || {
        let seq = next_comment_seq(&comments_dir)?;

        let fm = CommentFrontmatter {
            seq,
            issue: args.id,
            author: args.actor.clone(),
            kind: "comment".to_string(),
            created: created_iso.clone(),
            from: None,
            to: None,
            blocker: None,
        };
        let path = comment_path(&comments_dir, seq, &filename_iso, &args.actor);
        write_comment(
            &path,
            &CommentFile {
                frontmatter: fm,
                body: body.clone(),
            },
        )?;

        // Bump issue.updated. Reads the issue under the lock so the new
        // updated value is consistent with the comment we just wrote.
        let mut issue = read_issue(&issue_path)?;
        issue.frontmatter.updated = created_iso.clone();
        write_issue(&issue_path, &issue)?;

        Ok(seq)
    })?;

    if args.json {
        println!(
            "{{\"issue\":{},\"seq\":{seq}}}",
            args.id
        );
        eprintln!("warning: --json output schema is not yet stable");
    } else if !args.quiet {
        println!("Added comment #{seq:03} on issue #{}", args.id);
    }

    Ok(())
}
