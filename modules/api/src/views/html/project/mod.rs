use types::metadata::model::{ProjectMetadata, ResearchProject};

#[derive(askama::Template)]
#[template(path = "html/project/research_project_list.html")]
pub (crate) struct ProjectsListTemplate {
    pub projects: Vec<ProjectMetadata>
}

#[derive(askama::Template)]
#[template(path = "html/project/research_project_details.html")]
pub (crate) struct ProjectDetailsTemplate {
    pub project: ResearchProject,
}

#[derive(askama::Template)]
#[template(path = "html/project/404.html")]
pub (crate) struct NotFoundTemplate;