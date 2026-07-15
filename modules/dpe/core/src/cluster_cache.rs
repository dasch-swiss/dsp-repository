/// In-process cache for clusters.
///
/// All clusters are loaded from disk once on first access and held in memory
/// for the lifetime of the server process. This avoids rescanning the clusters
/// directory on every `get_project()` call.
use std::sync::OnceLock;

use super::cluster::{ClusterRaw, ClusterRef};
use super::utils::get_data_dir;

static CLUSTERS: OnceLock<Vec<ClusterRaw>> = OnceLock::new();

/// Return a reference to the cached cluster list, loading it on first call.
pub fn all_clusters() -> &'static [ClusterRaw] {
    CLUSTERS.get_or_init(load_all_clusters)
}

/// Returns the clusters in `clusters` whose `projects` list contains `shortcode`
/// (compared case-insensitively). Pure over the given slice so it can be unit-tested
/// without touching the process-global cache.
pub fn clusters_for_shortcode_in(clusters: &[ClusterRaw], shortcode: &str) -> Vec<ClusterRef> {
    clusters
        .iter()
        .filter(|raw| raw.projects.iter().any(|p| p.eq_ignore_ascii_case(shortcode)))
        .map(|raw| raw.clone().into_ref())
        .collect()
}

/// Returns the clusters (from the cache) a project shortcode belongs to.
pub fn clusters_for_shortcode(shortcode: &str) -> Vec<ClusterRef> {
    clusters_for_shortcode_in(all_clusters(), shortcode)
}

/// Returns the project shortcodes belonging to the cluster with the given `id`
/// (exact match), or `None` if no such cluster exists in `clusters`.
pub fn projects_for_cluster_in<'a>(clusters: &'a [ClusterRaw], id: &str) -> Option<&'a [String]> {
    clusters.iter().find(|raw| raw.id == id).map(|raw| raw.projects.as_slice())
}

/// Returns the project shortcodes belonging to the cluster `id` (from the cache),
/// or `None` if no such cluster exists.
pub fn projects_for_cluster(id: &str) -> Option<&'static [String]> {
    projects_for_cluster_in(all_clusters(), id)
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn cluster(id: &str, name: &str, projects: &[&str]) -> ClusterRaw {
        let mut description = HashMap::new();
        description.insert("en".to_string(), format!("{name} description"));
        ClusterRaw {
            id: id.to_string(),
            name: name.to_string(),
            description,
            pid: None,
            projects: projects.iter().map(|p| p.to_string()).collect(),
        }
    }

    fn fixtures() -> Vec<ClusterRaw> {
        vec![
            cluster("cluster-001", "EKWS", &["0812", "082A"]),
            cluster("cluster-005", "DaSCH", &["0803"]),
        ]
    }

    #[test]
    fn clusters_for_shortcode_in_matches_membership() {
        let clusters = fixtures();
        let found = clusters_for_shortcode_in(&clusters, "0812");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "cluster-001");
        assert_eq!(found[0].name, "EKWS");
    }

    #[test]
    fn clusters_for_shortcode_in_is_case_insensitive() {
        let clusters = fixtures();
        // membership list has "082A"; query with lowercase still matches.
        let found = clusters_for_shortcode_in(&clusters, "082a");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "cluster-001");
    }

    #[test]
    fn clusters_for_shortcode_in_returns_empty_for_unknown() {
        let clusters = fixtures();
        assert!(clusters_for_shortcode_in(&clusters, "9999").is_empty());
    }

    #[test]
    fn projects_for_cluster_in_returns_members() {
        let clusters = fixtures();
        assert_eq!(
            projects_for_cluster_in(&clusters, "cluster-001"),
            Some(["0812".to_string(), "082A".to_string()].as_slice())
        );
    }

    #[test]
    fn projects_for_cluster_in_unknown_id_is_none() {
        let clusters = fixtures();
        assert_eq!(projects_for_cluster_in(&clusters, "cluster-999"), None);
    }

    #[test]
    fn cluster_id_match_is_exact_case_sensitive() {
        let clusters = fixtures();
        // cluster ids are exact-match (case-sensitive), unlike project shortcodes.
        assert_eq!(projects_for_cluster_in(&clusters, "CLUSTER-001"), None);
    }
}
