use std::future::Future;
use crate::error::AppError;
use crate::metadata::model::{ProjectMetadata, Shortcode};

pub trait MetadataRepository {
    fn count(&self) -> impl Future<Output = Result<usize, AppError>> + Send ;
    fn find_all(&self) -> impl Future<Output=Result<Vec<ProjectMetadata>, AppError>> + Send;
    fn find_by_filter(&self, filter: &str) -> impl Future<Output = Result<Vec<ProjectMetadata>, AppError>> + Send;
    fn find_by_id(&self, id: &Shortcode) -> impl Future<Output=Result<Option<ProjectMetadata>, AppError>>+Send;
}
