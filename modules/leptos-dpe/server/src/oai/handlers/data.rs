//! Data directory helpers for OAI-PMH handlers.

/// Gets the data directory path.
pub fn get_data_dir() -> String {
    if let Ok(data_dir) = std::env::var("DATA_DIR") {
        return data_dir;
    }
    "modules/leptos-dpe/server/data".to_string()
}
