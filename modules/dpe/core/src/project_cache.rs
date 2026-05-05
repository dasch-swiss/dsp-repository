/// In-process cache for the full project list.
///
/// All projects are loaded from disk once on first access and held in memory
/// for the lifetime of the server process. This avoids re-reading and
/// re-deserializing every JSON file on every request.
///
/// Note: This module is already gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::collections::HashMap;
use std::sync::OnceLock;

use super::project::{Project, ProjectRaw};
use super::utils::get_data_dir;

static PROJECTS: OnceLock<Vec<Project>> = OnceLock::new();
static SHORTCODE_INDEX: OnceLock<HashMap<String, usize>> = OnceLock::new();

/// Return a reference to the cached project list, loading it on first call.
pub fn all_projects() -> &'static Vec<Project> {
    PROJECTS.get_or_init(load_all_projects)
}

/// O(1) lookup of a project by shortcode using the cached HashMap index.
pub fn project_by_shortcode(shortcode: &str) -> Option<&'static Project> {
    let index = SHORTCODE_INDEX.get_or_init(|| {
        all_projects()
            .iter()
            .enumerate()
            .map(|(i, p)| (p.shortcode.to_uppercase(), i))
            .collect()
    });
    index.get(&shortcode.to_uppercase()).map(|&i| &all_projects()[i])
}

fn load_all_projects() -> Vec<Project> {
    use std::fs;
    use std::path::PathBuf;

    let projects_dir = PathBuf::from(get_data_dir()).join("projects");

    let Ok(entries) = fs::read_dir(&projects_dir) else {
        tracing::warn!(dir = ?projects_dir, "failed to read projects directory");
        return vec![];
    };

    let mut projects = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<ProjectRaw>(&json).map(Project::from) {
                Ok(p) => projects.push(p),
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse project"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read project file"),
        }
    }
    projects
}
