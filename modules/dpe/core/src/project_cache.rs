//! In-process cache for the full project list.
//!
//! All projects are loaded from disk once on first access and held in memory
//! for the lifetime of the server process. This avoids re-reading and
//! re-deserializing every JSON file on every request.
use std::collections::HashMap;
use std::sync::OnceLock;

use shared_metadata::ProjectRaw;

use super::project::Project;
use super::utils::get_data_dir;

/// Both vectors come from the single directory pass in [`load_all_projects`]
/// and stay index-aligned: position `i` in one is the same project as
/// position `i` in the other, which is what lets [`SHORTCODE_INDEX`] (built
/// from the view-model vector only) also serve `all_projects_raw` and
/// `project_raw_by_shortcode`.
static PROJECTS_CACHE: OnceLock<(Vec<Project>, Vec<ProjectRaw>)> = OnceLock::new();
static SHORTCODE_INDEX: OnceLock<HashMap<String, usize>> = OnceLock::new();

/// Return a reference to the cached project list, loading it on first call.
pub fn all_projects() -> &'static Vec<Project> {
    &PROJECTS_CACHE.get_or_init(load_all_projects).0
}

/// Return a reference to the cached raw (wire-contract) project list, loading
/// it on first call. Index-aligned with [`all_projects`].
pub fn all_projects_raw() -> &'static Vec<ProjectRaw> {
    &PROJECTS_CACHE.get_or_init(load_all_projects).1
}

/// O(1) lookup of a project by shortcode using the cached HashMap index.
pub fn project_by_shortcode(shortcode: &str) -> Option<&'static Project> {
    shortcode_index().get(&shortcode.to_uppercase()).map(|&i| &all_projects()[i])
}

/// O(1) lookup of the raw (wire-contract) project by shortcode, using the same
/// index as [`project_by_shortcode`] — safe because the raw and view-model
/// vectors are built in the same pass and stay index-aligned.
pub fn project_raw_by_shortcode(shortcode: &str) -> Option<&'static ProjectRaw> {
    shortcode_index()
        .get(&shortcode.to_uppercase())
        .map(|&i| &all_projects_raw()[i])
}

fn shortcode_index() -> &'static HashMap<String, usize> {
    SHORTCODE_INDEX.get_or_init(|| {
        all_projects()
            .iter()
            .enumerate()
            .map(|(i, p)| (p.shortcode.to_uppercase(), i))
            .collect()
    })
}

fn load_all_projects() -> (Vec<Project>, Vec<ProjectRaw>) {
    load_projects_from(
        &std::path::PathBuf::from(get_data_dir()).join("projects"),
        crate::ark::ark_resolver_base_url(),
    )
}

/// The directory pass itself, separated from the cache so a test can run it
/// over the committed corpus with a chosen resolver host.
///
/// Same reason `record_cache::index_by_shortcode` is separate: the cache is a
/// process-global keyed on `DPE_DATA_DIR`, which a test cannot vary, so the
/// only way to prove the ingress normalisation actually happens on load is to
/// be able to call the loader directly.
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

    /// The loader normalises on the way in, over the real committed corpus.
    ///
    /// Run against the loader rather than the cache: the cache is a
    /// process-global keyed on `DPE_DATA_DIR`, so a test cannot give it a
    /// resolver. This is the wiring the whole placement rests on — if the ARK
    /// is not normalised here, nothing downstream fixes it.
    #[test]
    fn a_configured_resolver_normalises_every_project_on_load() {
        let (projects, raws) = load_projects_from(std::path::Path::new(COMMITTED), Some(PREVIEW));
        assert_eq!(projects.len(), 85, "the committed corpus");
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
