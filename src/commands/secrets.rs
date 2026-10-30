use std::io::BufRead;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde_json::Value;

use crate::api::client;
use crate::utils::config;

const SECRETS_AFTER_HELP: &str = r#"Examples:
  ww-cli secrets list
  ww-cli secrets set --file ./vendor-secrets.json
  ww-cli secrets set client_secret --stdin
  ww-cli secrets unset old_key
  ww-cli secrets clear --yes
"#;

#[derive(Debug, Subcommand)]
#[command(
    about = "Encrypted app-level secrets; only key names are ever shown.",
    after_help = SECRETS_AFTER_HELP
)]
pub enum SecretsCommand {
    /// List secret key names
    List,
    /// Merge secrets (--file JSON or KEY + --value / --stdin)
    Set {
        /// JSON file: root object must be string keys to string values (same shape as the API `secrets` field)
        #[arg(long)]
        file: Option<PathBuf>,
        /// Secret key name (omit when using --file)
        name: Option<String>,
        /// Secret value for single-key mode (avoid in shared shells; prefer --stdin)
        #[arg(long)]
        value: Option<String>,
        /// Read secret value for single-key mode from stdin (first line only)
        #[arg(long)]
        stdin: bool,
    },
    /// Remove one key by name
    Unset { key: String },
    /// Remove all keys (requires `--yes`)
    Clear {
        #[arg(long)]
        yes: bool,
    },
}

fn secrets_object_from_json_file(path: &PathBuf) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let v: Value = serde_json::from_str(&text).context("Invalid JSON in secrets file")?;
    if !v.is_object() {
        bail!("Secrets file root must be a JSON object, e.g. {{\"client_id\":\"...\",\"client_secret\":\"...\"}}");
    }
    for (k, val) in v.as_object().unwrap() {
        if !val.is_string() {
            bail!("All secret values must be JSON strings (check key: {})", k);
        }
    }
    Ok(v)
}

fn format_keys(keys: &[String]) -> String {
    if keys.is_empty() {
        "(none)".to_string()
    } else {
        keys.join(", ")
    }
}

pub fn run(
    command: SecretsCommand,
    app_id: Option<String>,
    api_key_override: Option<String>,
) -> Result<()> {
    let api_key = config::resolve_api_key(api_key_override)?;
    let api_url = config::get_api_url()?;
    let project_dir = std::env::current_dir().context("Failed to get current directory")?;
    let app_id = config::resolve_app_id(app_id, &project_dir)?;

    match command {
        SecretsCommand::List => {
            let keys = client::list_app_secret_keys(&api_url, &api_key, &app_id)?;
            if keys.is_empty() {
                println!("(no app secrets)");
            } else {
                for k in keys {
                    println!("{}", k);
                }
            }
        }
        SecretsCommand::Set {
            file,
            name,
            value,
            stdin,
        } => {
            let secrets_json = if let Some(path) = file {
                if name.is_some() || value.is_some() || stdin {
                    bail!("With --file, do not pass NAME, --value, or --stdin");
                }
                secrets_object_from_json_file(&path)?
            } else {
                let k = name
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Pass --file <path.json> or a NAME plus --value / --stdin (see ww-cli secrets set --help)"
                        )
                    })?;
                if value.is_some() && stdin {
                    bail!("Pass only one of --value or --stdin");
                }
                let v = if stdin {
                    let mut line = String::new();
                    std::io::stdin().lock().read_line(&mut line)?;
                    let t = line.trim_end_matches(['\n', '\r']).to_string();
                    if t.is_empty() {
                        bail!("stdin was empty");
                    }
                    t
                } else {
                    value.clone().ok_or_else(|| {
                        anyhow::anyhow!("With a key NAME, provide --value or --stdin")
                    })?
                };
                serde_json::json!({ k: v })
            };

            let keys = client::put_app_secrets(&api_url, &api_key, &app_id, &secrets_json)?;
            println!("Saved. Current secret keys: {}", format_keys(&keys));
        }
        SecretsCommand::Unset { key } => {
            let keys = client::delete_app_secret_key(&api_url, &api_key, &app_id, &key)?;
            println!("Removed {}. Remaining keys: {}", key, format_keys(&keys));
        }
        SecretsCommand::Clear { yes } => {
            if !yes {
                bail!("Refusing to clear all app secrets without --yes");
            }
            client::clear_app_secrets(&api_url, &api_key, &app_id)?;
            println!("All app secrets cleared.");
        }
    }
    Ok(())
}
