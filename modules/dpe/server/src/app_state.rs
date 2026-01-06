use services::metadata::MetadataServiceImpl;
use services::metadata_v2::project_repository::ProjectRepository;
use storage::metadata::InMemoryMetadataRepository;

#[derive(Debug, Clone)]
pub struct AppState {
    pub metadata_service: MetadataServiceImpl<InMemoryMetadataRepository>,
    pub project_repository: ProjectRepository,
}
