//! Who may reach a route, and what happens to a request that may not.
//!
//! ## The default is closed, and it is closed by the type system
//!
//! Authentication is an **extractor**, not a middleware, and that is the whole
//! design. A handler that names [`Authenticated`] in its arguments cannot run
//! without a live session, because the argument is what runs the check; a
//! handler that does not name it is visibly public at the point anyone reads it.
//! There is no ordering to get right and no sub-router to remember to attach
//! something to.
//!
//! A middleware layered over a group of routes would have been the other option,
//! and the router already carries one positional invariant of exactly that shape
//! — the traced/untraced split, which is invisible in the route table and
//! reversible by moving one line. Adding a second one, where the failure mode is
//! an unauthenticated route rather than a missing span, was not worth the
//! symmetry.
//!
//! ## What is deliberately public
//!
//! `/` (a redirect), the two login screens, `/logout`, `/healthz`, the telemetry
//! beacon, and the static assets. Everything else takes [`Authenticated`]. The
//! collection endpoint (REQ-5.1) is the one route that will be public *and*
//! serve data; it does not exist yet, and when it lands it is public by being
//! written without this extractor, which is a visible choice in its signature.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::Method;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::Utc;
use editor_core::records::User;

use super::session;
use crate::AppState;

/// The query key that carries where the user was going.
pub(crate) const NEXT: &str = "next";

/// The longest destination [`safe_next`] accepts.
///
/// Generous against the paths this service has and short enough that the value
/// cannot be used to build a long `Location` out of a request.
const MAX_NEXT_LEN: usize = 256;

/// A live session, or the request never reaches the handler.
///
/// The rejection is a redirect rather than a 401: every route behind this is a
/// page, and a browser handed a 401 with no `WWW-Authenticate` shows nothing
/// useful. Signing in and landing back on the page that was asked for is the
/// behaviour, which is what `next` carries.
#[derive(Debug, Clone)]
pub(crate) struct Authenticated(pub(crate) User);

impl FromRequestParts<AppState> for Authenticated {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match session::current(&state.db, &state.auth, &parts.headers, Utc::now()).await {
            Some(user) => Ok(Self(user)),
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
/// The two refusals are deliberately different. No session is a redirect to
/// login, because signing in fixes it. A session that is not RDU's is a 403
/// page, because signing in again will not: it is the same account, and sending
/// them to a login screen they are already past reads as a bug.
#[derive(Debug, Clone)]
pub(crate) struct Rdu(pub(crate) User);

impl FromRequestParts<AppState> for Rdu {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let Authenticated(user) = Authenticated::from_request_parts(parts, state).await?;
        if !user.is_rdu() {
            // Worth a line: a depositor reaching an administration URL is either
            // a stale bookmark or someone trying doors, and both are things an
            // operator should be able to see. The only identifier is the
            // account's own opaque id.
            // No `http.route` here on purpose. The enclosing server span
            // already carries it, set from `MatchedPath` by the OTel layer, so
            // an event field would be a second value for one semconv attribute
            // in a single trace — and the concrete path is the wrong one of the
            // two, since semconv defines `http.route` as the matched template.
            // `auth.subject` is what makes this line useful.
            tracing::info!("refused an RDU-only page to an account that is not RDU");
            return Err(crate::forbidden(state, &user, crate::depositors::RDU_ONLY));
        }
        Ok(Self(user))
    }
}

/// Where to send this request back to once its owner has signed in.
///
/// `GET` only, deliberately. `next` exists to land someone where they were
/// *going*, and a `POST`'s destination is a side effect rather than a page:
/// re-issuing it after sign-in is not something a redirect can do, and sending
/// the browser to a `GET` of the same path would look like the write happened.
/// What should happen to a write that arrives after a session expires — stash
/// and replay, or lean on draft autosave — is a decision the plan pins to the
/// form work, because it is the form that has something to lose.
fn destination(parts: &Parts) -> Option<&str> {
    if parts.method != Method::GET {
        return None;
    }
    safe_next(parts.uri.path())
}

/// The login URL, carrying `next` when there is a destination worth keeping.
///
/// No percent-encoding, and that is a property of [`safe_next`] rather than an
/// omission: the characters it admits are all legal, unreserved query
/// characters. Widening `safe_next` without adding encoding here would be a
/// header-injection bug in a `Location`, and what stops that are the *negative*
/// tests — admitting `?` fails `test_a_query_is_not_carried`, admitting `%`
/// fails `test_the_shapes_that_become_an_absolute_url_after_one_transformation_are_refused`.
/// `test_every_admitted_destination_is_safe_to_embed_without_encoding` documents
/// the pairing but iterates a fixed list, so it would not catch a widening on
/// its own.
pub(crate) fn login_url(next: Option<&str>) -> String {
    match next {
        Some(next) => format!("/login?{NEXT}={next}"),
        None => "/login".to_string(),
    }
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

/// `candidate` if it is a destination inside this service, otherwise `None`.
///
/// **This is an open-redirect check**, and it is the only one. The value reaches
/// us from a query string, so it is entirely attacker-chosen: a login link
/// carrying `next=https://evil.example` would otherwise turn our own sign-in
/// page into a redirector that a phishing mail can point at, arriving from the
/// real editor origin with a real TLS certificate.
///
/// It is an allowlist rather than a list of things to reject, because the reject
/// list for this is not knowable:
///
/// - `//evil.example` is protocol-relative and leaves the origin, while looking like a path.
/// - `/\evil.example` is normalised to `//evil.example` by browsers that treat a backslash as a
///   separator.
/// - `%2f%2fevil.example` is the same thing after one decode, which is why `%` is not admitted.
/// - A `\r` or `\n` splits the `Location` header.
///
/// So: a leading `/`, then ASCII alphanumerics and `/`, `-`, `_`, `.` only. No
/// `..` segment, because a destination that walks upward is either an attack or
/// a bug and neither should be followed. Not the login screens themselves, which
/// would loop.
///
/// The query is dropped rather than carried. No route in this service navigates
/// by query today, so admitting one would be widening the surface for nothing —
/// and it is `?` and `&` that make the encoding question real.
pub(crate) fn safe_next(candidate: &str) -> Option<&str> {
    let ok = !candidate.is_empty()
        && candidate.len() <= MAX_NEXT_LEN
        && candidate.starts_with('/')
        && !candidate.starts_with("//")
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
        && !candidate.split('/').any(|segment| segment == "..")
        && candidate != "/login"
        && !candidate.starts_with("/login/");
    ok.then_some(candidate)
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

    #[test]
    fn test_a_path_inside_the_service_is_a_destination() {
        for candidate in ["/", "/projects", "/projects/0801", "/depositors", "/projects/0801a"] {
            assert_eq!(safe_next(candidate), Some(candidate), "{candidate}");
        }
    }

    #[test]
    fn test_an_absolute_url_is_never_a_destination() {
        // The value comes from a query string, so a login link carrying it is
        // attacker-authored: without this, the editor's own sign-in page becomes
        // a redirector a phishing mail can point at, arriving from the real
        // origin with a real certificate.
        for candidate in [
            "https://evil.example",
            "http://evil.example/x",
            "//evil.example",
            "///evil.example",
            "javascript:alert(1)",
            "data:text/html,x",
        ] {
            assert_eq!(safe_next(candidate), None, "{candidate}");
        }
    }

    #[test]
    fn test_the_shapes_that_become_an_absolute_url_after_one_transformation_are_refused() {
        // Each of these is a path until something normalises or decodes it:
        // browsers treat `\` as a separator, and a proxy or framework may decode
        // `%2f` before the redirect is followed.
        for candidate in ["/\\evil.example", "/%2f%2fevil.example", "/%09/evil.example"] {
            assert_eq!(safe_next(candidate), None, "{candidate}");
        }
    }

    #[test]
    fn test_a_destination_cannot_split_the_location_header() {
        for candidate in ["/x\r\nSet-Cookie: a=b", "/x\nLocation: https://evil.example", "/x\r"] {
            assert_eq!(safe_next(candidate), None, "{candidate:?}");
        }
    }

    #[test]
    fn test_a_destination_that_walks_upward_is_refused() {
        assert_eq!(safe_next("/../admin"), None);
        assert_eq!(safe_next("/projects/../../etc"), None);
        // A `..` inside a segment is a filename, not a traversal.
        assert_eq!(safe_next("/projects/a..b"), Some("/projects/a..b"));
    }

    #[test]
    fn test_the_login_screens_are_not_destinations() {
        // Otherwise signing in sends the user back to the sign-in page.
        assert_eq!(safe_next("/login"), None);
        assert_eq!(safe_next("/login/code"), None);
    }

    #[test]
    fn test_a_query_is_not_carried() {
        // No route navigates by query, so admitting one would widen the surface
        // for nothing — and `?`/`&` are what make the encoding question real.
        assert_eq!(safe_next("/projects?x=1"), None);
        assert_eq!(safe_next("/projects#frag"), None);
    }

    #[test]
    fn test_an_overlong_destination_is_refused() {
        let long = format!("/{}", "a".repeat(MAX_NEXT_LEN));
        assert_eq!(safe_next(&long), None);
    }

    #[test]
    fn test_every_admitted_destination_is_safe_to_embed_without_encoding() {
        // `login_url` interpolates the value straight into a query string, which
        // is only sound while `safe_next` admits nothing that needs escaping.
        // Widening one without the other is a header-injection bug in a
        // `Location`, so the two are pinned together here.
        for candidate in ["/", "/projects/0801", "/a-b_c.d/e"] {
            let admitted = safe_next(candidate).expect("admitted");
            assert!(
                admitted
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '-' | '_' | '.')),
                "{admitted}"
            );
            assert_eq!(login_url(Some(admitted)), format!("/login?next={admitted}"));
        }
        assert_eq!(login_url(None), "/login");
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
        // A POST's destination is a side effect, not a page. Redirecting to a
        // GET of the same path after sign-in would look like the write landed.
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
