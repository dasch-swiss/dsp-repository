use std::collections::HashMap;

/// Returns the value for the first available language in the priority order: en -> de -> fr -> it.
/// Falls back to any available value if none of the preferred languages are present.
pub fn lang_value(map: &HashMap<String, String>) -> Option<&String> {
    ["en", "de", "fr", "it"]
        .iter()
        .find_map(|lang| map.get(*lang))
        .or_else(|| map.values().next())
}

#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
static DATA_DIR: OnceLock<String> = OnceLock::new();

/// Set the data directory path at startup. Must be called before any data access.
/// Thread-safe: uses OnceLock (first call wins, subsequent calls are no-ops).
#[cfg(not(target_arch = "wasm32"))]
pub fn set_data_dir(path: &str) {
    let _ = DATA_DIR.set(path.to_string());
}

/// Get the data directory path.
///
/// Priority: OnceLock (set by main.rs) → DATA_DIR env var → development default.
#[cfg(not(target_arch = "wasm32"))]
pub fn get_data_dir() -> String {
    if let Some(dir) = DATA_DIR.get() {
        return dir.clone();
    }

    if let Ok(data_dir) = std::env::var("DATA_DIR") {
        return data_dir;
    }

    "modules/dpe/server/data".to_string()
}
