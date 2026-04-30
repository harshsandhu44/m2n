use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use crate::config::Config;

pub fn run(title: &str) -> Result<()> {
    let config = Config::load()?;
    let notes_dir = std::path::Path::new(&config.notes_dir);
    fs::create_dir_all(notes_dir).context("Failed to create notes directory")?;

    let slug = slugify(title);
    let filename = format!("{}.md", slug);
    let path = notes_dir.join(&filename);

    if path.exists() {
        println!("Note already exists: {}", path.display());
        return Ok(());
    }

    let now = Local::now();
    let frontmatter = format!(
        "---\ntitle: \"{}\"\ndate: {}\nstatus: draft\ntags: []\n---\n\n# {}\n\n",
        title,
        now.format("%Y-%m-%dT%H:%M:%S%z"),
        title
    );

    fs::write(&path, frontmatter).with_context(|| format!("Failed to write {}", path.display()))?;
    println!("Created: {}", path.display());
    Ok(())
}

fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
