use anyhow::Result;

pub fn run(title: &str) -> Result<()> {
    println!("push: Notion integration not yet implemented (note: \"{}\")", title);
    println!("Set notion.token and notion.database_id in your config, then re-run.");
    Ok(())
}
