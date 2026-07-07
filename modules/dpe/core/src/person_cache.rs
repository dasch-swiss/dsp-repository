/// In-process cache for persons.
///
/// All persons are loaded from disk once on first access and held in a
/// HashMap keyed by person ID for O(1) lookup.
///
/// Note: This module is gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::collections::HashMap;
use std::sync::OnceLock;

use super::person::Person;
use super::utils::get_data_dir;

static PERSONS: OnceLock<HashMap<String, Person>> = OnceLock::new();

/// Return a reference to the cached person map, loading it on first call.
pub fn all_persons() -> &'static HashMap<String, Person> {
    PERSONS.get_or_init(load_all_persons)
}

fn load_all_persons() -> HashMap<String, Person> {
    use std::fs;
    use std::path::PathBuf;

    let persons_dir = PathBuf::from(get_data_dir()).join("persons");

    let Ok(entries) = fs::read_dir(&persons_dir) else {
        tracing::warn!(dir = ?persons_dir, "failed to read persons directory");
        return HashMap::new();
    };

    let mut persons = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Person>(&json) {
                Ok(p) => {
                    persons.insert(p.id.clone(), p);
                }
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse person"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read person file"),
        }
    }
    persons
}
