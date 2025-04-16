use types::error::AppError;
use types::metadata::model::{ProjectMetadata, Shortcode};
use types::metadata::metadata_repository::MetadataRepository;
use types::metadata::metadata_service::MetadataService;

#[derive(Debug, Clone)]
pub struct MetadataServiceImpl<R: MetadataRepository + Send + Sync> {
    pub repo: R,
}

impl<R: MetadataRepository + Send + Sync> MetadataService for MetadataServiceImpl<R> {
    async fn count(&self) -> Result<usize, AppError> {
        self.repo.count().await
    }

    async fn find_all(&self) -> Result<Vec<ProjectMetadata>, AppError> {
        self.repo.find_all().await
    }

    async fn find_by_filter(&self, filter: &str) -> Result<Vec<ProjectMetadata>, AppError> {
        self.repo.find_by_filter(filter).await
    }

    async fn find_by_id(&self, id: &Shortcode) -> Result<Option<ProjectMetadata>, AppError> {
        self.repo.find_by_id(id).await
    }
}