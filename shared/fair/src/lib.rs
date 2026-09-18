//! The FAIR exposure engine: one resolved graph per published object, and a
//! writer per representation that reads it.
//!
//! Nothing here knows where a page is rendered. The crate has no routes, no
//! Maud and no Axum: a writer returns a `String` or a `serde_json::Value`, and
//! the consuming service turns that into a response. It holds no path into a
//! service module either, which `.github/scripts/check-shared-paths.sh`
//! enforces — corpus-wide tests over the committed data stay in `dpe-api-oai`,
//! beside the data they read.

pub mod datacite;
pub mod dublin_core;
pub mod dublin_core_meta;
pub mod graph;
pub mod helpers;
pub mod project_graph;
pub mod record_datacite;
pub mod record_dublin_core;
pub mod resolve;
pub mod schema_org;
pub mod signposting;
#[cfg(test)]
pub(crate) mod test_support;
pub mod types;

pub use datacite::project_to_datacite;
pub use dublin_core::project_to_dublin_core;
pub use dublin_core_meta::project_to_dublin_core_meta;
pub use graph::{AgentKind, PartRef, RecordCreator, RecordGraph, ResolveContext};
pub use helpers::coar_access_right;
pub use project_graph::{
    DisciplineRef, FundingRef, LicenseRef, ProjectAgent, ProjectGraph, PublicationRef, SpatialRef, TemporalRef,
};
pub use record_datacite::record_to_datacite;
pub use record_dublin_core::record_to_dublin_core;
pub use schema_org::{project_to_schema_org, script_safe_json, SchemaOrgOptions};
pub use signposting::{project_to_link_set, Candidate, Link, LinkSet, UrlLayout};
pub use types::{
    DataCiteContributor, DataCiteCreator, DataCiteDate, DataCiteDescription, DataCiteFundingReference,
    DataCiteGeoLocation, DataCiteNameIdentifier, DataCiteRecord, DataCiteRelatedIdentifier, DataCiteRights,
    DataCiteSubject, DataCiteTitle, DublinCoreRecord,
};
