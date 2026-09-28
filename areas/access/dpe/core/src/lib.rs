//! DPE's domain layer: everything about the published metadata that only DPE
//! needs.
//!
//! The wire contract itself — the types a data file deserializes into, and the
//! rules for reading a value out of one — lives in `shared-metadata`, shared
//! with the editor. What is here is DPE's: the `Project` view model, the
//! process-global caches keyed on `DPE_DATA_DIR`, the repositories, cluster and
//! collection membership, contributor resolution, and the DSP-API records
//! client.

pub mod ark;
pub mod chronontology_cache;
pub mod cluster;
pub mod cluster_cache;
pub mod collection;
pub mod contributors;
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
pub use ark::{ark_resolver_base_url, set_ark_resolver_base_url};
pub use cluster::{ClusterRaw, ClusterRef};
pub use collection::CollectionRef;
pub use contributors::{load_organization, load_person, CachedContributorLookup, ResolvedContributor};
pub use cover_image_cache::cover_image_url;
pub use models::Page;
pub use project::{Project, VALID_TABS};
pub use project_cache::all_projects;
pub use project_repository::{FsProjectRepository, ProjectRepository};
pub use record_repository::{FsRecordRepository, RecordRepository};
pub use utils::{
    get_data_dir, get_public_dir, lang_value, language_display_name, set_data_dir, set_public_dir,
    set_show_placeholder_values, show_placeholder_values,
};

/// Everything resolving a project's metadata needs that DPE owns: the
/// contributor lookup and the two temporal-coverage tables, all three backed by
/// the process-global caches.
///
/// One function rather than three call sites reaching for three caches, so the
/// OAI endpoint and the landing page cannot end up resolving against different
/// inputs. Callers wrap the tuple in a `shared_fair::ResolveContext`; the tuple
/// itself names only `shared-metadata` types, so `dpe-core` never depends on the
/// exposure engine.
pub fn resolve_inputs() -> (
    &'static dyn shared_metadata::ContributorLookup,
    &'static std::collections::HashMap<String, shared_metadata::w3cdtf::W3cdtfRange>,
    &'static std::collections::HashMap<String, shared_metadata::temporal_enrichment::EnrichedDate>,
) {
    static LOOKUP: CachedContributorLookup = CachedContributorLookup;
    (
        &LOOKUP,
        chronontology_cache::all_periods(),
        temporal_enrichment_cache::all_enriched(),
    )
}
