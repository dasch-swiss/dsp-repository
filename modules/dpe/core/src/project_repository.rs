use super::project::Project;

/// Repository interface for accessing projects.
pub trait ProjectRepository {
    fn get_all(&self) -> &[Project];
    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project>;
}

/// Production implementation of [`ProjectRepository`] backed by the in-process cache.
#[cfg(not(target_arch = "wasm32"))]
pub struct FsProjectRepository;

#[cfg(not(target_arch = "wasm32"))]
impl Default for FsProjectRepository {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl FsProjectRepository {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ProjectRepository for FsProjectRepository {
    fn get_all(&self) -> &[Project] {
        super::project_cache::all_projects()
    }

    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project> {
        super::project_cache::project_by_shortcode(shortcode)
    }
}
