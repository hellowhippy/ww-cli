use anyhow::{Context, Result};

use crate::api::client;
use crate::utils::config;

pub fn make_global(api_key: Option<String>) -> Result<()> {
    let api_key = config::resolve_api_key(api_key)?;
    let api_url = config::get_api_url()?;
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    let app_id = config::get_local_app_id(&current_dir)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No app ID in this directory. Run `whippy-workflows-cli push` first to create or link an app."
        )
    })?;

    println!("Promoting app {} to global...", app_id);
    let updated = client::make_app_global(&api_url, &api_key, &app_id)?;

    let access_level = updated
        .get("data")
        .and_then(|data| data.get("access_level"))
        .and_then(|value| value.as_str())
        .unwrap_or("global");

    println!("✓ App is now {}.", access_level);
    Ok(())
}
