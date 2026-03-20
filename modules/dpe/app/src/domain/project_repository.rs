use super::project::Project;

/// Repository interface for accessing projects.
pub trait ProjectRepository {
    fn get_all(&self) -> Vec<Project>;
    fn get_by_shortcode(&self, shortcode: &str) -> Option<Project>;
}

/// Filesystem-backed implementation of [`ProjectRepository`].
///
/// Reads project JSON files from the configured data directory.
#[cfg(feature = "ssr")]
pub struct FsProjectRepository {
    data_dir: String,
}

#[cfg(feature = "ssr")]
impl FsProjectRepository {
    pub fn new(data_dir: String) -> Self {
        Self { data_dir }
    }

    fn read_all_projects(&self) -> Vec<Project> {
        use std::fs;
        use std::path::PathBuf;

        let projects_dir = PathBuf::from(&self.data_dir).join("projects");

        if !projects_dir.exists() {
            return Vec::new();
        }

        let Ok(entries) = fs::read_dir(&projects_dir) else {
            return Vec::new();
        };

        entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    fs::read_to_string(&path)
                        .ok()
                        .and_then(|content| serde_json::from_str::<Project>(&content).ok())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(feature = "ssr")]
impl ProjectRepository for FsProjectRepository {
    fn get_all(&self) -> Vec<Project> {
        self.read_all_projects()
    }

    fn get_by_shortcode(&self, shortcode: &str) -> Option<Project> {
        self.read_all_projects()
            .into_iter()
            .find(|project| project.shortcode == shortcode)
    }
}
