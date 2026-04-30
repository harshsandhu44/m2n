use crate::config::Config;
use crate::notion::NotionClient;
use anyhow::{Context, Result};

pub fn run(title: &str) -> Result<()> {
    let config = Config::load()?;

    let token = config
        .notion
        .token
        .as_deref()
        .filter(|t| !t.is_empty())
        .context(
            "notion.token is not set.\n\
             → Run `m2n init` to set up your Notion integration.",
        )?;

    let db_id = config
        .notion
        .database_id
        .as_deref()
        .filter(|d| !d.is_empty())
        .context(
            "notion.database_id is not set.\n\
             → Run `m2n init` to configure your Notion database.",
        )?;

    let client = NotionClient::new(token);
    let db_info = client
        .inspect_database(db_id)
        .context("Failed to inspect Notion database")?;

    let page_id = client
        .create_page(db_id, &db_info, title, Some("draft"), &[], vec![])
        .context("Failed to create Notion page")?;

    let url = format!("https://www.notion.so/{}", page_id.replace('-', ""));
    println!("Created: \"{}\"", title);
    println!("URL:    {}", url);

    Ok(())
}
