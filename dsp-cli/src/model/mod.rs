//! Domain models — layer 4 of dsp-cli/ADR-0008.
//!
//! Vocabulary follows `CONTEXT.md`: `Project`, `DataModel`, `ResourceType`,
//! `Resource`, `Field`, `Value`, `ValueType`, `BuiltIn`. DSP-API wire types
//! (`OntologyDto` etc.) live in `crate::client`, not here — see
//! `dsp-cli/CONTEXT.md`.

pub mod auth;
pub mod data_model;
pub mod dump;
pub mod project;
pub mod resource;
pub mod resource_type;
pub mod structure;
pub mod vocabulary;
pub use auth::LoginResponse;
pub use data_model::{DataModel, DataModelDetail, ResourceType, ResourceTypeSummary};
pub use dump::{CreateDumpOutcome, DumpStatus, DumpTask};
pub use project::{DataModelSummary, Project, ProjectDescription, ProjectDetail, ProjectRef};
pub use resource::{
    DatePoint, DateValue, FieldValues, FileValue, ResourceAccess, ResourceDetail, ResourcePage, ResourceSummary,
    ResourceVisibility, Value, ValueContent,
};
pub use resource_type::{Cardinality, Field, Representation, ResourceTypeDetail, ValueType};
pub use structure::{DataModelStructure, Relation, RelationKind};
pub use vocabulary::{LocalizedText, Vocabulary, VocabularyDetail, VocabularyHeader, VocabularyNode, VocabularyTree};
