use crate::domain::project::Project;

#[derive(askama::Template)]
#[template(path = "home/index.html")]
pub(crate) struct IndexTemplate {
    pub projects: Vec<Project>,
}
