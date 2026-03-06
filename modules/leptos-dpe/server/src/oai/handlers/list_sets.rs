//! Handler for the OAI-PMH ListSets verb.

use super::{build_error_response, OaiParams};
use crate::oai::error::OaiError;
use crate::oai::xml::OaiXmlBuilder;

/// Handles the ListSets verb.
pub fn handle_list_sets(params: &OaiParams) -> String {
    // ListSets accepts only resumptionToken
    if params.identifier.is_some()
        || params.metadata_prefix.is_some()
        || params.from.is_some()
        || params.until.is_some()
    {
        return build_error_response(OaiError::BadArgument(
            "Unexpected argument for ListSets".to_string(),
        ));
    }

    // We don't support resumption tokens in v1
    if params.resumption_token.is_some() {
        return build_error_response(OaiError::BadResumptionToken);
    }

    let mut builder = OaiXmlBuilder::new();
    builder.write_request("ListSets", &[]);
    builder.write_list_sets();

    builder.finish()
}

#[cfg(test)]
mod tests {
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

    fn normalize(xml: &str) -> String {
        xml.lines()
            .filter(|l| !l.trim_start().starts_with("<responseDate>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn golden(name: &str, actual: &str) -> String {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/oai/handlers/testdata/golden");
        std::fs::create_dir_all(&dir).expect("create golden dir");
        let path = dir.join(name);
        let normalized = normalize(actual);
        if path.exists() {
            std::fs::read_to_string(&path).expect("read golden file")
        } else {
            std::fs::write(&path, &normalized).expect("write golden file");
            normalized
        }
    }

    // ---- error cases ----

    #[test]
    fn unexpected_argument_returns_bad_argument() {
        let mut params = make_params();
        params.from = Some("2020-01-01".to_string());
        let xml = handle_list_sets(&params);
        assert!(xml.contains("<error code=\"badArgument\">"), "got: {}", xml);
        assert!(xml.contains("Unexpected argument for ListSets"), "got: {}", xml);
    }

    #[test]
    fn resumption_token_returns_bad_resumption_token() {
        let mut params = make_params();
        params.resumption_token = Some("some-token".to_string());
        let xml = handle_list_sets(&params);
        assert!(xml.contains("<error code=\"badResumptionToken\">"), "got: {}", xml);
    }

    fn validate_against_schema(xml: &str) {
        let xsd_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/oai/handlers/testdata/schemas/validate.xsd");

        let mut tmp = tempfile::NamedTempFile::new().expect("create temp file");
        std::io::Write::write_all(&mut tmp, xml.as_bytes()).expect("write temp file");

        let output = std::process::Command::new("xmllint")
            .arg("--noout")
            .arg("--schema")
            .arg(xsd_path)
            .arg(tmp.path())
            .output()
            .expect("xmllint must be available");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Schema validation failed:\n{}", stderr);
        }
    }

    // ---- golden tests ----

    #[test]
    fn golden_list_sets_response() {
        let params = make_params();
        let xml = handle_list_sets(&params);
        let expected = golden("list_sets.xml", &xml);
        assert_eq!(normalize(&xml), expected);
    }

    // ---- schema validation tests ----

    #[test]
    fn list_sets_response_is_valid_oai_pmh() {
        let params = make_params();
        let xml = handle_list_sets(&params);
        validate_against_schema(&xml);
    }
}
