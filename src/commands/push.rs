use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use crate::config::Config;
use crate::notion::{markdown_to_blocks, parse_note, serialize_frontmatter, NotionClient};

pub fn run(path_or_title: &str, dry_run: bool, open: bool) -> Result<()> {
    let config = Config::load()?;
    let path = resolve_path(path_or_title, config.notes_dir.as_deref())?;
    push_file(&path, &config, dry_run, open)
}

/// Called by `write --push` after the editor closes.
pub fn run_path(path: &Path) -> Result<()> {
    let config = Config::load()?;
    push_file(path, &config, false, false)
}

fn push_file(path: &Path, config: &Config, dry_run: bool, open: bool) -> Result<()> {
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

    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read {}", path.display()))?;

    let (mut fm, body) = parse_note(&raw);

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

    if dry_run {
        println!("Dry run — nothing will be pushed to Notion.\n");
        println!("  File:     {}", path.display());
        println!("  Title:    \"{}\"", title);
        println!(
            "  Status:   {}",
            fm.status.as_deref().unwrap_or("(none)")
        );
        if fm.tags.is_empty() {
            println!("  Tags:     (none)");
        } else {
            println!("  Tags:     [{}]", fm.tags.join(", "));
        }
        println!("  Database: {}", db_id);
        let blocks = markdown_to_blocks(body.trim_start());
        println!("  Blocks:   {}", blocks.len());
        return Ok(());
    }

    if fm.notion_id.is_some() {
        eprintln!(
            "Warning: this note was already pushed (notion_id present). Creating a new page."
        );
    }

    let client = NotionClient::new(token);
    let db_info = client
        .inspect_database(db_id)
        .context("Failed to inspect Notion database")?;

    // Warn about frontmatter fields that won't map to database properties
    if fm.status.as_deref().is_some_and(|s| !s.is_empty()) && db_info.status_prop.is_none() {
        eprintln!(
            "Warning: note has status=\"{}\" but the database has no Status (select) property — field skipped.\n\
             → Add a \"Status\" select property in Notion to sync this field.",
            fm.status.as_deref().unwrap()
        );
    }
    if !fm.tags.is_empty() && db_info.tags_prop.is_none() {
        eprintln!(
            "Warning: note has {} tag(s) but the database has no Tags (multi_select) property — field skipped.\n\
             → Add a \"Tags\" multi_select property in Notion to sync this field.",
            fm.tags.len()
        );
    }

    let blocks = markdown_to_blocks(body.trim_start());
    let page_id = client
        .create_page(db_id, &db_info, &title, fm.status.as_deref(), &fm.tags, blocks)
        .context("Failed to create Notion page")?;

    let url = format!("https://www.notion.so/{}", page_id.replace('-', ""));
    println!("Pushed: \"{}\"", title);
    println!("URL:    {}", url);

    fm.notion_id = Some(page_id);
    let new_content = format!("{}\n{}", serialize_frontmatter(&fm), body.trim_start());
    std::fs::write(path, new_content)
        .with_context(|| format!("Pushed successfully but failed to update {}", path.display()))?;

    if open {
        open_url(&url)?;
    }

    Ok(())
}

fn open_url(url: &str) -> Result<()> {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "xdg-open"
    };

    if cfg!(target_os = "windows") {
        std::process::Command::new(cmd)
            .args(["/c", "start", url])
            .spawn()
            .with_context(|| format!("Failed to open {}", url))?;
    } else {
        std::process::Command::new(cmd)
            .arg(url)
            .spawn()
            .with_context(|| format!("Failed to open {}", url))?;
    }

    Ok(())
}

fn resolve_path(path_or_title: &str, notes_dir: Option<&str>) -> Result<PathBuf> {
    let as_path = Path::new(path_or_title);
    if as_path.exists() {
        return Ok(as_path.to_path_buf());
    }

    if let Some(dir) = notes_dir {
        let slug = slugify(path_or_title);
        let candidate = Path::new(dir).join(format!("{}.md", slug));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!(
        "File not found: '{}'\nPass a path to an existing Markdown file.",
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
