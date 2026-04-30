mod commands;
mod config;
mod notion;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "m2n",
    version,
    about = "Markdown to Notion — local-first note CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize m2n config at ~/.config/m2n/config.toml
    Init,
    /// Create a new Markdown note
    New {
        /// Title of the note
        title: String,
    },
    /// Open an editor, then sync the note to Notion (creates if new, updates if exists)
    Write {
        /// Title of the note
        title: String,
    },
    /// Open an existing note in your editor and sync changes to Notion
    Edit {
        /// Title of the note
        title: String,
    },
    /// List all notes in your notes directory
    List,
    /// Check config, editor, and Notion connection
    Check,
    /// Push a note to Notion
    Push {
        /// File path or note title
        path: String,
        /// Preview what would be pushed without creating the Notion page
        #[arg(long)]
        dry_run: bool,
        /// Open the Notion page in your browser after pushing
        #[arg(long)]
        open: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => commands::init::run(),
        Command::New { title } => commands::new::run(&title),
        Command::Write { title } => commands::write::run(&title),
        Command::Edit { title } => commands::edit::run(&title),
        Command::List => commands::list::run(),
        Command::Check => commands::check::run(),
        Command::Push {
            path,
            dry_run,
            open,
        } => commands::push::run(&path, dry_run, open),
    }
}
