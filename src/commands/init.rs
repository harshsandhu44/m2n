use anyhow::Result;
use crate::config::{Config, config_path};

pub fn run() -> Result<()> {
    let path = config_path()?;

    if path.exists() {
        println!("Config already exists at {}", path.display());
        return Ok(());
    }

    let config = Config::default_config();
    config.save()?;
    println!("Initialized config at {}", path.display());
    println!("Notes will be saved to: {}", config.notes_dir);
    println!("\nEdit the config to set your Notion token and database_id.");
    Ok(())
}
