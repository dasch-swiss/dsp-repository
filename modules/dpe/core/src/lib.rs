//! DPE's domain layer: everything about the published metadata that only DPE
//! needs.
//!
//! The wire contract itself — the types a data file deserializes into, and the
//! rules for reading a value out of one — lives in `platform-metadata`, shared
//! with the editor. What is here is DPE's: the `Project` view model, the
//! process-global caches keyed on `DPE_DATA_DIR`, the repositories, cluster and
//! collection membership, contributor resolution, and the DSP-API records
//! client.

pub mod chronontology_cache;
pub mod cluster;
pub mod cluster_cache;
pub mod collection;
pub mod contributors;
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
pub use contributors::{
    is_organization_id, load_organization, load_person, CachedContributorLookup, ContributorLookup, ResolvedContributor,
};
pub use models::Page;
pub use project::{Project, VALID_TABS};
pub use project_cache::all_projects;
pub use project_repository::{FsProjectRepository, ProjectRepository};
pub use record_repository::{FsRecordRepository, RecordRepository};
pub use utils::{
    get_data_dir, lang_value, language_display_name, set_data_dir, set_show_placeholder_values, show_placeholder_values,
};
