//! `EditorState`, and the response glue around `deposit-shell`'s pages.

/// Shared state for the editor's handlers. Cheap to clone: everything heavy is behind an `Arc`.
#[derive(Clone)]
pub(crate) struct EditorState {
    /// Unhashed in dev, content-hashed in release.
    pub(crate) css_href: String,
    /// The editor's store, behind its ports, so a test can make a chosen storage call fail.
    pub(crate) editor: std::sync::Arc<dyn editor_core::repository::EditorRepositories>,
    /// Who a request's session belongs to, for the editor's extractors (`handlers::auth`). The
    /// editor reads no identity table: this port and [`Self::directory`] are what it asks.
    pub(crate) authenticator: std::sync::Arc<dyn editor_ports::Authenticator>,
    /// The names behind the account ids the editor's records hold.
    pub(crate) directory: std::sync::Arc<dyn editor_ports::UserDirectory>,
    /// The published project set, read once at startup from `EDITOR_DATA_DIR`. An immutable
    /// snapshot, so not behind a port.
    pub(crate) published: std::sync::Arc<editor_core::published::PublishedProjects>,
    /// The `temporalCoverage` resolution tables, read once at startup from `EDITOR_DATA_DIR`:
    /// the same tables `access-server validate` and `dpe-api-oai` decide with. Empty without a
    /// data directory, which refuses every free-text period at submit (the fail-safe way).
    pub(crate) temporal: std::sync::Arc<TemporalTables>,
    /// The persons and organizations a project may refer to by id, read once at startup from
    /// `EDITOR_DATA_DIR`. Empty makes every agent reference unresolvable.
    pub(crate) agents: std::sync::Arc<editor_core::agents::Agents>,
}

/// The two tables `unresolved_temporal_coverage` reads, kept together because two adjacent
/// map arguments are silently swappable.
#[derive(Default)]
pub(crate) struct TemporalTables {
    pub(crate) periods: std::collections::HashMap<String, shared_metadata::w3cdtf::W3cdtfRange>,
    pub(crate) enrichment: std::collections::HashMap<String, shared_metadata::temporal_enrichment::EnrichedDate>,
}

/// Render a page inside the document shell: the one place a `Markup` becomes a `Response`.
///
/// `viewer` is the signed-in account's display name, never its address: the header is on every
/// page and in every screenshot of one.
pub(crate) fn render(
    state: &EditorState,
    title: &str,
    status: axum::http::StatusCode,
    viewer: Option<&str>,
    content: maud::Markup,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let traceparent = crate::traceparent::extract_traceparent();
    let viewer = viewer.map(|name| deposit_shell::view::Viewer { name });
    let body = deposit_shell::view::page(title, traceparent.as_deref(), &state.css_href, viewer, content);
    (status, axum::response::Html(body.into_string())).into_response()
}

/// A 403 rendered as a page, so it is not a dead end. Only authenticated requests reach it,
/// so the header renders signed in.
pub(crate) fn forbidden(state: &EditorState, viewer: &str, message: &str) -> axum::response::Response {
    render(
        state,
        "No access — DaSCH Metadata Editor",
        axum::http::StatusCode::FORBIDDEN,
        Some(viewer),
        deposit_shell::pages::forbidden::forbidden(message),
    )
}

/// `GET /`: a redirect to `/projects`, which alone decides what a signed-out visitor gets.
/// The shell's logo links here from every page.
pub(crate) async fn root() -> axum::response::Redirect {
    axum::response::Redirect::to("/projects")
}

/// An instant as a page shows it: UTC, and labelled so. Shared so the surfaces that render
/// one cannot disagree on the format.
pub(crate) fn format_instant(at: chrono::DateTime<chrono::Utc>) -> String {
    at.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// The published spelling of a shortcode (`080C`), falling back to the stored, folded key
/// (`080c`). Shared by `review` and `approved` so the fold has one copy.
pub(crate) fn shortcode_as_published<'a>(state: &'a EditorState, stored: &'a str) -> String {
    state
        .published
        .get(stored)
        .map_or_else(|| stored.to_string(), |project| project.shortcode.clone())
}

pub(crate) fn project_name<'a>(state: &'a EditorState, shortcode: &str) -> Option<&'a str> {
    state.published.get(shortcode).map(|project| project.name.as_str())
}

/// A 404 for a project or account id that names nothing. Always renders signed out, as the
/// root's fallback does.
pub(crate) async fn not_found(
    axum::extract::State(state): axum::extract::State<EditorState>,
) -> axum::response::Response {
    render(
        &state,
        "Page not found — DaSCH Metadata Editor",
        axum::http::StatusCode::NOT_FOUND,
        None,
        deposit_shell::pages::not_found::not_found(),
    )
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;

    use super::*;
    use crate::fixtures;

    #[tokio::test]
    async fn forbidden_renders_the_page_shell_with_a_403_and_a_way_out() {
        // The reader is signed in, so the header renders their name and a way out.
        let (state, _) = fixtures::test_state("forbidden").await;
        let response = forbidden(&state, "A Depositor", "This project is not assigned to your account.");
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        let body = fixtures::body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("This project is not assigned to your account."), "{body}");
        assert!(body.contains(r#"<a href="/projects""#), "{body}");
        assert!(body.contains("A Depositor"), "{body}");
    }

    #[tokio::test]
    async fn the_root_redirects_to_the_project_list() {
        let response = root().await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(fixtures::location(&response).as_deref(), Some("/projects"));
    }
}
