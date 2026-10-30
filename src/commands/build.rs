use anyhow::{Context, Result};
use std::fs;

use crate::bundler;
use crate::extractor;
use crate::generators;
use crate::utils::zip;

pub fn run() -> Result<()> {
    println!("Building integration...");

    // Get current directory (should be the app directory)
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Validate project structure
    let src_dir = current_dir.join("src");
    if !src_dir.exists() {
        anyhow::bail!("src/ directory not found. Are you in the right directory?");
    }

    let index_ts = src_dir.join("index.ts");
    let index_js = src_dir.join("index.js");
    if !index_ts.exists() && !index_js.exists() {
        anyhow::bail!("No index.ts or index.js found in src/ directory");
    }

    // Create build directory
    let build_dir = current_dir.join(".build");
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).context("Failed to clean build directory")?;
    }
    fs::create_dir_all(&build_dir).context("Failed to create build directory")?;

    // Bundle the code
    let bundled_path =
        bundler::bundle(&current_dir, &build_dir).context("Failed to bundle code")?;

    // Generate Worker wrapper
    let _wrapper_path = generators::generate_worker_wrapper(&build_dir, &bundled_path)
        .context("Failed to generate Worker wrapper")?;

    println!("✓ Worker wrapper generated");

    // Generate wrangler.toml for local development
    generators::generate_wrangler_toml(&build_dir).context("Failed to generate wrangler.toml")?;

    println!("✓ Generated wrangler.toml");

    // Extract definition
    let mut definition =
        extractor::extract_definition(&bundled_path).context("Failed to extract app definition")?;

    extractor::normalize_definition_samples(&mut definition);

    // Save definition to app.json (for backward compatibility and validation)
    let app_json_path = current_dir.join("app.json");
    fs::write(&app_json_path, serde_json::to_string_pretty(&definition)?)
        .context("Failed to write app.json")?;

    println!("✓ App definition saved to app.json");

    // Create build.zip (contains bundled code + wrapper)
    let build_zip_path = current_dir.join("build.zip");
    zip::create_build_zip(&build_dir, &build_zip_path).context("Failed to create build.zip")?;

    println!("✓ Created build.zip");

    // Create source.zip (contains original source files)
    let source_zip_path = current_dir.join("source.zip");
    zip::create_source_zip(&current_dir, &source_zip_path)
        .context("Failed to create source.zip")?;

    println!("✓ Created source.zip");

    println!("\n✓ Build completed successfully!");
    println!("  Build artifacts:");
    println!("    - build.zip (for deployment)");
    println!("    - source.zip (for source code backup)");
    println!("    - app.json (app definition)");

    Ok(())
}
