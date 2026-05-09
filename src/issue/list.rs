use std::cmp::Ordering;
use std::path::PathBuf;

use anyhow::Result;

use crate::issue::create::UserError;
use crate::storage::config::RepoPaths;
use crate::storage::issue_file::{IssueFrontmatter, enumerate_issue_ids, read_issue};

const TERMINAL_STATES: &[&str] = &["done", "dropped"];

pub struct ListArgs {
    pub repo_root: PathBuf,
    pub states: Option<Vec<String>>,
    pub types: Option<Vec<String>>,
    pub blockers: Option<Vec<String>>,
    pub priorities: Option<Vec<String>>,
    pub epic: Option<String>,
    pub open: bool,
    pub closed: bool,
    pub all: bool,
    pub grep: Option<String>,
    pub sort: SortField,
}

#[derive(Copy, Clone)]
pub enum SortField {
    Id,
    Created,
    Updated,
    Priority,
}

impl SortField {
    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "id" => SortField::Id,
            "created" => SortField::Created,
            "updated" => SortField::Updated,
            "priority" => SortField::Priority,
            other => {
                return Err(UserError(format!(
                    "invalid --sort '{other}'; expected one of: id, created, updated, priority"
                ))
                .into());
            }
        })
    }
}

pub fn run(args: ListArgs) -> Result<()> {
    if [args.open, args.closed, args.all].iter().filter(|b| **b).count() > 1 {
        return Err(UserError(
            "--open, --closed, and --all are mutually exclusive".into(),
        )
        .into());
    }

    let paths = RepoPaths::new(args.repo_root.clone());
    let ids = enumerate_issue_ids(&paths.issues_dir())?;

    let need_body = args.grep.is_some();
    let mut rows: Vec<(IssueFrontmatter, String)> = Vec::with_capacity(ids.len());
    for id in ids {
        let issue = read_issue(&paths.issue_md(id))?;
        let body = if need_body { issue.body } else { String::new() };
        rows.push((issue.frontmatter, body));
    }

    rows.retain(|(fm, body)| matches_filters(fm, body, &args));
    rows.sort_by(|(a, _), (b, _)| compare(a, b, args.sort));

    if rows.is_empty() {
        println!("No issues match.");
        return Ok(());
    }

    print_table(&rows);
    Ok(())
}

fn matches_filters(fm: &IssueFrontmatter, body: &str, args: &ListArgs) -> bool {
    // Default scope is --open (active states only) unless --closed or --all.
    if !args.all {
        let is_terminal = TERMINAL_STATES.contains(&fm.state.as_str());
        if args.closed {
            if !is_terminal {
                return false;
            }
        } else if is_terminal {
            // implicit --open
            return false;
        }
    }
    if let Some(states) = &args.states {
        if !states.iter().any(|s| s == &fm.state) {
            return false;
        }
    }
    if let Some(types) = &args.types {
        if !types.iter().any(|t| t == &fm.issue_type) {
            return false;
        }
    }
    if let Some(blockers) = &args.blockers {
        let bm = match &fm.blocker {
            Some(b) => blockers.iter().any(|x| x == b),
            None => blockers.iter().any(|x| x == "unset"),
        };
        if !bm {
            return false;
        }
    }
    if let Some(priorities) = &args.priorities {
        let pri_match = match &fm.priority {
            Some(p) => priorities.iter().any(|x| x == p),
            None => priorities.iter().any(|x| x == "unset"),
        };
        if !pri_match {
            return false;
        }
    }
    if let Some(epic) = &args.epic {
        match &fm.epic {
            Some(e) if e == epic => {}
            _ => return false,
        }
    }
    if let Some(needle) = &args.grep {
        let in_title = fm.title.contains(needle);
        let in_body = body.contains(needle);
        if !in_title && !in_body {
            return false;
        }
    }
    true
}

fn compare(a: &IssueFrontmatter, b: &IssueFrontmatter, field: SortField) -> Ordering {
    match field {
        SortField::Id => a.id.cmp(&b.id),
        SortField::Created => b.created.cmp(&a.created),
        SortField::Updated => b.updated.cmp(&a.updated),
        SortField::Priority => priority_rank(&a.priority)
            .cmp(&priority_rank(&b.priority))
            .then(b.updated.cmp(&a.updated)),
    }
}

fn priority_rank(p: &Option<String>) -> u8 {
    match p.as_deref() {
        Some("p0") => 0,
        Some("p1") => 1,
        Some("p2") => 2,
        _ => 3,
    }
}

fn print_table(rows: &[(IssueFrontmatter, String)]) {
    let title_width = 50_usize;
    println!(
        "{:>4}  {:<10}  {:<8}  {:<3}  {:<width$}  {}",
        "ID",
        "STATE",
        "TYPE",
        "PRI",
        "TITLE",
        "UPDATED",
        width = title_width,
    );
    for (fm, _) in rows {
        let title = truncate(&fm.title, title_width);
        let pri = fm.priority.as_deref().unwrap_or("-");
        println!(
            "{id:>4}  {state:<10}  {ty:<8}  {pri:<3}  {title:<width$}  {updated}",
            id = fm.id,
            state = fm.state,
            ty = fm.issue_type,
            title = title,
            updated = fm.updated,
            width = title_width,
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}
