use anyhow::{Context, Result};
use std::fs;

use crate::api::client;
use crate::utils::config;

pub fn run(
    api_key: Option<String>,
    force: bool,
    create: bool,
    allow_duplicate: bool,
) -> Result<()> {
    println!("Pushing integration...");

    let api_key = config::resolve_api_key(api_key)?;
    let api_url = config::get_api_url()?;
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Check required files exist
    let build_zip_path = current_dir.join("build.zip");
    let source_zip_path = current_dir.join("source.zip");
    let app_json_path = current_dir.join("app.json");

    if !build_zip_path.exists() {
        anyhow::bail!("build.zip not found. Run 'whippy-workflows-cli build' first.");
    }
    if !source_zip_path.exists() {
        anyhow::bail!("source.zip not found. Run 'whippy-workflows-cli build' first.");
    }
    if !app_json_path.exists() {
        anyhow::bail!("app.json not found. Run 'whippy-workflows-cli build' first.");
    }

    // Read files
    let build_zip = fs::read(&build_zip_path).context("Failed to read build.zip")?;
    let source_zip = fs::read(&source_zip_path).context("Failed to read source.zip")?;

    let app_json_content = fs::read_to_string(&app_json_path).context("Failed to read app.json")?;
    let mut definition: serde_json::Value =
        serde_json::from_str(&app_json_content).context("Failed to parse app.json")?;

    let package_json_path = current_dir.join("package.json");
    let (app_name, version_from_package) = if package_json_path.exists() {
        let pkg_content = fs::read_to_string(&package_json_path)?;
        let pkg: serde_json::Value = serde_json::from_str(&pkg_content)?;
        let name = pkg
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "my-app".to_string());
        let ver = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        (name, ver)
    } else {
        ("my-app".to_string(), None)
    };

    // Prefer package.json version so bumping there (without re-running build) still creates a new version
    let version: String = version_from_package
        .or_else(|| {
            definition
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .ok_or_else(|| anyhow::anyhow!("Version not found in package.json or app.json"))?;

    // Sync version into definition so the payload matches
    if let Some(obj) = definition.as_object_mut() {
        obj.insert(
            "version".to_string(),
            serde_json::Value::String(version.clone()),
        );
    }

    // Determine language and runtime from project
    let language = if current_dir.join("src").join("index.ts").exists() {
        "ts"
    } else {
        "js"
    };
    let runtime_type = "cloudflare";

    // Resolve the app to push to. Push is update-only: it never silently creates an
    // app. When .whippyapprc is missing there is no app_id to update, so we refuse the
    // push (this is what previously caused accidental duplicate apps). Creating a new
    // app is an explicit action gated behind --create.
    let app_id = match config::get_local_app_id(&current_dir)? {
        Some(id) => {
            println!("Using existing app ID: {}", id);
            id
        }
        None if create => {
            println!("Creating new app: {}", app_name);
            let id = client::create_app(
                &api_url,
                &api_key,
                &app_name,
                &format!("Custom integration: {}", app_name),
                allow_duplicate,
            )?;
            config::set_local_app_id(&current_dir, &id)?;
            println!("✓ App created: {}", id);
            id
        }
        None => {
            anyhow::bail!(concat!(
                "Cannot push app because .whippyapprc is missing.\n\n",
                "ww-cli push updates an existing app and requires an app_id.\n",
                "If this is a new app, run `ww-cli push --create` first.\n",
                "If this is an existing app, restore the .whippyapprc file for this app."
            ));
        }
    };

    // Guard against version regressions: reject pushing a semver lower than the highest
    // existing version so an accidental package.json edit can't overwrite an older draft
    // while a newer version already exists. `--force` bypasses this for intentional fixes.
    let existing_versions = match client::list_versions(&api_url, &api_key, &app_id) {
        Ok(versions) => versions,
        Err(e) => {
            eprintln!(
                "Warning: Failed to fetch existing versions ({}), skipping version-conflict check.",
                e
            );
            Vec::new()
        }
    };
    if let Ok(target) = semver::Version::parse(&version) {
        if let Some(max_version) = client::max_semver_version(&existing_versions) {
            if target < max_version {
                if force {
                    eprintln!(
                        "\u{26A0} Forcing push of {} while {} already exists.",
                        version, max_version
                    );
                } else {
                    let suggested = format!(
                        "{}.{}.{}",
                        max_version.major,
                        max_version.minor,
                        max_version.patch + 1
                    );
                    anyhow::bail!(
                        "Refusing to push version {} because a newer version {} already exists.\n\
                         Bump the \"version\" in package.json or app.json to a higher value (e.g. {}) and push again,\n\
                         or pass --force to intentionally overwrite an older draft.",
                        version,
                        max_version,
                        suggested
                    );
                }
            }
        }
    } else {
        eprintln!(
            "Warning: Version '{}' is not valid semver, skipping version-conflict check.",
            version
        );
    }

    // Create or update app version: same version = update in place, new version = create
    let version_data = client::create_or_update_app_version(
        &api_url,
        &api_key,
        &app_id,
        &version,
        &definition,
        language,
        runtime_type,
        &build_zip,
        &source_zip,
    )?;

    let data = version_data.get("data");
    let id = data.and_then(|v| v.get("id").and_then(|id| id.as_str()));
    let ver = data.and_then(|v| v.get("version").and_then(|s| s.as_str()));

    if let Some(version_id) = id {
        println!("✓ App version pushed successfully!");
        println!("  Version ID: {}", version_id);
        if let Some(ver) = ver {
            println!("  Version: {}", ver);
        }
        config::set_local_app_version_id(&current_dir, version_id)?;
    }

    // Show API warnings (e.g. runtime deployment failed)
    let mut has_warnings = false;
    if let Some(warnings) = version_data.get("warnings").and_then(|w| w.as_array()) {
        for w in warnings {
            if let Some(s) = w.as_str() {
                eprintln!("⚠ {}", s);
                has_warnings = true;
            }
        }
        if has_warnings {
            eprintln!();
            eprintln!("  Run 'whippy-workflows-cli push' again to retry deployment.");
        }
    }

    Ok(())
}
