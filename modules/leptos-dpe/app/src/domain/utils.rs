/// Get the data directory path, supporting both development and production environments
#[allow(dead_code)]
pub fn get_data_dir() -> String {
    // Try environment variable first (for production/custom deployments)
    if let Ok(data_dir) = std::env::var("DATA_DIR") {
        return data_dir;
    }

    // For development with cargo-leptos, use relative path from workspace root
    // cargo-leptos runs from the workspace root
    "modules/leptos-dpe/server/data".to_string()
}
