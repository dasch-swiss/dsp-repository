//! The DSP research-metadata wire contract, shared by DPE and the editor.
//!
//! What belongs here is what both services must agree on: the shape a
//! `projects/*.json`, `persons/*.json`, `organizations/*.json` or records file
//! deserializes into, and the rules that decide what a value in it *means* —
//! placeholder detection, the deterministic multilingual lookup key, W3CDTF
//! formatting, and temporal-coverage resolution.
//!
//! What does not belong here is anything one service owns: DPE's `Project` view
//! model and its lossy conversions, the process-global caches keyed on
//! `DPE_DATA_DIR`, the repositories, cluster membership, and the DSP-API records
//! client all stay in `dpe-core`. Table loading is exposed as
//! `load_from(data_dir)` so each service supplies its own directory rather than
//! this crate reaching for one.

pub mod chronontology;
pub mod models;
pub mod organization;
pub mod person;
pub mod project;
pub mod record;
pub mod temporal_coverage;
pub mod temporal_enrichment;
pub mod utils;
pub mod w3cdtf;

// Re-exports for convenience
pub use models::AuthorityFileReference;
pub use organization::{Address, Organization};
pub use person::{is_role_job_title, Person, JOB_TITLE_ROLE_WORDS};
pub use project::{
    is_valid_shortcode, AccessRights, AccessRightsType, Attribution, Discipline, Funding, Grant, LegalInfo, License,
    ProjectRaw, ProjectStatus, Publication, TemporalCoverage, ACCESS_RIGHTS_VALUES, MAX_SHORTCODE_LEN,
};
pub use record::{
    record_datestamp, Pid as RecordPid, Record, RecordFile, RecordLegalInfo, RecordLicense, ARK_PATH_PREFIX,
};
pub use utils::{is_placeholder, multilingual_value, Multilingual};
