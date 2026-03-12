use leptos::prelude::*;

use super::models::Page;
use super::project::Project;

#[server]
pub async fn list_type_of_data() -> Result<Vec<String>, ServerFnError> {
    use std::collections::HashSet;

    use super::project_cache::all_projects;

    let mut types: HashSet<String> = HashSet::new();
    for project in all_projects() {
        if let Some(t) = &project.type_of_data {
            types.extend(t.iter().cloned());
        }
    }
    let mut result: Vec<String> = types.into_iter().collect();
    result.sort();
    Ok(result)
}

#[server]
pub async fn list_data_languages() -> Result<Vec<String>, ServerFnError> {
    use std::collections::HashSet;

    use super::project_cache::all_projects;
    use super::utils::lang_value;

    let mut languages: HashSet<String> = HashSet::new();
    for project in all_projects() {
        if let Some(langs) = &project.data_language {
            for lang_map in langs {
                if let Some(val) = lang_value(lang_map) {
                    languages.insert(val.clone());
                }
            }
        }
    }
    let mut result: Vec<String> = languages.into_iter().collect();
    result.sort();
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
#[server]
pub async fn list_projects(
    ongoing: Option<bool>,
    finished: Option<bool>,
    search: Option<String>,
    page: Option<i32>,
    page_size: Option<i32>,
    type_of_data: Option<String>,
    data_language: Option<String>,
    access_rights: Option<String>,
) -> Result<Page, ServerFnError> {
    use super::project::{AccessRightsType, ProjectQuery, ProjectStatus};
    use super::project_cache::all_projects;
    use super::utils::lang_value;

    let query = ProjectQuery {
        ongoing,
        finished,
        search,
        page,
        type_of_data,
        data_language,
        access_rights,
        dialog: None,
    };

    let items_per_page = page_size.unwrap_or(9).max(1) as usize;
    let search_lower = query.search().to_lowercase();
    let type_of_data_filter = query.type_of_data();
    let data_language_filter = query.data_language();
    let access_rights_filter = query.access_rights();

    let mut filtered_projects: Vec<&Project> = all_projects()
        .iter()
        .filter(|project| {
            // Status filter
            let is_ongoing = project.status == ProjectStatus::Ongoing;
            let is_finished = project.status == ProjectStatus::Finished;
            let status_match = match (query.ongoing, query.finished) {
                (None, None) => true,
                _ => (query.ongoing() && is_ongoing) || (query.finished() && is_finished),
            };

            // Search filter
            let search_match = if search_lower.is_empty() {
                true
            } else {
                let status_str = match project.status {
                    ProjectStatus::Ongoing => "ongoing",
                    ProjectStatus::Finished => "finished",
                };
                project.name.to_lowercase().contains(&search_lower)
                    || project.short_description.to_lowercase().contains(&search_lower)
                    || project.shortcode.to_lowercase().contains(&search_lower)
                    || status_str.contains(&search_lower)
            };

            // Type of data filter
            let type_match = if type_of_data_filter.is_empty() {
                true
            } else {
                project
                    .type_of_data
                    .as_ref()
                    .map(|types| types.iter().any(|t| type_of_data_filter.contains(t)))
                    .unwrap_or(false)
            };

            // Data language filter
            let language_match = if data_language_filter.is_empty() {
                true
            } else {
                project
                    .data_language
                    .as_ref()
                    .map(|langs| {
                        langs.iter().any(|lang_map| {
                            lang_value(lang_map)
                                .map(|v| data_language_filter.contains(v))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            };

            // Access rights filter
            let access_rights_match = if access_rights_filter.is_empty() {
                true
            } else {
                let label = match project.access_rights.access_rights {
                    AccessRightsType::FullOpenAccess => "Full Open Access",
                    AccessRightsType::OpenAccessWithRestrictions => {
                        "Open Access with Restrictions"
                    }
                    AccessRightsType::EmbargoedAccess => "Embargoed Access",
                    AccessRightsType::MetadataOnlyAccess => "Metadata only Access",
                };
                access_rights_filter.iter().any(|f| f == label)
            };

            status_match && search_match && type_match && language_match && access_rights_match
        })
        .collect();

    filtered_projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let total_items = filtered_projects.len() as i32;
    let nr_pages = (filtered_projects.len()).div_ceil(items_per_page).max(1) as i32;

    let page_index = (query.page() - 1).max(0) as usize;
    let start_idx = page_index * items_per_page;
    let end_idx = (start_idx + items_per_page).min(filtered_projects.len());

    let items: Vec<Project> =
        filtered_projects[start_idx..end_idx].iter().map(|p| (*p).clone()).collect();

    Ok(Page { items, nr_pages, total_items })
}

#[server]
pub async fn get_project(shortcode: String) -> Result<Option<Project>, ServerFnError> {
    use std::fs;
    use std::path::PathBuf;

    use super::cluster::ClusterRaw;
    use super::collection::CollectionRef;
    use super::project::ProjectRaw;
    use super::utils::get_data_dir;

    let data_dir = get_data_dir();
    let data_path = PathBuf::from(data_dir);
    let projects_dir = data_path.join("projects");

    let entries = fs::read_dir(projects_dir)
        .map_err(|e| ServerFnError::new(format!("Failed to read projects directory: {}", e)))?;

    for entry in entries {
        let entry = entry
            .map_err(|e| ServerFnError::new(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(filename_str) = path.file_name().and_then(|n| n.to_str()) {
                if filename_str.starts_with(&shortcode) && filename_str.ends_with(".json") {
                    let json_data = fs::read_to_string(&path)
                        .map_err(|e| ServerFnError::new(format!("Failed to read file: {}", e)))?;

                    let raw: ProjectRaw = serde_json::from_str(&json_data)
                        .map_err(|e| ServerFnError::new(format!("Failed to parse JSON: {}", e)))?;

                    let collection_ids = raw.collections.clone().unwrap_or_default();
                    let mut project = Project::from(raw);

                    // Resolve clusters by reverse lookup
                    let clusters_dir = data_path.join("clusters");
                    project.clusters = fs::read_dir(&clusters_dir)
                        .into_iter()
                        .flatten()
                        .flatten()
                        .filter_map(|entry| {
                            let path = entry.path();
                            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                                return None;
                            }
                            let json = fs::read_to_string(&path).ok()?;
                            let raw: ClusterRaw = serde_json::from_str(&json).ok()?;
                            if raw.projects.iter().any(|p| p == &shortcode) {
                                Some(raw.into_ref())
                            } else {
                                None
                            }
                        })
                        .collect();

                    // Resolve collection IDs
                    let collections_dir = data_path.join("collections");
                    project.collections = collection_ids
                        .iter()
                        .filter_map(|id| {
                            let collection_path = collections_dir.join(format!("{}.json", id));
                            fs::read_to_string(&collection_path)
                                .ok()
                                .and_then(|json| serde_json::from_str::<CollectionRef>(&json).ok())
                        })
                        .collect();

                    return Ok(Some(project));
                }
            }
        }
    }

    Ok(None)
}
