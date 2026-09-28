//! The capability-owned corpus: every cache DPE loads from disk, held on one
//! value instead of behind process-global statics.
//!
//! `Corpus::new` does no I/O; each field loads lazily on its own first access,
//! exactly as the statics it replaces did. Production builds and leaks exactly
//! one `Corpus` (`Box::leak`, in `dpe-server`) and holds the resulting
//! `&'static Corpus` for the life of the process — the only production leak.
//! Tests leak one per fixture directory, which the statics this replaces made
//! impossible: nothing names a `Corpus` except whoever was handed it, so
//! several can coexist in one test binary. `Corpus` derives neither `Clone`
//! nor `Copy`: a clone would start with every `OnceLock` empty and reload the
//! whole corpus a second time, which defeats the point of caching it.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use shared_metadata::temporal_enrichment::EnrichedDate;
use shared_metadata::w3cdtf::W3cdtfRange;
use shared_metadata::{Organization, Person, ProjectRaw, Record};

use crate::cluster::ClusterRaw;
use crate::project::Project;

/// Deployment configuration a [`Corpus`] loads from: where the data files and
/// the public (static asset) directory live, and the ARK host every project
/// and record is normalised to as it enters the caches (see [`crate::ark`]).
pub struct CorpusSettings {
    pub data_dir: String,
    pub public_dir: String,
    pub ark_resolver_base_url: Option<String>,
}

/// One capability's whole corpus: every cache DPE reads from disk, loaded
/// lazily on first access. See the module doc for lifetime and ownership.
pub struct Corpus {
    pub(crate) settings: CorpusSettings,
    pub(crate) projects_cache: OnceLock<(Vec<Project>, Vec<ProjectRaw>)>,
    pub(crate) project_index_cache: OnceLock<HashMap<String, usize>>,
    pub(crate) records_cache: OnceLock<Vec<Record>>,
    pub(crate) record_index_cache: OnceLock<HashMap<String, Vec<&'static Record>>>,
    pub(crate) persons_cache: OnceLock<HashMap<String, Person>>,
    pub(crate) organizations_cache: OnceLock<HashMap<String, Organization>>,
    pub(crate) clusters_cache: OnceLock<Vec<ClusterRaw>>,
    pub(crate) periods_cache: OnceLock<HashMap<String, W3cdtfRange>>,
    pub(crate) enrichment_cache: OnceLock<HashMap<String, EnrichedDate>>,
    pub(crate) covers_cache: OnceLock<HashSet<String>>,
}

impl Corpus {
    pub fn new(settings: CorpusSettings) -> Self {
        Self {
            settings,
            projects_cache: OnceLock::new(),
            project_index_cache: OnceLock::new(),
            records_cache: OnceLock::new(),
            record_index_cache: OnceLock::new(),
            persons_cache: OnceLock::new(),
            organizations_cache: OnceLock::new(),
            clusters_cache: OnceLock::new(),
            periods_cache: OnceLock::new(),
            enrichment_cache: OnceLock::new(),
            covers_cache: OnceLock::new(),
        }
    }

    /// The directory this corpus loads its data files from.
    pub fn data_dir(&self) -> &str {
        &self.settings.data_dir
    }

    /// The directory `ServeDir` serves as static assets, this corpus's covers
    /// among them.
    pub fn public_dir(&self) -> &str {
        &self.settings.public_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two corpora over two temporary data directories, each holding a
    /// different committed project, both leaked in this one test binary: each
    /// serves only the project under its own directory. This could not be
    /// written against the statics `Corpus` replaces — a second
    /// `set_data_dir` call was a silent no-op, so only one data directory
    /// could ever be observed per process.
    #[test]
    fn two_corpora_over_two_data_dirs_each_serve_their_own_projects() {
        let dir_a = tempfile::tempdir().expect("tempdir");
        let dir_b = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir_a.path().join("projects")).unwrap();
        std::fs::create_dir_all(dir_b.path().join("projects")).unwrap();
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects/0803_incunabula.json"),
            dir_a.path().join("projects/0803_incunabula.json"),
        )
        .unwrap();
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects/0868_solec.json"),
            dir_b.path().join("projects/0868_solec.json"),
        )
        .unwrap();

        let corpus_a: &'static Corpus = Box::leak(Box::new(Corpus::new(CorpusSettings {
            data_dir: dir_a.path().to_string_lossy().into_owned(),
            public_dir: dir_a.path().to_string_lossy().into_owned(),
            ark_resolver_base_url: None,
        })));
        let corpus_b: &'static Corpus = Box::leak(Box::new(Corpus::new(CorpusSettings {
            data_dir: dir_b.path().to_string_lossy().into_owned(),
            public_dir: dir_b.path().to_string_lossy().into_owned(),
            ark_resolver_base_url: None,
        })));

        assert!(corpus_a.project_by_shortcode("0803").is_some());
        assert!(corpus_a.project_by_shortcode("0868").is_none());
        assert!(corpus_b.project_by_shortcode("0868").is_some());
        assert!(corpus_b.project_by_shortcode("0803").is_none());
    }
}
