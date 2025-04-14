use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use types::metadata::model::ResearchProject;
use types::metadata::repository::MetadataRepository;

pub struct InMemoryMetadataRepository {
    data: Arc<RwLock<HashMap<String, ResearchProject>>>,
}

impl InMemoryMetadataRepository {
    fn new() -> Self {
        let data = Arc::new(RwLock::new(HashMap::new()));
        InMemoryMetadataRepository { data }
    }

    fn add(&self, project_metadata: ResearchProject) {
        let mut data = self.data.write().unwrap();
        data.insert(project_metadata.shortcode.to_string(), project_metadata);
    }

    fn get_by_shortcode(&self, shortcode: &str) -> Option<ResearchProject> {
        let data = self.data.read().unwrap();
        data.get(shortcode).cloned()
    }

    fn remove(&self, id: &str) {
        let mut data = self.data.write().unwrap();
        data.remove(id);
    }
}

impl Default for InMemoryMetadataRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataRepository for InMemoryMetadataRepository {
    async fn count(&self) -> Result<usize, types::error::AppError> {
        todo!()
    }

    async fn find_all(&self) -> Result<Vec<ResearchProject>, types::error::AppError> {
        todo!()
    }

    async fn find_by_filter(&self, filter: &str) -> Result<Vec<ResearchProject>, types::error::AppError> {
        todo!()
    }

    async fn find_by_id(&self, id: &str) -> Result<ResearchProject, types::error::AppError> {
        todo!()
    }
}
