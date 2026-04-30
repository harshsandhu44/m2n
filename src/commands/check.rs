use anyhow::Result;
use crate::config::{Config, config_path};

pub fn run() -> Result<()> {
    let path = config_path()?;
    println!("Config path : {}", path.display());

    match Config::load() {
        Err(e) => {
            println!("Config       : MISSING — {}", e);
            println!("\nRun `m2n init` to create a config.");
            return Ok(());
        }
        Ok(config) => {
            println!("Notes dir   : {}", config.notes_dir);
            println!("Editor      : {}", config.editor());

            let token_set = config.notion.token.as_deref().map(|t| !t.is_empty()).unwrap_or(false);
            println!("Notion token: {}", if token_set { "set" } else { "not set" });

            let db_set = config.notion.database_id.as_deref().map(|d| !d.is_empty()).unwrap_or(false);
            println!("Database ID : {}", if db_set { "set" } else { "not set" });
        }
    }

    let editor_bin = std::env::var("EDITOR").unwrap_or_else(|_| "(not set)".to_string());
    println!("$EDITOR     : {}", editor_bin);

    Ok(())
}
