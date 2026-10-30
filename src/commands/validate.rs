use anyhow::{Context, Result};
use std::fs;

/// Validate an app definition and return a list of error messages (empty if valid).
pub fn validate_definition(definition: &serde_json::Value) -> Vec<String> {
    let mut errors = Vec::new();

    errors.extend(crate::commands::schema::validate_schema_keys(&definition));
    errors.extend(crate::commands::quality::validate_definition_quality(
        &definition,
    ));

    // Check required top-level fields
    match definition.get("version") {
        None => {
            errors.push("Missing required field: version".to_string());
        }
        Some(v) => {
            if v.is_null() {
                errors.push("Version field is null".to_string());
            } else if let Some(version) = v.as_str() {
                // Validate semver format (e.g., 1.0.0, 1.2.3, 1.0.0-beta)
                // Check if it starts with digit.digit.digit pattern
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() < 3 {
                    errors.push(format!(
                        "Version must be in semver format (e.g., 1.0.0), got: '{}'",
                        version
                    ));
                } else {
                    // Check first part is all digits
                    if !parts[0].chars().all(|c| c.is_ascii_digit()) || parts[0].is_empty() {
                        errors.push(format!(
                            "Version major part must be numeric, got: '{}'",
                            version
                        ));
                    }
                    // Check second part is all digits
                    else if !parts[1].chars().all(|c| c.is_ascii_digit()) || parts[1].is_empty() {
                        errors.push(format!(
                            "Version minor part must be numeric, got: '{}'",
                            version
                        ));
                    }
                    // Check third part starts with digits (may have suffix like -beta)
                    else if !parts[2]
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    {
                        errors.push(format!(
                            "Version patch part must start with a digit, got: '{}'",
                            version
                        ));
                    }
                }
            } else {
                errors.push(format!(
                    "Version must be a string, got: {}",
                    serde_json::to_string(v).unwrap_or_else(|_| "unknown".to_string())
                ));
            }
        }
    }

    if !definition.get("platform_version").is_some() {
        errors.push("Missing required field: platform_version".to_string());
    }

    const VALID_AUTH_TYPES: [&str; 6] = [
        "api_key",
        "oauth2",
        "basic_auth",
        "session",
        "custom",
        "none",
    ];

    if !definition.get("authentication").is_some() {
        errors.push(
            "Missing required field: authentication. Example: authentication: { type: \"api_key\", \
             fields: [{ key: \"api_key\", label: \"API Key\" }] }. See README for schema."
                .to_string(),
        );
    } else if let Some(auth) = definition.get("authentication") {
        if !auth.get("type").is_some() {
            errors.push(
                "Authentication must have a 'type' field. Supported types: api_key, oauth2, \
                 basic_auth, session, custom, none. Example: { type: \"api_key\", fields: [...] }."
                    .to_string(),
            );
        } else if let Some(auth_type) = auth.get("type").and_then(|t| t.as_str()) {
            if !VALID_AUTH_TYPES.contains(&auth_type) {
                errors.push(format!(
                    "Invalid authentication type: '{}'. Supported types are: {}. \
                     Example: authentication: {{ type: \"api_key\", fields: [{{ key: \"api_key\", label: \"API Key\" }}] }}",
                    auth_type,
                    VALID_AUTH_TYPES.join(", ")
                ));
            } else {
                // Types that require at least one field
                if ["api_key", "basic_auth", "custom", "session"].contains(&auth_type) {
                    match auth.get("fields") {
                        Some(serde_json::Value::Array(arr)) if !arr.is_empty() => {
                            for (i, field) in arr.iter().enumerate() {
                                if let Some(obj) = field.as_object() {
                                    if !obj.get("key").and_then(|k| k.as_str()).is_some()
                                        || !obj.get("label").and_then(|l| l.as_str()).is_some()
                                    {
                                        errors.push(format!(
                                            "Authentication field {} must have 'key' and 'label' (strings). \
                                             Example: {{ key: \"api_key\", label: \"API Key\", type: \"string\", required: true }}",
                                            i
                                        ));
                                    }
                                }
                            }
                        }
                        _ => {
                            errors.push(format!(
                                "{} authentication requires a non-empty 'fields' array. \
                                 Each field must have 'key' and 'label'. \
                                 Example: fields: [{{ key: \"api_key\", label: \"API Key\", type: \"string\", required: true }}]",
                                auth_type
                            ));
                        }
                    }
                    if auth_type == "session" {
                        validate_session_config(&mut errors, auth);
                    }
                } else if auth_type == "oauth2" {
                    validate_oauth2_config(&mut errors, auth);
                    if let Some(fields) = auth.get("fields") {
                        if !fields.is_array() {
                            errors.push(
                                "OAuth2 authentication: 'fields' must be an array (or omit for OAuth-only). \
                                 Example: fields: [{ key: \"subdomain\", label: \"Subdomain\" }]"
                                    .to_string(),
                            );
                        } else if let Some(arr) = fields.as_array() {
                            for (i, field) in arr.iter().enumerate() {
                                if let Some(obj) = field.as_object() {
                                    if !obj.get("key").and_then(|k| k.as_str()).is_some()
                                        || !obj.get("label").and_then(|l| l.as_str()).is_some()
                                    {
                                        errors.push(format!(
                                            "Authentication field {} must have 'key' and 'label' (strings).",
                                            i
                                        ));
                                    } else if let Some(flag) = obj.get("include_in_authorize_url") {
                                        if !flag.is_boolean() {
                                            errors.push(format!(
                                                "Authentication field {} 'include_in_authorize_url' must be a boolean.",
                                                i
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let has_triggers = definition
        .get("triggers")
        .and_then(|t| t.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);

    let has_creates = definition
        .get("creates")
        .and_then(|c| c.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);

    let has_searches = definition
        .get("searches")
        .and_then(|s| s.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);

    if !has_triggers && !has_creates && !has_searches {
        errors.push(
            "Definition must have at least one trigger, create, or search action".to_string(),
        );
    }

    // Validate triggers
    if let Some(triggers) = definition.get("triggers").and_then(|t| t.as_object()) {
        for (key, trigger) in triggers {
            validate_action(&mut errors, "trigger", key, trigger, true);
        }
    }
    validate_ingress(&mut errors, definition);
    validate_central_ingress(&mut errors, definition);

    // Validate creates
    if let Some(creates) = definition.get("creates").and_then(|c| c.as_object()) {
        for (key, create) in creates {
            validate_action(&mut errors, "create", key, create, false);
        }
    }

    // Validate searches
    if let Some(searches) = definition.get("searches").and_then(|s| s.as_object()) {
        for (key, search) in searches {
            validate_action(&mut errors, "search", key, search, false);
        }
    }

    // Validate hydrators (optional)
    if let Some(hydrators) = definition.get("hydrators").and_then(|h| h.as_object()) {
        for (key, hydrator) in hydrators {
            validate_action(&mut errors, "hydrator", key, hydrator, false);
        }
    }

    validate_dehydrate_pointers(&mut errors, &definition);

    errors
}

fn validate_ingress(errors: &mut Vec<String>, definition: &serde_json::Value) {
    match definition.get("ingress") {
        None => {}
        Some(value) if value.is_null() => {}
        Some(ingress) => match ingress.get("handle").and_then(|handle| handle.as_str()) {
            Some("ingress.handle") => {}
            Some(_) => {
                errors.push("ingress.handle must resolve to ingress.handle".to_string());
            }
            None => {
                errors.push("ingress.handle is required".to_string());
            }
        },
    }
}

fn validate_central_ingress(errors: &mut Vec<String>, definition: &serde_json::Value) {
    let ingress_handler_valid = definition
        .get("ingress")
        .and_then(|ingress| ingress.get("handle"))
        .and_then(|handle| handle.as_str())
        == Some("ingress.handle");

    let Some(triggers) = definition
        .get("triggers")
        .and_then(|value| value.as_object())
    else {
        return;
    };

    let mut has_central_ingress = false;

    for (key, trigger) in triggers {
        let Some(operation) = trigger.get("operation").and_then(|value| value.as_object()) else {
            continue;
        };

        match operation.get("delivery").and_then(|value| value.as_str()) {
            None | Some("direct") => {}
            Some("central_ingress") => {
                has_central_ingress = true;
                if operation.contains_key("perform") {
                    errors.push(format!(
                        "Trigger '{key}' with delivery central_ingress must not define operation.perform; use ingress.handle"
                    ));
                }
                validate_routing(errors, key, operation.get("routing"), operation, definition);
            }
            Some(delivery) => errors.push(format!(
                "Trigger '{key}' has invalid delivery mode: '{delivery}'. Must be one of: direct, central_ingress"
            )),
        }
    }

    if has_central_ingress && !ingress_handler_valid {
        errors.push("central_ingress triggers require ingress.handle".to_string());
    }
}

fn validate_routing(
    errors: &mut Vec<String>,
    trigger_key: &str,
    routing: Option<&serde_json::Value>,
    operation: &serde_json::Map<String, serde_json::Value>,
    definition: &serde_json::Value,
) {
    let Some(routing) = routing.and_then(|value| value.as_object()) else {
        errors.push(format!(
            "Trigger '{trigger_key}' with delivery central_ingress requires a routing object"
        ));
        return;
    };

    let Some(keys) = routing.get("keys").and_then(|value| value.as_array()) else {
        errors.push(format!(
            "Trigger '{trigger_key}' with delivery central_ingress requires routing.keys"
        ));
        return;
    };

    let keys: Vec<&str> = keys
        .iter()
        .filter_map(|value| value.as_str().filter(|key| !key.is_empty()))
        .collect();

    if keys.is_empty()
        || keys.len()
            != routing
                .get("keys")
                .and_then(|value| value.as_array())
                .map_or(0, Vec::len)
    {
        errors.push(format!(
            "Trigger '{trigger_key}' routing.keys must be non-empty strings"
        ));
        return;
    }

    let values = routing.get("values").and_then(|value| value.as_object());
    if routing.get("values").is_some() && values.is_none() {
        errors.push(format!(
            "Trigger '{trigger_key}' routing.values must be an object when present"
        ));
    }
    if let Some(values) = values {
        let unknown: Vec<&str> = values
            .keys()
            .map(String::as_str)
            .filter(|key| !keys.contains(key))
            .collect();
        if !unknown.is_empty() {
            errors.push(format!(
                "Trigger '{trigger_key}' routing.values contains keys not in routing.keys: {}",
                unknown.join(", ")
            ));
        }

        if values
            .values()
            .any(|value| !(value.is_string() || value.is_number() || value.is_boolean()))
        {
            errors.push(format!(
                "Trigger '{trigger_key}' routing.values must contain only scalar values"
            ));
        }
    }

    let from_input = routing.get("from_input").and_then(|value| value.as_array());
    if routing.get("from_input").is_some() && from_input.is_none() {
        errors.push(format!(
            "Trigger '{trigger_key}' routing.from_input must be a list when present"
        ));
    }
    if let Some(from_input) = from_input {
        let unknown: Vec<&str> = from_input
            .iter()
            .filter_map(|value| value.as_str())
            .filter(|key| !keys.contains(key))
            .collect();
        if from_input.iter().any(|value| !value.is_string()) || !unknown.is_empty() {
            errors.push(format!(
                "Trigger '{trigger_key}' routing.from_input must contain only routing.keys"
            ));
        }

        let declared = field_keys(operation.get("input_fields"));
        let unresolved: Vec<&str> = from_input
            .iter()
            .filter_map(|value| value.as_str())
            .filter(|key| !declared.contains(*key))
            .collect();
        if !unresolved.is_empty() {
            errors.push(format!(
                "Trigger '{trigger_key}' routing.from_input references undeclared input fields: {}",
                unresolved.join(", ")
            ));
        }
    }

    let from_credential = routing
        .get("from_credential")
        .and_then(|value| value.as_object());
    if routing.get("from_credential").is_some() && from_credential.is_none() {
        errors.push(format!(
            "Trigger '{trigger_key}' routing.from_credential must be an object when present"
        ));
    }
    if let Some(from_credential) = from_credential {
        let invalid = from_credential.iter().any(|(key, value)| {
            !keys.contains(&key.as_str())
                || value.as_str().map(|field| field.is_empty()).unwrap_or(true)
        });
        if invalid {
            errors.push(format!(
                "Trigger '{trigger_key}' routing.from_credential must map routing.keys to non-empty credential field names"
            ));
        }

        let declared = credential_field_keys(definition);
        let unresolved: Vec<&str> = from_credential
            .values()
            .filter_map(|value| value.as_str())
            .filter(|field| !declared.contains(*field))
            .collect();
        if !unresolved.is_empty() {
            errors.push(format!(
                "Trigger '{trigger_key}' routing.from_credential references undeclared credential fields: {}",
                unresolved.join(", ")
            ));
        }
    }

    let overlapping = overlapping_routing_keys(values, from_input, from_credential);
    if !overlapping.is_empty() {
        errors.push(format!(
            "Trigger '{trigger_key}' routing keys must not appear in more than one of values, from_input, or from_credential: {}",
            overlapping.join(", ")
        ));
    }

    let covered: std::collections::HashSet<&str> = values
        .into_iter()
        .flat_map(|map| map.keys().map(String::as_str))
        .chain(
            from_input
                .into_iter()
                .flatten()
                .filter_map(|value| value.as_str()),
        )
        .chain(
            from_credential
                .into_iter()
                .flat_map(|map| map.keys().map(String::as_str)),
        )
        .collect();

    let missing: Vec<&str> = keys
        .into_iter()
        .filter(|key| !covered.contains(key))
        .collect();

    if !missing.is_empty() {
        errors.push(format!(
            "Trigger '{trigger_key}' routing.keys must each be covered by values, from_input, or from_credential. Missing: {}",
            missing.join(", ")
        ));
    }

    let has_input = from_input.map(|list| !list.is_empty()).unwrap_or(false);
    let has_credential = from_credential.map(|map| !map.is_empty()).unwrap_or(false);
    if !has_input && !has_credential {
        errors.push(format!(
            "Trigger '{trigger_key}' central_ingress routing must include from_input or from_credential so subscriptions are tenant-scoped"
        ));
    }
}

const STANDARD_CREDENTIAL_FIELDS: &[&str] = &[
    "access_token",
    "refresh_token",
    "expires_at",
    "token_type",
];

fn field_keys(fields: Option<&serde_json::Value>) -> std::collections::HashSet<&str> {
    fields
        .and_then(|value| value.as_array())
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| {
                    field
                        .get("key")
                        .and_then(|key| key.as_str())
                        .filter(|key| !key.is_empty())
                })
                .collect()
        })
        .unwrap_or_default()
}

fn credential_field_keys(definition: &serde_json::Value) -> std::collections::HashSet<&str> {
    let mut keys = field_keys(
        definition
            .get("authentication")
            .and_then(|auth| auth.get("fields")),
    );
    keys.extend(STANDARD_CREDENTIAL_FIELDS.iter().copied());
    keys
}

fn overlapping_routing_keys<'a>(
    values: Option<&'a serde_json::Map<String, serde_json::Value>>,
    from_input: Option<&'a Vec<serde_json::Value>>,
    from_credential: Option<&'a serde_json::Map<String, serde_json::Value>>,
) -> Vec<&'a str> {
    let value_keys: std::collections::HashSet<&str> = values
        .into_iter()
        .flat_map(|map| map.keys().map(String::as_str))
        .collect();
    let input_keys: std::collections::HashSet<&str> = from_input
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str())
        .collect();
    let credential_keys: std::collections::HashSet<&str> = from_credential
        .into_iter()
        .flat_map(|map| map.keys().map(String::as_str))
        .collect();

    let mut overlapping: Vec<&str> = value_keys
        .intersection(&input_keys)
        .chain(value_keys.intersection(&credential_keys))
        .chain(input_keys.intersection(&credential_keys))
        .copied()
        .collect();
    overlapping.sort_unstable();
    overlapping.dedup();
    overlapping
}

const DEHYDRATE_FILE_PREFIX: &str = "whippy:dehydrate-file:";

fn validate_dehydrate_pointers(errors: &mut Vec<String>, definition: &serde_json::Value) {
    let hydrator_keys: std::collections::HashSet<String> = definition
        .get("hydrators")
        .and_then(|h| h.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default();

    let mut pointers: Vec<String> = Vec::new();
    collect_json_strings(definition, &mut pointers);

    for pointer in pointers {
        if !pointer.starts_with(DEHYDRATE_FILE_PREFIX) {
            continue;
        }

        let encoded = pointer.trim_start_matches(DEHYDRATE_FILE_PREFIX);
        if encoded.is_empty() || encoded.contains("...") {
            continue;
        }

        let decoded = match base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            encoded,
        ) {
            Ok(bytes) => bytes,
            Err(_) => {
                errors.push(format!(
                    "Invalid dehydrated file pointer (base64 decode failed): {}",
                    truncate_str(&pointer, 80)
                ));
                continue;
            }
        };

        let payload: serde_json::Value = match serde_json::from_slice(&decoded) {
            Ok(v) => v,
            Err(_) => {
                errors.push(format!(
                    "Invalid dehydrated file pointer (JSON parse failed): {}",
                    truncate_str(&pointer, 80)
                ));
                continue;
            }
        };

        let hydrator_key = payload
            .get("hydrator")
            .or_else(|| payload.get("hydrator_key"))
            .and_then(|v| v.as_str());

        match hydrator_key {
            None => errors.push(
                "Dehydrated file pointer missing 'hydrator' field in sample or output"
                    .to_string(),
            ),
            Some(key) if hydrator_keys.is_empty() => errors.push(format!(
                "Dehydrated file pointer references hydrator '{}' but no hydrators are defined",
                key
            )),
            Some(key) if !hydrator_keys.contains(key) => errors.push(format!(
                "Dehydrated file pointer references unknown hydrator '{}'. Register it under hydrators in index exports.",
                key
            )),
            Some(_) => {}
        }
    }
}

fn collect_json_strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_json_strings(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                collect_json_strings(item, out);
            }
        }
        _ => {}
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

fn validate_session_config(errors: &mut Vec<String>, auth: &serde_json::Value) {
    let Some(session_config) = auth.get("session_config") else {
        return;
    };
    if !session_config.is_object() {
        errors.push("session_config must be an object when present".to_string());
        return;
    }
    if let Some(perform) = session_config.get("perform") {
        if !perform.as_str().map(|s| !s.is_empty()).unwrap_or(false) {
            errors
                .push("session_config.perform must be a non-empty string when present".to_string());
        }
    }
}

fn validate_oauth2_config(errors: &mut Vec<String>, auth: &serde_json::Value) {
    let Some(config) = auth.get("oauth2_config") else {
        errors.push(
            "OAuth2 authentication requires 'oauth2_config' with authorize_url, and get_access_token on the auth object (the function; build emits the path)."
                .to_string(),
        );
        return;
    };
    let Some(obj) = config.as_object() else {
        errors.push("OAuth2 authentication: 'oauth2_config' must be an object.".to_string());
        return;
    };
    // authorize_url: string or object with url (and optional params)
    match obj.get("authorize_url") {
        None => {
            errors.push(
                "OAuth2 oauth2_config must include 'authorize_url' (string or object with 'url' and optional 'params')."
                    .to_string(),
            );
        }
        Some(serde_json::Value::String(s)) if s.is_empty() => {
            errors.push(
                "OAuth2 oauth2_config 'authorize_url' must be a non-empty string.".to_string(),
            );
        }
        Some(serde_json::Value::String(_)) => {}
        Some(serde_json::Value::Object(o)) => {
            if !o
                .get("url")
                .and_then(|v| v.as_str())
                .map_or(false, |s| !s.is_empty())
            {
                errors.push(
                    "OAuth2 oauth2_config 'authorize_url' object must have a non-empty 'url' string."
                        .to_string(),
                );
            }
        }
        Some(_) => {
            errors.push(
                "OAuth2 oauth2_config 'authorize_url' must be a string or an object with 'url' (and optional 'params')."
                    .to_string(),
            );
        }
    }
    // get_access_token: required on auth object (build emits path from the function)
    match auth.get("get_access_token").and_then(|v| v.as_str()) {
        None => {
            errors.push(
                "OAuth2 authentication must include get_access_token (the function; build emits the path)."
                    .to_string(),
            );
        }
        Some(s) if s.is_empty() => {
            errors.push(
                "OAuth2 authentication 'get_access_token' must be a non-empty string.".to_string(),
            );
        }
        Some(_) => {}
    }
    // Optional: refresh_access_token, test — if present must be non-empty string
    for key in ["refresh_access_token", "test"] {
        if let Some(v) = auth.get(key) {
            if !v.as_str().map_or(false, |s| !s.is_empty()) {
                errors.push(format!(
                    "OAuth2 authentication '{}' must be a non-empty string when present.",
                    key
                ));
            }
        }
    }
}

pub fn run() -> Result<()> {
    println!("Validating integration...");

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Check if app.json exists, if not suggest running build first
    let app_json_path = current_dir.join("app.json");
    if !app_json_path.exists() {
        println!("⚠ app.json not found. Running build to generate it...");
        crate::commands::build::run()?;
    }

    // Read and parse app.json
    let content = fs::read_to_string(&app_json_path).context("Failed to read app.json")?;

    let definition: serde_json::Value =
        serde_json::from_str(&content).context("Failed to parse app.json as JSON")?;

    let errors = validate_definition(&definition);
    let has_triggers = definition
        .get("triggers")
        .and_then(|t| t.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);
    let has_creates = definition
        .get("creates")
        .and_then(|c| c.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);
    let has_searches = definition
        .get("searches")
        .and_then(|s| s.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);
    let has_hydrators = definition
        .get("hydrators")
        .and_then(|h| h.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);

    if errors.is_empty() {
        println!("✓ Validation passed!");
        println!("  - All required fields present");
        println!("  - Structure is valid");
        if has_triggers {
            println!("  - Triggers: ✓");
        }
        if has_creates {
            println!("  - Creates: ✓");
        }
        if has_searches {
            println!("  - Searches: ✓");
        }
        if has_hydrators {
            println!("  - Hydrators: ✓");
        }
        Ok(())
    } else {
        println!("✗ Validation failed with {} error(s):\n", errors.len());
        for (i, error) in errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
        anyhow::bail!("Validation failed");
    }
}

fn validate_action(
    errors: &mut Vec<String>,
    action_type: &str,
    key: &str,
    action: &serde_json::Value,
    is_trigger: bool,
) {
    if !action.is_object() {
        errors.push(format!("{} '{}' must be an object", action_type, key));
        return;
    }

    // Check key field matches
    if let Some(action_key) = action.get("key").and_then(|k| k.as_str()) {
        if action_key != key {
            errors.push(format!(
                "{} '{}' has mismatched key: '{}'",
                action_type, key, action_key
            ));
        }
    } else {
        errors.push(format!("{} '{}' must have a 'key' field", action_type, key));
    }

    // Check noun
    if !action.get("noun").and_then(|n| n.as_str()).is_some() {
        errors.push(format!(
            "{} '{}' must have a 'noun' field",
            action_type, key
        ));
    }

    // Check display
    if let Some(display) = action.get("display") {
        if !display.get("label").and_then(|l| l.as_str()).is_some() {
            errors.push(format!(
                "{} '{}' display must have a 'label' field",
                action_type, key
            ));
        }
    } else {
        errors.push(format!(
            "{} '{}' must have a 'display' object with 'label'",
            action_type, key
        ));
    }

    // Check operation
    if !action.get("operation").is_some() {
        errors.push(format!(
            "{} '{}' must have an 'operation' object",
            action_type, key
        ));
    } else if let Some(operation) = action.get("operation") {
        // For triggers, check type
        if is_trigger {
            match operation.get("type").and_then(|t| t.as_str()) {
                Some("webhook") => {}
                Some(op_type) => {
                    errors.push(format!(
                        "Trigger '{}' has invalid operation type: '{}'. Must be: webhook",
                        key, op_type
                    ));
                }
                None => {
                    errors.push(format!(
                        "Trigger '{}' operation.type must be \"webhook\"",
                        key
                    ));
                }
            }
        }

        // Check input_fields (if present, must be array)
        if let Some(input_fields) = operation.get("input_fields") {
            if !input_fields.is_array() {
                errors.push(format!(
                    "{} '{}' input_fields must be a list",
                    action_type, key
                ));
            } else if let Some(fields) = input_fields.as_array() {
                for (i, field) in fields.iter().enumerate() {
                    if !field.is_object() {
                        errors.push(format!(
                            "{} '{}' input field {} must be a map",
                            action_type, key, i
                        ));
                    } else {
                        if !field.get("key").and_then(|k| k.as_str()).is_some() {
                            errors.push(format!(
                                "{} '{}' input field {} must have 'key' and 'label' fields",
                                action_type, key, i
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid_definition() -> serde_json::Value {
        serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{
                    "key": "api_key",
                    "label": "API Key",
                    "help_text": "Create or copy your API key from the provider developer settings page."
                }]
            },
            "creates": {
                "create_contact": {
                    "key": "create_contact",
                    "noun": "Contact",
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "sample": {
                            "data": {
                                "id": "123",
                                "name": "Test User",
                                "email": "test@example.com"
                            }
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn validate_definition_accepts_minimal_valid() {
        let def = minimal_valid_definition();
        let errors = validate_definition(&def);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_definition_accepts_ingress_null() {
        let mut def = minimal_valid_definition();
        def["ingress"] = serde_json::Value::Null;
        let errors = validate_definition(&def);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_definition_rejects_empty_ingress_object() {
        let mut def = minimal_valid_definition();
        def["ingress"] = serde_json::json!({});
        let errors = validate_definition(&def);
        assert!(errors
            .iter()
            .any(|error| error == "ingress.handle is required"));
    }

    #[test]
    fn validate_definition_rejects_invalid_ingress_handle() {
        let mut def = minimal_valid_definition();
        def["ingress"] = serde_json::json!({ "handle": "other.handle" });
        let errors = validate_definition(&def);
        assert!(errors
            .iter()
            .any(|error| error == "ingress.handle must resolve to ingress.handle"));
    }

    #[test]
    fn validate_definition_accepts_central_ingress() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "custom",
                "fields": [{ "key": "team_id", "label": "Team ID", "help_text": "Slack team ID" }]
            },
            "ingress": { "handle": "ingress.handle" },
            "triggers": {
                "app_mention": {
                    "key": "app_mention",
                    "noun": "Mention",
                    "display": { "label": "App Mention" },
                    "operation": {
                        "type": "webhook",
                        "sample": { "event": "app_mention" },
                        "delivery": "central_ingress",
                        "routing": {
                            "keys": ["tenant_id", "event_type"],
                            "values": { "event_type": "app_mention" },
                            "from_credential": { "tenant_id": "team_id" }
                        }
                    }
                }
            }
        });

        let errors = validate_definition(&def);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_definition_rejects_central_ingress_without_handler() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": { "type": "none" },
            "triggers": {
                "app_mention": {
                    "key": "app_mention",
                    "noun": "Mention",
                    "display": { "label": "App Mention" },
                    "operation": {
                        "type": "webhook",
                        "sample": { "event": "app_mention" },
                        "delivery": "central_ingress",
                        "routing": {
                            "keys": ["tenant_id"],
                            "from_credential": { "tenant_id": "team_id" }
                        }
                    }
                }
            }
        });

        let errors = validate_definition(&def);
        assert!(errors
            .iter()
            .any(|error| error == "central_ingress triggers require ingress.handle"));
    }

    #[test]
    fn validate_definition_rejects_central_ingress_with_perform() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": { "type": "none" },
            "ingress": { "handle": "ingress.handle" },
            "triggers": {
                "app_mention": {
                    "key": "app_mention",
                    "noun": "Mention",
                    "display": { "label": "App Mention" },
                    "operation": {
                        "type": "webhook",
                        "sample": { "event": "app_mention" },
                        "perform": {},
                        "delivery": "central_ingress",
                        "routing": {
                            "keys": ["tenant_id", "event_type"],
                            "values": { "event_type": "app_mention" },
                            "from_credential": { "tenant_id": "team_id" }
                        }
                    }
                }
            }
        });

        let errors = validate_definition(&def);
        assert!(errors.iter().any(|error| error.contains(
            "must not define operation.perform; use ingress.handle"
        )));
    }

    #[test]
    fn validate_definition_rejects_central_ingress_without_tenant_source() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": { "type": "none" },
            "ingress": { "handle": "ingress.handle" },
            "triggers": {
                "app_mention": {
                    "key": "app_mention",
                    "noun": "Mention",
                    "display": { "label": "App Mention" },
                    "operation": {
                        "type": "webhook",
                        "sample": { "event": "app_mention" },
                        "delivery": "central_ingress",
                        "routing": {
                            "keys": ["event_type"],
                            "values": { "event_type": "app_mention" }
                        }
                    }
                }
            }
        });

        let errors = validate_definition(&def);
        assert!(errors.iter().any(|error| error.contains(
            "must include from_input or from_credential so subscriptions are tenant-scoped"
        )));
    }

    #[test]
    fn validate_definition_rejects_overlapping_routing_sources() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "custom",
                "fields": [{ "key": "team_id", "label": "Team ID", "help_text": "Slack team ID" }]
            },
            "ingress": { "handle": "ingress.handle" },
            "triggers": {
                "app_mention": {
                    "key": "app_mention",
                    "noun": "Mention",
                    "display": { "label": "App Mention" },
                    "operation": {
                        "type": "webhook",
                        "sample": { "event": "app_mention" },
                        "delivery": "central_ingress",
                        "routing": {
                            "keys": ["tenant_id"],
                            "values": { "tenant_id": "shared" },
                            "from_credential": { "tenant_id": "team_id" }
                        }
                    }
                }
            }
        });

        let errors = validate_definition(&def);
        assert!(errors.iter().any(|error| error.contains(
            "must not appear in more than one of values, from_input, or from_credential"
        )));
    }

    #[test]
    fn validate_definition_rejects_undeclared_from_input_and_credential_fields() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": { "type": "none" },
            "ingress": { "handle": "ingress.handle" },
            "triggers": {
                "app_mention": {
                    "key": "app_mention",
                    "noun": "Mention",
                    "display": { "label": "App Mention" },
                    "operation": {
                        "type": "webhook",
                        "sample": { "event": "app_mention" },
                        "delivery": "central_ingress",
                        "routing": {
                            "keys": ["tenant_id", "channel_id"],
                            "from_input": ["channel_id"],
                            "from_credential": { "tenant_id": "team_id" }
                        }
                    }
                }
            }
        });

        let errors = validate_definition(&def);
        assert!(errors.iter().any(|error| error.contains(
            "routing.from_input references undeclared input fields"
        )));
        assert!(errors.iter().any(|error| error.contains(
            "routing.from_credential references undeclared credential fields"
        )));
    }

    #[test]
    fn validate_definition_rejects_missing_version() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().remove("version");
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("version")));
    }

    #[test]
    fn validate_definition_rejects_invalid_semver() {
        let mut def = minimal_valid_definition();
        def.as_object_mut()
            .unwrap()
            .insert("version".to_string(), serde_json::json!("not-semver"));
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("semver") || e.contains("Version")));
    }

    #[test]
    fn validate_definition_rejects_missing_platform_version() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().remove("platform_version");
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("platform_version")));
    }

    #[test]
    fn validate_definition_rejects_missing_authentication() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().remove("authentication");
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("authentication")));
    }

    #[test]
    fn validate_definition_rejects_no_actions() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [{ "key": "api_key", "label": "API Key" }]
            },
            "creates": {},
            "triggers": {},
            "searches": {}
        });
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("at least one")));
    }

    #[test]
    fn validate_definition_rejects_invalid_auth_type() {
        let mut def = minimal_valid_definition();
        def.as_object_mut()
            .unwrap()
            .get_mut("authentication")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("type".to_string(), serde_json::json!("invalid_type"));
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("authentication") || e.contains("Invalid")));
    }

    fn minimal_valid_oauth2_definition() -> serde_json::Value {
        serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "oauth2",
                "fields": [],
                "oauth2_config": {
                    "authorize_url": "https://example.com/oauth/authorize",
                    "client_id": "client",
                    "client_secret": "secret",
                    "scopes": "read"
                },
                "get_access_token": "authentication.get_access_token",
                "refresh_access_token": "authentication.refresh_access_token",
                "test": "authentication.test"
            },
            "creates": {
                "create_contact": {
                    "key": "create_contact",
                    "noun": "Contact",
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "sample": { "data": { "id": "123", "name": "Test User" } }
                    }
                }
            }
        })
    }

    #[test]
    fn validate_definition_accepts_oauth2_with_required_config() {
        let def = minimal_valid_oauth2_definition();
        let errors = validate_definition(&def);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_definition_rejects_oauth2_without_oauth2_config() {
        let mut def = minimal_valid_oauth2_definition();
        def.as_object_mut()
            .unwrap()
            .get_mut("authentication")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("oauth2_config");
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("oauth2_config")));
    }

    #[test]
    fn validate_definition_rejects_oauth2_without_authorize_url() {
        let mut def = minimal_valid_oauth2_definition();
        def.as_object_mut()
            .unwrap()
            .get_mut("authentication")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .get_mut("oauth2_config")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("authorize_url");
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("authorize_url")));
    }

    #[test]
    fn validate_definition_rejects_oauth2_without_get_access_token() {
        let mut def = minimal_valid_oauth2_definition();
        def.as_object_mut()
            .unwrap()
            .get_mut("authentication")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("get_access_token");
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("get_access_token")));
    }

    #[test]
    fn validate_definition_rejects_trigger_invalid_operation_type() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().insert(
            "triggers".to_string(),
            serde_json::json!({
                "new_thing": {
                    "key": "new_thing",
                    "noun": "Thing",
                    "display": { "label": "New Thing" },
                    "operation": { "type": "invalid", "sample": { "id": "123" } }
                }
            }),
        );
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("operation type") || e.contains("webhook")));
    }

    #[test]
    fn validate_definition_rejects_trigger_polling_operation_type() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().insert(
            "triggers".to_string(),
            serde_json::json!({
                "new_thing": {
                    "key": "new_thing",
                    "noun": "Thing",
                    "display": { "label": "New Thing" },
                    "operation": { "type": "polling", "sample": { "id": "123" } }
                }
            }),
        );
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("webhook")));
    }

    #[test]
    fn validate_definition_rejects_invalid_input_field_type() {
        let mut def = minimal_valid_definition();
        def.as_object_mut()
            .unwrap()
            .get_mut("creates")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .get_mut("create_contact")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .get_mut("operation")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "input_fields".to_string(),
                serde_json::json!([{
                    "key": "name",
                    "label": "Full name",
                    "type": "strng",
                    "help_text": "The contact full name as it should appear in the CRM."
                }]),
            );
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains(".type must be one of")));
    }

    #[test]
    fn validate_definition_rejects_create_missing_key() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().insert(
            "creates".to_string(),
            serde_json::json!({
                "bad_action": {
                    "noun": "Thing",
                    "display": { "label": "Thing" },
                    "operation": { "sample": { "id": "123" } }
                }
            }),
        );
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("key") || e.contains("'key'")));
    }

    fn auth_field() -> serde_json::Value {
        serde_json::json!({
            "key": "api_key",
            "label": "API Key",
            "help_text": "Create or copy your API key from the provider developer settings page."
        })
    }

    #[test]
    fn validate_definition_accepts_hydrators() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [auth_field()]
            },
            "triggers": {
                "new_file": {
                    "key": "new_file",
                    "noun": "File",
                    "display": { "label": "New File" },
                    "operation": {
                        "type": "webhook",
                        "sample": {
                            "document": "whippy:dehydrate-file:eyJoeWRyYXRvciI6ImRvd25sb2FkX2ZpbGUiLCJpbnB1dERhdGEiOnsiaWQiOiIxIn0sImFwcF92ZXJzaW9uX2lkIjoidGVzdCJ9",
                            "filename": "resume.pdf"
                        }
                    }
                }
            },
            "hydrators": {
                "download_file": {
                    "key": "download_file",
                    "noun": "File",
                    "display": { "label": "Download File" },
                    "operation": {
                        "sample": {
                            "data": {
                                "url": "https://example.com/file.pdf",
                                "filename": "file.pdf"
                            }
                        }
                    }
                }
            }
        });
        let errors = validate_definition(&def);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn validate_definition_rejects_unknown_hydrator_in_pointer() {
        let def = serde_json::json!({
            "version": "1.0.0",
            "platform_version": "1.0",
            "authentication": {
                "type": "api_key",
                "fields": [auth_field()]
            },
            "creates": {
                "upload": {
                    "key": "upload",
                    "noun": "File",
                    "display": { "label": "Upload" },
                    "operation": {
                        "sample": {
                            "data": {
                                "file": "whippy:dehydrate-file:eyJoeWRyYXRvciI6Im1pc3NpbmciLCJpbnB1dERhdGEiOnt9LCJhcHBfdmVyc2lvbl9pZCI6InRlc3QifQ",
                                "filename": "upload.pdf"
                            }
                        }
                    }
                }
            },
            "hydrators": {
                "download_file": {
                    "key": "download_file",
                    "noun": "File",
                    "display": { "label": "Download File" },
                    "operation": {
                        "sample": {
                            "data": {
                                "url": "https://example.com/file.pdf",
                                "filename": "file.pdf"
                            }
                        }
                    }
                }
            }
        });
        let errors = validate_definition(&def);
        assert!(errors.iter().any(|e| e.contains("unknown hydrator")));
    }

    #[test]
    fn validate_definition_rejects_unknown_operation_key() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().insert(
            "creates".to_string(),
            serde_json::json!({
                "create_contact": {
                    "key": "create_contact",
                    "noun": "Contact",
                    "display": { "label": "Create Contact" },
                    "operation": { "not_allowed": true }
                }
            }),
        );
        let errors = validate_definition(&def);
        assert!(errors
            .iter()
            .any(|e| e.contains("Unknown key at creates.create_contact.operation")));
    }

    #[test]
    fn validate_definition_accepts_dynamic_input_fields() {
        let mut def = minimal_valid_definition();
        def.as_object_mut().unwrap().insert(
            "creates".to_string(),
            serde_json::json!({
                "create_contact": {
                    "key": "create_contact",
                    "noun": "Contact",
                    "display": { "label": "Create Contact" },
                    "operation": {
                        "dynamic_input_fields": {
                            "perform": "get_worksheet_row_input_fields",
                            "required_input_keys": ["spreadsheet_id", "worksheet_title"]
                        }
                    }
                }
            }),
        );
        let errors = validate_definition(&def);
        assert!(
            !errors.iter().any(|e| e.contains("dynamic_input_fields")),
            "unexpected errors: {:?}",
            errors
        );
    }

    #[test]
    fn validate_definition_rejects_unnormalized_create_sample() {
        let mut def = minimal_valid_definition();
        def.as_object_mut()
            .unwrap()
            .get_mut("creates")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .get_mut("create_contact")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .get_mut("operation")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "sample".to_string(),
                serde_json::json!({ "id": "123", "name": "Test User" }),
            );
        let errors = validate_definition(&def);
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.contains("data") && e.contains("ww-cli build")));
    }

    #[test]
    fn truncate_str_does_not_panic_on_multibyte_utf8() {
        let s = "é".repeat(40);
        let truncated = truncate_str(&s, 10);
        assert!(truncated.ends_with("..."));
        assert_eq!(truncated.chars().filter(|c| *c == 'é').count(), 10);
    }
}
