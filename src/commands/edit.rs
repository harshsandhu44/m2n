use crate::config::Config;
use crate::util::slugify;
use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn run(title: &str) -> Result<()> {
    let config = Config::load()?;
    let editor = config.editor();

    let notes_dir = config
        .notes_dir()
        .context("notes_dir not configured.\n→ Run `m2n init` to set your notes directory.")?;

    let slug = slugify(title);
    let note_path = notes_dir.join(format!("{}.md", slug));

    if !note_path.exists() {
        bail!("No note found for \"{title}\".\n→ Use `m2n write \"{title}\"` to create it.");
    }

    let status = Command::new(&editor)
        .arg(&note_path)
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with non-zero status");
    }

    super::push::run_path(&note_path)
}
