mod commands;
mod config;
mod notion;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "m2n", version, about = "Markdown to Notion — local-first note CLI")]
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
    /// Open a note in your editor
    Write {
        /// Title or filename of the note
        title: String,
        /// Push to Notion after saving
        #[arg(long)]
        push: bool,
    },
    /// Edit a note (alias for write)
    Edit {
        /// Title or filename of the note
        title: String,
        /// Push to Notion after saving
        #[arg(long)]
        push: bool,
    },
    /// Check config, editor, and Notion connection
    Check,
    /// Push a note to Notion
    Push {
        /// File path or note title
        path: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init => commands::init::run(),
        Command::New { title } => commands::new::run(&title),
        Command::Write { title, push } | Command::Edit { title, push } => {
            commands::write::run(&title, push)
        }
        Command::Check => commands::check::run(),
        Command::Push { path } => commands::push::run(&path),
    }
}
