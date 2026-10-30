use anyhow::{Context, Result};
use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use crate::commands::build;
use serde_json::Value;

pub fn run(function: Option<String>, data: Option<PathBuf>, _app_id: Option<String>) -> Result<()> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Check if build exists, build if needed
    let build_dir = current_dir.join(".build");
    if !build_dir.exists() || !build_dir.join("index.js").exists() {
        println!("Build not found. Building...");
        build::run()?;
    }

    // Load app.json to understand the function structure
    let app_json_path = current_dir.join("app.json");
    if !app_json_path.exists() {
        anyhow::bail!("app.json not found. Run 'whippy-workflows-cli build' first.");
    }

    let app_json: Value = serde_json::from_str(
        &fs::read_to_string(&app_json_path).context("Failed to read app.json")?,
    )?;

    // If no function provided, just show available functions and exit
    let function = match function {
        Some(f) => f,
        None => {
            println!("Available functions:");
            print_available_functions(&app_json);
            println!("\nUsage: whippy-workflows-cli invoke <function_name> [--data <file>]");
            println!("Example: whippy-workflows-cli invoke create_contact");
            println!("With auth: whippy-workflows-cli invoke create_contact --data test-data.json");
            return Ok(());
        }
    };

    println!("Invoking function: {}", function);

    // Resolve function path (support both full path and short form)
    let function_path = resolve_function_path(&function, &app_json)?;
    println!("Function path: {}\n", function_path);

    // Load or generate test bundle
    let bundle = if let Some(data_path) = data {
        load_bundle_from_file(&data_path, &app_json)?
    } else {
        println!("Using sample bundle (placeholder auth). For real auth: whippy-workflows-cli invoke {} --data test-data.json\n", function);
        generate_sample_bundle(&function_path, &app_json)?
    };

    const WRANGLER_PORT: u16 = 8787;
    let wrangler_url = format!("http://localhost:{}", WRANGLER_PORT);

    if port_in_use(WRANGLER_PORT) {
        anyhow::bail!(
            "Port {} is already in use. Stop the process using it before running invoke (e.g. lsof -i :{}).",
            WRANGLER_PORT,
            WRANGLER_PORT
        );
    }
    println!("Starting wrangler dev server...");

    // Check if wrangler is available (try global first, then npx)
    let (wrangler_cmd, use_npx) = if Command::new("wrangler").arg("--version").output().is_ok() {
        ("wrangler", false)
    } else {
        // Try npx wrangler
        if Command::new("npx").arg("--version").output().is_ok() {
            ("npx", true)
        } else {
            anyhow::bail!(
                "wrangler not found. Install it with: npm install -g wrangler\n\
                Or ensure npx is available (comes with npm)"
            );
        }
    };

    // Start wrangler dev in the background
    // Note: We let wrangler output to terminal so user can see what's happening
    let mut wrangler_cmd_builder = Command::new(wrangler_cmd);
    if use_npx {
        wrangler_cmd_builder.arg("wrangler");
    }
    // Use stdin(Stdio::null()) so wrangler/Node never read from the terminal.
    // Otherwise killing wrangler can cause "read EIO" and leave the shell stuck.
    // On Unix, put wrangler in its own process group so we can kill the whole tree
    // (wrangler + workerd + esbuild) on exit and avoid orphaned processes / deadlocks.
    #[cfg(unix)]
    wrangler_cmd_builder.process_group(0);
    let wrangler_process = wrangler_cmd_builder
        .stdin(Stdio::null())
        .arg("dev")
        .arg("--port")
        .arg(WRANGLER_PORT.to_string())
        .arg("--local")
        .current_dir(&build_dir)
        .spawn()
        .context("Failed to start wrangler dev server")?;

    // Ensure wrangler (and its workerd child) is killed on every exit path (success, bail, or panic)
    let mut wrangler_guard = KillOnDrop(Some(wrangler_process), WRANGLER_PORT);

    // Wait for server to be ready by polling
    println!("Waiting for wrangler dev server to start (this may take 10-20 seconds)...");
    let poll_addr = SocketAddr::from(([127, 0, 0, 1], WRANGLER_PORT));
    let max_attempts = 30; // 30 seconds max
    let mut ready = false;
    for attempt in 1..=max_attempts {
        thread::sleep(Duration::from_secs(1));

        if TcpStream::connect_timeout(&poll_addr, Duration::from_secs(2)).is_ok() {
            ready = true;
            break;
        }
        if attempt % 3 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().ok();
        }

        // Check if process died
        if wrangler_guard
            .0
            .as_mut()
            .and_then(|c| c.try_wait().ok())
            .flatten()
            .is_some()
        {
            anyhow::bail!(
                "Wrangler dev server process exited unexpectedly. \
                Check the output above for errors. Common issues:\n\
                - Missing wrangler.toml (should be auto-generated by build)\n\
                - Port {} already in use\n\
                - Syntax errors in your code",
                WRANGLER_PORT
            );
        }
    }

    if !ready {
        anyhow::bail!(
            "Wrangler dev server did not start within {} seconds.\n\
            Possible issues:\n\
            - Port {} is already in use (try: lsof -i :{})\n\
            - Wrangler is still initializing (try waiting longer)\n\
            - Check wrangler output above for errors",
            max_attempts,
            WRANGLER_PORT,
            WRANGLER_PORT
        );
    }

    println!("\n✓ Server is ready");

    // Send request to local worker
    let client = reqwest::blocking::Client::new();
    let request_body = serde_json::json!({
        "function": function_path,
        "bundle": bundle
    });

    println!("\nSending request to {}...", wrangler_url);
    println!(
        "Request body: {}",
        serde_json::to_string_pretty(&request_body)?
    );

    let response = client
        .post(&wrangler_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .context("Failed to send request to wrangler dev server")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("Request failed with status {}: {}", status, body);
    }

    let result: Value = response.json().context("Failed to parse response")?;

    println!("\n✓ Function executed successfully!");
    println!("Result:");
    println!("{}", serde_json::to_string_pretty(&result)?);

    Ok(())
}

/// True if something is already listening on the given port (e.g. another app or leftover wrangler).
fn port_in_use(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

/// Holds the wrangler child process and kills it and its whole process tree on drop
/// (wrangler + workerd + esbuild) so no dangling process is left and the next invoke can use the port.
struct KillOnDrop(pub Option<Child>, u16);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let pid = self.0.as_ref().map(Child::id);
        if let Some(ref mut child) = self.0 {
            let _ = child.kill();
        }
        // On Unix we started wrangler in its own process group; kill the whole group
        // so workerd and esbuild (and any other children) are terminated and don't deadlock.
        #[cfg(unix)]
        if let Some(pid) = pid {
            kill_process_group(pid);
        }
        // Fallback: clean up anything still on our port (e.g. if process group kill missed something).
        kill_processes_on_port(self.1);
    }
}

/// Kills the process group whose leader has the given pid (Unix). Used so wrangler + workerd + esbuild all exit.
#[cfg(unix)]
fn kill_process_group(pid: u32) {
    let _ = Command::new("kill")
        .args(["-9", &format!("-{}", pid)])
        .output();
}

/// Kills any process listening on the given port (e.g. workerd left by wrangler).
/// Only used when we're exiting invoke after having started wrangler, so the port is ours.
#[cfg(unix)]
fn kill_processes_on_port(port: u16) {
    let port_str = format!(":{}", port);
    if let Ok(out) = Command::new("lsof").args(["-ti", &port_str]).output() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for s in stdout.split_whitespace() {
            if let Ok(pid) = s.parse::<i32>() {
                let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
            }
        }
    }
}

#[cfg(not(unix))]
fn kill_processes_on_port(_port: u16) {}

/// Resolves function path from short form (e.g., "create_contact") to full path
/// (e.g., "creates.create_contact.operation.perform")
fn resolve_function_path(function: &str, app_json: &Value) -> Result<String> {
    // If it's already a full path, return as-is
    if function.contains('.')
        && (function.starts_with("creates.")
            || function.starts_with("searches.")
            || function.starts_with("triggers.")
            || function.starts_with("hydrators.")
            || function.starts_with("ingress.")
            || function.starts_with("authentication."))
    {
        return Ok(function.to_string());
    }

    // Try to find in creates
    if let Some(creates) = app_json.get("creates").and_then(|c| c.as_object()) {
        if creates.contains_key(function) {
            return Ok(format!("creates.{}.operation.perform", function));
        }
    }

    // Try to find in searches
    if let Some(searches) = app_json.get("searches").and_then(|s| s.as_object()) {
        if searches.contains_key(function) {
            return Ok(format!("searches.{}.operation.perform", function));
        }
    }

    // Try to find in triggers
    if let Some(triggers) = app_json.get("triggers").and_then(|t| t.as_object()) {
        if triggers.contains_key(function) {
            return Ok(format!("triggers.{}.operation.perform", function));
        }
    }

    // Try to find in hydrators
    if let Some(hydrators) = app_json.get("hydrators").and_then(|h| h.as_object()) {
        if hydrators.contains_key(function) {
            return Ok(format!("hydrators.{}.operation.perform", function));
        }
    }

    let creates = get_function_list(app_json.get("creates"));
    let searches = get_function_list(app_json.get("searches"));
    let triggers = get_function_list(app_json.get("triggers"));
    let hydrators = get_function_list(app_json.get("hydrators"));
    let ingress = get_function_list(app_json.get("ingress"));
    let auth_note = if app_json
        .get("authentication")
        .and_then(|a| a.get("oauth2_config"))
        .is_some()
    {
        "\n          OAuth2: authentication.get_access_token, authentication.refresh_access_token, authentication.test"
    } else {
        ""
    };
    anyhow::bail!(
        "Function '{}' not found in app definition.\n\n\
        Usage:\n\
          whippy-workflows-cli invoke <function_name>\n\
          whippy-workflows-cli invoke <full_path>\n\n\
        Examples:\n\
          whippy-workflows-cli invoke create_contact\n\
          whippy-workflows-cli invoke creates.create_contact.operation.perform\n\
          whippy-workflows-cli invoke authentication.get_access_token --data test.json\n\n\
        Available functions:\n\
          Creates: {}\n\
          Searches: {}\n\
          Triggers: {}\n\
          Hydrators: {}\n\
          Ingress: {}{}\n\n\
        Tip: Use the short form (e.g., \"create_contact\") or the full path\n\
        (e.g., \"creates.create_contact.operation.perform\").",
        function,
        creates,
        searches,
        triggers,
        hydrators,
        ingress,
        auth_note
    )
}

fn get_function_list(section: Option<&Value>) -> String {
    if let Some(obj) = section.and_then(|s| s.as_object()) {
        obj.keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        "none".to_string()
    }
}

/// Print available functions as a helper to the user
fn print_available_functions(app_json: &Value) {
    let creates = get_function_list(app_json.get("creates"));
    let searches = get_function_list(app_json.get("searches"));
    let triggers = get_function_list(app_json.get("triggers"));
    let hydrators = get_function_list(app_json.get("hydrators"));
    let ingress = get_function_list(app_json.get("ingress"));

    let mut has_functions = false;

    if creates != "none" {
        println!("  Creates: {}", creates);
        has_functions = true;
    }
    if searches != "none" {
        println!("  Searches: {}", searches);
        has_functions = true;
    }
    if triggers != "none" {
        println!("  Triggers: {}", triggers);
        has_functions = true;
    }
    if hydrators != "none" {
        println!("  Hydrators: {}", hydrators);
        has_functions = true;
    }
    if ingress != "none" {
        println!("  Ingress: {}", ingress);
        has_functions = true;
    }

    if !has_functions {
        println!("  No functions found in app definition.");
    }
}

/// Load bundle from JSON file
fn load_bundle_from_file(data_path: &PathBuf, app_json: &Value) -> Result<Value> {
    let content = fs::read_to_string(data_path).context("Failed to read data file")?;

    let mut bundle: Value =
        serde_json::from_str(&content).context("Failed to parse data file as JSON")?;

    // Ensure required fields exist
    if !bundle.is_object() {
        anyhow::bail!("Bundle data must be a JSON object");
    }

    // Set defaults for missing fields using as_object_mut
    {
        let bundle_obj = bundle.as_object_mut().unwrap();

        if !bundle_obj.contains_key("input_data") {
            bundle_obj.insert("input_data".to_string(), serde_json::json!({}));
        }
        if !bundle_obj.contains_key("auth_data") {
            bundle_obj.insert("auth_data".to_string(), build_auth_data(app_json));
        }
        if !bundle_obj.contains_key("meta") {
            bundle_obj.insert(
                "meta".to_string(),
                serde_json::json!({
                    "workflow_id": "test-workflow",
                    "workflow_version_id": "test-version",
                    "is_loading_sample": false,
                    "page": 1,
                    "limit": 50
                }),
            );
        }
        if !bundle_obj.contains_key("target_url") {
            bundle_obj.insert(
                "target_url".to_string(),
                serde_json::json!("http://localhost:3000/webhook"),
            );
        }
    }

    Ok(bundle)
}

/// Generate sample bundle from app definition
fn generate_sample_bundle(function_path: &str, app_json: &Value) -> Result<Value> {
    // Extract input fields from the function definition
    let input_data = extract_sample_input_data(function_path, app_json)?;
    let auth_data = build_auth_data(app_json);

    let bundle = serde_json::json!({
        "input_data": input_data,
        "auth_data": auth_data,
        "meta": {
            "workflow_id": "test-workflow",
            "workflow_version_id": "test-version",
            "is_loading_sample": false,
            "page": 1,
            "limit": 50
        },
        "target_url": "http://localhost:3000/webhook"
    });

    Ok(bundle)
}

/// Extract sample input data from function definition
fn extract_sample_input_data(function_path: &str, app_json: &Value) -> Result<Value> {
    let parts: Vec<&str> = function_path.split('.').collect();
    if parts.len() < 2 {
        return Ok(serde_json::json!({}));
    }

    let action_type = parts[0]; // "creates", "searches", "triggers", or "hydrators"
    let action_key = parts[1];

    let input_fields = app_json
        .get(action_type)
        .and_then(|a| a.get(action_key))
        .and_then(|a| a.get("operation"))
        .and_then(|o| o.get("input_fields"))
        .and_then(|f| f.as_array());

    let mut input_data = serde_json::Map::new();

    if let Some(fields) = input_fields {
        for field in fields {
            if let Some(key) = field.get("key").and_then(|k| k.as_str()) {
                // Generate sample value based on type
                let sample_value = match field.get("type").and_then(|t| t.as_str()) {
                    Some("string") => serde_json::json!("sample_string"),
                    Some("integer") | Some("number") => serde_json::json!(123),
                    Some("boolean") => serde_json::json!(true),
                    Some("file") => serde_json::json!({
                        "base64": "dGVzdA==",
                        "filename": "sample.txt",
                        "content_type": "text/plain"
                    }),
                    Some("array") => serde_json::json!([]),
                    Some("object") => serde_json::json!({}),
                    _ => serde_json::json!("sample_value"),
                };
                input_data.insert(key.to_string(), sample_value);
            }
        }
    }

    Ok(serde_json::Value::Object(input_data))
}

/// Build auth_data from app definition
fn build_auth_data(app_json: &Value) -> Value {
    if let Some(auth) = app_json.get("authentication") {
        if let Some(fields) = auth.get("fields").and_then(|f| f.as_array()) {
            let mut auth_data = serde_json::Map::new();
            for field in fields {
                if let Some(key) = field.get("key").and_then(|k| k.as_str()) {
                    // Use sample/test values
                    auth_data.insert(key.to_string(), serde_json::json!("test_value"));
                }
            }
            return serde_json::Value::Object(auth_data);
        }
    }
    serde_json::json!({})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_app_json() -> Value {
        serde_json::json!({
            "version": "1.0.0",
            "creates": {
                "create_contact": { "key": "create_contact" },
                "create_user": { "key": "create_user" }
            },
            "searches": {
                "find_contact": { "key": "find_contact" }
            },
            "triggers": {
                "new_contact": {
                    "key": "new_contact",
                    "operation": { "type": "webhook" }
                }
            }
        })
    }

    #[test]
    fn resolve_function_path_short_form_creates() {
        let app = sample_app_json();
        assert_eq!(
            resolve_function_path("create_contact", &app).unwrap(),
            "creates.create_contact.operation.perform"
        );
        assert_eq!(
            resolve_function_path("create_user", &app).unwrap(),
            "creates.create_user.operation.perform"
        );
    }

    #[test]
    fn resolve_function_path_short_form_searches() {
        let app = sample_app_json();
        assert_eq!(
            resolve_function_path("find_contact", &app).unwrap(),
            "searches.find_contact.operation.perform"
        );
    }

    #[test]
    fn resolve_function_path_short_form_triggers() {
        let app = sample_app_json();
        assert_eq!(
            resolve_function_path("new_contact", &app).unwrap(),
            "triggers.new_contact.operation.perform"
        );
    }

    #[test]
    fn resolve_function_path_full_path_passthrough() {
        let app = sample_app_json();
        let full = "creates.create_contact.operation.perform";
        assert_eq!(resolve_function_path(full, &app).unwrap(), full);
    }

    #[test]
    fn resolve_function_path_ingress_passthrough() {
        let app = sample_app_json();
        assert_eq!(
            resolve_function_path("ingress.handle", &app).unwrap(),
            "ingress.handle"
        );
    }

    #[test]
    fn resolve_function_path_unknown_returns_error() {
        let app = sample_app_json();
        assert!(resolve_function_path("nonexistent", &app).is_err());
    }

    #[test]
    fn get_function_list_none_for_missing_section() {
        assert_eq!(get_function_list(None), "none");
    }

    #[test]
    fn get_function_list_empty_for_empty_object() {
        let section = serde_json::json!({});
        assert_eq!(get_function_list(Some(&section)), "");
    }

    #[test]
    fn get_function_list_joins_keys() {
        let section = serde_json::json!({
            "create_contact": {},
            "create_user": {}
        });
        let result = get_function_list(Some(&section));
        assert!(result.contains("create_contact"));
        assert!(result.contains("create_user"));
    }
}
