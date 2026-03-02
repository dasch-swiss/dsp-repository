//! Handler for the OAI-PMH GetRecord verb.

use super::data::get_project_by_shortcode;
use super::{build_error_response, OaiParams, SUPPORTED_PREFIXES};
use crate::oai::error::OaiError;
use crate::oai::metadata::{parse_oai_identifier, ProjectOaiExt};
use crate::oai::xml::OaiXmlBuilder;
use app::domain::Project;

/// Handles the GetRecord verb.
pub fn handle_get_record(params: &OaiParams) -> String {
    let identifier = match require_identifier(params) {
        Ok(id) => id,
        Err(e) => return build_error_response(e),
    };

    let prefix = match require_metadata_prefix(params) {
        Ok(p) => p,
        Err(e) => return build_error_response(e),
    };

    if let Err(e) = reject_unexpected_args(params) {
        return build_error_response(e);
    }

    let project = match resolve_project(identifier) {
        Ok(p) => p,
        Err(e) => return build_error_response(e),
    };

    build_response(identifier, prefix, &project)
}

fn require_identifier(params: &OaiParams) -> Result<&str, OaiError> {
    params
        .identifier
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("identifier argument is required".to_string()))
}

fn require_metadata_prefix(params: &OaiParams) -> Result<&str, OaiError> {
    let prefix = params
        .metadata_prefix
        .as_deref()
        .ok_or_else(|| OaiError::BadArgument("metadataPrefix argument is required".to_string()))?;

    if !SUPPORTED_PREFIXES.contains(&prefix) {
        return Err(OaiError::CannotDisseminateFormat);
    }

    Ok(prefix)
}

fn reject_unexpected_args(params: &OaiParams) -> Result<(), OaiError> {
    if params.from.is_some()
        || params.until.is_some()
        || params.set.is_some()
        || params.resumption_token.is_some()
    {
        return Err(OaiError::BadArgument(
            "Unexpected argument for GetRecord".to_string(),
        ));
    }
    Ok(())
}

fn resolve_project(identifier: &str) -> Result<Project, OaiError> {
    let shortcode = parse_oai_identifier(identifier).ok_or(OaiError::IdDoesNotExist)?;
    get_project_by_shortcode(&shortcode).ok_or(OaiError::IdDoesNotExist)
}

fn build_response(identifier: &str, prefix: &str, project: &Project) -> String {
    let record = project.to_oai_record(prefix);

    let mut builder = OaiXmlBuilder::new();
    builder.write_request(
        "GetRecord",
        &[("identifier", identifier), ("metadataPrefix", prefix)],
    );
    builder.start_element("GetRecord");
    builder.write_record(&record);
    builder.end_element("GetRecord");
    builder.finish()
}
