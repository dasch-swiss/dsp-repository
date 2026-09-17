//! Metadata transformation for OAI-PMH records.
//!
//! This module handles the transformation of Research Projects and Records into Dublin Core
//! and DataCite 4.6 metadata formats, following the DaSCH Metadata to DataCite
//! mapping specification.

#[cfg(test)]
mod corpus;
mod types;

use dpe_core::cluster_cache::clusters_for_shortcode_in;
use dpe_core::ClusterRaw;
use shared_fair::{
    project_to_datacite, project_to_dublin_core, record_to_datacite, record_to_dublin_core, ProjectGraph,
    ResolveContext,
};
pub use shared_fair::{DataCiteNameIdentifier, DataCiteRecord, DublinCoreRecord};
use shared_metadata::{record_datestamp, ContributorLookup, ProjectRaw, Record, ARK_PATH_PREFIX};
pub use types::{OaiRecord, OaiRecordHeader};

// Namespace identifier for OAI record identifiers (OAI identifier format:
// `oai:<namespace-identifier>:<local-identifier>`). This is a persistent, host-independent
// identifier authority and is deliberately distinct from the access `baseURL`.
const OAI_IDENTIFIER_PREFIX: &str = "oai:dasch.swiss:";

/// Creates an OAI identifier from a project shortcode.
pub fn make_oai_identifier(shortcode: &str) -> String {
    format!("{}{}{}", OAI_IDENTIFIER_PREFIX, ARK_PATH_PREFIX, shortcode)
}

/// Creates an OAI identifier from a project PID URL.
///
/// Extracts the ARK path from the PID URL (e.g. `https://ark.dasch.swiss/ark:/72163/1/0801d`)
/// and wraps it in the OAI identifier prefix.
fn make_oai_identifier_from_pid(pid: &str) -> Option<String> {
    let pos = pid.find(ARK_PATH_PREFIX)?;
    Some(format!("{}{}", OAI_IDENTIFIER_PREFIX, &pid[pos..]))
}

/// Parses an OAI identifier and extracts the ARK suffix.
pub fn parse_oai_identifier(identifier: &str) -> Option<String> {
    if !identifier.starts_with(OAI_IDENTIFIER_PREFIX) {
        return None;
    }
    let ark_part = &identifier[OAI_IDENTIFIER_PREFIX.len()..];
    ark_part.strip_prefix(ARK_PATH_PREFIX).map(|s| s.to_string())
}

/// Builds the set specs for a project shortcode: its `project:{shortcode}` set and
/// one `cluster:{id}` set per cluster the project belongs to (resolved from the
/// given cluster slice). The caller prepends the entity-type set spec.
fn membership_set_specs(entity_type: &str, shortcode: &str, clusters: &[ClusterRaw]) -> Vec<String> {
    let mut specs = vec![entity_type.to_string(), format!("project:{shortcode}")];
    for cluster in clusters_for_shortcode_in(clusters, shortcode) {
        specs.push(format!("cluster:{}", cluster.id));
    }
    specs
}

/// Creates an OAI record from a project for the given metadata prefix.
pub fn to_oai_record(
    project: &ProjectRaw,
    metadata_prefix: &str,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
) -> OaiRecord {
    // The temporal tables come from `resolve_inputs`, the one place in DPE that
    // says what resolution needs, so this endpoint and the landing page cannot
    // drift apart. The lookup it returns is discarded: handlers take theirs as a
    // parameter, which is how the tests inject an in-memory double.
    let (_cached_lookup, periods, enriched) = dpe_core::resolve_inputs();
    let ctx = ResolveContext::new(lookup, periods, enriched);
    let identifier = if !shared_metadata::is_placeholder(&project.pid) && !project.pid.is_empty() {
        make_oai_identifier_from_pid(&project.pid).unwrap_or_else(|| make_oai_identifier(&project.shortcode))
    } else {
        make_oai_identifier(&project.shortcode)
    };
    let header = OaiRecordHeader {
        identifier,
        datestamp: if !shared_metadata::is_placeholder(&project.start_date) && !project.start_date.is_empty() {
            project.start_date.clone()
        } else {
            "2015-01-01".to_string()
        },
        set_specs: membership_set_specs("entityType:ResearchProject", &project.shortcode, clusters),
    };

    // No records: `parts` feeds a future `hasPart`, which neither OAI writer reads.
    let graph = ProjectGraph::build(project, &ctx, &[]);

    let dublin_core = if metadata_prefix == "oai_dc" {
        Some(project_to_dublin_core(&graph))
    } else {
        None
    };

    let datacite = if metadata_prefix == "oai_datacite" {
        Some(project_to_datacite(&graph))
    } else {
        None
    };

    OaiRecord { header, dublin_core, datacite }
}

/// Creates an OAI record from a Record for the given metadata prefix.
pub fn to_oai_record_from_record(record: &Record, metadata_prefix: &str, clusters: &[ClusterRaw]) -> OaiRecord {
    let suffix_owned = record.pid.ark_suffix();
    let suffix = &suffix_owned;
    let header = OaiRecordHeader {
        identifier: make_oai_identifier(suffix),
        datestamp: record_datestamp(record),
        set_specs: membership_set_specs("entityType:Record", &record.pid.shortcode, clusters),
    };

    let dublin_core = if metadata_prefix == "oai_dc" {
        Some(record_to_dublin_core(record))
    } else {
        None
    };

    let datacite = if metadata_prefix == "oai_datacite" {
        Some(record_to_datacite(record))
    } else {
        None
    };

    OaiRecord { header, dublin_core, datacite }
}

/// Checks if a record matches the given date filter.
pub fn matches_date_filter_record(record: &Record, from: Option<&str>, until: Option<&str>) -> bool {
    let datestamp = record_datestamp(record);
    if let Some(from_date) = from {
        if datestamp.as_str() < from_date {
            return false;
        }
    }
    if let Some(until_date) = until {
        if datestamp.as_str() > until_date {
            return false;
        }
    }
    true
}

/// Checks if a project matches the given date filter.
pub fn matches_date_filter(project: &ProjectRaw, from: Option<&str>, until: Option<&str>) -> bool {
    let datestamp = if !shared_metadata::is_placeholder(&project.start_date) && !project.start_date.is_empty() {
        &project.start_date
    } else {
        "2015-01-01"
    };

    if let Some(from_date) = from {
        if datestamp < from_date {
            return false;
        }
    }

    if let Some(until_date) = until {
        if datestamp > until_date {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{make_oai_identifier, parse_oai_identifier};

    #[test]
    fn test_make_oai_identifier() {
        let id = make_oai_identifier("0801");
        assert_eq!(id, "oai:dasch.swiss:ark:/72163/1/0801");
    }

    #[test]
    fn test_parse_oai_identifier() {
        let shortcode = parse_oai_identifier("oai:dasch.swiss:ark:/72163/1/0801");
        assert_eq!(shortcode, Some("0801".to_string()));
    }

    #[test]
    fn test_parse_oai_identifier_invalid() {
        let shortcode = parse_oai_identifier("invalid:identifier");
        assert_eq!(shortcode, None);
    }
}
