/// In-process cache for organizations.
///
/// All organizations are loaded from disk once on first access and held in a
/// HashMap keyed by organization ID for O(1) lookup.
///
/// Note: This module is gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::collections::HashMap;
use std::sync::OnceLock;

use super::organization::Organization;
use super::utils::get_data_dir;

static ORGANIZATIONS: OnceLock<HashMap<String, Organization>> = OnceLock::new();

/// Return a reference to the cached organization map, loading it on first call.
pub fn all_organizations() -> &'static HashMap<String, Organization> {
    ORGANIZATIONS.get_or_init(load_all_organizations)
}

fn load_all_organizations() -> HashMap<String, Organization> {
    use std::fs;
    use std::path::PathBuf;

    let orgs_dir = PathBuf::from(get_data_dir()).join("organizations");

    let Ok(entries) = fs::read_dir(&orgs_dir) else {
        tracing::warn!(dir = ?orgs_dir, "failed to read organizations directory");
        return HashMap::new();
    };

    let mut orgs = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Organization>(&json) {
                Ok(o) => {
                    orgs.insert(o.id.clone(), o);
                }
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse organization"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read organization file"),
        }
    }
    orgs
}
