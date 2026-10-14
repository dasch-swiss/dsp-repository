//! Behind the `test-support` feature, which only dev-dependencies enable, so the release build
//! does not compile it.
//!
//! The committed corpora, not fixtures: every project page renders from them, and a fixture of
//! invented projects would let a change that only works for invented data pass. The real set
//! carries the mixed-case shortcodes, the `MISSING` placeholders and the filenames that disagree
//! with their shortcode.

use std::sync::{Arc, OnceLock};

use editor_core::published::PublishedProjects;

use crate::shell::TemporalTables;

/// The committed project corpus, loaded once for the whole test binary.
///
/// Parsing 85 files per test state would be paid by every handler test, and most never look at a
/// project. Behind a `OnceLock` and shared by `Arc`, which the set's immutability makes safe.
pub(crate) fn published_corpus() -> Arc<PublishedProjects> {
    static CORPUS: OnceLock<Arc<PublishedProjects>> = OnceLock::new();
    CORPUS
        .get_or_init(|| {
            let dir = editor_core::checkout_dpe_data_dir().join("projects");
            let (published, errors) = PublishedProjects::load_from(&dir);
            assert!(errors.is_empty(), "the committed corpus should load cleanly: {errors:?}");
            Arc::new(published)
        })
        .clone()
}

/// The committed agent set, loaded once for the whole test binary. See [`published_corpus`] for
/// why this is cached rather than reloaded per test.
///
/// Every committed contributor and contact reference resolves through it, so invented agents
/// would let a change that only works for invented ids pass.
pub(crate) fn agent_corpus() -> Arc<editor_core::agents::Agents> {
    static AGENTS: OnceLock<Arc<editor_core::agents::Agents>> = OnceLock::new();
    AGENTS
        .get_or_init(|| {
            let dir = editor_core::checkout_dpe_data_dir();
            let (agents, errors) =
                editor_core::agents::Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
            assert!(errors.is_empty(), "the committed agent set should load cleanly: {errors:?}");
            Arc::new(agents)
        })
        .clone()
}

/// The committed temporal-resolution tables, loaded once for the whole test binary.
///
/// A fixture of invented periods would let submit validation pass while disagreeing with what
/// `access-server validate` decides about the committed projects, which is the whole point of
/// sharing the tables.
pub(crate) fn temporal_tables() -> Arc<TemporalTables> {
    static TABLES: OnceLock<Arc<TemporalTables>> = OnceLock::new();
    TABLES
        .get_or_init(|| {
            let dir = editor_core::checkout_dpe_data_dir();
            Arc::new(TemporalTables {
                periods: shared_metadata::chronontology::load_from(&dir),
                enrichment: shared_metadata::temporal_enrichment::load_from(&dir),
            })
        })
        .clone()
}
