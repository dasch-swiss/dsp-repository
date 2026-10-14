/// The query key that carries where the user was going.
pub const NEXT: &str = "next";

/// The longest destination [`safe_next`] accepts.
///
/// Generous against the paths this service has and short enough that the value
/// cannot be used to build a long `Location` out of a request.
const MAX_NEXT_LEN: usize = 256;

/// The login URL, carrying `next` when there is a destination worth keeping.
///
/// No percent-encoding, and that is a property of [`safe_next`] rather than an
/// omission: the characters it admits are all legal, unreserved query
/// characters. Widening `safe_next` without adding encoding here would be a
/// header-injection bug in a `Location`, and the negative tests below are what
/// stop that.
pub fn login_url(next: Option<&str>) -> String {
    match next {
        Some(next) => format!("/login?{NEXT}={next}"),
        None => "/login".to_string(),
    }
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
/// A `..` segment is refused because a destination that walks upward is either
/// an attack or a bug, and the login screens are refused because they would
/// loop. The query is dropped: no route here navigates by query, and `?` and `&`
/// are what make the encoding question real.
pub fn safe_next(candidate: &str) -> Option<&str> {
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
    use super::*;

    #[test]
    fn test_a_path_inside_the_service_is_a_destination() {
        for candidate in ["/", "/projects", "/projects/0801", "/depositors", "/projects/0801a"] {
            assert_eq!(safe_next(candidate), Some(candidate), "{candidate}");
        }
    }

    #[test]
    fn test_an_absolute_url_is_never_a_destination() {
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
}
