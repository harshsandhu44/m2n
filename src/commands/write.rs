use crate::config::Config;
use crate::util::slugify;
use anyhow::{Context, Result, bail};
use chrono::Local;
use std::process::Command;

pub fn run(title: &str) -> Result<()> {
    let config = Config::load()?;
    let editor = config.editor();

    let notes_dir = config
        .notes_dir()
        .context("notes_dir not configured.\n→ Run `m2n init` to set your notes directory.")?;
    std::fs::create_dir_all(&notes_dir)
        .with_context(|| format!("Failed to create notes directory: {}", notes_dir.display()))?;

    let slug = slugify(title);
    let note_path = notes_dir.join(format!("{}.md", slug));

    if !note_path.exists() {
        let now = Local::now();
        let initial = format!(
            "---\ntitle: \"{}\"\ndate: {}\nstatus: draft\ntags: []\n---\n\n# {}\n\n",
            title.replace('"', "\\\""),
            now.format("%Y-%m-%dT%H:%M:%S%z"),
            title,
        );
        std::fs::write(&note_path, initial)
            .with_context(|| format!("Failed to create note at {}", note_path.display()))?;
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
