
use serde::{Serialize, Deserialize};
use serde_with::skip_serializing_none;
use types::error::AppError;
use types::metadata::model::{ProjectMetadata, ResearchProject};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    pub project: ProjectDto,
    pub datasets: Vec<DatasetDto>,
    #[serde(default)]
    pub persons: Vec<PersonDto>,
    pub organizations: Vec<OrganizationDto>,
    #[serde(default)]
    pub grants: Vec<GrantDto>,
}

impl From<ProjectMetadata> for MetadataDto {
    fn from(metadata: ProjectMetadata) -> Self {
        Self {
            project: ProjectDto::from(metadata.research_project),
            datasets: vec![],
            persons: vec![],
            organizations: vec![],
            grants: vec![],
        }
    }
}

impl TryFrom<MetadataDto> for ProjectMetadata {
    type Error = types::error::AppError;

    fn try_from(dto: MetadataDto) -> Result<Self, Self::Error> {
        Ok(Self {
            research_project: ResearchProject::try_from(dto.project)?,
            datasets: vec![], // TODO: Implement conversion for datasets
            persons: vec![], // TODO: Implement conversion for persons
            organizations: vec![], // TODO: Implement conversion for organizations
            grants: vec![], // TODO: Implement conversion for grants
        })
    }
}



#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "__type")]
#[serde(rename = "Project")]
pub struct ProjectDto {
    pub shortcode: String,
    pub name: String,
}

impl From<ResearchProject> for ProjectDto {
    fn from(p: ResearchProject) -> Self {
        Self {
            shortcode: p.shortcode.as_string(),
            name: p.name,
        }
    }
}

impl TryFrom<ProjectDto> for ResearchProject {
    type Error = AppError;

    fn try_from(dto: ProjectDto) -> Result<Self, Self::Error> {
        Ok(Self {
            shortcode: dto.shortcode.try_into()?,
            name: dto.name,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetDto {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDto {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationDto {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantDto {}