//! OAI-PMH verb handlers.
//!
//! This module implements the six required OAI-PMH 2.0 verbs:
//! - Identify
//! - ListMetadataFormats
//! - ListSets
//! - ListIdentifiers
//! - ListRecords
//! - GetRecord

mod get_record;
mod identify;
mod list_identifiers;
mod list_metadata_formats;
mod list_records;
mod list_sets;

use axum::extract::Query;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use dpe_core::{
    cluster_cache, CachedContributorLookup, ClusterRaw, ContributorLookup, FsProjectRepository, FsRecordRepository,
    ProjectRepository, RecordRepository,
};
use get_record::handle_get_record;
use identify::handle_identify;
use list_identifiers::handle_list_identifiers;
use list_metadata_formats::handle_list_metadata_formats;
use list_records::handle_list_records;
use list_sets::handle_list_sets;
use serde::Deserialize;

use super::error::OaiError;
use super::xml::OaiXmlBuilder;
use crate::metadata::{
    matches_date_filter, matches_date_filter_record, to_oai_record, to_oai_record_from_record, OaiRecord,
};

/// Query parameters for OAI-PMH requests.
#[derive(Debug, Deserialize)]
pub struct OaiParams {
    pub verb: Option<String>,
    pub identifier: Option<String>,
    #[serde(rename = "metadataPrefix")]
    pub metadata_prefix: Option<String>,
    pub from: Option<String>,
    pub until: Option<String>,
    pub set: Option<String>,
    #[serde(rename = "resumptionToken")]
    pub resumption_token: Option<String>,
}

/// Supported metadata formats.
pub const SUPPORTED_PREFIXES: [&str; 2] = ["oai_dc", "oai_datacite"];

/// Main OAI-PMH handler that dispatches to verb-specific handlers.
pub async fn oai_handler(Query(params): Query<OaiParams>) -> impl IntoResponse {
    let repo = FsProjectRepository::new();
    let record_repo = FsRecordRepository::new();
    let clusters = cluster_cache::all_clusters();
    let lookup = CachedContributorLookup;

    let xml = match params.verb.as_deref() {
        Some("Identify") => handle_identify(&params, &repo),
        Some("ListMetadataFormats") => handle_list_metadata_formats(&params, &repo),
        Some("ListSets") => handle_list_sets(&params, &repo, clusters),
        Some("ListIdentifiers") => handle_list_identifiers(&params, &repo, &record_repo, clusters, &lookup),
        Some("ListRecords") => handle_list_records(&params, &repo, &record_repo, clusters, &lookup),
        Some("GetRecord") => handle_get_record(&params, &repo, &record_repo, clusters, &lookup),
        Some(_) => build_error_response(OaiError::BadVerb, None),
        None => build_error_response(OaiError::BadVerb, None),
    };

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml; charset=utf-8")], xml)
}

/// Builds an error response. Pass `Some(verb)` for recognized verbs so the verb is echoed
/// in the request element per OAI-PMH 2.0 section 3.6. Pass `None` only for badVerb.
pub fn build_error_response(error: OaiError, verb: Option<&str>) -> String {
    let mut builder = OaiXmlBuilder::new();
    match verb {
        Some(v) => builder.write_error_request_with_verb(v),
        None => builder.write_error_request(),
    }
    builder.write_error(&error);
    builder.finish()
}

/// The syntactic meaning of a `set` argument, resolved purely from the string —
/// no repository or cluster-cache access. Existence of a named project/cluster is
/// validated later, in `validate_list_params`.
#[derive(Debug, PartialEq)]
pub enum SetSyntax {
    /// No `set` argument — include everything.
    All,
    /// An `entityType:*` set, expanded to which entity kinds it selects.
    EntityType { clusters: bool, projects: bool, records: bool },
    /// `project:{shortcode}` — the records of one research project.
    Project(String),
    /// `cluster:{id}` — the project entries of a cluster plus all their records.
    Cluster(String),
}

/// Parses the syntactic meaning of a `set` argument.
///
/// Returns `Err(OaiError::BadArgument)` for an unrecognised prefix or an empty
/// `project:`/`cluster:` value. Whether a named project/cluster actually exists
/// is checked separately (repository/cache access required).
pub fn parse_set_syntax(set: Option<&str>) -> Result<SetSyntax, OaiError> {
    let Some(set) = set else {
        return Ok(SetSyntax::All);
    };

    match set {
        "entityType:ProjectCluster" => Ok(SetSyntax::EntityType { clusters: true, projects: false, records: false }),
        "entityType:ResearchProject" => Ok(SetSyntax::EntityType { clusters: false, projects: true, records: false }),
        "entityType:Record" => Ok(SetSyntax::EntityType { clusters: false, projects: false, records: true }),
        _ => {
            if let Some(shortcode) = set.strip_prefix("project:") {
                if shortcode.is_empty() {
                    return Err(OaiError::BadArgument("set argument has an empty project shortcode".to_string()));
                }
                Ok(SetSyntax::Project(shortcode.to_string()))
            } else if let Some(id) = set.strip_prefix("cluster:") {
                if id.is_empty() {
                    return Err(OaiError::BadArgument("set argument has an empty cluster id".to_string()));
                }
                Ok(SetSyntax::Cluster(id.to_string()))
            } else {
                Err(OaiError::BadArgument(format!("unsupported set: {set}")))
            }
        }
    }
}

/// Validates the common parameters for ListIdentifiers and ListRecords and returns
/// the validated metadata prefix and the filtered list of OAI records.
///
/// Returns `Err(OaiError)` if any validation step fails.
pub fn validate_list_params<'a>(
    params: &'a OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
) -> Result<(&'a str, Vec<OaiRecord>), OaiError> {
    // metadataPrefix is required. Validated BEFORE the set, so a request that is
    // missing/invalid in both reports the prefix error (precedence preserved).
    let prefix = params
        .metadata_prefix
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("metadataPrefix argument is required".to_string()))?;

    // Validate metadataPrefix
    if !SUPPORTED_PREFIXES.contains(&prefix) {
        return Err(OaiError::CannotDisseminateFormat);
    }

    // identifier is not valid for list verbs
    if params.identifier.is_some() {
        return Err(OaiError::BadArgument("identifier argument is not allowed".to_string()));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return Err(OaiError::BadResumptionToken);
    }

    // Syntactic parse — an unsupported set or empty value is a badArgument.
    let syntax = parse_set_syntax(params.set.as_deref())?;

    let from = params.from.as_deref();
    let until = params.until.as_deref();

    let oai_records = match syntax {
        SetSyntax::All => collect_entities(repo, record_repo, prefix, clusters, lookup, from, until, true, true),
        SetSyntax::EntityType {
            projects: include_projects,
            records: include_records,
            // `entityType:ProjectCluster` selects nothing today (clusters are not
            // emitted as first-class OAI items), so its records list is empty.
            clusters: _,
        } => collect_entities(
            repo,
            record_repo,
            prefix,
            clusters,
            lookup,
            from,
            until,
            include_projects,
            include_records,
        ),
        SetSyntax::Project(shortcode) => {
            // Existence check: an unknown project shortcode is a badArgument.
            if repo.get_by_shortcode(&shortcode).is_none() {
                return Err(OaiError::BadArgument(format!("unknown project set: project:{shortcode}")));
            }
            // Records of this project only — no project entry (REQ-1.3).
            record_repo
                .get_all()
                .iter()
                .filter(|r| r.pid.shortcode.eq_ignore_ascii_case(&shortcode))
                .filter(|r| matches_date_filter_record(r, from, until))
                .map(|r| to_oai_record_from_record(r, prefix, clusters))
                .collect()
        }
        SetSyntax::Cluster(id) => {
            // Existence check: an unknown cluster id is a badArgument.
            let Some(member_shortcodes) = cluster_cache::projects_for_cluster_in(clusters, &id) else {
                return Err(OaiError::BadArgument(format!("unknown cluster set: cluster:{id}")));
            };
            collect_cluster(repo, record_repo, prefix, clusters, lookup, from, until, member_shortcodes)
        }
    };

    if oai_records.is_empty() {
        return Err(OaiError::NoRecordsMatch);
    }

    Ok((prefix, oai_records))
}

/// Collects project and/or record OAI items for the entity-type and `All` cases.
#[allow(clippy::too_many_arguments)]
fn collect_entities(
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    prefix: &str,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
    from: Option<&str>,
    until: Option<&str>,
    include_projects: bool,
    include_records: bool,
) -> Vec<OaiRecord> {
    let mut oai_records: Vec<OaiRecord> = if include_projects {
        repo.get_all()
            .iter()
            .filter(|p| matches_date_filter(p, from, until))
            .map(|p| to_oai_record(p, prefix, clusters, lookup))
            .collect()
    } else {
        Vec::new()
    };

    if include_records {
        let mut record_oai: Vec<OaiRecord> = record_repo
            .get_all()
            .iter()
            .filter(|r| matches_date_filter_record(r, from, until))
            .map(|r| to_oai_record_from_record(r, prefix, clusters))
            .collect();
        oai_records.append(&mut record_oai);
    }

    oai_records
}

/// Collects all entities under a cluster: the project entries whose shortcode is a
/// cluster member (and which exist in the repository) plus all their records.
/// Project entries are deduplicated by shortcode and records by ARK suffix, so a
/// shortcode listed more than once does not produce duplicate items (REQ-2.5).
#[allow(clippy::too_many_arguments)]
fn collect_cluster(
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    prefix: &str,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
    from: Option<&str>,
    until: Option<&str>,
    member_shortcodes: &[String],
) -> Vec<OaiRecord> {
    let is_member = |shortcode: &str| member_shortcodes.iter().any(|m| m.eq_ignore_ascii_case(shortcode));

    // Project entries: members that actually exist in the repository, deduplicated
    // by shortcode (case-insensitive).
    let mut seen_projects: Vec<String> = Vec::new();
    let mut oai_records: Vec<OaiRecord> = repo
        .get_all()
        .iter()
        .filter(|p| is_member(&p.shortcode))
        .filter(|p| matches_date_filter(p, from, until))
        .filter(|p| {
            let key = p.shortcode.to_ascii_lowercase();
            if seen_projects.contains(&key) {
                false
            } else {
                seen_projects.push(key);
                true
            }
        })
        .map(|p| to_oai_record(p, prefix, clusters, lookup))
        .collect();

    // Records of member projects, deduplicated by ARK suffix.
    let mut seen_records: Vec<String> = Vec::new();
    let mut record_oai: Vec<OaiRecord> = record_repo
        .get_all()
        .iter()
        .filter(|r| is_member(&r.pid.shortcode))
        .filter(|r| matches_date_filter_record(r, from, until))
        .filter(|r| {
            let key = r.pid.ark_suffix();
            if seen_records.contains(&key) {
                false
            } else {
                seen_records.push(key);
                true
            }
        })
        .map(|r| to_oai_record_from_record(r, prefix, clusters))
        .collect();
    oai_records.append(&mut record_oai);

    oai_records
}

/// Builds the request parameter list shared by list verb XML responses.
pub fn build_list_request_params<'a>(prefix: &'a str, params: &'a OaiParams) -> Vec<(&'a str, &'a str)> {
    let mut request_params = vec![("metadataPrefix", prefix)];
    if let Some(ref from) = params.from {
        request_params.push(("from", from.as_str()));
    }
    if let Some(ref until) = params.until {
        request_params.push(("until", until.as_str()));
    }
    if let Some(ref set) = params.set {
        request_params.push(("set", set.as_str()));
    }
    request_params
}

#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod tests {
    use super::{build_error_response, parse_set_syntax, SetSyntax};
    use crate::error::OaiError;

    #[test]
    fn parse_set_syntax_entity_types_and_all() {
        assert_eq!(parse_set_syntax(None).unwrap(), SetSyntax::All);
        assert_eq!(
            parse_set_syntax(Some("entityType:ProjectCluster")).unwrap(),
            SetSyntax::EntityType { clusters: true, projects: false, records: false }
        );
        assert_eq!(
            parse_set_syntax(Some("entityType:ResearchProject")).unwrap(),
            SetSyntax::EntityType { clusters: false, projects: true, records: false }
        );
        assert_eq!(
            parse_set_syntax(Some("entityType:Record")).unwrap(),
            SetSyntax::EntityType { clusters: false, projects: false, records: true }
        );
    }

    #[test]
    fn parse_set_syntax_project_and_cluster() {
        assert_eq!(
            parse_set_syntax(Some("project:0803")).unwrap(),
            SetSyntax::Project("0803".to_string())
        );
        assert_eq!(
            parse_set_syntax(Some("cluster:cluster-001")).unwrap(),
            SetSyntax::Cluster("cluster-001".to_string())
        );
    }

    #[test]
    fn parse_set_syntax_unknown_or_empty_is_bad_argument() {
        assert!(matches!(parse_set_syntax(Some("garbage")), Err(OaiError::BadArgument(_))));
        assert!(matches!(
            parse_set_syntax(Some("entityType:Unknown")),
            Err(OaiError::BadArgument(_))
        ));
        assert!(matches!(parse_set_syntax(Some("project:")), Err(OaiError::BadArgument(_))));
        assert!(matches!(parse_set_syntax(Some("cluster:")), Err(OaiError::BadArgument(_))));
    }

    #[test]
    fn bad_verb_error_omits_verb_attribute() {
        let xml = build_error_response(OaiError::BadVerb, None);
        assert!(xml.contains("<error code=\"badVerb\">"), "got: {}", xml);
        assert!(!xml.contains("verb="), "badVerb must not echo a verb attribute, got: {}", xml);
    }

    #[test]
    fn recognized_verb_error_echoes_verb_attribute() {
        let xml = build_error_response(OaiError::BadArgument("test".to_string()), Some("ListRecords"));
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(
            xml.contains("verb=\"ListRecords\""),
            "verb should be echoed in request element, got: {}",
            xml
        );
    }
}
