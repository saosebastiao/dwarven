use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod init;

#[derive(Parser)]
#[command(name = "dwarven", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Operate on the repository rooted at <path>; defaults to current directory.
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    /// Attribution for any mutation; defaults to env DWARVEN_ACTOR, then "maintainer".
    #[arg(long, global = true)]
    actor: Option<String>,

    /// Suppress non-essential stdout.
    #[arg(long, global = true)]
    quiet: bool,

    /// Machine-parseable JSON output. Not yet implemented.
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize .dwarven/ in the current directory.
    Init {
        /// Host adapter to install (e.g., claude-code). May be repeated. Not yet implemented.
        #[arg(long)]
        host: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo_root = match cli.repo {
        Some(p) => p,
        None => std::env::current_dir()?,
    };

    if cli.json {
        eprintln!("warning: --json output is not yet implemented; producing human output");
    }

    match cli.command {
        Command::Init { host } => init::run(&repo_root, &host, cli.quiet),
    }
}
