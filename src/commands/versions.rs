use anyhow::{Context, Result};
use serde_json::Value;

use crate::api::client;
use crate::utils::config;

fn get_display(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| {
            x.as_str()
                .map(String::from)
                .or_else(|| x.as_i64().map(|n| n.to_string()))
        })
        .unwrap_or_else(|| "-".to_string())
}

pub fn run(app_id: Option<String>) -> Result<()> {
    let api_key = config::resolve_api_key(None)?;
    let api_url = config::get_api_url()?;
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let app_id = config::resolve_app_id(app_id, &current_dir)?;

    println!("Listing versions for app {}...\n", app_id);

    let versions = client::list_versions(&api_url, &api_key, &app_id)?;

    if versions.is_empty() {
        println!("No versions found.");
        return Ok(());
    }

    println!(
        "{:<8} {:<36} {:<10} {:<10}",
        "VERSION", "ID", "LANGUAGE", "RUNTIME"
    );
    println!("{}", "-".repeat(68));

    for v in &versions {
        println!(
            "{:<8} {:<36} {:<10} {:<10}",
            get_display(v, "version"),
            get_display(v, "id"),
            get_display(v, "language"),
            get_display(v, "runtime_type")
        );
    }

    Ok(())
}
