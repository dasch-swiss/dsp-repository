use std::collections::HashMap;

/// Returns the value for the first available language in the priority order: en → de → fr → it.
/// Falls back to any available value if none of the preferred languages are present.
pub fn lang_value(map: &HashMap<String, String>) -> Option<&String> {
    ["en", "de", "fr", "it"]
        .iter()
        .find_map(|lang| map.get(*lang))
        .or_else(|| map.values().next())
}

/// Get the data directory path, supporting both development and production environments
pub fn get_data_dir() -> String {
    // Try environment variable first (for production/custom deployments)
    if let Ok(data_dir) = std::env::var("DATA_DIR") {
        return data_dir;
    }

    // For development with cargo-leptos, use relative path from workspace root
    // cargo-leptos runs from the workspace root
    "modules/dpe/server/data".to_string()
}
