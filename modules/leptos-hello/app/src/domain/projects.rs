use leptos::prelude::*;

use super::models::Page;
use super::project::{Project, ProjectQuery, ProjectStatus};

#[server]
pub async fn list_projects(
    ongoing: Option<bool>,
    finished: Option<bool>,
    search: Option<String>,
    page: Option<i32>,
    page_size: Option<i32>,
) -> Result<Page, ServerFnError> {
    let query = ProjectQuery {
        ongoing,
        finished,
        search,
        page,
    };
    use std::fs;

    let items_per_page = page_size.unwrap_or(9).max(1) as usize;

    let projects_dir = "server/data/projects";
    let mut projects = Vec::new();

    // Read all entries in the projects directory
    let entries = fs::read_dir(projects_dir)
        .map_err(|e| ServerFnError::new(format!("Failed to read projects directory: {}", e)))?;

    // Iterate through all JSON files
    for entry in entries {
        let entry = entry
            .map_err(|e| ServerFnError::new(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    // Check if the file ends with .json
                    if filename_str.ends_with(".json") {
                        // Read and parse the JSON file
                        match fs::read_to_string(&path) {
                            Ok(json_data) => {
                                match serde_json::from_str::<Project>(&json_data) {
                                    Ok(project) => projects.push(project),
                                    Err(e) => {
                                        // Log error but continue with other files
                                        eprintln!("Failed to parse {}: {}", filename_str, e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to read {}: {}", filename_str, e);
                            }
                        }
                    }
                }
            }
        }
    }

    // Apply filtering
    let search_lower = query.search().to_lowercase();
    let filtered_projects: Vec<Project> = projects
        .into_iter()
        .filter(|project| {
            // Status filter
            let is_ongoing = project.status == ProjectStatus::Ongoing;
            let is_finished = project.status == ProjectStatus::Finished;
            let status_match = (query.ongoing() && is_ongoing) || (query.finished() && is_finished);

            // Search filter - check all properties
            let status_str = match project.status {
                ProjectStatus::Ongoing => "ongoing",
                ProjectStatus::Finished => "finished",
            };
            let search_match = if search_lower.is_empty() {
                true
            } else {
                project.name.to_lowercase().contains(&search_lower)
                    || project
                        .short_description
                        .to_lowercase()
                        .contains(&search_lower)
                    || project.shortcode.to_lowercase().contains(&search_lower)
                    || status_str.contains(&search_lower)
            };

            status_match && search_match
        })
        .collect();

    // Calculate pagination
    let total_items = filtered_projects.len() as i32;
    let nr_pages = (total_items as usize).div_ceil(items_per_page).max(1) as i32;

    // Get the page of items
    let page_index = (query.page() - 1).max(0) as usize;
    let start_idx = page_index * items_per_page;
    let end_idx = (start_idx + items_per_page).min(total_items as usize);

    let items: Vec<Project> = filtered_projects
        .into_iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .collect();

    Ok(Page {
        items,
        nr_pages,
        total_items,
    })
}

#[server]
pub async fn get_project(shortcode: String) -> Result<Option<Project>, ServerFnError> {
    use std::fs;

    let projects_dir = "server/data/projects";

    // Read all entries in the projects directory
    let entries = fs::read_dir(projects_dir)
        .map_err(|e| ServerFnError::new(format!("Failed to read projects directory: {}", e)))?;

    // Find the file that starts with the shortcode
    for entry in entries {
        let entry = entry
            .map_err(|e| ServerFnError::new(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    // Check if the filename starts with the shortcode and ends with .json
                    if filename_str.starts_with(&shortcode) && filename_str.ends_with(".json") {
                        // Read and parse the JSON file
                        let json_data = fs::read_to_string(&path).map_err(|e| {
                            ServerFnError::new(format!("Failed to read file: {}", e))
                        })?;

                        let project: Project = serde_json::from_str(&json_data).map_err(|e| {
                            ServerFnError::new(format!("Failed to parse JSON: {}", e))
                        })?;

                        return Ok(Some(project));
                    }
                }
            }
        }
    }

    Ok(None)
}
