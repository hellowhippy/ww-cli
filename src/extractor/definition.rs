use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

pub fn extract_definition(bundled_code_path: &Path) -> Result<serde_json::Value> {
    println!("Extracting app definition...");

    // Find package.json to read version
    // bundled_code_path is typically .build/app.js, so we need to go up to project root
    let project_root = bundled_code_path
        .parent() // .build
        .and_then(|p| p.parent()) // project root
        .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?;

    let package_json_path = project_root.join("package.json");
    let version = if package_json_path.exists() {
        let content =
            fs::read_to_string(&package_json_path).context("Failed to read package.json")?;
        let pkg: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse package.json")?;
        pkg.get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "1.0.0".to_string())
    } else {
        "1.0.0".to_string()
    };

    // Create a temporary directory for the extraction script
    let temp_dir = TempDir::new().context("Failed to create temp directory")?;
    let extract_script = temp_dir.path().join("extract.js");

    // Get the absolute path to the bundled code
    let bundled_abs = bundled_code_path.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize path: {}",
            bundled_code_path.display()
        )
    })?;

    // Create a package.json for the temp directory to enable ES modules
    let package_json = r#"{"type": "module"}"#;
    fs::write(temp_dir.path().join("package.json"), package_json)
        .context("Failed to write package.json")?;

    // Create Node.js script to extract definition
    let script = format!(
        r#"
import app from '{app_path}';

// Recursively strip functions; replace with path string so keys stay (Zapier-style).
function stripFunctions(obj, path = '') {{
  if (obj === null || obj === undefined) {{
    return null;
  }}

  if (typeof obj === 'function') {{
    return path || null;
  }}

  if (Array.isArray(obj)) {{
    return obj.map((item, i) => stripFunctions(item, path ? `${{path}}[${{i}}]` : '')).filter(item => item !== null);
  }}

  if (typeof obj === 'object') {{
    const result = {{}};
    for (const [key, value] of Object.entries(obj)) {{
      const childPath = path ? `${{path}}.${{key}}` : key;
      if (value === null) {{
        result[key] = null;
        continue;
      }}
      const stripped = stripFunctions(value, childPath);
      if (stripped !== null) {{
        result[key] = stripped;
      }}
    }}
    return result;
  }}

  return obj;
}}

try {{
  const definition = stripFunctions(app);
  // Always set version from package.json (in case it was lost during stripping or is null/undefined)
  definition.version = '{version}';
  console.log(JSON.stringify(definition, null, 2));
}} catch (error) {{
  console.error('Error extracting definition:', error);
  process.exit(1);
}}
"#,
        app_path = bundled_abs.to_str().unwrap(),
        version = version
    );

    fs::write(&extract_script, script).context("Failed to write extraction script")?;

    // Run the script with Node.js
    let output = Command::new("node")
        .arg(&extract_script)
        .current_dir(temp_dir.path())
        .output()
        .context("Failed to run Node.js extraction script. Make sure Node.js is installed.")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Definition extraction failed: {}", stderr);
    }

    let stdout = String::from_utf8(output.stdout).context("Failed to read extraction output")?;

    let definition: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse extracted definition as JSON")?;

    println!("✓ Definition extracted successfully");
    Ok(definition)
}
