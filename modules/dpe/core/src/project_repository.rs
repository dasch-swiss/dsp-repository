use super::project::Project;

/// Repository interface for accessing projects.
pub trait ProjectRepository {
    fn get_all(&self) -> Vec<Project>;
    fn get_by_shortcode(&self, shortcode: &str) -> Option<Project>;
}

/// Production implementation of [`ProjectRepository`] backed by the in-process cache.
#[cfg(not(target_arch = "wasm32"))]
pub struct FsProjectRepository;

#[cfg(not(target_arch = "wasm32"))]
impl FsProjectRepository {
    pub fn new(_data_dir: String) -> Self {
        Self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ProjectRepository for FsProjectRepository {
    fn get_all(&self) -> Vec<Project> {
        super::project_cache::all_projects().to_vec()
    }

    fn get_by_shortcode(&self, shortcode: &str) -> Option<Project> {
        super::project_cache::all_projects()
            .iter()
            .find(|project| project.shortcode == shortcode)
            .cloned()
    }
}
