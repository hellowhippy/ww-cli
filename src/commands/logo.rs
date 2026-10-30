use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::api::client;
use crate::utils::config;

/// Upload a branding logo and optionally set `brand_color` (same app identity as `push`).
pub fn run(path: Option<PathBuf>, color: Option<String>, api_key: Option<String>) -> Result<()> {
    let api_key = config::resolve_api_key(api_key)?;
    let api_url = config::get_api_url()?;
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    let app_id = config::get_local_app_id(&current_dir)?
        .ok_or_else(|| anyhow::anyhow!("No app ID in this directory. Run `whippy-workflows-cli push` first to create or link an app."))?;

    let file_path = path.unwrap_or_else(|| current_dir.join("assets/logo.png"));
    if !file_path.exists() {
        anyhow::bail!(
            "Logo file not found: {}\n\nExpected a square transparent PNG (min 512×512), or pass a path explicitly.",
            file_path.display()
        );
    }

    let png =
        fs::read(&file_path).with_context(|| format!("Failed to read {}", file_path.display()))?;
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("logo.png");

    println!("Uploading logo for app {} …", app_id);
    let resp = client::upload_app_logo(&api_url, &api_key, &app_id, &png, filename)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&resp).unwrap_or_else(|_| "OK".to_string())
    );

    if let Some(hex) = color {
        println!("Setting brand_color to {} …", hex);
        let updated = client::update_app_brand_color(&api_url, &api_key, &app_id, &hex)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&updated).unwrap_or_else(|_| "OK".to_string())
        );
    }

    println!("Done.");
    Ok(())
}
