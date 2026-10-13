//! The two cookies the login flow uses, written by hand and read through
//! `identity_core::cookie`.
//!
//! - `__Host-` prefix — a browser refuses to store the cookie unless it is `Secure`, `Path=/` and
//!   carries no `Domain`. Without that last part, any `*.dasch.swiss` host can set a
//!   `Domain=dasch.swiss` cookie of the same name, which our own requests would then carry and
//!   which we could not tell apart from ours. It does **not** stop a sibling host triggering a
//!   request that carries the real cookie — `Sec-Fetch-Site` does that.
//! - `HttpOnly` — script cannot read it, so an XSS cannot exfiltrate the session.
//! - `Secure` — never sent over plaintext. `http://localhost` counts as a trustworthy origin in
//!   Chrome and Firefox, so local development still works.
//! - `SameSite=Lax` — not `Strict`, which would drop the cookie on the first navigation *into* the
//!   editor from a link in mail or chat, so the user would arrive signed out. `Lax` is not a CSRF
//!   control by itself, which is why one exists separately.
//!
//! The argument is in `docs/src/editor/authentication.md`.

use std::time::Duration;

use axum::http::header::{HeaderMap, HeaderValue, COOKIE};

/// Attributes shared by both cookies. `Path=/` and `Secure` are not style — the
/// `__Host-` prefix makes a browser reject the cookie without them.
const ATTRIBUTES: &str = "Path=/; Secure; HttpOnly; SameSite=Lax";

/// Every `Cookie` header value on the request, walking all of them rather than
/// the first: HTTP/2 clients are free to split them.
pub(crate) fn values(headers: &HeaderMap) -> Vec<&str> {
    headers.get_all(COOKIE).iter().filter_map(|value| value.to_str().ok()).collect()
}

/// Read one cookie by name from the request's `Cookie` headers.
pub(crate) fn read(headers: &HeaderMap, name: &str) -> Option<String> {
    identity_core::cookie::read(&values(headers), name)
}

/// A `Set-Cookie` value storing `value` for `max_age`, which is bounded by the
/// thing the token addresses — a cookie that outlives its row is a request that
/// looks authenticated and is not.
pub(crate) fn set(name: &str, value: &str, max_age: Duration) -> HeaderValue {
    // Values here are base64url, so there is nothing to escape; a header value
    // that somehow could not be built is treated as no cookie at all rather than
    // being papered over.
    HeaderValue::from_str(&format!("{name}={value}; {ATTRIBUTES}; Max-Age={}", max_age.as_secs()))
        .unwrap_or_else(|_| HeaderValue::from_static(""))
}

/// A `Set-Cookie` value that removes the cookie. The attributes have to match
/// the ones it was set with, or the browser keeps the original alongside the
/// empty one and the user stays signed in.
pub(crate) fn clear(name: &str) -> HeaderValue {
    HeaderValue::from_str(&format!("{name}=; {ATTRIBUTES}; Max-Age=0")).unwrap_or_else(|_| HeaderValue::from_static(""))
}

#[cfg(test)]
mod tests {
    use identity_core::cookie::SESSION;

    use super::*;

    fn headers(cookies: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for cookie in cookies {
            headers.append(COOKIE, HeaderValue::from_str(cookie).unwrap());
        }
        headers
    }

    #[test]
    fn test_reads_across_several_cookie_headers() {
        let headers = headers(&["other=1", "__Host-editor_session=abc"]);
        assert_eq!(read(&headers, SESSION), Some("abc".to_string()));
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
