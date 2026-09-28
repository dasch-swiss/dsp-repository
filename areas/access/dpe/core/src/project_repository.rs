use shared_metadata::ProjectRaw;

use super::corpus::Corpus;
use super::project::Project;

/// Repository interface for accessing projects.
pub trait ProjectRepository {
    fn get_all(&self) -> &[Project];
    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project>;
    fn get_all_raw(&self) -> &[ProjectRaw];
    fn get_raw_by_shortcode(&self, shortcode: &str) -> Option<&ProjectRaw>;
}

/// Production implementation of [`ProjectRepository`] backed by a corpus's
/// in-process cache.
pub struct FsProjectRepository {
    corpus: &'static Corpus,
}

impl FsProjectRepository {
    pub fn new(corpus: &'static Corpus) -> Self {
        Self { corpus }
    }
}

impl ProjectRepository for FsProjectRepository {
    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_all(&self) -> &[Project] {
        self.corpus.all_projects()
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_by_shortcode(&self, shortcode: &str) -> Option<&Project> {
        self.corpus.project_by_shortcode(shortcode)
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_all_raw(&self) -> &[ProjectRaw] {
        self.corpus.all_projects_raw()
    }

    #[tracing::instrument(skip(self), fields(otel.kind = "internal"))]
    fn get_raw_by_shortcode(&self, shortcode: &str) -> Option<&ProjectRaw> {
        self.corpus.project_raw_by_shortcode(shortcode)
    }
}
