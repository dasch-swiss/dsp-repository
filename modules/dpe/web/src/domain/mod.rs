// Domain types are defined in dpe-core and re-exported here for backward compatibility.
// Leptos-specific server functions and ProjectQuery (with Params derive) remain in this crate.

// Leptos-specific modules
pub mod contributors;
pub mod project;
pub mod projects;

// Re-export all core domain types for backward compatibility
// Re-export sync helpers (formerly `#[server]` server functions, now plain
// `pub fn` gated on non-wasm) and ProjectQuery from local modules.
#[cfg(not(target_arch = "wasm32"))]
pub use contributors::get_contributors;
pub use dpe_core::project::Publication;
// Re-export record::Pid (ARK-based) as the default Pid — it's the more commonly used one.
// Project::Pid (url + text for publications) is accessed via project::Pid when needed.
pub use dpe_core::record::Pid;
#[cfg(feature = "ssr")]
pub use dpe_core::{all_projects, get_data_dir, load_organization, load_person};
pub use dpe_core::{
    cluster, collection, lang_value, models, organization, person, record, record_datestamp, utils, AccessRights,
    AccessRightsType, Attribution, AuthorityFileReference, ClusterRef, CollectionRef, Discipline, Funding, Grant,
    LegalInfo, License, Organization, Page, Person, Project, ProjectRaw, ProjectRepository, ProjectStatus, Record,
    RecordLegalInfo, RecordLicense, RecordRepository, ResolvedContributor, TemporalCoverage, ACCESS_RIGHTS_VALUES,
    ARK_PATH_PREFIX,
};
#[cfg(feature = "ssr")]
pub use dpe_core::{FsProjectRepository, FsRecordRepository};
pub use project::ProjectQuery;
#[cfg(not(target_arch = "wasm32"))]
pub use projects::{get_project, list_data_languages, list_projects, list_type_of_data};
