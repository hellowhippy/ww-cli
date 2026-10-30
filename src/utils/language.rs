use anyhow::Result;

pub fn normalize(language: &str) -> Result<String> {
    // Normalize language names (e.g., "ts" -> "typescript", "js" -> "javascript")
    match language.to_lowercase().as_str() {
        "ts" | "typescript" => Ok("typescript".to_string()),
        "js" | "javascript" => Ok("javascript".to_string()),
        _ => Ok(language.to_lowercase()),
    }
}
