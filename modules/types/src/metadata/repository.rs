use crate::error::AppError;
use crate::metadata::model::ResearchProject;

pub trait MetadataRepository {
    async fn count(&self) -> Result<usize, AppError>;
    async fn find_all(&self) -> Result<Vec<ResearchProject>, AppError>;
    async fn find_by_filter(&self, filter: &str) -> Result<Vec<ResearchProject>, AppError>;
    async fn find_by_id(&self, id: &str) -> Result<ResearchProject, AppError>;
}
