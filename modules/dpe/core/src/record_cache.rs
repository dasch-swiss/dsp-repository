/// In-process cache for all records.
///
/// All records are loaded from disk once on first access and held in memory
/// for the lifetime of the server process, mirroring the project cache pattern.
///
/// Note: This module is already gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::sync::OnceLock;

use super::record::Record;
use super::utils::get_data_dir;

static RECORDS: OnceLock<Vec<Record>> = OnceLock::new();

/// Return a reference to the cached record list, loading it on first call.
pub fn all_records() -> &'static Vec<Record> {
    RECORDS.get_or_init(load_all_records)
}

fn load_all_records() -> Vec<Record> {
    use std::fs;
    use std::path::PathBuf;

    let records_dir = PathBuf::from(get_data_dir()).join("records");

    if !records_dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(&records_dir) else {
        tracing::warn!(dir = ?records_dir, "failed to read records directory");
        return Vec::new();
    };

    let mut records = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Vec<Record>>(&json) {
                Ok(recs) => records.extend(recs),
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse records"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read records file"),
        }
    }
    records
}
