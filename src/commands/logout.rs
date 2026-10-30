use anyhow::Result;

use crate::utils::config;

pub fn run() -> Result<()> {
    let config_path = config::get_config_path()?;

    if config_path.exists() {
        std::fs::remove_file(&config_path)?;
        println!("✓ Logged out successfully");
        println!("  Removed API key from {}", config_path.display());
    } else {
        println!("Not logged in (no stored API key found)");
    }

    Ok(())
}
