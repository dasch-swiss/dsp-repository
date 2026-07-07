//! Handler for the OAI-PMH ListSets verb.

use dpe_core::{ClusterRaw, ProjectRepository};

use super::{build_error_response, OaiParams};
use crate::error::OaiError;
use crate::xml::OaiXmlBuilder;

/// Handles the ListSets verb.
///
/// Advertises the static `entityType:*` sets plus a dynamic `project:{shortcode}`
/// set per known project and a `cluster:{id}` set per cluster.
pub fn handle_list_sets(params: &OaiParams, repo: &dyn ProjectRepository, clusters: &[ClusterRaw]) -> String {
    // ListSets accepts only resumptionToken
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
    {
        return build_error_response(
            OaiError::BadArgument("Unexpected argument for ListSets".to_string()),
            Some("ListSets"),
        );
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return build_error_response(OaiError::BadResumptionToken, Some("ListSets"));
    }

    // Dynamic project sets: (setSpec, setName) = (project:{shortcode}, project name).
    let project_sets: Vec<(String, String)> = repo
        .get_all()
        .iter()
        .map(|p| (format!("project:{}", p.shortcode), p.name.clone()))
        .collect();

    // Dynamic cluster sets: (setSpec, setName) = (cluster:{id}, cluster name).
    let cluster_sets: Vec<(String, String)> =
        clusters.iter().map(|c| (format!("cluster:{}", c.id), c.name.clone())).collect();

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListSets", &[]);
    builder.write_list_sets(&project_sets, &cluster_sets);

    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::{cluster_fixture, golden, incunabula_project, normalize, InMemoryProjectRepository};
    use super::*;

    fn make_params() -> OaiParams {
        OaiParams {
            verb: Some("ListSets".to_string()),
            identifier: None,
            metadata_prefix: None,
            from: None,
            until: None,
            set: None,
            resumption_token: None,
        }
    }

    fn repo() -> InMemoryProjectRepository {
        InMemoryProjectRepository::new(vec![incunabula_project()])
    }

    fn clusters() -> Vec<ClusterRaw> {
        vec![cluster_fixture("cluster-001", "EKWS", &["0803"])]
    }

    // ---- error cases ----

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params();
        params.from = Some("2020-01-01".to_string());
        let xml = handle_list_sets(&params, &repo(), &clusters());
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("Unexpected argument for ListSets"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params();
        params.resumption_token = Some("some-token".to_string());
        let xml = handle_list_sets(&params, &repo(), &clusters());
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    // ---- dynamic set content ----

    #[test]
    fn advertises_static_project_and_cluster_sets() {
        let params = make_params();
        let xml = handle_list_sets(&params, &repo(), &clusters());
        // static sets remain
        assert!(xml.contains("entityType:ProjectCluster"), "got: {}", xml);
        assert!(xml.contains("entityType:ResearchProject"), "got: {}", xml);
        // dynamic project set with the project name as setName
        assert!(xml.contains("<setSpec>project:0803</setSpec>"), "got: {}", xml);
        // dynamic cluster set with the cluster name as setName
        assert!(xml.contains("<setSpec>cluster:cluster-001</setSpec>"), "got: {}", xml);
        assert!(
            xml.contains("<setName>EKWS</setName>"),
            "cluster setName should be the name, got: {}",
            xml
        );
    }

    #[test]
    fn is_never_empty_with_no_projects_or_clusters() {
        let params = make_params();
        let xml = handle_list_sets(&params, &InMemoryProjectRepository::new(vec![]), &[]);
        assert!(
            xml.contains("entityType:ResearchProject"),
            "static sets always present, got: {}",
            xml
        );
    }

    // ---- golden tests ----

    #[test]
    fn golden_list_sets_response() {
        let params = make_params();
        let xml = handle_list_sets(&params, &repo(), &clusters());
        let expected = golden("list_sets.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_sets_response_is_valid_oai_pmh() {
        let params = make_params();
        let xml = handle_list_sets(&params, &repo(), &clusters());
        crate::handlers::test_utils::validate_against_schema(&xml);
    }
}
