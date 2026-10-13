//! The two cookie names the login flow uses, and the scan that finds one in the
//! `Cookie` headers of a request.
//!
//! The scan is pure over header values, so it needs no HTTP types; the
//! attributes the cookies are written with belong to the layer that writes them.

/// The session cookie.
pub const SESSION: &str = "__Host-editor_session";

/// The pre-auth binding: which browser asked for the outstanding code, and
/// therefore the only browser that may spend it.
pub const LOGIN: &str = "__Host-editor_login";

/// Read one cookie by name out of every `Cookie` header value, not just the
/// first: HTTP/2 clients are free to split them.
#[must_use]
pub fn read(cookie_headers: &[&str], name: &str) -> Option<String> {
    cookie_headers
        .iter()
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| key.trim() == name)
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reads_one_cookie_out_of_several_on_one_header() {
        let headers = ["other=1; __Host-editor_login=token-x; another=2"];
        assert_eq!(read(&headers, LOGIN), Some("token-x".to_string()));
        assert_eq!(read(&headers, SESSION), None);
    }

    #[test]
    fn test_reads_across_several_cookie_headers() {
        let headers = ["other=1", "__Host-editor_session=abc"];
        assert_eq!(read(&headers, SESSION), Some("abc".to_string()));
    }

    #[test]
    fn test_tolerates_the_whitespace_browsers_actually_send() {
        let headers = ["  other=1 ;  __Host-editor_session=abc  "];
        assert_eq!(read(&headers, SESSION), Some("abc".to_string()));
    }

    #[test]
    fn test_an_empty_value_is_no_cookie() {
        // What `clear` leaves behind until the browser drops it. Treating it as
        // present would look up the empty token on every request.
        assert_eq!(read(&["__Host-editor_session="], SESSION), None);
    }

    #[test]
    fn test_a_prefix_of_the_name_does_not_match() {
        // `editor_session` is not `__Host-editor_session`, and matching loosely
        // would let a cookie without the prefix stand in for one with it.
        let headers = ["editor_session=abc; x__Host-editor_session=def"];
        assert_eq!(read(&headers, SESSION), None);
    }

    #[test]
    fn test_a_value_containing_an_equals_sign_survives_intact() {
        // base64 padding is `=`, and splitting on every `=` would truncate it.
        assert_eq!(read(&["__Host-editor_session=ab=cd=="], SESSION), Some("ab=cd==".to_string()));
    }
}
