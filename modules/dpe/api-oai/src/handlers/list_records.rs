//! Handler for the OAI-PMH ListRecords verb.

use dpe_core::{ClusterRaw, ContributorLookup, ProjectRepository, RecordRepository};

use super::{build_error_response, next_page_token, validate_list_params, OaiParams};
use crate::resumption::page_size;
use crate::xml::OaiXmlBuilder;

/// Handles the ListRecords verb.
pub fn handle_list_records(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
) -> String {
    handle_list_records_paged(params, repo, record_repo, clusters, lookup, page_size())
}

/// ListRecords with an explicit page size, so tests can exercise paging without
/// large fixtures. Production callers use [`handle_list_records`].
pub fn handle_list_records_paged(
    params: &OaiParams,
    repo: &dyn ProjectRepository,
    record_repo: &dyn RecordRepository,
    clusters: &[ClusterRaw],
    lookup: &dyn ContributorLookup,
    page_size: usize,
) -> String {
    let page = match validate_list_params(params, repo, record_repo, clusters, lookup, page_size) {
        Ok(result) => result,
        Err(err) => return build_error_response(err, Some("ListRecords")),
    };

    let request_params: Vec<(&str, &str)> = page.request_params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListRecords", &request_params);

    let end = (page.offset + page.page_size).min(page.records.len());
    builder.start_element("ListRecords");
    for record in &page.records[page.offset..end] {
        builder.write_record(record);
    }
    if page.records.len() > page.page_size {
        let token = next_page_token(&page, end);
        builder.write_resumption_token(token.as_deref(), page.records.len(), page.offset);
    }
    builder.end_element("ListRecords");

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
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("metadataPrefix argument is required"), "got: {}", xml);
    }

    #[test]
    fn unsupported_metadata_prefix_returns_cannot_disseminate() {
        let params = make_params(Some("marc21"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"cannotDisseminateFormat\">"), "got: {}", xml);
    }

    #[test]
    fn malformed_resumption_token_returns_bad_resumption_token() {
        let mut params = make_params(None);
        params.resumption_token = Some("not-a-valid-token!!!".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_with_other_arguments_returns_bad_argument() {
        // OAI-PMH: resumptionToken is exclusive with all other arguments except verb.
        let mut params = make_params(Some("oai_dc"));
        params.resumption_token = Some("anytoken".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    // ---- paging / resumption token ----

    #[test]
    fn single_page_emits_no_resumption_token() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![project_with_shortcode("0001"), project_with_shortcode("0002")]);
        let xml =
            handle_list_records_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup(), 2);
        assert!(!xml.contains("<resumptionToken"), "no token for a single page, got: {}", xml);
    }

    #[test]
    fn first_page_emits_token_with_list_size_and_cursor() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
        ]);
        let xml =
            handle_list_records_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup(), 2);
        assert!(xml.contains("completeListSize=\"3\""), "got: {}", xml);
        assert!(xml.contains("cursor=\"0\""), "got: {}", xml);
        assert!(extract_token(&xml).is_some(), "first page should carry a token, got: {}", xml);
        assert!(xml.contains("ark:/72163/1/0001"), "got: {}", xml);
        assert!(xml.contains("ark:/72163/1/0002"), "got: {}", xml);
        assert!(
            !xml.contains("ark:/72163/1/0003"),
            "third item belongs to page two, got: {}",
            xml
        );
    }

    #[test]
    fn paging_walks_the_whole_list_without_duplicates_or_gaps() {
        // Five projects, page size 2 -> pages of 2, 2, 1. Follow tokens to the end.
        let repo = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
            project_with_shortcode("0004"),
            project_with_shortcode("0005"),
        ]);
        let lookup = incunabula_lookup();

        // One inner vec per page, so the assertion pins each page's exact
        // contents (and thereby the 2-2-1 split, ordering, and no dup/gap).
        let mut pages: Vec<Vec<String>> = Vec::new();
        let mut params = make_params(Some("oai_dc"));
        loop {
            assert!(pages.len() < 10, "paging did not terminate");
            let xml = handle_list_records_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &lookup, 2);
            assert!(!xml.contains("<error"), "no page should error, got: {}", xml);
            let page: Vec<String> = ["0001", "0002", "0003", "0004", "0005"]
                .into_iter()
                .filter(|code| xml.contains(&format!("ark:/72163/1/{code}")))
                .map(str::to_string)
                .collect();
            pages.push(page);
            match extract_token(&xml) {
                Some(token) => {
                    // Next request carries only verb + resumptionToken.
                    params = OaiParams {
                        verb: Some("ListRecords".to_string()),
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
            vec![vec!["0001", "0002"], vec!["0003", "0004"], vec!["0005"]],
            "expected pages of 2, 2, 1 with every item once, in order"
        );
    }

    #[test]
    fn final_page_emits_empty_resumption_token() {
        let repo = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
        ]);
        let lookup = incunabula_lookup();
        let first = handle_list_records_paged(
            &make_params(Some("oai_dc")),
            &repo,
            &InMemoryRecordRepository::empty(),
            &[],
            &lookup,
            2,
        );
        let token = extract_token(&first).expect("first page has a token");

        let mut params = make_params(None);
        params.resumption_token = Some(token);
        let last = handle_list_records_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &lookup, 2);
        assert!(
            last.contains("<resumptionToken"),
            "final page carries the element, got: {}",
            last
        );
        assert!(extract_token(&last).is_none(), "final token must be empty, got: {}", last);
        assert!(last.contains("cursor=\"2\""), "final page cursor should be 2, got: {}", last);
        assert!(
            last.contains("ark:/72163/1/0003"),
            "final page holds the last item, got: {}",
            last
        );
    }

    #[test]
    fn resumed_request_echoes_only_the_token_in_request_element() {
        let repo = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
        ]);
        let lookup = incunabula_lookup();
        let first = handle_list_records_paged(
            &make_params(Some("oai_dc")),
            &repo,
            &InMemoryRecordRepository::empty(),
            &[],
            &lookup,
            2,
        );
        let token = extract_token(&first).expect("first page has a token");

        let mut params = make_params(None);
        params.resumption_token = Some(token.clone());
        let second = handle_list_records_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &lookup, 2);
        assert!(
            second.contains(&format!("resumptionToken=\"{token}\"")),
            "request element should echo the token, got: {}",
            second
        );
        assert!(
            !second.contains("metadataPrefix=\"oai_dc\""),
            "resumed request must not echo filters, got: {}",
            second
        );
    }

    #[test]
    fn stale_token_past_end_of_list_returns_bad_resumption_token() {
        // Encode an offset that is valid for a longer list, then present it against
        // a shorter one: the offset now points past the end.
        let big = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
            project_with_shortcode("0004"),
        ]);
        let lookup = incunabula_lookup();
        // Walk to a later page against the big repo to get an offset=2 token.
        let first = handle_list_records_paged(
            &make_params(Some("oai_dc")),
            &big,
            &InMemoryRecordRepository::empty(),
            &[],
            &lookup,
            2,
        );
        let token = extract_token(&first).expect("token present");

        // Present the offset-2 token against a two-item list (offset 2 == len).
        let small =
            InMemoryProjectRepository::new(vec![project_with_shortcode("0001"), project_with_shortcode("0002")]);
        let mut params = make_params(None);
        params.resumption_token = Some(token);
        let xml = handle_list_records_paged(&params, &small, &InMemoryRecordRepository::empty(), &[], &lookup, 2);
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Unknown".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn empty_project_cluster_set_returns_no_records_match() {
        // entityType:ProjectCluster is a known set that selects nothing today.
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:ProjectCluster".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_project_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("project:9999".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn unknown_cluster_set_returns_bad_argument() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-999".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
    }

    #[test]
    fn project_set_returns_only_that_projects_records() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("project:0803".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[], &incunabula_lookup());
        assert!(
            xml.contains("oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
            "record identifier should be present, got: {}",
            xml
        );
        assert!(
            !xml.contains("oai:dasch.swiss:ark:/72163/1/0803\""),
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
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_with_no_records_returns_no_records_match() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    #[test]
    fn record_set_filter_returns_only_record_metadata() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[], &incunabula_lookup());
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

    // ---- cluster set tests ----

    #[test]
    fn cluster_set_returns_project_entries_and_records() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("cluster:cluster-001".to_string());
        let clusters = vec![cluster_fixture("cluster-001", "EKWS", &["0803"])];
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &clusters, &incunabula_lookup());
        // project entry present (identifier closes with </identifier>)
        assert!(
            xml.contains("<identifier>oai:dasch.swiss:ark:/72163/1/0803</identifier>"),
            "project entry should be present, got: {}",
            xml
        );
        // record present
        assert!(
            xml.contains("oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"),
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
        let xml = handle_list_records(
            &params,
            &repo,
            &InMemoryRecordRepository::empty(),
            &clusters,
            &incunabula_lookup(),
        );
        assert!(
            xml.contains("<identifier>oai:dasch.swiss:ark:/72163/1/0803</identifier>"),
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
        let xml = handle_list_records(
            &params,
            &repo,
            &InMemoryRecordRepository::empty(),
            &clusters,
            &incunabula_lookup(),
        );
        assert!(xml.contains("<error code=\"noRecordsMatch\">"), "got: {}", xml);
    }

    // ---- golden tests ----

    #[test]
    fn golden_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        let expected = golden("list_records_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_oai_datacite_response() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        let expected = golden("list_records_oai_datacite.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_mixed_oai_dc_response() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[], &incunabula_lookup());
        let expected = golden("list_records_mixed_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    #[test]
    fn golden_record_only_oai_dc_response() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(
            &params,
            &InMemoryProjectRepository::new(vec![]),
            &record_repo,
            &[],
            &incunabula_lookup(),
        );
        let expected = golden("list_records_record_only_oai_dc.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_records_oai_dc_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_oai_datacite_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_datacite"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let xml = handle_list_records(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_mixed_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![incunabula_project()]);
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(&params, &repo, &record_repo, &[], &incunabula_lookup());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_paged_response_is_valid_oai_pmh() {
        let params = make_params(Some("oai_dc"));
        let repo = InMemoryProjectRepository::new(vec![
            project_with_shortcode("0001"),
            project_with_shortcode("0002"),
            project_with_shortcode("0003"),
        ]);
        let xml =
            handle_list_records_paged(&params, &repo, &InMemoryRecordRepository::empty(), &[], &incunabula_lookup(), 2);
        assert!(xml.contains("<resumptionToken"), "sanity: response is paged, got: {}", xml);
        crate::handlers::test_utils::validate_against_schema(&xml);
    }

    #[test]
    fn list_records_record_only_response_is_valid_oai_pmh() {
        let mut params = make_params(Some("oai_dc"));
        params.set = Some("entityType:Record".to_string());
        let record_repo = InMemoryRecordRepository::new(vec![first_0803_record()]);
        let xml = handle_list_records(
            &params,
            &InMemoryProjectRepository::new(vec![]),
            &record_repo,
            &[],
            &incunabula_lookup(),
        );
        crate::handlers::test_utils::validate_against_schema(&xml);
    }
}
