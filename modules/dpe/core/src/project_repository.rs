use shared_metadata::ProjectRaw;

use super::project::Project;

/// Repository interface for accessing projects.
pub trait ProjectRepository {
    fn get_all(&self) -> &[Project];
    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project>;
    fn get_all_raw(&self) -> &[ProjectRaw];
    fn get_raw_by_shortcode(&self, shortcode: &str) -> Option<&ProjectRaw>;
}

/// Production implementation of [`ProjectRepository`] backed by the in-process cache.
pub struct FsProjectRepository;

impl Default for FsProjectRepository {
    fn default() -> Self {
        Self
    }
}

impl FsProjectRepository {
    pub fn new() -> Self {
        Self
    }
}

impl ProjectRepository for FsProjectRepository {
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_all(&self) -> &[Project] {
        super::project_cache::all_projects()
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project> {
        super::project_cache::project_by_shortcode(shortcode)
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_all_raw(&self) -> &[ProjectRaw] {
        super::project_cache::all_projects_raw()
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_raw_by_shortcode(&self, shortcode: &str) -> Option<&ProjectRaw> {
        super::project_cache::project_raw_by_shortcode(shortcode)
    }
}
