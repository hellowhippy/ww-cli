use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const WORKER_INDEX_TEMPLATE: &str = include_str!("worker_index.template.js");

#[cfg(test)]
mod tests {
    #[test]
    fn worker_template_supports_multipart() {
        assert!(super::WORKER_INDEX_TEMPLATE.contains("buildMultipartFormData"));
        assert!(super::WORKER_INDEX_TEMPLATE.contains("options.multipart"));
    }

    #[test]
    fn worker_template_supports_file_helpers() {
        assert!(super::WORKER_INDEX_TEMPLATE.contains("stashFile"));
        assert!(super::WORKER_INDEX_TEMPLATE.contains("dehydrateFile"));
        assert!(super::WORKER_INDEX_TEMPLATE.contains("whippy:dehydrate-file:"));
        assert!(super::WORKER_INDEX_TEMPLATE.contains("options.raw"));
    }

    #[test]
    fn worker_template_slices_uint8array_views() {
        assert!(super::WORKER_INDEX_TEMPLATE.contains("value.byteOffset"));
        assert!(super::WORKER_INDEX_TEMPLATE.contains("value.byteLength"));
    }

    #[test]
    fn worker_template_rejects_non_finite_cache_expiration() {
        assert!(super::WORKER_INDEX_TEMPLATE.contains("Number.isFinite(cacheExpiration)"));
    }

    #[test]
    fn worker_template_supports_redirect_option() {
        assert!(super::WORKER_INDEX_TEMPLATE.contains("options.redirect"));
        assert!(super::WORKER_INDEX_TEMPLATE.contains("redirect: options.redirect || 'follow'"));
    }
}

pub fn generate_worker_wrapper(build_dir: &PathBuf, bundled_app_path: &PathBuf) -> Result<PathBuf> {
    let wrapper_path = build_dir.join("index.js");

    let app_relative_path = bundled_app_path
        .strip_prefix(build_dir)
        .with_context(|| "Failed to get relative path for bundled app")?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path encoding"))?;

    let wrapper_code = WORKER_INDEX_TEMPLATE.replace("__APP_IMPORT_PATH__", app_relative_path);

    fs::write(&wrapper_path, wrapper_code)
        .with_context(|| format!("Failed to write Worker wrapper: {}", wrapper_path.display()))?;

    Ok(wrapper_path)
}
