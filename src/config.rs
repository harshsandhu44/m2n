use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    // Kept optional for backward compat; no longer used by new/write
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes_dir: Option<String>,
    #[serde(default)]
    pub editor: Option<String>,
    pub notion: NotionConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NotionConfig {
    // token is read but never printed
    pub token: Option<String>,
    pub database_id: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let content = fs::read_to_string(&path).with_context(|| {
            format!(
                "Config not found at {}. Run `m2n init` first.",
                path.display()
            )
        })?;
        let config: Config = toml::from_str(&content).context("Failed to parse config.toml")?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    pub fn notes_dir(&self) -> Option<PathBuf> {
        self.notes_dir.as_deref().map(|d| {
            if let Some(rest) = d.strip_prefix("~/") {
                dirs::home_dir().unwrap_or_default().join(rest)
            } else if d == "~" {
                dirs::home_dir().unwrap_or_default()
            } else {
                PathBuf::from(d)
            }
        })
    }

    pub fn editor(&self) -> String {
        if let Some(ed) = &self.editor {
            return ed.clone();
        }
        if let Ok(ed) = std::env::var("EDITOR")
            && !ed.is_empty()
        {
            return ed;
        }
        for fallback in ["nvim", "vim", "nano"] {
            if which(fallback) {
                return fallback.to_string();
            }
        }
        "vim".to_string()
    }
}

pub fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".config").join("m2n").join("config.toml"))
}

fn which(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
