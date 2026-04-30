use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use crate::config::Config;

pub fn run(title: &str) -> Result<()> {
    let config = Config::load()?;
    let notes_dir = Path::new(&config.notes_dir);

    let path = resolve_note(notes_dir, title)?;
    let editor = config.editor();

    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    Ok(())
}

fn resolve_note(notes_dir: &Path, title: &str) -> Result<std::path::PathBuf> {
    // accept exact filename or slug
    let as_path = Path::new(title);
    if as_path.exists() {
        return Ok(as_path.to_path_buf());
    }

    let slug = slugify(title);
    let by_slug = notes_dir.join(format!("{}.md", slug));
    if by_slug.exists() {
        return Ok(by_slug);
    }

    // fall back: create it on the fly
    super::new::run(title)?;
    Ok(by_slug)
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
