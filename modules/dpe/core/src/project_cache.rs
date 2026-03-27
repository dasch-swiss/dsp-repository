/// In-process cache for the full project list.
///
/// All projects are loaded from disk once on first access and held in memory
/// for the lifetime of the server process. This avoids re-reading and
/// re-deserializing every JSON file on every request.
///
/// Note: This module is already gated with `#[cfg(not(target_arch = "wasm32"))]` in lib.rs.
use std::sync::OnceLock;

use super::project::{Project, ProjectRaw};
use super::utils::get_data_dir;

static PROJECTS: OnceLock<Vec<Project>> = OnceLock::new();

/// Return a reference to the cached project list, loading it on first call.
pub fn all_projects() -> &'static Vec<Project> {
    PROJECTS.get_or_init(load_all_projects)
}

fn load_all_projects() -> Vec<Project> {
    use std::fs;
    use std::path::PathBuf;

    let projects_dir = PathBuf::from(get_data_dir()).join("projects");

    let Ok(entries) = fs::read_dir(&projects_dir) else {
        eprintln!("project_cache: failed to read {:?}", projects_dir);
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
                Err(e) => eprintln!("project_cache: failed to parse {}: {}", filename, e),
            },
            Err(e) => eprintln!("project_cache: failed to read {}: {}", filename, e),
        }
    }
    projects
}
