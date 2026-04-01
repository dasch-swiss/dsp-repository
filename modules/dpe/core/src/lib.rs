pub mod cluster;
pub mod collection;
pub mod contributors;
pub mod models;
pub mod organization;
pub mod person;
pub mod project;
#[cfg(not(target_arch = "wasm32"))]
pub mod cluster_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod organization_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod person_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod project_cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod record_cache;
pub mod project_repository;
pub mod record;
pub mod record_repository;
pub mod utils;

// Re-exports for convenience
pub use cluster::ClusterRef;
pub use collection::CollectionRef;
pub use contributors::ResolvedContributor;
pub use models::{AuthorityFileReference, Page};
pub use organization::Organization;
pub use person::Person;
pub use project::{
    AccessRights, AccessRightsType, Attribution, Discipline, Funding, Grant, LegalInfo, License,
    Project, ProjectRaw, ProjectStatus, TemporalCoverage, ACCESS_RIGHTS_VALUES,
};
pub use project_repository::ProjectRepository;
pub use project::Publication;
pub use record::{record_datestamp, Pid as RecordPid, Record, RecordLegalInfo, RecordLicense, ARK_PATH_PREFIX};
pub use record_repository::RecordRepository;
pub use utils::lang_value;

#[cfg(not(target_arch = "wasm32"))]
pub use contributors::{load_organization, load_person};
#[cfg(not(target_arch = "wasm32"))]
pub use project_cache::all_projects;
#[cfg(not(target_arch = "wasm32"))]
pub use project_repository::FsProjectRepository;
#[cfg(not(target_arch = "wasm32"))]
pub use record_repository::FsRecordRepository;
#[cfg(not(target_arch = "wasm32"))]
pub use utils::{get_data_dir, set_data_dir};
