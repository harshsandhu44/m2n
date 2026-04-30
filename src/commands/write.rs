use crate::config::Config;
use anyhow::{Context, Result, bail};
use chrono::Local;
use std::process::Command;

pub fn run(title: &str) -> Result<()> {
    let config = Config::load()?;
    let editor = config.editor();

    let slug = slugify(title);
    let tmp_path = std::env::temp_dir().join(format!("m2n-{}.md", slug));

    let now = Local::now();
    let initial = format!(
        "---\ntitle: \"{}\"\ndate: {}\nstatus: draft\ntags: []\n---\n\n# {}\n\n",
        title.replace('"', "\\\""),
        now.format("%Y-%m-%dT%H:%M:%S%z"),
        title,
    );
    std::fs::write(&tmp_path, initial)
        .with_context(|| format!("Failed to create temp file {}", tmp_path.display()))?;

    let status = Command::new(&editor)
        .arg(&tmp_path)
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    if !status.success() {
        std::fs::remove_file(&tmp_path).ok();
        bail!("Editor exited with non-zero status");
    }

    let result = super::push::run_path(&tmp_path);
    std::fs::remove_file(&tmp_path).ok();
    result
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
