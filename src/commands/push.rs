use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use crate::config::Config;
use crate::notion::{markdown_to_blocks, parse_note, serialize_frontmatter, NotionClient};

pub fn run(path_or_title: &str) -> Result<()> {
    let config = Config::load()?;
    let path = resolve_path(path_or_title, &config.notes_dir)?;
    push_file(&path, &config)
}

/// Called by `write --push` after the editor closes.
pub fn run_path(path: &Path) -> Result<()> {
    let config = Config::load()?;
    push_file(path, &config)
}

fn push_file(path: &Path, config: &Config) -> Result<()> {
    let token = config
        .notion
        .token
        .as_deref()
        .filter(|t| !t.is_empty())
        .context("notion.token is not set. Run `m2n init` and edit your config.")?;

    let db_id = config
        .notion
        .database_id
        .as_deref()
        .filter(|d| !d.is_empty())
        .context("notion.database_id is not set. Run `m2n init` and edit your config.")?;

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read {}", path.display()))?;

    let (mut fm, body) = parse_note(&raw);

    // Title fallback: frontmatter → first H1 → filename stem
    let title = fm
        .title
        .clone()
        .or_else(|| first_h1(&body))
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    if fm.notion_id.is_some() {
        eprintln!(
            "Warning: this note was already pushed (notion_id present). Creating a new page."
        );
    }

    let client = NotionClient::new(token);
    let db_info = client
        .inspect_database(db_id)
        .context("Failed to inspect Notion database")?;

    let blocks = markdown_to_blocks(body.trim_start());
    let page_id = client
        .create_page(db_id, &db_info, &title, fm.status.as_deref(), &fm.tags, blocks)
        .context("Failed to create Notion page")?;

    let url = format!("https://www.notion.so/{}", page_id.replace('-', ""));
    println!("Pushed: \"{}\"", title);
    println!("URL:    {}", url);

    // Write notion_id back into frontmatter
    fm.notion_id = Some(page_id);
    let new_content = format!("{}\n{}", serialize_frontmatter(&fm), body.trim_start());
    std::fs::write(path, new_content)
        .with_context(|| format!("Pushed successfully but failed to update {}", path.display()))?;

    Ok(())
}

fn resolve_path(path_or_title: &str, notes_dir: &str) -> Result<PathBuf> {
    let as_path = Path::new(path_or_title);
    if as_path.exists() {
        return Ok(as_path.to_path_buf());
    }

    let slug = slugify(path_or_title);
    let candidate = Path::new(notes_dir).join(format!("{}.md", slug));
    if candidate.exists() {
        return Ok(candidate);
    }

    bail!(
        "Note not found: '{}'\nTried: {}\nCreate it with `m2n new \"{}\"`",
        path_or_title,
        candidate.display(),
        path_or_title
    );
}

fn first_h1(body: &str) -> Option<String> {
    body.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l[2..].trim().to_string())
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
