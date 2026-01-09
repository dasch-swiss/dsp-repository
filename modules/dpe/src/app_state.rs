use crate::services::project_repository::ProjectRepository;

#[derive(Debug, Clone)]
pub struct AppState {
    pub project_repository: ProjectRepository,
}
