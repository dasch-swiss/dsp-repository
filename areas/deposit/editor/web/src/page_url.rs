//! The editor's part of the root's page-URL normalizer.
//!
//! Bounds the `page.url` metric attribute to the editor's own routes so browser metrics stay
//! breakable down by page without exploding cardinality on arbitrary paths. The editor is
//! root-mounted, unlike DPE's `/dpe/…` prefix, since it runs on its own hostname. `/` and
//! `/projects/{shortcode}` are deliberately absent: both are redirects, so neither renders the
//! beacon script and no beacon can report them. The normalizer still answers sanely if one
//! somehow did.
//!
//! A query string is stripped before matching, so the normalizer survives the beacon one day
//! sending `location.href` rather than today's `location.pathname`.

const KNOWN_ROUTES: &[&str] = &["/projects", "/states", "/review", "/collection"];

/// The editor's part of the root's page-URL normalizer: `None` hands the path to the next
/// capability's.
pub fn normalize_page_url(url: &str) -> Option<&'static str> {
    // Everything below matches on a whole path, so a query or a fragment has to
    // come off first or it turns every page view into a miss.
    let url = url.split(['?', '#']).next().unwrap_or(url);
    for &route in KNOWN_ROUTES {
        if route == url {
            return Some(route);
        }
    }
    // Pattern matches for the routes with a variable segment, without
    // allocating. A shortcode and an account id are both unbounded sets, so
    // letting either through verbatim is the cardinality explosion this module
    // exists to prevent.
    if let Some(rest) = url.strip_prefix("/projects/") {
        // Both segments collapse. A section id is a closed set and would be safe
        // to keep, but a shortcode is not, and the pair is one attribute value:
        // `/projects/0801/sections/overview` and `/projects/080C/sections/overview`
        // have to arrive as the same one.
        if let Some((shortcode, tail)) = rest.split_once('/') {
            if !shortcode.is_empty() {
                // The entity form for one proposal, and its row actions. The segment is an
                // `entity_id`, allocated without bound, so it collapses.
                if let Some(proposal) = tail.strip_prefix("entities/") {
                    if proposal.is_empty() {
                        return None;
                    }
                    if !proposal.contains('/') {
                        return Some("/projects/{shortcode}/entities/{proposal}");
                    }
                    if let Some(rest) = proposal.split_once('/').map(|(_, rest)| rest) {
                        if let Some(rest) = rest.strip_prefix("fields/") {
                            if rest.ends_with("/add") {
                                return Some("/projects/{shortcode}/entities/{proposal}/fields/{field}/add");
                            }
                            if rest.ends_with("/remove") {
                                return Some("/projects/{shortcode}/entities/{proposal}/fields/{field}/{key}/remove");
                            }
                        }
                    }
                    return None;
                }
                if let Some(section) = tail.strip_prefix("sections/") {
                    if !section.is_empty() && !section.contains('/') {
                        return Some("/projects/{shortcode}/sections/{section}");
                    }
                    // The row actions render the full page shell on the
                    // no-JavaScript path, so they emit a beacon of their own and
                    // need their own entries.
                    if let Some(rest) = section.split_once('/').map(|(_, rest)| rest) {
                        if let Some(rest) = rest.strip_prefix("fields/") {
                            if rest.ends_with("/add") {
                                return Some("/projects/{shortcode}/sections/{section}/fields/{field}/add");
                            }
                            if rest.ends_with("/remove") {
                                return Some("/projects/{shortcode}/sections/{section}/fields/{field}/{key}/remove");
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(rest) = url.strip_prefix("/review/") {
        if !rest.is_empty() && !rest.contains('/') {
            return Some("/review/{shortcode}");
        }
        return None;
    }
    // An approved record's id, an unbounded set.
    if let Some(rest) = url.strip_prefix("/collection/") {
        return match rest.split_once('/') {
            Some((id, "discard")) if !id.is_empty() => Some("/collection/{id}/discard"),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::{routes, RouteKind};

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/projects"), Some("/projects"));
        assert_eq!(normalize_page_url("/states"), Some("/states"));
        assert_eq!(normalize_page_url("/review"), Some("/review"));
        assert_eq!(normalize_page_url("/collection"), Some("/collection"));
    }

    #[test]
    fn variable_segments_collapse_to_their_pattern() {
        assert_eq!(
            normalize_page_url("/projects/0801/sections/overview"),
            Some("/projects/{shortcode}/sections/{section}")
        );
        assert_eq!(
            normalize_page_url("/projects/080C/sections/dataset"),
            Some("/projects/{shortcode}/sections/{section}")
        );
        assert_eq!(normalize_page_url("/review/0801d"), Some("/review/{shortcode}"));
        assert_eq!(normalize_page_url("/review/080C"), Some("/review/{shortcode}"));
        let id = "0c1cd9ff-9a9f-4b0e-9b0a-3f2f8f0b7a11";
        assert_eq!(
            normalize_page_url(&format!("/collection/{id}/discard")),
            Some("/collection/{id}/discard")
        );
    }

    #[test]
    fn a_path_the_editor_does_not_render_is_left_to_the_next_normalizer() {
        for url in [
            "/",
            "/login",
            "/depositors",
            "/healthz",
            "/projects/",
            "/projects/0801",
            "/projects//sections/overview",
            "/projects/0801/sections/",
            "/projects/0801/sections/a/b",
            "/review/",
            "/review/0801d/fields",
            "/collection/",
            "/collection//discard",
            "/collection/abc/delete",
        ] {
            assert_eq!(normalize_page_url(url), None, "{url}");
        }
    }

    /// Panics on an unknown placeholder rather than guessing a value the normalizer accepts.
    fn sample(placeholder: &str) -> &'static str {
        match placeholder {
            "shortcode" => "080C",
            "section" => "overview",
            "field" => "keywords",
            "key" => "r0",
            "proposal" => "person-417",
            "id" => "0c1cd9ff-9a9f-4b0e-9b0a-3f2f8f0b7a11",
            other => panic!("no sample value for `{{{other}}}`: add one to `sample`"),
        }
    }

    fn with_samples(template: &str) -> String {
        template
            .split('/')
            .map(|segment| match segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                Some(placeholder) => sample(placeholder),
                None => segment,
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    #[test]
    fn every_full_page_route_is_labelled_with_its_own_template() {
        let pages: Vec<&str> = routes()
            .into_iter()
            .filter(|(_, kind, _)| *kind == RouteKind::Page)
            .map(|(path, _, _)| path)
            .collect();
        assert!(!pages.is_empty(), "the route table lists no full-page route");
        for template in pages {
            let url = with_samples(template);
            assert_eq!(normalize_page_url(&url), Some(template), "{url}");
        }
    }
}
