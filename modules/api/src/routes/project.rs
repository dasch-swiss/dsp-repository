use askama::Template;
use types::metadata::model::{ProjectMetadata, ResearchProject};

use crate::views::html::project::{NotFoundTemplate, ProjectDetailsTemplate, ProjectsListTemplate};

pub fn get_projects_list_page(projects: Vec<ProjectMetadata>) -> String {
    let view = ProjectsListTemplate { projects };
    view.render().unwrap()
}

pub fn get_project_details_page(project: ResearchProject) -> String {
    let view = ProjectDetailsTemplate { project };
    view.render().unwrap()
}

pub fn get_not_found_page() -> String {
    let view = NotFoundTemplate;
    view.render().unwrap()
}
