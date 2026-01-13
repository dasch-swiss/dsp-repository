use crate::domain::project::DisciplineItem::{self, Reference};
use crate::domain::project::Project;

#[derive(askama::Template)]
#[template(path = "project/index.html")]
pub(crate) struct ProjectIndexTemplate {
    pub projects: Vec<Project>,
}

#[derive(askama::Template)]
#[template(path = "project/show.html")]
pub(crate) struct ProjectShowTemplate {
    pub project: Project,
}

impl ProjectShowTemplate {
    /// Get the English language label from data_language field
    pub fn data_language_en(&self) -> String {
        self.project
            .data_language
            .as_ref()
            .and_then(|vec| vec.first())
            .and_then(|map| map.get("en"))
            .cloned()
            .unwrap_or_default()
    }

    pub fn type_of_data(&self) -> Vec<String> {
        self.project.type_of_data.clone().into_iter().flatten().collect()
    }

    pub fn disciplines(&self) -> Vec<String> {
        self.project
            .disciplines
            .clone()
            .into_iter()
            .flat_map(|d: DisciplineItem| match d {
                Reference(reference) => reference.text,
                _ => None,
            })
            .collect()
    }

    pub fn keywords(&self) -> Vec<String> {
        self.project
            .keywords
            .clone()
            .into_iter()
            .flat_map(|map| map.get("en").map(|s| s.clone()))
            .collect()
    }
}
