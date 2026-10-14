//! Who may reach a route, and what happens to a request that may not.
//!
//! Authentication is an **extractor**, not a middleware. A handler that names
//! [`Authenticated`] cannot run without a live session, because the argument is
//! what runs the check, and a handler that does not name it is visibly public in
//! its signature. The argument against a middleware is in
//! `docs/src/editor/authentication.md`.
//!
//! Deliberately public: `/` (a redirect), the two login screens, `/logout`,
//! `/healthz`, the telemetry beacon and the static assets. Everything else takes
//! an `Authenticated`: this module's for identity's own pages (the depositor
//! list), `editor_auth`'s in `editor-server`, over the `Authenticator` port, for the editor's.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Method;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::Utc;
use deposit_shell::sign_in::{login_url, safe_next, NEXT};
use identity_core::records::User;
use identity_core::session;

use super::cookie;
use crate::shell::IdentityState;

/// A live session, or the request never reaches the handler.
///
/// The rejection is a redirect rather than a 401: every route behind this is a
/// page, and a browser handed a 401 with no `WWW-Authenticate` shows nothing
/// useful. Signing in and landing back on the page that was asked for is the
/// behaviour, which is what `next` carries.
#[derive(Debug, Clone)]
pub(crate) struct Authenticated(pub(crate) User);

impl FromRequestParts<IdentityState> for Authenticated {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &IdentityState) -> Result<Self, Self::Rejection> {
        match session::current(&*state.store, &state.auth, &cookie::values(&parts.headers), Utc::now()).await {
            Some(viewer) => Ok(Self(viewer.user)),
            None => Err(Redirect::to(&login_url(destination(parts))).into_response()),
        }
    }
}

/// A live session belonging to an RDU member, or the request never reaches the
/// handler.
///
/// Composed from [`Authenticated`] rather than repeating the session lookup, so
/// there is one place that decides what "signed in" means and one that decides
/// what "RDU" means.
///
/// The two refusals differ: no session redirects to login, because signing in
/// fixes it; a session that is not RDU's is a 403 page, because signing in again
/// will not, and a login screen they are already past reads as a bug.
#[derive(Debug, Clone)]
pub(crate) struct Rdu(pub(crate) User);

impl FromRequestParts<IdentityState> for Rdu {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &IdentityState) -> Result<Self, Self::Rejection> {
        let Authenticated(user) = Authenticated::from_request_parts(parts, state).await?;
        if !user.is_rdu() {
            // Worth a line: a depositor reaching an administration URL is
            // either a stale bookmark or someone trying doors. No `http.route`
            // field here on purpose — the enclosing server span already carries
            // it from `MatchedPath`, and semconv defines it as the matched
            // template rather than the concrete path.
            tracing::info!("refused an RDU-only page to an account that is not RDU");
            return Err(crate::shell::forbidden(state, &user.name, deposit_shell::RDU_ONLY));
        }
        Ok(Self(user))
    }
}

/// Where to send this request back to once its owner has signed in.
///
/// `GET` only: a `POST`'s destination is a side effect rather than a page, and
/// sending the browser to a `GET` of the same path after sign-in would look like
/// the write happened.
pub fn destination(parts: &Parts) -> Option<&str> {
    if parts.method != Method::GET {
        return None;
    }
    safe_next(parts.uri.path())
}

/// The validated destination carried by a login URL's query, if any.
///
/// Parsed by hand rather than through `Query`, so a malformed or unexpected
/// query string cannot turn a sign-in page into a 400 — a login screen that
/// refuses to render is a worse failure than one that forgets where the reader
/// was going.
///
/// Nothing is percent-decoded, and that is deliberate rather than missing.
/// [`safe_next`] refuses `%`, so an encoded value is dropped and the reader
/// lands on the root — fail-closed. Decoding first would mean `%2f%2fevil.example`
/// arrived at the check already looking like a path.
pub(crate) fn next_from(uri: &axum::http::Uri) -> Option<&str> {
    uri.query()?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == NEXT)
        .and_then(|(_, value)| safe_next(value))
}

/// Where a signed-in user should land: the destination they asked for, or the
/// service root.
pub(crate) fn destination_or_root(next: Option<&str>) -> &str {
    next.and_then(safe_next).unwrap_or("/")
}

#[cfg(test)]
mod tests {
    use axum::http::Request;

    use super::*;

    fn parts(method: &str, uri: &str) -> Parts {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(())
            .expect("the request should build")
            .into_parts()
            .0
    }

    fn uri(value: &str) -> axum::http::Uri {
        value.parse().expect("the uri should parse")
    }

    #[test]
    fn test_a_destination_is_read_back_out_of_a_login_url() {
        assert_eq!(next_from(&uri("/login?next=/projects/0801")), Some("/projects/0801"));
        assert_eq!(next_from(&uri("/login?a=1&next=/projects&b=2")), Some("/projects"));
    }

    #[test]
    fn test_no_query_and_no_next_key_are_both_no_destination() {
        assert_eq!(next_from(&uri("/login")), None);
        assert_eq!(next_from(&uri("/login?other=1")), None);
        assert_eq!(next_from(&uri("/login?next=")), None);
    }

    #[test]
    fn test_a_hostile_destination_in_the_query_is_dropped_rather_than_followed() {
        // This is the reachable form of the open redirect: the query is whatever
        // a link in a phishing mail put there.
        for query in [
            "/login?next=https://evil.example",
            "/login?next=//evil.example",
            "/login?next=%2f%2fevil.example",
        ] {
            assert_eq!(next_from(&uri(query)), None, "{query}");
        }
    }

    #[test]
    fn test_a_malformed_query_does_not_break_the_login_page() {
        // Hand-parsed rather than extracted, so a query with no `=` is a missing
        // destination and not a 400 on the one page that has to work.
        assert_eq!(next_from(&uri("/login?next")), None);
        assert_eq!(next_from(&uri("/login?&&&")), None);
    }

    #[test]
    fn test_a_get_is_sent_back_to_where_it_was_going() {
        assert_eq!(destination(&parts("GET", "/projects/0801")), Some("/projects/0801"));
    }

    #[test]
    fn test_a_get_with_a_query_keeps_only_its_path() {
        assert_eq!(destination(&parts("GET", "/projects/0801?x=1")), Some("/projects/0801"));
    }

    #[test]
    fn test_a_write_is_not_sent_back_to_itself() {
        // A POST's destination is a side effect, not a page.
        assert_eq!(destination(&parts("POST", "/depositors")), None);
        assert_eq!(destination(&parts("DELETE", "/depositors/x")), None);
    }

    #[test]
    fn test_a_signed_in_user_lands_on_the_root_when_there_is_nowhere_better() {
        assert_eq!(destination_or_root(None), "/");
        assert_eq!(destination_or_root(Some("/projects")), "/projects");
        // Re-checked at the point of use, not trusted from wherever it was
        // carried: the value crossed a query string on the way here.
        assert_eq!(destination_or_root(Some("https://evil.example")), "/");
    }
}
