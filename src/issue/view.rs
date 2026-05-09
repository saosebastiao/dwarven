use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::NotFoundError;
use crate::storage::comment_file::{CommentFile, list_comments};
use crate::storage::config::RepoPaths;
use crate::storage::issue_file::read_issue;

pub struct ViewArgs {
    pub repo_root: PathBuf,
    pub id: u64,
    pub no_comments: bool,
    pub state_history: bool,
    pub last: Option<usize>,
}

pub fn run(args: ViewArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    let issue_path = paths.issue_md(args.id);
    if !issue_path.exists() {
        return Err(NotFoundError(format!("issue #{} not found", args.id)).into());
    }
    let issue = read_issue(&issue_path)?;
    let fm = &issue.frontmatter;

    println!("Issue #{}: {}", fm.id, fm.title);
    field("type", &fm.issue_type);
    field("state", &fm.state);
    if let Some(p) = &fm.priority {
        field("priority", p);
    }
    if let Some(b) = &fm.blocker {
        field("blocker", b);
    }
    if let Some(e) = &fm.epic {
        field("epic", e);
    }
    if !fm.blocked_by.is_empty() {
        field("blocked_by", &format_id_list(&fm.blocked_by));
    }
    if !fm.blocks.is_empty() {
        field("blocks", &format_id_list(&fm.blocks));
    }
    field("created", &format!("{} by {}", fm.created, fm.created_by));
    field("updated", &fm.updated);

    if !issue.body.trim().is_empty() {
        println!();
        print!("{}", issue.body);
        if !issue.body.ends_with('\n') {
            println!();
        }
    }

    if args.no_comments {
        return Ok(());
    }

    let mut comments = list_comments(&paths.comments_dir(args.id))?;
    if args.state_history {
        comments.retain(|c| c.frontmatter.kind == "state-change");
    }
    if let Some(n) = args.last {
        if comments.len() > n {
            let drop = comments.len() - n;
            comments.drain(..drop);
        }
    }

    if comments.is_empty() {
        return Ok(());
    }

    println!();
    println!("--- comments ({}) ---", comments.len());
    for c in &comments {
        print_comment(c);
    }
    Ok(())
}

fn print_comment(c: &CommentFile) {
    let fm = &c.frontmatter;
    let summary = match fm.kind.as_str() {
        "state-change" => match (&fm.from, &fm.to) {
            (Some(f), Some(t)) => format!("state-change: {f} → {t}"),
            _ => "state-change".to_string(),
        },
        "blocker-set" => match &fm.blocker {
            Some(b) => format!("blocker-set: {b}"),
            None => "blocker-set".to_string(),
        },
        "blocker-cleared" => match &fm.blocker {
            Some(b) => format!("blocker-cleared: {b}"),
            None => "blocker-cleared".to_string(),
        },
        _ => "comment".to_string(),
    };
    println!();
    println!(
        "[#{seq:03} {created} {author}]  {summary}",
        seq = fm.seq,
        created = fm.created,
        author = fm.author,
    );
    if !c.body.trim().is_empty() {
        print!("{}", c.body);
        if !c.body.ends_with('\n') {
            println!();
        }
    }
}

fn field(label: &str, value: &str) {
    println!("  {:<11} {}", format!("{label}:"), value);
}

fn format_id_list(ids: &[u64]) -> String {
    let strs: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
    strs.join(", ")
}
