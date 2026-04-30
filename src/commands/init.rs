use crate::config::{Config, NotionConfig, config_path};
use crate::notion::{NotionClient, normalize_db_id};
use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::PathBuf;

pub fn run() -> Result<()> {
    let path = config_path()?;

    if path.exists() {
        print!(
            "Config already exists at {}. Overwrite? [y/N]: ",
            path.display()
        );
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if answer.trim().to_lowercase() != "y" {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!("Welcome to m2n — Markdown to Notion.\n");
    println!("Create an integration at https://www.notion.so/my-integrations,");
    println!("then open your database in Notion → '...' → Connections → add it.\n");

    let token = prompt("Notion integration token: ")?;
    if token.is_empty() {
        bail!("Token cannot be empty. Run `m2n init` again when you have one.");
    }

    let raw_db = prompt("Notion database URL or ID: ")?;
    if raw_db.is_empty() {
        bail!("Database URL or ID cannot be empty.");
    }

    let db_id = normalize_db_id(&raw_db).with_context(|| {
        format!(
            "Could not find a 32-character Notion ID in: {}\n\
             → Paste the full database URL (e.g. https://www.notion.so/...) or the bare ID.",
            raw_db
        )
    })?;

    println!();
    print!("Testing connection... ");
    io::stdout().flush()?;
    let client = NotionClient::new(&token);
    let name = client.check_auth().context("Authentication failed")?;
    println!("✓  Connected as \"{}\"", name);

    print!("Checking database...  ");
    io::stdout().flush()?;
    let db_info = client.inspect_database(&db_id).with_context(|| {
        format!(
            "Cannot access database '{}'.\n\
             → Make sure the database is shared with your integration (Notion → '...' → Connections).",
            db_id
        )
    })?;

    let status_mark = if db_info.status_prop.is_some() {
        "✓"
    } else {
        "—"
    };
    let tags_mark = if db_info.tags_prop.is_some() {
        "✓"
    } else {
        "—"
    };
    println!(
        "✓  Database accessible (title: \"{}\", status: {}, tags: {})",
        db_info.title_prop, status_mark, tags_mark
    );

    if db_info.status_prop.is_none() {
        println!(
            "   Note: no \"Status\" (select) property found — status will be skipped on push.\n\
             → Add a \"Status\" select property in Notion to sync this field."
        );
    }
    if db_info.tags_prop.is_none() {
        println!(
            "   Note: no \"Tags\" (multi_select) property found — tags will be skipped on push.\n\
             → Add a \"Tags\" multi_select property in Notion to sync this field."
        );
    }

    let raw_dir = prompt("Notes directory (e.g. ~/notes): ")?;
    let notes_dir = if raw_dir.is_empty() {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("notes")
    } else if let Some(rest) = raw_dir.strip_prefix("~/") {
        dirs::home_dir().unwrap_or_default().join(rest)
    } else if raw_dir == "~" {
        dirs::home_dir().unwrap_or_default()
    } else {
        PathBuf::from(&raw_dir)
    };
    std::fs::create_dir_all(&notes_dir)
        .with_context(|| format!("Failed to create notes directory: {}", notes_dir.display()))?;
    println!("✓  Notes directory: {}", notes_dir.display());

    let config = Config {
        notes_dir: Some(notes_dir.to_string_lossy().into_owned()),
        editor: None,
        notion: NotionConfig {
            token: Some(token),
            database_id: Some(db_id),
        },
    };
    config.save()?;

    println!("\nConfig saved to {}", config_path()?.display());
    println!("Run `m2n write \"My Note\"` to create a note and sync it to Notion.");
    println!("Run `m2n list` to see all your notes.");

    Ok(())
}

fn prompt(label: &str) -> Result<String> {
    print!("{}", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
