/// In-process cache for clusters.
///
/// All clusters are loaded from disk once on first access and held in memory
/// for the lifetime of the server process. This avoids rescanning the clusters
/// directory on every `get_project()` call.
///
/// Note: This module is gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::sync::OnceLock;

use super::cluster::ClusterRaw;
use super::utils::get_data_dir;

static CLUSTERS: OnceLock<Vec<ClusterRaw>> = OnceLock::new();

/// Return a reference to the cached cluster list, loading it on first call.
pub fn all_clusters() -> &'static [ClusterRaw] {
    CLUSTERS.get_or_init(load_all_clusters)
}

fn load_all_clusters() -> Vec<ClusterRaw> {
    use std::fs;
    use std::path::PathBuf;

    let clusters_dir = PathBuf::from(get_data_dir()).join("clusters");

    let Ok(entries) = fs::read_dir(&clusters_dir) else {
        tracing::warn!(dir = ?clusters_dir, "failed to read clusters directory");
        return vec![];
    };

    let mut clusters = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<ClusterRaw>(&json) {
                Ok(c) => clusters.push(c),
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse cluster"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read cluster file"),
        }
    }
    clusters
}
