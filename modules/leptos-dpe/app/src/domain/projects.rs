use leptos::prelude::*;

use super::models::Page;
use super::project::{Project, ProjectView};

#[server]
pub async fn list_projects(
    ongoing: Option<bool>,
    finished: Option<bool>,
    search: Option<String>,
    page: Option<i32>,
    view: Option<ProjectView>,
    page_size: Option<i32>,
) -> Result<Page, ServerFnError> {
    use super::project::{ProjectQuery, ProjectStatus};
    use super::project_repository::{FsProjectRepository, ProjectRepository};
    use super::utils::get_data_root_dir;

    let query = ProjectQuery { ongoing, finished, search, page, view };
    let items_per_page = page_size.unwrap_or(9).max(1) as usize;

    let repo = FsProjectRepository::new(get_data_root_dir());
    let projects = repo.get_all();

    let search_lower = query.search().to_lowercase();
    let filtered: Vec<Project> = projects
        .into_iter()
        .filter(|project| {
            let is_ongoing = project.status == ProjectStatus::Ongoing;
            let is_finished = project.status == ProjectStatus::Finished;
            let status_match = (query.ongoing() && is_ongoing) || (query.finished() && is_finished);

            let status_str = match project.status {
                ProjectStatus::Ongoing => "ongoing",
                ProjectStatus::Finished => "finished",
            };
            let search_match = if search_lower.is_empty() {
                true
            } else {
                project.name.to_lowercase().contains(&search_lower)
                    || project.short_description.to_lowercase().contains(&search_lower)
                    || project.shortcode.to_lowercase().contains(&search_lower)
                    || status_str.contains(&search_lower)
            };

            status_match && search_match
        })
        .collect();

    let total_items = filtered.len() as i32;
    let nr_pages = (filtered.len()).div_ceil(items_per_page).max(1) as i32;
    let page_index = (query.page() - 1).max(0) as usize;
    let start_idx = page_index * items_per_page;
    let items = filtered.into_iter().skip(start_idx).take(items_per_page).collect();

    Ok(Page { items, nr_pages, total_items })
}

#[server]
pub async fn get_project(shortcode: String) -> Result<Option<Project>, ServerFnError> {
    use super::project_repository::{FsProjectRepository, ProjectRepository};
    use super::utils::get_data_root_dir;

    let repo = FsProjectRepository::new(get_data_root_dir());
    Ok(repo.get_by_shortcode(&shortcode))
}
