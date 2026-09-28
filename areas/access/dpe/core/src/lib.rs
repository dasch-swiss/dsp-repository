//! DPE's domain layer: everything about the published metadata that only DPE
//! needs.
//!
//! The wire contract itself — the types a data file deserializes into, and the
//! rules for reading a value out of one — lives in `shared-metadata`, shared
//! with the editor. What is here is DPE's: the `Project` view model, the
//! [`Corpus`] a capability owns and everything cached on it, the repositories,
//! cluster and collection membership, contributor resolution, and the DSP-API
//! records client.

pub mod ark;
pub mod chronontology_cache;
pub mod cluster;
pub mod cluster_cache;
pub mod collection;
pub mod contributors;
pub mod corpus;
pub mod cover_image_cache;
pub mod models;
pub mod organization_cache;
pub mod person_cache;
pub mod project;
pub mod project_cache;
pub mod project_repository;
pub mod record_cache;
pub mod record_repository;
pub mod temporal_enrichment_cache;
pub mod utils;

// Re-exports for convenience
pub use cluster::{ClusterRaw, ClusterRef};
pub use collection::CollectionRef;
pub use contributors::{CachedContributorLookup, ResolvedContributor};
pub use corpus::{Corpus, CorpusSettings};
pub use models::Page;
pub use project::{Project, VALID_TABS};
pub use project_repository::{FsProjectRepository, ProjectRepository};
pub use record_repository::{FsRecordRepository, RecordRepository};
pub use utils::{lang_value, language_display_name};

impl Corpus {
    /// Everything resolving a project's metadata needs that DPE owns: the
    /// contributor lookup and the two temporal-coverage tables, all three
    /// backed by this corpus's caches.
    ///
    /// One method rather than three call sites reaching for three caches, so
    /// the OAI endpoint and the landing page cannot end up resolving against
    /// different inputs. Callers wrap the tuple in a `shared_fair::ResolveContext`;
    /// the tuple itself names only `shared-metadata` types, so `dpe-core` never
    /// depends on the exposure engine.
    ///
    /// Returns the lookup by value — it borrows `self` and there is nowhere
    /// process-global left to put a `&'static` one — so a caller keeps it
    /// alive alongside the other two and passes `&lookup`.
    pub fn resolve_inputs(
        &'static self,
    ) -> (
        CachedContributorLookup,
        &'static std::collections::HashMap<String, shared_metadata::w3cdtf::W3cdtfRange>,
        &'static std::collections::HashMap<String, shared_metadata::temporal_enrichment::EnrichedDate>,
    ) {
        (
            CachedContributorLookup { corpus: self },
            self.all_periods(),
            self.all_enriched(),
        )
    }
}
