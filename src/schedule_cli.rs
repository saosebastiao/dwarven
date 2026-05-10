//! `dwarven schedule next` CLI surface (`dep-graph.md#R5.3`). Computes
//! the queue locally via the scheduler module and prints the top N.

use std::path::PathBuf;

use anyhow::Result;

use crate::scheduler::{ScheduledIssue, compute};
use crate::storage::config::RepoPaths;

pub struct NextArgs {
    pub repo_root: PathBuf,
    pub quiet: bool,
    pub json: bool,
    pub state: Option<String>,
    pub count: usize,
    pub actionable_only: bool,
}

pub fn run_next(args: NextArgs) -> Result<()> {
    let paths = RepoPaths::new(args.repo_root.clone());
    let mut rows = compute(&paths)?;

    if let Some(s) = &args.state {
        rows.retain(|r| comma_match(&r.state, s));
    }
    if args.actionable_only {
        rows.retain(|r| r.actionable);
    }

    let top: Vec<&ScheduledIssue> = rows.iter().take(args.count).collect();

    if args.json {
        let serialized = serde_json::to_string(&top)?;
        println!("{serialized}");
        eprintln!("warning: --json output schema is not yet stable");
        return Ok(());
    }

    if args.quiet {
        return Ok(());
    }

    if top.is_empty() {
        println!("No issues match.");
        return Ok(());
    }

    print_table(&top);
    Ok(())
}

fn comma_match(value: &str, csv: &str) -> bool {
    csv.split(',').any(|s| s == value)
}

fn print_table(rows: &[&ScheduledIssue]) {
    let title_width = 40_usize;
    println!(
        "{:>4}  {:>4}  {:<10}  {:<8}  {:<3}  {:>7}  {:>9}  {:>7}  {:<width$}",
        "RANK", "ID", "STATE", "TYPE", "PRI", "SCORE", "OVERRIDE", "EFF", "TITLE",
        width = title_width,
    );
    for (i, r) in rows.iter().enumerate() {
        let pri = r.priority.as_deref().unwrap_or("-");
        let override_cell = match r.override_value {
            Some(v) => format!("{v:.2}"),
            None => "-".to_string(),
        };
        let actionable_marker = if !r.actionable { " (blocked)" } else { "" };
        let title = format!("{}{}", truncate(&r.title, title_width - actionable_marker.len()), actionable_marker);
        println!(
            "{rank:>4}  {id:>4}  {state:<10}  {ty:<8}  {pri:<3}  {score:>7.2}  {ovr:>9}  {eff:>7.2}  {title:<width$}",
            rank = i + 1,
            id = r.id,
            state = r.state,
            ty = r.r#type,
            pri = pri,
            score = r.score,
            ovr = override_cell,
            eff = r.effective_priority,
            title = title,
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
