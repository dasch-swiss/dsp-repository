//! The two cookies the login flow uses, read and written by hand.
//!
//! Hand-rolled rather than `axum-extra`'s `CookieJar`: there are exactly two
//! cookies, neither is signed or encrypted (both carry an opaque 256-bit token
//! that means nothing without the row it addresses), and the alternative pulls
//! in `cookie` and `time` for one `Set-Cookie` line. The same reasoning kept
//! `RightmostXffKeyExtractor` local.
//!
//! ## The attributes, and which threat each one answers
//!
//! - `__Host-` prefix — a browser refuses to store the cookie unless it is `Secure`, `Path=/` and
//!   carries no `Domain`. That last part is the point: without it, any `*.dasch.swiss` host can set
//!   a `Domain=dasch.swiss` cookie of the same name, which our own requests would then carry and
//!   which we could not tell apart from ours. It does **not** stop a sibling host triggering a
//!   request that carries the real cookie — `Sec-Fetch-Site` does that.
//! - `HttpOnly` — script cannot read it, so an XSS cannot exfiltrate the session.
//! - `Secure` — never sent over plaintext. `http://localhost` counts as a trustworthy origin in
//!   Chrome and Firefox, so local development still works.
//! - `SameSite=Lax` — not `Strict`, per REQ-6.3. `Strict` would drop the cookie on the first
//!   navigation *into* the editor from a link in mail or chat, so the user would arrive signed out
//!   and sign in again for nothing. `Lax` still sends the cookie on top-level cross-site `GET`, so
//!   it is not a CSRF control by itself — which is why one exists separately.

use std::time::Duration;

use axum::http::header::{HeaderMap, HeaderValue, COOKIE};

/// The authenticated session (REQ-6.3).
pub(crate) const SESSION: &str = "__Host-editor_session";

/// The pre-auth binding: which browser asked for the outstanding code, and
/// therefore the only browser that may spend it.
pub(crate) const LOGIN: &str = "__Host-editor_login";

/// Attributes shared by both cookies. `Path=/` and `Secure` are not style — the
/// `__Host-` prefix makes a browser reject the cookie without them.
const ATTRIBUTES: &str = "Path=/; Secure; HttpOnly; SameSite=Lax";

/// Read one cookie by name.
///
/// Walks every `Cookie` header, not just the first: HTTP/2 clients are free to
/// split them, and a session that works over HTTP/1.1 and not HTTP/2 is a
/// miserable thing to diagnose.
pub(crate) fn read(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.trim() == name)
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// A `Set-Cookie` value storing `value` for `max_age`.
///
/// `max_age` is bounded by the thing the token addresses — a cookie that
/// outlives its row is a request that looks authenticated and is not.
pub(crate) fn set(name: &str, value: &str, max_age: Duration) -> HeaderValue {
    // Values here are base64url, so there is nothing to escape; a header value
    // that somehow could not be built is treated as no cookie at all rather than
    // being papered over.
    HeaderValue::from_str(&format!("{name}={value}; {ATTRIBUTES}; Max-Age={}", max_age.as_secs()))
        .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// A `Set-Cookie` value that removes the cookie.
///
/// The attributes have to match the ones it was set with, or the browser keeps
/// the original alongside the empty one and the user stays signed in.
pub(crate) fn clear(name: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("{name}=; {ATTRIBUTES}; Max-Age=0")).unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(cookies: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for cookie in cookies {
            headers.append(COOKIE, HeaderValue::from_str(cookie).unwrap());
        }
        headers
    }

    #[test]
    fn test_reads_a_single_cookie() {
        assert_eq!(read(&headers(&["__Host-editor_session=abc"]), SESSION), Some("abc".to_string()));
    }

    #[test]
    fn test_reads_one_cookie_out_of_several_on_one_header() {
        let headers = headers(&["other=1; __Host-editor_login=token-x; another=2"]);
        assert_eq!(read(&headers, LOGIN), Some("token-x".to_string()));
        assert_eq!(read(&headers, SESSION), None);
    }

    #[test]
    fn test_reads_across_several_cookie_headers() {
        // HTTP/2 clients may split them, and a session that works on HTTP/1.1 but
        // not HTTP/2 is a miserable thing to diagnose.
        let headers = headers(&["other=1", "__Host-editor_session=abc"]);
        assert_eq!(read(&headers, SESSION), Some("abc".to_string()));
    }

    #[test]
    fn test_tolerates_the_whitespace_browsers_actually_send() {
        let headers = headers(&["  other=1 ;  __Host-editor_session=abc  "]);
        assert_eq!(read(&headers, SESSION), Some("abc".to_string()));
    }

    #[test]
    fn test_an_empty_value_is_no_cookie() {
        // What `clear` leaves behind until the browser drops it. Treating it as
        // present would look up the empty token on every request.
        assert_eq!(read(&headers(&["__Host-editor_session="]), SESSION), None);
    }

    #[test]
    fn test_a_prefix_of_the_name_does_not_match() {
        // `editor_session` is not `__Host-editor_session`, and matching loosely
        // would let a cookie without the prefix stand in for one with it.
        let headers = headers(&["editor_session=abc; x__Host-editor_session=def"]);
        assert_eq!(read(&headers, SESSION), None);
    }

    #[test]
    fn test_a_value_containing_an_equals_sign_survives_intact() {
        // base64 padding is `=`, and splitting on every `=` would truncate it.
        assert_eq!(
            read(&headers(&["__Host-editor_session=ab=cd=="]), SESSION),
            Some("ab=cd==".to_string())
        );
    }

    #[test]
    fn test_no_cookie_header_at_all() {
        assert_eq!(read(&HeaderMap::new(), SESSION), None);
    }

    #[test]
    fn test_set_carries_every_attribute_the_host_prefix_requires() {
        // A `__Host-` cookie missing `Secure` or `Path=/`, or carrying a
        // `Domain`, is silently dropped by the browser — the request simply
        // arrives with no cookie, which reads like a server bug.
        let rendered = set(SESSION, "abc", Duration::from_secs(3600));
        let rendered = rendered.to_str().unwrap();
        assert!(rendered.starts_with("__Host-editor_session=abc;"), "{rendered}");
        assert!(rendered.contains("Path=/"), "{rendered}");
        assert!(rendered.contains("Secure"), "{rendered}");
        assert!(rendered.contains("HttpOnly"), "{rendered}");
        assert!(rendered.contains("SameSite=Lax"), "{rendered}");
        assert!(rendered.contains("Max-Age=3600"), "{rendered}");
        assert!(
            !rendered.contains("Domain"),
            "a Domain attribute makes the browser reject it: {rendered}"
        );
    }

    #[test]
    fn test_clear_matches_the_attributes_it_was_set_with() {
        // A mismatch leaves the original cookie in place beside the empty one,
        // and the user stays signed in after pressing sign out.
        let set = set(SESSION, "abc", Duration::from_secs(3600));
        let cleared = clear(SESSION);
        let (set, cleared) = (set.to_str().unwrap(), cleared.to_str().unwrap());
        for attribute in ["Path=/", "Secure", "HttpOnly", "SameSite=Lax"] {
            assert!(
                set.contains(attribute) && cleared.contains(attribute),
                "{attribute}: {set} / {cleared}"
            );
        }
        assert!(cleared.contains("Max-Age=0"), "{cleared}");
        assert!(cleared.starts_with("__Host-editor_session=;"), "{cleared}");
    }
}
