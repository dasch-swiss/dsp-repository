use types::error::AppError;
use types::metadata::model::ResearchProject;
use types::metadata::repository::MetadataRepository;
use types::metadata::service::MetadataService;

pub struct MetadataServiceImpl<R: MetadataRepository> {
    repo: R,
}

impl<R: MetadataRepository> MetadataService for MetadataServiceImpl<R> {
    async fn count(&self) -> Result<usize, AppError> {
        self.repo.count().await
    }

    async fn find_all(&self) -> Result<Vec<ResearchProject>, AppError> {
        self.repo.find_all().await
    }

    async fn find_by_filter(&self, filter: &str) -> Result<Vec<ResearchProject>, AppError> {
        self.repo.find_by_filter(filter).await
    }

    async fn find_by_id(&self, id: &str) -> Result<ResearchProject, AppError> {
        self.repo.find_by_id(id).await
    }
}
