use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};
use log::{info, trace};
use dto::metadata::MetadataDto;
use types::error::AppError;
use types::metadata::model::{ProjectMetadata, Shortcode};
use types::metadata::metadata_repository::MetadataRepository;
use crate::file_utils::load_json_file_paths;

#[derive(Debug, Clone)]
pub struct InMemoryMetadataRepository {
    data: Arc<RwLock<HashMap<Shortcode, ProjectMetadata>>>,
}

impl InMemoryMetadataRepository {
    pub fn new(metadata: Vec<ProjectMetadata>) -> Self {
        let data = Arc::new(RwLock::new(HashMap::new()));

        for meta in metadata {
            let mut data = data.write().unwrap();
            data.insert(meta.research_project.shortcode.to_owned(), meta);
        }

        Self { data }
    }

    pub fn new_from_path(data_path: &Path) -> Self {
        info!("Init Repository {:?}", data_path);
        let data = Arc::new(RwLock::new(HashMap::new()));

        let file_paths = load_json_file_paths(data_path);
        info!("Found {} projects", file_paths.len());

        let mut known_shortcodes: Vec<Shortcode> = Vec::new();
        for file in file_paths {
            let file = File::open(file).expect("open file.");
            let dto: MetadataDto = serde_json::from_reader(file).expect("parse file as JSON");
            let entity: ProjectMetadata = dto.try_into().expect("convert DTO to domain");

            let mut data = data.write().unwrap();
            
            let shortcode = entity.research_project.shortcode.to_owned();
            if known_shortcodes.contains(&shortcode) {
                panic!("Duplicate shortcode: {:?}", shortcode);
            }
            known_shortcodes.push(shortcode);

            data.insert(entity.research_project.shortcode.to_owned(), entity);
        }

        {
            let count = data.read().unwrap();
            trace!("Stored {} projects", count.values().len());
        }

        Self { data }
    }
}

impl Default for InMemoryMetadataRepository {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl MetadataRepository for InMemoryMetadataRepository {
    async fn count(&self) -> Result<usize, AppError> {
        Ok(self.data.read().unwrap().len())
    }

    async fn find_all(&self) -> Result<Vec<ProjectMetadata>, AppError> {
        Ok(self.data.read().unwrap().values().cloned().collect())
    }

    async fn find_by_filter(
        &self,
        filter: &str,
    ) -> Result<Vec<ProjectMetadata>, AppError> {
        let _ = filter;
        Ok(vec![])
    }

    async fn find_by_id(&self, id: &Shortcode) -> Result<Option<ProjectMetadata>, AppError> {
        Ok(self.data.read().unwrap().get(id).map(|p| p.clone()))
    }
}

#[tokio::test]
async fn test_new_from_path() {
    let data_path = Path::new("../../data");
    let repo = InMemoryMetadataRepository::new_from_path(data_path);
    assert!(repo.count().await.unwrap() > 0usize);
}
