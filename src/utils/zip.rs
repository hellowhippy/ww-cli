use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

pub fn create_build_zip(build_dir: &PathBuf, output_path: &PathBuf) -> Result<()> {
    let file = fs::File::create(output_path)
        .with_context(|| format!("Failed to create zip file: {}", output_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    add_directory_to_zip(&mut zip, build_dir, build_dir, &options)?;
    zip.finish()
        .with_context(|| "Failed to finalize zip file")?;

    Ok(())
}

pub fn create_source_zip(source_dir: &PathBuf, output_path: &PathBuf) -> Result<()> {
    let file = fs::File::create(output_path)
        .with_context(|| format!("Failed to create zip file: {}", output_path.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    add_directory_to_zip(&mut zip, source_dir, source_dir, &options)?;
    zip.finish()
        .with_context(|| "Failed to finalize zip file")?;

    Ok(())
}

fn add_directory_to_zip(
    zip: &mut ZipWriter<fs::File>,
    dir: &Path,
    base_path: &Path,
    options: &FileOptions,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            // Skip node_modules and other build artifacts
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "node_modules" || n == ".git" || n == "target" || n == "dist")
                .unwrap_or(false)
            {
                continue;
            }

            add_directory_to_zip(zip, &path, base_path, options)?;
        } else if metadata.is_file() {
            // Skip certain files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') && name != ".gitkeep" {
                    continue;
                }
                if name.ends_with(".zip") || name == "package-lock.json" || name == "yarn.lock" {
                    continue;
                }
            }

            let relative_path = path
                .strip_prefix(base_path)
                .with_context(|| format!("Failed to get relative path for: {}", path.display()))?;

            let relative_path_str = relative_path.to_str().ok_or_else(|| {
                anyhow::anyhow!("Invalid path encoding: {}", relative_path.display())
            })?;

            zip.start_file(relative_path_str, *options)
                .with_context(|| format!("Failed to add file to zip: {}", relative_path_str))?;

            let mut file_content = fs::read(&path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;
            zip.write_all(&mut file_content)?;
        }
    }

    Ok(())
}
