use anyhow::{Context, Result};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;

/// Hint to show when API returns an auth-related error so users run login first.
const LOGIN_HINT: &str = "\n\nRun 'ww-cli login' to sign in, then try again.";

fn is_auth_related_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || status == reqwest::StatusCode::SERVICE_UNAVAILABLE
}

pub fn validate_api_key(api_url: &str, api_key: &str) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/api/v1/apps", api_url);

    let response = match client.get(&url).header("x-whippy-key", api_key).send() {
        Ok(resp) => resp,
        Err(e) => {
            anyhow::bail!(
                "Failed to connect to API at {}\n\nError: {}\n\nTip: Set WHIPPY_WORKFLOWS_API_URL if using a non-default URL.\nDefault production API is https://workflows.whippy.ai\nExample (local): export WHIPPY_WORKFLOWS_API_URL=http://localhost:4003",
                api_url,
                e
            );
        }
    };

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("Invalid API key. Please check your key and try again.");
        }
        anyhow::bail!(
            "API request failed with status: {}. Please check your API URL and try again.",
            response.status()
        );
    }

    Ok(())
}

/// Create a new app.
///
/// This always creates a fresh app; it never reuses an existing app that happens to
/// share the same name. The backend rejects a duplicate name in the organization with
/// an `APP_ALREADY_EXISTS` error unless `allow_duplicate` is set, which prevents the
/// accidental duplicate apps that missing `.whippyapprc` files used to cause.
pub fn create_app(
    api_url: &str,
    api_key: &str,
    name: &str,
    description: &str,
    allow_duplicate: bool,
) -> Result<String> {
    let client = reqwest::blocking::Client::new();

    let create_url = format!("{}/api/v1/apps", api_url);
    let app_data = serde_json::json!({
        "allow_duplicate": allow_duplicate,
        "app": {
            "name": name,
            "description": description,
            "access_level": "private"
        }
    });

    let response = client
        .post(&create_url)
        .header("x-whippy-key", api_key)
        .header("Content-Type", "application/json")
        .json(&app_data)
        .send()
        .context("Failed to create app")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, body, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());

        // Surface the backend duplicate-name guard with an actionable message so the
        // developer can decide whether they really meant to create a new app.
        if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
            if parsed.get("error").and_then(|e| e.as_str()) == Some("APP_ALREADY_EXISTS") {
                let mut message = format!(
                    "An app named '{}' already exists in your organization.",
                    name
                );
                if let Some(existing) = parsed.get("existing_apps").and_then(|a| a.as_array()) {
                    message.push_str("\n\nExisting apps:");
                    for app in existing {
                        let id = app.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                        let existing_name = app.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                        message.push_str(&format!("\n  - {} ({})", existing_name, id));
                    }
                }
                message.push_str(concat!(
                    "\n\nIf this is an existing app, restore its .whippyapprc file instead of creating a new one.\n",
                    "If you really want a separate app with the same name, re-run with --create --allow-duplicate."
                ));
                anyhow::bail!("{}", message);
            }
        }

        anyhow::bail!("Failed to create app: {} - {}", status, body);
    }

    let app: Value = response
        .json()
        .context("Failed to parse create app response")?;
    let app_id = app
        .get("data")
        .and_then(|d| d.get("id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| {
            // Debug: print the actual response structure
            eprintln!("Unexpected response structure: {}", serde_json::to_string_pretty(&app).unwrap_or_else(|_| "Failed to serialize".to_string()));
            anyhow::anyhow!("App created but ID not found in response. Expected structure: {{\"data\": {{\"id\": \"...\"}}}}")
        })?;

    Ok(app_id.to_string())
}

/// Create or update an app version (same version = update in place, new version = create).
/// You bump the version in app.json when you want a new version; repeated push with same version overwrites.
pub fn create_or_update_app_version(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    version: &str,
    definition: &Value,
    language: &str,
    runtime_type: &str,
    build_zip: &[u8],
    source_zip: &[u8],
) -> Result<Value> {
    let client = reqwest::blocking::Client::new();

    let versions = list_versions(api_url, api_key, app_id)?;
    let existing_version_id = find_version_id_by_version(&versions, version);

    let (version_url, is_update) = if let Some(version_id) = existing_version_id {
        (
            format!("{}/api/v1/apps/{}/versions/{}", api_url, app_id, version_id),
            true,
        )
    } else {
        (
            format!("{}/api/v1/apps/{}/versions", api_url, app_id),
            false,
        )
    };

    let definition_json = serde_json::to_string(definition)?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("app_version[version]", version.to_string())
        .text("app_version[description]", format!("Version {}", version))
        .text("app_version[language]", language.to_string())
        .text("app_version[runtime_type]", runtime_type.to_string())
        .text("app_version[definition]", definition_json)
        .part(
            "build_zip",
            reqwest::blocking::multipart::Part::bytes(build_zip.to_vec())
                .file_name("build.zip")
                .mime_str("application/zip")?,
        )
        .part(
            "source_zip",
            reqwest::blocking::multipart::Part::bytes(source_zip.to_vec())
                .file_name("source.zip")
                .mime_str("application/zip")?,
        );

    let response = if is_update {
        client
            .put(&version_url)
            .header("x-whippy-key", api_key)
            .multipart(form)
            .send()
    } else {
        client
            .post(&version_url)
            .header("x-whippy-key", api_key)
            .multipart(form)
            .send()
    }
    .context("Failed to create/update app version")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, body, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to create/update app version: {} - {}", status, body);
    }

    let version_data: Value = response
        .json()
        .context("Failed to parse app version response")?;
    Ok(version_data)
}

pub fn list_versions(api_url: &str, api_key: &str, app_id: &str) -> Result<Vec<Value>> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/api/v1/apps/{}/versions", api_url, app_id);

    let response = client
        .get(&url)
        .header("x-whippy-key", api_key)
        .send()
        .context("Failed to list versions")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, body, LOGIN_HINT);
    }

    if !response.status().is_success() {
        anyhow::bail!("Failed to list versions: {}", response.status());
    }

    let data: Value = response
        .json()
        .context("Failed to parse versions response")?;
    let versions = data
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| vec![]);

    Ok(versions)
}

/// Find version ID by version string (e.g. "1.0.0")
/// Multipart logo upload (`logo` field). PNG, square, min 512×512, transparent background.
pub fn upload_app_logo(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    png_bytes: &[u8],
    filename: &str,
) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/api/v1/apps/{}/logo", api_url, app_id);

    let form = reqwest::blocking::multipart::Form::new().part(
        "logo",
        reqwest::blocking::multipart::Part::bytes(png_bytes.to_vec())
            .file_name(filename.to_string())
            .mime_str("image/png")?,
    );

    let response = client
        .post(&url)
        .header("x-whippy-key", api_key)
        .multipart(form)
        .send()
        .context("Failed to upload logo")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, body, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Logo upload failed: {} - {}", status, body);
    }

    response
        .json()
        .context("Failed to parse logo upload response")
}

/// PUT app with only `brand_color` (hex, not #FFFFFF).
pub fn update_app_brand_color(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    brand_color: &str,
) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/api/v1/apps/{}", api_url, app_id);
    let body = serde_json::json!({
        "app": { "brand_color": brand_color }
    });

    let response = client
        .put(&url)
        .header("x-whippy-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .context("Failed to update app brand color")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, text, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Update failed: {} - {}", status, text);
    }

    response.json().context("Failed to parse update response")
}

/// Promote an app to global visibility. The CLI intentionally does not expose
/// a corresponding downgrade from global to private.
pub fn make_app_global(api_url: &str, api_key: &str, app_id: &str) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/api/v1/apps/{}", api_url, app_id);

    let response = client
        .put(&url)
        .header("x-whippy-key", api_key)
        .header("Content-Type", "application/json")
        .json(&make_app_global_body())
        .send()
        .context("Failed to make app global")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, text, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Update failed: {} - {}", status, text);
    }

    response.json().context("Failed to parse update response")
}

fn make_app_global_body() -> Value {
    serde_json::json!({
        "app": { "access_level": "global" }
    })
}

pub fn find_version_id_by_version(versions: &[Value], version: &str) -> Option<String> {
    for v in versions {
        if v.get("version").and_then(|ver| ver.as_str()) == Some(version) {
            return v.get("id").and_then(|id| {
                id.as_str()
                    .map(|s| s.to_string())
                    .or_else(|| id.as_i64().map(|n| n.to_string()))
            });
        }
    }
    None
}

/// Compute the highest valid semver among the given versions.
/// Versions whose `version` field is missing or not valid semver are ignored,
/// so unconventional schemes never block a push.
pub fn max_semver_version(versions: &[Value]) -> Option<semver::Version> {
    versions
        .iter()
        .filter_map(|v| v.get("version").and_then(|ver| ver.as_str()))
        .filter_map(|s| semver::Version::parse(s).ok())
        .max()
}

/// Publish a draft app version (POST .../versions/:id/publish).
pub fn publish_app_version(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    version_id: &str,
) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}/api/v1/apps/{}/versions/{}/publish",
        api_url, app_id, version_id
    );

    let response = client
        .post(&url)
        .header("x-whippy-key", api_key)
        .send()
        .context("Failed to publish app version")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to publish app version: {} - {}", status, body);
    }

    let data: Value = response
        .json()
        .context("Failed to parse publish response")?;
    Ok(data)
}

fn parse_secret_keys_from_response(body: &Value) -> Result<Vec<String>> {
    let keys = body
        .get("data")
        .and_then(|d| d.get("keys"))
        .and_then(|k| k.as_array())
        .ok_or_else(|| {
            anyhow::anyhow!("Unexpected response: expected {{\"data\":{{\"keys\":[...]}}}}")
        })?;
    keys.iter()
        .map(|v| {
            v.as_str()
                .map(String::from)
                .ok_or_else(|| anyhow::anyhow!("Secret key name must be a string in API response"))
        })
        .collect()
}

fn secrets_url(api_url: &str, app_id: &str, suffix: &str) -> String {
    let base = api_url.trim_end_matches('/');
    format!("{}/api/v1/apps/{}/secrets{}", base, app_id, suffix)
}

/// List app secret key names (values are never returned).
pub fn list_app_secret_keys(api_url: &str, api_key: &str, app_id: &str) -> Result<Vec<String>> {
    let client = reqwest::blocking::Client::new();
    let url = secrets_url(api_url, app_id, "/keys");

    let response = client
        .get(&url)
        .header("x-whippy-key", api_key)
        .send()
        .context("Failed to list app secret keys")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, body, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to list app secret keys: {} - {}", status, body);
    }

    let data: Value = response
        .json()
        .context("Failed to parse secret keys response")?;
    parse_secret_keys_from_response(&data)
}

/// Merge string key/value pairs into app secrets; returns current key names.
pub fn put_app_secrets(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    secrets: &Value,
) -> Result<Vec<String>> {
    let client = reqwest::blocking::Client::new();
    let url = secrets_url(api_url, app_id, "");
    let body = serde_json::json!({ "secrets": secrets });

    let response = client
        .put(&url)
        .header("x-whippy-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .context("Failed to set app secrets")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, text, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to set app secrets: {} - {}", status, text);
    }

    let data: Value = response
        .json()
        .context("Failed to parse set secrets response")?;
    parse_secret_keys_from_response(&data)
}

/// Remove one app secret by key name; returns remaining key names.
pub fn delete_app_secret_key(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    key: &str,
) -> Result<Vec<String>> {
    let client = reqwest::blocking::Client::new();
    let enc = utf8_percent_encode(key, NON_ALPHANUMERIC).to_string();
    let url = secrets_url(api_url, app_id, &format!("/{}", enc));

    let response = client
        .delete(&url)
        .header("x-whippy-key", api_key)
        .send()
        .context("Failed to delete app secret")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, text, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to delete app secret: {} - {}", status, text);
    }

    let data: Value = response
        .json()
        .context("Failed to parse delete secret response")?;
    parse_secret_keys_from_response(&data)
}

/// Clear all app secrets; returns key names (empty).
pub fn clear_app_secrets(api_url: &str, api_key: &str, app_id: &str) -> Result<Vec<String>> {
    let client = reqwest::blocking::Client::new();
    let url = secrets_url(api_url, app_id, "");

    let response = client
        .delete(&url)
        .header("x-whippy-key", api_key)
        .send()
        .context("Failed to clear app secrets")?;

    if is_auth_related_status(response.status()) {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Authentication failed ({}): {}{}", status, text, LOGIN_HINT);
    }

    if !response.status().is_success() {
        let status = response.status();
        let text = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to clear app secrets: {} - {}", status, text);
    }

    let data: Value = response
        .json()
        .context("Failed to parse clear secrets response")?;
    parse_secret_keys_from_response(&data)
}

/// Deprecate a published app version (POST .../versions/:id/deprecate).
/// Optionally send reason and/or deprecation_date (ISO 8601). If deprecation_date is omitted, the API defaults to now.
pub fn deprecate_app_version(
    api_url: &str,
    api_key: &str,
    app_id: &str,
    version_id: &str,
    reason: Option<&str>,
    deprecation_date: Option<&str>,
) -> Result<Value> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}/api/v1/apps/{}/versions/{}/deprecate",
        api_url, app_id, version_id
    );

    let mut body = serde_json::map::Map::new();
    if let Some(r) = reason {
        body.insert(
            "reason".to_string(),
            serde_json::Value::String(r.to_string()),
        );
    }
    if let Some(d) = deprecation_date {
        body.insert(
            "deprecation_date".to_string(),
            serde_json::Value::String(d.to_string()),
        );
    }

    let response = client
        .post(&url)
        .header("x-whippy-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .context("Failed to deprecate app version")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Failed to deprecate app version: {} - {}", status, body);
    }

    let data: Value = response
        .json()
        .context("Failed to parse deprecate response")?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_secret_keys_from_response_reads_data_keys() {
        let v = serde_json::json!({"data":{"keys":["client_id","client_secret"]}});
        assert_eq!(
            parse_secret_keys_from_response(&v).unwrap(),
            vec!["client_id".to_string(), "client_secret".to_string()]
        );
    }

    #[test]
    fn secrets_url_builds_expected_paths() {
        assert_eq!(
            secrets_url("https://api.example", "app-1", "/keys"),
            "https://api.example/api/v1/apps/app-1/secrets/keys"
        );
        assert_eq!(
            secrets_url("https://api.example/", "app-1", ""),
            "https://api.example/api/v1/apps/app-1/secrets"
        );
    }

    #[test]
    fn find_version_id_by_version_finds_by_string_id() {
        let versions = serde_json::json!([
            {"id": "v1", "version": "1.0.0"},
            {"id": "v2", "version": "1.0.1"},
        ]);
        let arr = versions.as_array().unwrap();
        assert_eq!(
            find_version_id_by_version(arr, "1.0.0"),
            Some("v1".to_string())
        );
        assert_eq!(
            find_version_id_by_version(arr, "1.0.1"),
            Some("v2".to_string())
        );
    }

    #[test]
    fn find_version_id_by_version_finds_by_numeric_id() {
        let versions = serde_json::json!([
            {"id": 101, "version": "1.0.0"},
            {"id": 102, "version": "2.0.0"},
        ]);
        let arr = versions.as_array().unwrap();
        assert_eq!(
            find_version_id_by_version(arr, "1.0.0"),
            Some("101".to_string())
        );
        assert_eq!(
            find_version_id_by_version(arr, "2.0.0"),
            Some("102".to_string())
        );
    }

    #[test]
    fn find_version_id_by_version_returns_none_when_missing() {
        let versions = serde_json::json!([
            {"id": "v1", "version": "1.0.0"},
        ]);
        let arr = versions.as_array().unwrap();
        assert_eq!(find_version_id_by_version(arr, "9.9.9"), None);
    }

    #[test]
    fn max_semver_version_returns_highest() {
        let versions = serde_json::json!([
            {"id": "v1", "version": "1.0.2"},
            {"id": "v2", "version": "1.0.4"},
            {"id": "v3", "version": "1.0.3"},
        ]);
        let arr = versions.as_array().unwrap();
        assert_eq!(
            max_semver_version(arr),
            Some(semver::Version::parse("1.0.4").unwrap())
        );
    }

    #[test]
    fn max_semver_version_skips_non_semver_strings() {
        let versions = serde_json::json!([
            {"id": "v1", "version": "1.0.2"},
            {"id": "v2", "version": "latest"},
            {"id": "v3", "version": "not-a-version"},
            {"id": "v4", "version": "2.0.0"},
        ]);
        let arr = versions.as_array().unwrap();
        assert_eq!(
            max_semver_version(arr),
            Some(semver::Version::parse("2.0.0").unwrap())
        );
    }

    #[test]
    fn max_semver_version_orders_prerelease_below_release() {
        let versions = serde_json::json!([
            {"id": "v1", "version": "1.0.0-beta"},
            {"id": "v2", "version": "1.0.0"},
        ]);
        let arr = versions.as_array().unwrap();
        assert_eq!(
            max_semver_version(arr),
            Some(semver::Version::parse("1.0.0").unwrap())
        );
    }

    #[test]
    fn max_semver_version_returns_none_for_empty_or_invalid() {
        let empty: Vec<Value> = Vec::new();
        assert_eq!(max_semver_version(&empty), None);

        let versions = serde_json::json!([{"id": "v1", "version": "nightly"}]);
        let arr = versions.as_array().unwrap();
        assert_eq!(max_semver_version(arr), None);
    }

    #[test]
    fn make_app_global_body_only_promotes_to_global() {
        assert_eq!(
            make_app_global_body(),
            serde_json::json!({
                "app": { "access_level": "global" }
            })
        );
    }
}
