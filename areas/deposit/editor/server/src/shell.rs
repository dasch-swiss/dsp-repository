//! `AppState`, the page-rendering helpers, and the shell's own routes (`/`, 404).

/// Shared state for the handlers. Cheap to clone: everything heavy is behind an `Arc`.
#[derive(Clone)]
pub(crate) struct AppState {
    /// Unhashed in dev, content-hashed in release.
    pub(crate) css_href: String,
    /// Behind the ports rather than [`crate::db::Database`], so a test can make a chosen
    /// storage call fail; the auth flow's error branches depend on it.
    pub(crate) db: std::sync::Arc<dyn editor_core::repository::Repositories>,
    /// Behind a trait so a test can watch what was sent and make sending fail.
    pub(crate) mailer: std::sync::Arc<dyn crate::mail::Mailer>,
    pub(crate) auth: crate::auth::AuthConfig,
    /// Whether the code-entry screen may show the login code. Resolved once at startup from
    /// [`crate::config::EditorConfig::reveals_login_code`], never re-derived.
    pub(crate) reveal_login_code: bool,
    /// The bearer token `POST /api/v1/collection-report` checks against. `None` refuses every
    /// call, so an unconfigured service never accepts an empty token.
    pub(crate) collection_token: Option<crate::config::Secret>,
    /// The published project set, read once at startup from `EDITOR_DATA_DIR`. An immutable
    /// snapshot, so not behind a port.
    pub(crate) published: std::sync::Arc<editor_core::published::PublishedProjects>,
    /// The `temporalCoverage` resolution tables, read once at startup from `EDITOR_DATA_DIR`:
    /// the same tables `dpe-server validate` and `dpe-api-oai` decide with. Empty without a
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
/// `viewer` is a `&User` so the header's choice is made here, once: the name, never the
/// address. The header is on every page and in every screenshot of one.
pub(crate) fn render(
    state: &AppState,
    title: &str,
    status: axum::http::StatusCode,
    viewer: Option<&editor_core::records::User>,
    content: maud::Markup,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let traceparent = crate::traceparent::extract_traceparent();
    let viewer = viewer.map(|user| editor_web::view::Viewer { name: &user.name });
    let body = editor_web::view::page(title, traceparent.as_deref(), &state.css_href, viewer, content);
    (status, axum::response::Html(body.into_string())).into_response()
}

/// A 403 rendered as a page, so it is not a dead end. Only authenticated requests reach it,
/// so the header renders signed in.
pub(crate) fn forbidden(
    state: &AppState,
    user: &editor_core::records::User,
    message: &str,
) -> axum::response::Response {
    render(
        state,
        "No access — DaSCH Metadata Editor",
        axum::http::StatusCode::FORBIDDEN,
        Some(user),
        editor_web::pages::forbidden::forbidden(message),
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
/// (`080c`). Shared by `review` and `collection` so the fold has one copy.
pub(crate) fn shortcode_as_published<'a>(state: &'a AppState, stored: &'a str) -> String {
    state
        .published
        .get(stored)
        .map_or_else(|| stored.to_string(), |project| project.shortcode.clone())
}

pub(crate) fn project_name<'a>(state: &'a AppState, shortcode: &str) -> Option<&'a str> {
    state.published.get(shortcode).map(|project| project.name.as_str())
}

/// 404 fallback for `ServeDir` misses and for project or account ids that name nothing.
/// Always renders signed out: `ServeDir`'s not-found service has no session in hand.
pub(crate) async fn not_found(axum::extract::State(state): axum::extract::State<AppState>) -> axum::response::Response {
    let content = maud::html! {
        h1 class="font-display text-2xl mb-2" { "Page not found" }
        p { "The page you asked for does not exist." }
    };
    render(
        &state,
        "Page not found — DaSCH Metadata Editor",
        axum::http::StatusCode::NOT_FOUND,
        None,
        content,
    )
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;

    use super::*;
    use crate::test_support;

    #[tokio::test]
    async fn not_found_renders_the_page_shell_with_a_404() {
        let (state, _) = test_support::test_state("not-found").await;
        let response = not_found(axum::extract::State(state)).await;
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
        // It has to arrive inside the shell, with the header's route back.
        let body = test_support::body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("DaSCH Metadata Editor"), "{body}");
        assert!(body.contains("Page not found"), "{body}");
    }

    #[tokio::test]
    async fn forbidden_renders_the_page_shell_with_a_403_and_a_way_out() {
        // The reader is signed in, so the header renders their name and a way out.
        let (state, _) = test_support::test_state("forbidden").await;
        let user = editor_core::records::User {
            id: uuid::Uuid::new_v4(),
            email: "a.depositor@example.test".to_string(),
            name: "A Depositor".to_string(),
            role: editor_core::records::Role::Depositor,
            shortcodes: vec!["0801".to_string()],
            failed_logins: 0,
            failed_login_at: None,
            last_code_at: None,
            created_at: chrono::Utc::now(),
        };

        let response = forbidden(&state, &user, "This project is not assigned to your account.");
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
        let body = test_support::body_string(response).await;
        assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
        assert!(body.contains("This project is not assigned to your account."), "{body}");
        assert!(body.contains(r#"<a href="/projects""#), "{body}");
        assert!(body.contains("A Depositor"), "{body}");
        // The header shows the name, never the address — it is on every page and
        // in every screenshot of one.
        assert!(!body.contains("a.depositor@example.test"), "{body}");
    }

    #[tokio::test]
    async fn the_root_redirects_to_the_project_list() {
        let response = root().await.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::SEE_OTHER);
        assert_eq!(test_support::location(&response).as_deref(), Some("/projects"));
    }
}
