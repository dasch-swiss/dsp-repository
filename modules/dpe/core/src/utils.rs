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
    if DATA_DIR.set(path.to_string()).is_err() {
        tracing::warn!(
            new = path,
            current = DATA_DIR.get().unwrap().as_str(),
            "set_data_dir called again but data dir is already set"
        );
    }
}

/// Get the data directory path.
///
/// Priority: OnceLock (set by main.rs) → DPE_DATA_DIR env var → DATA_DIR env var → development default.
/// Falls back to setting the OnceLock from env/default on first call.
#[cfg(not(target_arch = "wasm32"))]
pub fn get_data_dir() -> &'static str {
    DATA_DIR.get_or_init(|| {
        std::env::var("DPE_DATA_DIR")
            .or_else(|_| std::env::var("DATA_DIR"))
            .unwrap_or_else(|_| "modules/dpe/server/data".to_string())
    })
}
