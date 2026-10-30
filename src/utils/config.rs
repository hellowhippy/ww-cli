use anyhow::Result;
use std::path::PathBuf;

/// Default API URL when not set via config or env.
pub const DEFAULT_API_URL: &str = "https://workflows.whippy.ai";

pub fn get_config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| anyhow::anyhow!("Could not find home directory"))?;
    Ok(PathBuf::from(home).join(".whippyrc"))
}

pub fn get_stored_api_key() -> Result<Option<String>> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;

    Ok(config["api_key"].as_str().map(|s| s.to_string()))
}

pub fn get_api_url() -> Result<String> {
    let config_path = get_config_path()?;
    if !config_path.exists() {
        return Ok(default_api_url());
    }

    let content = std::fs::read_to_string(config_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;

    Ok(config["api_url"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_api_url()))
}

/// API URL from env or default (no config file).
pub fn default_api_url() -> String {
    std::env::var("WHIPPY_WORKFLOWS_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

/// Get the app_id stored in the local project (similar to .zapierapprc)
pub fn get_local_app_id(project_dir: &PathBuf) -> Result<Option<String>> {
    let app_rc_path = project_dir.join(".whippyapprc");
    if !app_rc_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(app_rc_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;
    Ok(config["app_id"].as_str().map(|s| s.to_string()))
}

/// Get the app_version_id (UUID) stored in the local project.
pub fn get_local_app_version_id(project_dir: &PathBuf) -> Result<Option<String>> {
    let app_rc_path = project_dir.join(".whippyapprc");
    if !app_rc_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(app_rc_path)?;
    let config: serde_json::Value = serde_json::from_str(&content)?;
    Ok(config["app_version_id"].as_str().map(|s| s.to_string()))
}

/// Store the app_id in the local project
pub fn set_local_app_id(project_dir: &PathBuf, app_id: &str) -> Result<()> {
    let app_rc_path = project_dir.join(".whippyapprc");
    let mut config: serde_json::Value = if app_rc_path.exists() {
        let content = std::fs::read_to_string(&app_rc_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    let obj = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid config"))?;
    obj.insert(
        "app_id".to_string(),
        serde_json::Value::String(app_id.to_string()),
    );
    std::fs::write(&app_rc_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

/// Store the app_version_id (UUID) in the local project. Preserves existing app_id.
pub fn set_local_app_version_id(project_dir: &PathBuf, app_version_id: &str) -> Result<()> {
    let app_rc_path = project_dir.join(".whippyapprc");
    let mut config: serde_json::Value = if app_rc_path.exists() {
        let content = std::fs::read_to_string(&app_rc_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    let obj = config
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Invalid config"))?;
    obj.insert(
        "app_version_id".to_string(),
        serde_json::Value::String(app_version_id.to_string()),
    );
    std::fs::write(&app_rc_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

/// Resolve API key: CLI arg, then stored config, then env.
pub fn resolve_api_key(override_key: Option<String>) -> Result<String> {
    override_key
        .or_else(|| get_stored_api_key().ok().flatten())
        .or_else(|| std::env::var("WHIPPY_API_KEY").ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "API key required. Run 'ww-cli login' first to sign in, or set WHIPPY_API_KEY."
            )
        })
}

/// Resolve app ID: CLI arg, then .whippyapprc in project dir.
pub fn resolve_app_id(override_id: Option<String>, project_dir: &PathBuf) -> Result<String> {
    override_id
        .or_else(|| get_local_app_id(project_dir).ok().flatten())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "App ID required. Run from an integration directory (after 'whippy-workflows-cli push') or pass --app-id <id>"
            )
        })
}
