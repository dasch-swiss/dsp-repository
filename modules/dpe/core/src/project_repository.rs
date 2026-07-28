use super::project::Project;

/// Repository interface for accessing projects.
pub trait ProjectRepository {
    fn get_all(&self) -> &[Project];
    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project>;
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
}
