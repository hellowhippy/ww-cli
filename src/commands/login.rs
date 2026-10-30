use anyhow::{Context, Result};
use std::io::{self, Write};

use crate::api::client;
use crate::utils::config;

pub fn run(api_key: Option<String>) -> Result<()> {
    // Get API key from argument, env var, or prompt
    let key = api_key
        .or_else(|| std::env::var("WHIPPY_API_KEY").ok())
        .or_else(|| prompt_for_api_key().ok())
        .context("API key required")?;

    // Validate API key by making a test request
    println!("Validating API key...");
    validate_api_key(&key)?;

    // Store in ~/.whippyrc
    let config_path = config::get_config_path()?;
    let api_url = config::default_api_url();

    let config = serde_json::json!({
        "api_key": key,
        "api_url": api_url
    });

    // Create parent directory if it doesn't exist
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

    println!("✓ Logged in successfully");
    println!("  API key stored in {}", config_path.display());
    Ok(())
}

/// Read a line from stdin until Enter (LF or CR). Some terminals (e.g. in IDEs)
/// send only CR on Enter, and read_line() would block forever waiting for LF.
fn read_line_stdin() -> Result<String> {
    use std::io::Read;
    let mut input = String::new();
    for byte in io::stdin().bytes() {
        let byte = byte?;
        if byte == b'\n' || byte == b'\r' {
            break;
        }
        input.push(byte as char);
    }
    Ok(input.trim().to_string())
}

fn prompt_for_api_key() -> Result<String> {
    print!("Enter your API key (get it from https://app.whippy.co/developer/keys): ");
    io::stdout().flush()?;

    let key = read_line_stdin()?;

    if key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    Ok(key)
}

fn validate_api_key(api_key: &str) -> Result<()> {
    let api_url = config::default_api_url();
    client::validate_api_key(&api_url, api_key)?;
    Ok(())
}
