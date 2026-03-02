//! Data access helpers shared across OAI-PMH handlers.

use std::fs;
use std::path::Path;

use app::domain::Project;

use crate::oai::xml::EARLIEST_DATESTAMP;

/// Parses the set filter and returns (include_clusters, include_projects).
pub fn parse_set_filter(set: Option<&str>) -> (bool, bool) {
    match set {
        Some("entityType:ProjectCluster") => (true, false),
        Some("entityType:ResearchProject") => (false, true),
        None => (true, true),
        Some(_) => (false, false), // Unknown set
    }
}

/// Gets the data directory path.
pub fn get_data_dir() -> String {
    if let Ok(data_dir) = std::env::var("DATA_DIR") {
        return data_dir;
    }
    "modules/leptos-dpe/server/data".to_string()
}

/// Loads all projects from the data directory.
pub fn get_all_projects() -> Vec<Project> {
    let projects_dir = format!("{}/projects", get_data_dir());
    let path = Path::new(&projects_dir);

    if !path.exists() {
        return Vec::new();
    }

    let mut projects = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    if let Ok(project) = serde_json::from_str::<Project>(&content) {
                        projects.push(project);
                    }
                }
            }
        }
    }

    projects
}

/// Gets a project by its shortcode.
pub fn get_project_by_shortcode(shortcode: &str) -> Option<Project> {
    let projects_dir = format!("{}/projects", get_data_dir());
    let path = Path::new(&projects_dir);

    if !path.exists() {
        return None;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let file_path = entry.path();
            if file_path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&file_path) {
                    if let Ok(project) = serde_json::from_str::<Project>(&content) {
                        if project.shortcode == shortcode {
                            return Some(project);
                        }
                    }
                }
            }
        }
    }

    None
}

/// Gets the earliest datestamp from all projects.
pub fn get_earliest_datestamp() -> String {
    let projects = get_all_projects();

    projects
        .iter()
        .filter_map(|p| {
            if p.start_date != "MISSING" && !p.start_date.is_empty() {
                Some(p.start_date.as_str())
            } else {
                None
            }
        })
        .min()
        .unwrap_or(EARLIEST_DATESTAMP)
        .to_string()
}
