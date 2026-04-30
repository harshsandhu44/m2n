use crate::config::Config;
use crate::notion::parse_note;
use anyhow::{Context, Result};

pub fn run() -> Result<()> {
    let config = Config::load()?;
    let notes_dir = config
        .notes_dir()
        .context("notes_dir not configured.\n→ Run `m2n init` to set your notes directory.")?;

    if !notes_dir.exists() {
        println!("No notes yet. Run `m2n write \"Title\"` to create one.");
        return Ok(());
    }

    let mut entries: Vec<(String, String, bool)> = std::fs::read_dir(&notes_dir)
        .with_context(|| format!("Cannot read notes directory: {}", notes_dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .filter_map(|e| {
            let raw = std::fs::read_to_string(e.path()).ok()?;
            let (fm, _) = parse_note(&raw);
            let title = fm.title.unwrap_or_else(|| {
                e.path()
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            });
            let synced = fm.notion_id.is_some();
            let status = fm.status.unwrap_or_else(|| "draft".to_string());
            Some((title, status, synced))
        })
        .collect();

    if entries.is_empty() {
        println!("No notes yet. Run `m2n write \"Title\"` to create one.");
        return Ok(());
    }

    entries.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    let max_title = entries.iter().map(|(t, _, _)| t.len()).max().unwrap_or(20);
    let width = max_title.max(20);

    for (title, status, synced) in &entries {
        let tag = if *synced {
            "[synced]"
        } else if status == "draft" {
            "[draft]"
        } else {
            "[unsynced]"
        };
        println!("{:<width$}  {}", title, tag, width = width);
    }

    Ok(())
}
