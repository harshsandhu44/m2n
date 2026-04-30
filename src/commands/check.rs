use anyhow::Result;
use crate::config::{Config, config_path};
use crate::notion::NotionClient;

pub fn run() -> Result<()> {
    let path = config_path()?;
    println!("Config path : {}", path.display());

    let config = match Config::load() {
        Err(e) => {
            println!("Config      : MISSING — {}", e);
            println!("\nRun `m2n init` to create a config.");
            return Ok(());
        }
        Ok(c) => c,
    };

    println!("Notes dir   : {}", config.notes_dir);
    println!("Editor      : {}", config.editor());

    let editor_env = std::env::var("EDITOR").unwrap_or_else(|_| "(not set)".to_string());
    println!("$EDITOR     : {}", editor_env);

    let token = config.notion.token.as_deref().filter(|t| !t.is_empty());
    let db_id = config.notion.database_id.as_deref().filter(|d| !d.is_empty());

    println!("Notion token: {}", if token.is_some() { "set" } else { "not set" });
    println!("Database ID : {}", db_id.unwrap_or("not set"));

    if let Some(tok) = token {
        let client = NotionClient::new(tok);

        match client.check_auth() {
            Ok(name) => println!("Notion auth : ok ({})", name),
            Err(e) => {
                println!("Notion auth : FAILED — {}", e);
                return Ok(());
            }
        }

        if let Some(db) = db_id {
            match client.inspect_database(db) {
                Ok(info) => {
                    println!("Database    : accessible");
                    println!("Title prop  : {}", info.title_prop);
                    println!(
                        "Status prop : {}",
                        info.status_prop.as_deref().unwrap_or("not found")
                    );
                    println!(
                        "Tags prop   : {}",
                        info.tags_prop.as_deref().unwrap_or("not found")
                    );
                }
                Err(e) => println!("Database    : FAILED — {}", e),
            }
        }
    }

    Ok(())
}
