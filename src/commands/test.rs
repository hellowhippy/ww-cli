use anyhow::{Context, Result};
use std::process::Command;

/// Run the test suite (executes `npm test`)
pub fn run() -> Result<()> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Check if package.json exists
    let package_json_path = current_dir.join("package.json");
    if !package_json_path.exists() {
        anyhow::bail!(
            "package.json not found. Make sure you're in a Whippy integration directory.\n\
            Run 'whippy-workflows-cli init <name>' to create a new integration."
        );
    }

    // Check if test script exists in package.json
    let package_json_content =
        std::fs::read_to_string(&package_json_path).context("Failed to read package.json")?;
    let package_json: serde_json::Value =
        serde_json::from_str(&package_json_content).context("Failed to parse package.json")?;

    if package_json["scripts"]["test"].is_null() {
        anyhow::bail!(
            "No test script found in package.json.\n\
            Add a test script to your package.json:\n\
            \n\
            {{\n\
              \"scripts\": {{\n\
                \"test\": \"your-test-command\"\n\
              }}\n\
            }}"
        );
    }

    println!("Running test suite...");
    println!("Executing: npm test\n");

    // Run npm test
    let status = Command::new("npm")
        .arg("test")
        .current_dir(&current_dir)
        .status()
        .context("Failed to run npm test. Make sure npm is installed and in your PATH.")?;

    if !status.success() {
        anyhow::bail!(
            "Test suite failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
    }

    println!("\n✓ All tests passed!");
    Ok(())
}
