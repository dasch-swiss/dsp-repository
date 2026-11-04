//! The editor's handlers. Only these modules use axum; the views in `pages/`, `form/` and
//! `entity/` do not.

use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::{registry, Shape};

use crate::form::widgets::{agent_results, appended_field};
use crate::handlers::auth::Account;
use crate::shell::EditorState;

pub(crate) mod approved;
pub(crate) mod auth;
pub(crate) mod entities;
pub(crate) mod projects;
pub(crate) mod review;
pub(crate) mod sections;

/// The header the vendored Datastar bundle sets on every fetch it makes.
///
/// Presence is the whole test: the bundle sends the literal `true`, and a
/// request that carries the name at all came from it. A hand-built request can
/// set it too, which changes nothing that matters — both renderings are the same
/// data, and neither is a permission.
pub(crate) const DATASTAR_REQUEST: &str = "datastar-request";

/// Whether this request came from the Datastar bundle.
pub(crate) fn is_enhanced(headers: &HeaderMap) -> bool {
    headers.contains_key(DATASTAR_REQUEST)
}

/// What a surface calls itself when storage does not answer: the full log
/// message, and the noun the problem page uses ("page" or "form").
pub(crate) struct StorageSurface {
    pub(crate) log: &'static str,
    pub(crate) noun: &'static str,
}

/// Storage would not answer, so the surface cannot show what it should.
pub(crate) fn storage_error(
    state: &EditorState,
    user: &Account,
    surface: StorageSurface,
    what: &str,
    error: &impl std::fmt::Display,
) -> Response {
    tracing::error!(error = %error, operation = what, "{}", surface.log);
    crate::shell::render(
        state,
        "Page unavailable — DaSCH Metadata Editor",
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(&user.actor.display_name),
        deposit_shell::pages::problem::unavailable(&format!(
            "The editor could not reach its database, so this {} is not showing what it should. Try again; if it \
             keeps happening, the service needs attention.",
            surface.noun
        )),
    )
}

/// The query a type-ahead search sends: the picker it is for and what was typed. Both `Option`s
/// and no other key is read (Datastar adds its own), so a malformed request is a 404 or an empty
/// result rather than a 400, which would drop the response and fire the page's telemetry error
/// listener.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct AgentSearchParams {
    pub(crate) r#for: Option<String>,
    pub(crate) q: Option<String>,
}

const MAX_PICKER_NAME: usize = 200;

/// The field id a search's `for` starts with, when it is a picker name a form can have: at most
/// [`MAX_PICKER_NAME`] characters of `[A-Za-z0-9._-]`. The name is echoed into markup and a
/// JavaScript string, so nothing else is let through; the caller still checks the root is a field
/// of its own form.
pub(crate) fn picker_root(name: &str) -> Option<&str> {
    if name.len() > MAX_PICKER_NAME || !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return None;
    }
    name.split('.').next()
}

/// The agent ids `field`'s stored value holds: the contributors of the attribution rows, the ids
/// of the agent rows, and none for a shape whose agents an append search does not add.
fn stored_agent_ids(field: &registry::Field, draft: &ProjectDraft) -> Vec<String> {
    let rows = draft
        .get(field.id)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    match field.shape {
        Some(Shape::AttributionRows) => rows
            .iter()
            .filter_map(|row| row.get("contributor").and_then(serde_json::Value::as_str))
            .map(str::to_string)
            .collect(),
        Some(Shape::AgentRows) => rows.iter().filter_map(serde_json::Value::as_str).map(str::to_string).collect(),
        _ => Vec::new(),
    }
}

/// The answer to a type-ahead search for the picker `unique` of `field`: the results fragment,
/// 200 for every `query`, nothing written.
///
/// An append search (`unique` as `append_unique` makes it) excludes the agents `field` holds and
/// offers to append them; every other picker excludes nothing and offers a choice. Appending is
/// valid only for attribution and agent rows, so a forged `.new` on any other field is answered
/// with an empty container, never with an option that appends. `propose` adds the propose buttons
/// to an empty result, where the form dispatches them.
pub(crate) fn agent_search_response(
    unique: &str,
    field: &registry::Field,
    draft: &ProjectDraft,
    agents: &AgentScope<'_>,
    query: Option<&str>,
    propose: bool,
) -> Response {
    let appending = appended_field(unique) == Some(field.id);
    let appendable = matches!(field.shape, Some(Shape::AttributionRows | Shape::AgentRows));
    let forged = appended_field(unique).is_some() && !(appending && appendable);
    let held = if appending && appendable {
        stored_agent_ids(field, draft)
    } else {
        Vec::new()
    };
    let exclude: Vec<&str> = held.iter().map(String::as_str).collect();
    let query = if forged { "" } else { query.unwrap_or_default() };
    let results = agent_results(unique, query, agents, &exclude, propose);
    (StatusCode::OK, axum::response::Html(results.into_string())).into_response()
}
