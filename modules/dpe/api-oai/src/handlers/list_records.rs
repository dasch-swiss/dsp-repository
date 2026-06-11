//! Handler for the OAI-PMH ListRecords verb.

use dpe_core::{ClusterRaw, ProjectRepository, RecordRepository};

use super::{build_error_response, build_list_request_params, validate_list_params, OaiParams};
use crate::xml::OaiXmlBuilder;

/// Handles the ListRecords verb.
pub fn handle_list_records(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    clusters: &[ClusterRaw],
) -> String {
    let (prefix, records) = match validate_list_params(params, repo, record_repo, clusters) {
        Ok(result) => result,
        Err(err) => return build_error_response(err, Some("ListRecords")),
    };

    let request_params = build_list_request_params(prefix, params);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListRecords", &request_params);

    builder.start_element("ListRecords");
    for record in &records {
        builder.write_record(record);
    }
    builder.end_element("ListRecords");

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{
        cluster_fixture, first_0803_record, golden, incunabula_project, normalize, InMemoryProjectRepository,
        InMemoryRecordRepository,
    };
    use super::*;

    fn make_params(metadata_prefix: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("ListRecords".to_string()),
            identifier: None,
            metadata_prefix: metadata_prefix.map(str::to_string),
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    // ---- error cases ----

    #[test]
    fn missing_metadata_prefix_returns_bad_argument() {
        let params = make_params(None);
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("marc21"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params(Some("oai_dc"));
        params.resumption_token = Some("some-token".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Unknown".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn empty_project_cluster_set_returns_no_records_match() {
        // entityType:ProjectCluster is a known set that selects nothing today.
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ProjectCluster".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_project_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("project:9999".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_cluster_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-999".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn project_set_returns_only_that_projects_records() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("project:0803".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[]);
        assert!(
            xml.contains("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "record identifier should be present, got: {}",
            xml
        );
        assert!(
            !xml.contains("oai:meta.dasch.swiss:ark:/72163/1/0803\""),
            "project entry should be absent (records only), got: {}",
            xml
        );
        assert!(
            xml.contains("project:0803"),
            "record header should carry project setSpec, got: {}",
            xml
        );
    }

    #[test]
    fn project_set_real_project_no_records_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("project:0803".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_with_no_records_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_returns_only_record_metadata() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[]);
        assert!(
            xml.contains("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "record identifier should be present, got: {}",
            xml
        );
        assert!(
            !xml.contains("oai:meta.dasch.swiss:ark:/72163/1/0803\""),
            "project identifier should be absent, got: {}",
            xml
        );
        assert!(xml.contains("entityType:Record"), "set spec should be Record, got: {}", xml);
    }

    // ---- cluster set tests ----

    #[test]
    fn cluster_set_returns_project_entries_and_records() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-001".to_string());
        let clusters = vec![cluster_fixture("cluster-001", "EKWS", &["0803"])];
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &clusters);
        // project entry present (identifier closes with </identifier>)
        assert!(
            xml.contains("<identifier>oai:meta.dasch.swiss:ark:/72163/1/0803</identifier>"),
            "project entry should be present, got: {}",
            xml
        );
        // record present
        assert!(
            xml.contains("oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "member record should be present, got: {}",
            xml
        );
        assert!(
            xml.contains("cluster:cluster-001"),
            "headers should carry cluster setSpec, got: {}",
            xml
        );
    }

    #[test]
    fn cluster_set_recordless_projects_still_returns_project_entries() {
        // A known cluster whose member project exists but has no records is a
        // success: the project entry is returned (asymmetry with project: sets).
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-001".to_string());
        let clusters = vec![cluster_fixture("cluster-001", "EKWS", &["0803"])];
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &clusters);
        assert!(
            xml.contains("<identifier>oai:meta.dasch.swiss:ark:/72163/1/0803</identifier>"),
            "project entry should be present even with no records, got: {}",
            xml
        );
        assert!(!xml.contains("<error"), "should be a success, got: {}", xml);
    }

    #[test]
    fn cluster_set_no_matching_projects_returns_no_records_match() {
        // Cluster exists but its listed shortcodes match no real project.
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-001".to_string());
        let clusters = vec![cluster_fixture("cluster-001", "EKWS", &["9999"])];
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &clusters);
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        let expected = golden("list_records_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        let expected = golden("list_records_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_mixed_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[]);
        let expected = golden("list_records_mixed_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_record_only_oai_dc_response() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &InMemoryProjectRepository::new(vec![]), &record_repo, &[]);
        let expected = golden("list_records_record_only_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_records_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[]);
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_mixed_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[]);
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_record_only_response_is_valid_oai_pmh() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &InMemoryProjectRepository::new(vec![]), &record_repo, &[]);
        crate::handlers::test_utils::validate_against_schema(&xml);
    }
}
