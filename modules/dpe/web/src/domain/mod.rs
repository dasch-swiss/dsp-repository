// Domain types are defined in dpe-core and re-exported here for backward compatibility.
// Leptos-specific server functions and ProjectQuery (with Params derive) remain in this crate.

// Leptos-specific modules
pub mod contributors;
pub mod organizations;
pub mod persons;
pub mod project;
pub mod projects;

// Re-export all core domain types for backward compatibility
pub use dpe_core::cluster;
pub use dpe_core::collection;
pub use dpe_core::models;
pub use dpe_core::organization;
pub use dpe_core::person;
pub use dpe_core::record;
pub use dpe_core::utils;
#[cfg(feature = "ssr")]
pub use dpe_core::{all_projects, get_data_dir, load_organization, load_person};
pub use dpe_core::{
    lang_value, record_datestamp, AccessRights, AccessRightsType, Attribution, AuthorityFileReference, ClusterRef,
    CollectionRef, Discipline, Funding, Grant, LegalInfo, License, Organization, Page, Person, Project, ProjectRaw,
    ProjectStatus, ResolvedContributor, Record, RecordLegalInfo, RecordLicense, TemporalCoverage,
    ACCESS_RIGHTS_VALUES, ARK_PATH_PREFIX,
};
pub use dpe_core::project::Publication;
// Re-export record::Pid (ARK-based) as the default Pid — it's the more commonly used one.
// Project::Pid (url + text for publications) is accessed via project::Pid when needed.
pub use dpe_core::record::Pid;
#[cfg(feature = "ssr")]
pub use dpe_core::{FsProjectRepository, FsRecordRepository};
pub use dpe_core::{ProjectRepository, RecordRepository};

// Re-export server functions and ProjectQuery from local modules
pub use contributors::get_contributors;
pub use organizations::get_organization;
pub use persons::get_person;
pub use project::ProjectQuery;
pub use projects::{get_project, list_data_languages, list_projects, list_type_of_data};
