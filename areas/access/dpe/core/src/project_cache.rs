//! [`Corpus`]'s cache of the full project list.
//!
//! All projects are loaded from disk once on first access and held in memory
//! for the lifetime of the corpus. This avoids re-reading and re-deserializing
//! every JSON file on every request.
use std::collections::HashMap;

use shared_metadata::ProjectRaw;

use super::corpus::Corpus;
use super::project::Project;

impl Corpus {
    /// Return a reference to the cached project list, loading it on first call.
    pub fn all_projects(&'static self) -> &'static Vec<Project> {
        &self.projects_cache.get_or_init(|| self.load_all_projects()).0
    }

    /// Return a reference to the cached raw (wire-contract) project list, loading
    /// it on first call. Index-aligned with [`Corpus::all_projects`].
    pub fn all_projects_raw(&'static self) -> &'static Vec<ProjectRaw> {
        &self.projects_cache.get_or_init(|| self.load_all_projects()).1
    }

    /// O(1) lookup of a project by shortcode using the cached HashMap index.
    pub fn project_by_shortcode(&'static self, shortcode: &str) -> Option<&'static Project> {
        self.project_index()
            .get(&shortcode.to_uppercase())
            .map(|&i| &self.all_projects()[i])
    }

    /// O(1) lookup of the raw (wire-contract) project by shortcode, using the same
    /// index as [`Corpus::project_by_shortcode`] — safe because the raw and view-model
    /// vectors are built in the same pass and stay index-aligned.
    pub fn project_raw_by_shortcode(&'static self, shortcode: &str) -> Option<&'static ProjectRaw> {
        self.project_index()
            .get(&shortcode.to_uppercase())
            .map(|&i| &self.all_projects_raw()[i])
    }

    fn project_index(&'static self) -> &'static HashMap<String, usize> {
        self.project_index_cache.get_or_init(|| {
            self.all_projects()
                .iter()
                .enumerate()
                .map(|(i, p)| (p.shortcode.to_uppercase(), i))
                .collect()
        })
    }

    fn load_all_projects(&self) -> (Vec<Project>, Vec<ProjectRaw>) {
        load_projects_from(
            &std::path::PathBuf::from(&self.settings.data_dir).join("projects"),
            self.settings.ark_resolver_base_url.as_deref(),
        )
    }
}

/// The directory pass itself, separated from the cache so a test can run it
/// over the committed corpus with a chosen resolver host.
///
/// Same reason `record_cache::index_by_shortcode` is separate: proving the
/// ingress normalisation actually happens on load needs the loader called
/// directly, independent of any particular corpus's settings.
pub(crate) fn load_projects_from(
    projects_dir: &std::path::Path,
    ark_resolver: Option<&str>,
) -> (Vec<Project>, Vec<ProjectRaw>) {
    use std::fs;

    let Ok(entries) = fs::read_dir(projects_dir) else {
        tracing::warn!(dir = ?projects_dir, "failed to read projects directory");
        return (vec![], vec![]);
    };

    let mut projects = Vec::new();
    let mut projects_raw = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        match fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<ProjectRaw>(&json) {
                Ok(mut raw) => {
                    // Ingress: the ARK host is normalised here, before either
                    // vector is built, so the view model and the wire contract
                    // cannot disagree about it and no reader downstream needs
                    // to know a resolver exists. See `crate::ark`.
                    crate::ark::normalise_project(&mut raw, ark_resolver);
                    projects.push(Project::from(raw.clone()));
                    projects_raw.push(raw);
                }
                Err(e) => tracing::warn!(file = %filename, error = %e, "failed to parse project"),
            },
            Err(e) => tracing::warn!(file = %filename, error = %e, "failed to read project file"),
        }
    }
    (projects, projects_raw)
}

#[cfg(test)]
mod ingress_tests {
    use super::*;

    const COMMITTED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects");
    const PREVIEW: &str = "https://dpe-pr-391-pbjdzenira-oa.a.run.app";

    /// The `*.json` files directly under `dir`.
    fn json_files_in(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .expect("a data directory should be readable")
            .flatten()
            .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
            .count()
    }

    /// The loader normalises on the way in, over the real committed corpus.
    ///
    /// Run against the loader rather than a corpus: this is the wiring the
    /// whole placement rests on — if the ARK is not normalised here, nothing
    /// downstream fixes it.
    #[test]
    fn a_configured_resolver_normalises_every_project_on_load() {
        let (projects, raws) = load_projects_from(std::path::Path::new(COMMITTED), Some(PREVIEW));
        assert_eq!(
            projects.len(),
            json_files_in(std::path::Path::new(COMMITTED)),
            "the committed corpus"
        );
        assert_eq!(projects.len(), raws.len(), "the two vectors stay index-aligned");

        for (project, raw) in projects.iter().zip(&raws) {
            assert!(
                raw.pid.starts_with(PREVIEW),
                "{}: the raw pid should carry the resolver, got {:?}",
                raw.shortcode,
                raw.pid
            );
            // The view model is built from the normalised raw in the same pass,
            // which is what makes the sidebar permalink correct for free.
            assert_eq!(project.pid, raw.pid, "{}: view model and wire contract", raw.shortcode);
            assert!(
                raw.pid.contains("ark:/72163/1/"),
                "{}: the ARK path is untouched, got {:?}",
                raw.shortcode,
                raw.pid
            );
        }
    }

    /// Unset — every deployment but a PR preview — the loader changes nothing.
    #[test]
    fn with_no_resolver_the_loader_leaves_every_recorded_ark_alone() {
        let (_, raws) = load_projects_from(std::path::Path::new(COMMITTED), None);
        assert!(!raws.is_empty());
        for raw in &raws {
            assert!(
                raw.pid.starts_with("https://ark.dasch.swiss/"),
                "{}: {:?}",
                raw.shortcode,
                raw.pid
            );
        }
    }

    /// Recorded text is quoted, not asserted: 083D records its own ARK as the
    /// project's website, and many citations mention one.
    #[test]
    fn recorded_text_keeps_the_host_the_corpus_records() {
        let (_, raws) = load_projects_from(std::path::Path::new(COMMITTED), Some(PREVIEW));
        let raw = raws.iter().find(|raw| raw.shortcode == "083D").expect("083D is committed");
        assert!(raw.pid.starts_with(PREVIEW), "{:?}", raw.pid);
        assert!(
            raw.url
                .as_ref()
                .expect("083D records a url")
                .to_string()
                .contains("ark.dasch.swiss"),
            "{:?}",
            raw.url
        );
        assert!(raw.how_to_cite.contains("ark.dasch.swiss"), "{}", raw.how_to_cite);
    }
}
