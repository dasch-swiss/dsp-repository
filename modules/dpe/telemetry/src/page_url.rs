/// Known DPE route patterns for page_url normalization.
pub const KNOWN_ROUTES: &[&str] = &["/", "/projects", "/about"];

/// Normalize a page URL to a known route pattern.
/// Returns "other" for unrecognized paths to prevent metric cardinality explosion.
pub fn normalize_page_url(url: &str) -> &'static str {
    for &route in KNOWN_ROUTES {
        if route == url {
            return route;
        }
    }
    // Pattern match for /projects/{shortcode} without allocating
    let trimmed = url.trim_matches('/');
    match trimmed.split_once('/') {
        Some(("projects", id)) if !id.is_empty() && !id.contains('/') => "/projects/{id}",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/projects"), "/projects");
        assert_eq!(normalize_page_url("/about"), "/about");
        assert_eq!(normalize_page_url("/"), "/");
    }

    #[test]
    fn project_detail() {
        assert_eq!(normalize_page_url("/projects/0803"), "/projects/{id}");
    }

    #[test]
    fn unknown_returns_other() {
        assert_eq!(normalize_page_url("/admin/secret"), "other");
        assert_eq!(normalize_page_url("/foo/bar/baz"), "other");
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn never_panics(s in "\\PC{0,200}") {
                let _ = normalize_page_url(&s);
            }

            #[test]
            fn output_is_bounded(s in "\\PC{0,200}") {
                let result = normalize_page_url(&s);
                let allowed = ["/", "/projects", "/about", "/projects/{id}", "other"];
                prop_assert!(allowed.contains(&result), "unexpected: {result} for: {s}");
            }

            #[test]
            fn project_detail_never_leaks_id(id in "[a-zA-Z0-9]{1,20}") {
                let url = format!("/projects/{id}");
                prop_assert_eq!(normalize_page_url(&url), "/projects/{id}");
            }
        }
    }
}
