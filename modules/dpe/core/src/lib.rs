pub mod cluster;
#[cfg(not(target_arch = "wasm32"))]
pub mod cluster_cache;
pub mod collection;
pub mod contributors;
pub mod models;
pub mod organization;
#[cfg(not(target_arch = "wasm32"))]
pub mod organization_cache;
pub mod person;
#[cfg(not(target_arch = "wasm32"))]
pub mod person_cache;
pub mod project;
#[cfg(not(target_arch = "wasm32"))]
pub mod project_cache;
pub mod project_repository;
pub mod record;
#[cfg(not(target_arch = "wasm32"))]
pub mod record_cache;
pub mod record_repository;
pub mod utils;

// Re-exports for convenience
pub use cluster::ClusterRef;
pub use collection::CollectionRef;
pub use contributors::ResolvedContributor;
#[cfg(not(target_arch = "wasm32"))]
pub use contributors::{load_organization, load_person};
pub use models::{AuthorityFileReference, Page};
pub use organization::Organization;
pub use person::Person;
pub use project::{
    AccessRights, AccessRightsType, Attribution, Discipline, Funding, Grant, LegalInfo, License, Project, ProjectRaw,
    ProjectStatus, Publication, TemporalCoverage, ACCESS_RIGHTS_VALUES,
};
#[cfg(not(target_arch = "wasm32"))]
pub use project_cache::all_projects;
#[cfg(not(target_arch = "wasm32"))]
pub use project_repository::FsProjectRepository;
pub use project_repository::ProjectRepository;
pub use record::{record_datestamp, Pid as RecordPid, Record, RecordLegalInfo, RecordLicense, ARK_PATH_PREFIX};
#[cfg(not(target_arch = "wasm32"))]
pub use record_repository::FsRecordRepository;
pub use record_repository::RecordRepository;
#[cfg(not(target_arch = "wasm32"))]
pub use utils::{get_data_dir, set_data_dir, set_show_placeholder_values, show_placeholder_values};
pub use utils::{is_placeholder, lang_value, language_display_name};
