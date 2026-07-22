//! Handler for the OAI-PMH ListIdentifiers verb.

use dpe_core::{ClusterRaw, ContributorLookup, ProjectRepository, RecordRepository};

use super::{build_error_response, next_page_token, validate_list_params, OaiParams};
use crate::resumption::page_size;
use crate::xml::OaiXmlBuilder;

/// Handles the ListIdentifiers verb.
pub fn handle_list_identifiers(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
) -> String {
    handle_list_identifiers_paged(params, repo, record_repo, clusters, lookup, page_size())
}

/// ListIdentifiers with an explicit page size, so tests can exercise paging
/// without large fixtures. Production callers use [`handle_list_identifiers`].
pub fn handle_list_identifiers_paged(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
    page_size: usize,
) -> String {
    let page = match validate_list_params(params, repo, record_repo, clusters, lookup, page_size) {
        Ok(result) => result,
        Err(err) => return build_error_response(err, Some("ListIdentifiers")),
    };

    let request_params: Vec<(&str, &str)> = page.request_params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListIdentifiers", &request_params);

    let end = (page.offset + page.page_size).min(page.records.len());
    builder.start_element("ListIdentifiers");
    for record in &page.records[page.offset..end] {
        builder.write_record_header(&record.header.identifier, &record.header.datestamp, &record.header.set_specs);
    }
    if page.records.len() > page.page_size {
        let token = next_page_token(&page, end);
        builder.write_resumption_token(token.as_deref(), page.records.len(), page.offset);
    }
    builder.end_element("ListIdentifiers");

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{
        cluster_fixture, first_0803_record, golden, incunabula_lookup, incunabula_project, normalize,
        project_with_shortcode, InMemoryProjectRepository, InMemoryRecordRepository,
    };
    use super::*;

    /// Extracts the resumption token text from a list response, if present and
    /// non-empty. Returns `None` for a final (empty) token or when absent.
    fn extract_token(xml: &str) -> Option<String> {
        let start = xml.find("<resumptionToken")?;
        let rest = &xml[start..];
        let gt = rest.find('>')?;
        if rest[..gt].ends_with('/') {
            return None;
        }
        let after = &rest[gt + 1..];
        let end = after.find("</resumptionToken>")?;
        let token = after[..end].trim();
        (!token.is_empty()).then(|| token.to_string())
    }

    fn make_params(metadata_prefix: Option<&str>) -> OaiParams {
        OaiParams {
            verb: Some("ListIdentifiers".to_string()),
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
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("marc21"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn malformed_resumption_token_returns_bad_resumption_token() {
        let mut params = make_params(None);
        params.resumption_token = Some("not-a-valid-token!!!".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_with_other_arguments_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.resumption_token = Some("anytoken".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    // ---- paging / resumption token ----

    #[test]
    fn single_page_emits_no_resumption_token() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![project_with_shortcode("0001"), project_with_shortcode("0002")]);
        let xml = handle_list_identifiers_paged(
            &params,
            &repo,
            &InMemoryRecordRepository::empty(),
            &[],
            &incunabula_lookup(),
            2,
        );
        assert!(!xml.contains("<resumptionToken"), "no token for a single page, got: {}", xml);
    }

    #[test]
    fn paging_walks_the_whole_list_of_identifiers() {
        let repo = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
        ]);
        let lookup = incunabula_lookup();
        // One inner vec per page, so the assertion pins each page's exact
        // contents (and thereby the 2-1 split, ordering, and no dup/gap).
        let mut pages: Vec<Vec<String>> = Vec::new();
        let mut params = make_params(Some("oai_dc"));
        loop {
            assert!(pages.len() < 10, "paging did not terminate");
            let xml =
                handle_list_identifiers_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &lookup, 2);
            assert!(!xml.contains("<error"), "no page should error, got: {}", xml);
            let page: Vec<String> = ["0001", "0002", "0003"]
                .into_iter()
                .filter(|code| xml.contains(&format!("ark:/72163/1/{code}")))
                .map(str::to_string)
                .collect();
            pages.push(page);
            match extract_token(&xml) {
                Some(token) => {
                    params = OaiParams {
                        verb: Some("ListIdentifiers".to_string()),
                        identifier: None,
                        metadata_prefix: None,
                        from: None,
                        until: None,
                        set: None,
                        resumption_token: Some(token),
                    };
                }
                None => break,
            }
        }
        assert_eq!(
            pages,
            vec![vec!["0001", "0002"], vec!["0003"]],
            "expected pages of 2, 1 with every identifier once, in order"
        );
    }

    #[test]
    fn unknown_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Unknown".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn empty_project_cluster_set_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ProjectCluster".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn project_set_returns_only_record_identifiers() {
        // ListIdentifiers honours project: sets identically to ListRecords (REQ-1.2).
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("project:0803".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(&params, &repo, &record_repo, &[], &incunabula_lookup());
        assert!(
            xml.contains("oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "record identifier should be present, got: {}",
            xml
        );
        assert!(
            !xml.contains("<identifier>oai:dasch.swiss:ark:/72163/1/0803</identifier>"),
            "project entry should be absent (records only), got: {}",
            xml
        );
    }

    #[test]
    fn cluster_set_returns_project_and_record_identifiers() {
        // ListIdentifiers honours cluster: sets identically to ListRecords (REQ-2.3).
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-001".to_string());
        let clusters = vec![cluster_fixture("cluster-001", "EKWS", &["0803"])];
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(&params, &repo, &record_repo, &clusters, &incunabula_lookup());
        assert!(
            xml.contains("<identifier>oai:dasch.swiss:ark:/72163/1/0803</identifier>"),
            "project entry should be present, got: {}",
            xml
        );
        assert!(
            xml.contains("oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "member record should be present, got: {}",
            xml
        );
    }

    #[test]
    fn record_set_filter_with_no_records_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_returns_only_record_identifier() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(&params, &repo, &record_repo, &[], &incunabula_lookup());
        assert!(
            xml.contains("oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "record identifier should be present, got: {}",
            xml
        );
        assert!(
            !xml.contains("oai:dasch.swiss:ark:/72163/1/0803\""),
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
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        let expected = golden("list_identifiers_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        let expected = golden("list_identifiers_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_mixed_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(&params, &repo, &record_repo, &[], &incunabula_lookup());
        let expected = golden("list_identifiers_mixed_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_record_only_oai_dc_response() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(
            &params,
            &InMemoryProjectRepository::new(vec![]),
            &record_repo,
            &[],
            &incunabula_lookup(),
        );
        let expected = golden("list_identifiers_record_only_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_identifiers_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_identifiers_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml =
            handle_list_identifiers(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_identifiers_mixed_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(&params, &repo, &record_repo, &[], &incunabula_lookup());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_identifiers_record_only_response_is_valid_oai_pmh() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_identifiers(
            &params,
            &InMemoryProjectRepository::new(vec![]),
            &record_repo,
            &[],
            &incunabula_lookup(),
        );
        crate::handlers::test_utils::validate_against_schema(&xml);
    }
}
