use anyhow::{Context, Result};

use crate::api::client;
use crate::utils::config;

/// Deprecates a published version via POST .../versions/:id/deprecate.
/// Optionally pass --date (ISO 8601) for a future deprecation date; if omitted, the API uses now.
/// The Cloudflare worker is torn down after the deprecation date (daily cron).
pub fn run(
    version: String,
    date: Option<String>,
    reason: Option<String>,
    app_id: Option<String>,
) -> Result<()> {
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

    let reason_ref = reason.as_deref();
    let date_ref = date.as_deref();
    client::deprecate_app_version(
        &api_url,
        &api_key,
        &app_id,
        &version_id,
        reason_ref,
        date_ref,
    )?;

    if let Some(d) = &date {
        println!(
            "✓ Version {} deprecated (removal date: {}). Runtime will be removed after that date (daily cron).",
            version, d
        );
    } else {
        println!(
            "✓ Version {} deprecated. Runtime will be removed after the deprecation date (daily cron).",
            version
        );
    }
    Ok(())
}
