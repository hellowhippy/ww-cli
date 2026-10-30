use anyhow::{Context, Result};

use crate::api::client;
use crate::utils::config;

pub fn run(version: String, app_id: Option<String>) -> Result<()> {
    let api_key = config::resolve_api_key(None)?;
    let api_url = config::get_api_url()?;
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let app_id = config::resolve_app_id(app_id, &current_dir)?;

    let versions = client::list_versions(&api_url, &api_key, &app_id)?;
    let version_id = client::find_version_id_by_version(&versions, &version).ok_or_else(|| {
        anyhow::anyhow!(
            "Version '{}' not found. Run 'whippy-workflows-cli versions' to list versions.",
            version
        )
    })?;

    let response_data = client::publish_app_version(&api_url, &api_key, &app_id, &version_id)?;

    println!("✓ Version {} promoted to published.", version);

    if let Some(warnings) = response_data.get("warnings").and_then(|w| w.as_array()) {
        for w in warnings {
            if let Some(s) = w.as_str() {
                eprintln!("⚠ {}", s);
            }
        }
    }

    Ok(())
}
