//! In-process cache for the full project list.
//!
//! All projects are loaded from disk once on first access and held in memory
//! for the lifetime of the server process. This avoids re-reading and
//! re-deserializing every JSON file on every request.
use std::collections::HashMap;
use std::sync::OnceLock;

use shared_metadata::ProjectRaw;

use super::project::Project;
use super::utils::get_data_dir;

/// Both vectors come from the single directory pass in [`load_all_projects`]
/// and stay index-aligned: position `i` in one is the same project as
/// position `i` in the other, which is what lets [`SHORTCODE_INDEX`] (built
/// from the view-model vector only) also serve `all_projects_raw` and
/// `project_raw_by_shortcode`.
static PROJECTS_CACHE: OnceLock<(Vec<Project>, Vec<ProjectRaw>)> = OnceLock::new();
static SHORTCODE_INDEX: OnceLock<HashMap<String, usize>> = OnceLock::new();

/// Return a reference to the cached project list, loading it on first call.
pub fn all_projects() -> &'static Vec<Project> {
    &PROJECTS_CACHE.get_or_init(load_all_projects).0
}

/// Return a reference to the cached raw (wire-contract) project list, loading
/// it on first call. Index-aligned with [`all_projects`].
pub fn all_projects_raw() -> &'static Vec<ProjectRaw> {
    &PROJECTS_CACHE.get_or_init(load_all_projects).1
}

/// O(1) lookup of a project by shortcode using the cached HashMap index.
pub fn project_by_shortcode(shortcode: &str) -> Option<&'static Project> {
    shortcode_index().get(&shortcode.to_uppercase()).map(|&i| &all_projects()[i])
}

/// O(1) lookup of the raw (wire-contract) project by shortcode, using the same
/// index as [`project_by_shortcode`] — safe because the raw and view-model
/// vectors are built in the same pass and stay index-aligned.
pub fn project_raw_by_shortcode(shortcode: &str) -> Option<&'static ProjectRaw> {
    shortcode_index()
        .get(&shortcode.to_uppercase())
        .map(|&i| &all_projects_raw()[i])
}

fn shortcode_index() -> &'static HashMap<String, usize> {
    SHORTCODE_INDEX.get_or_init(|| {
        all_projects()
            .iter()
            .enumerate()
            .map(|(i, p)| (p.shortcode.to_uppercase(), i))
            .collect()
    })
}

fn load_all_projects() -> (Vec<Project>, Vec<ProjectRaw>) {
    use std::fs;
    use std::path::PathBuf;

    let projects_dir = PathBuf::from(get_data_dir()).join("projects");

    let Ok(entries) = fs::read_dir(&projects_dir) else {
        tracing::warn!(dir = ?projects_dir, "failed to read projects directory");
        return (vec![], vec![]);
    };

    let mut projects = Vec::new();
    let mut projects_raw = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<ProjectRaw>(&json) {
                Ok(raw) => {
                    projects.push(Project::from(raw.clone()));
                    projects_raw.push(raw);
                }
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse project"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read project file"),
        }
    }
    (projects, projects_raw)
}
