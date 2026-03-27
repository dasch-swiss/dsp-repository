//! Handler for the OAI-PMH ListRecords verb.

use dpe_web::domain::{ProjectRepository, RecordRepository};

use super::{build_error_response, build_list_request_params, validate_list_params, OaiParams};
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListRecords verb.
pub fn handle_list_records(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
) -> String {
    let (prefix, records) = match validate_list_params(params, repo, record_repo) {
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
    use super::*;

    use super::super::test_utils::{first_0803_record, golden, incunabula_project, normalize, InMemoryProjectRepository, InMemoryRecordRepository};

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
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("marc21"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params(Some("oai_dc"));
        params.resumption_token = Some("some-token".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_set_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Unknown".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn cluster_set_with_no_clusters_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ProjectCluster".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_with_no_records_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_returns_only_record_metadata() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo);
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

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        let expected = golden("list_records_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        let expected = golden("list_records_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_mixed_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo);
        let expected = golden("list_records_mixed_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_record_only_oai_dc_response() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &InMemoryProjectRepository::new(vec![]), &record_repo);
        let expected = golden("list_records_record_only_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_records_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty());
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_mixed_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_record_only_response_is_valid_oai_pmh() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &InMemoryProjectRepository::new(vec![]), &record_repo);
        crate::oai::handlers::test_utils::validate_against_schema(&xml);
    }
}
