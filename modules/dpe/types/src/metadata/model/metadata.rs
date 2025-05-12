use crate::metadata::model::{Dataset, Grant, Organization, Person, ResearchProject};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMetadata {
    pub research_project: ResearchProject,
    pub datasets: Vec<Dataset>,
    pub persons: Vec<Person>,
    pub organizations: Vec<Organization>,
    pub grants: Vec<Grant>,
}
