//! Identity's part of the root's page-URL normalizer: `None` hands the path to the next
//! capability's. An account id is unbounded, so it collapses to its pattern.

const KNOWN_ROUTES: &[&str] = &["/login", "/login/code", "/depositors", "/depositors/new"];

pub fn normalize_page_url(url: &str) -> Option<&'static str> {
    let url = url.split(['?', '#']).next().unwrap_or(url);
    if let Some(route) = KNOWN_ROUTES.iter().copied().find(|route| *route == url) {
        return Some(route);
    }
    match url.strip_prefix("/depositors/")?.split_once('/') {
        Some((id, "edit")) if !id.is_empty() => Some("/depositors/{id}/edit"),
        Some((id, "remove")) if !id.is_empty() => Some("/depositors/{id}/remove"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::{routes, RouteKind};

    #[test]
    fn known_routes() {
        assert_eq!(normalize_page_url("/login"), Some("/login"));
        assert_eq!(normalize_page_url("/login/code"), Some("/login/code"));
        assert_eq!(normalize_page_url("/depositors"), Some("/depositors"));
        assert_eq!(normalize_page_url("/depositors/new"), Some("/depositors/new"));
    }

    #[test]
    fn an_account_id_collapses_to_its_pattern() {
        let id = "0c1cd9ff-9a9f-4b0e-9b0a-3f2f8f0b7a11";
        assert_eq!(
            normalize_page_url(&format!("/depositors/{id}/edit")),
            Some("/depositors/{id}/edit")
        );
        assert_eq!(
            normalize_page_url(&format!("/depositors/{id}/remove")),
            Some("/depositors/{id}/remove")
        );
    }

    #[test]
    fn a_path_identity_does_not_render_is_left_to_the_next_normalizer() {
        for url in [
            "/",
            "/projects",
            "/login/code/extra",
            "/depositors/",
            "/depositors//edit",
            "/depositors/abc/delete",
            "/depositors/0c1cd9ff-9a9f-4b0e",
        ] {
            assert_eq!(normalize_page_url(url), None, "{url}");
        }
    }

    #[test]
    fn a_query_is_stripped_rather_than_losing_the_page() {
        assert_eq!(normalize_page_url("/login?next=/projects"), Some("/login"));
        assert_eq!(normalize_page_url("/depositors#top"), Some("/depositors"));
    }

    /// Panics on an unknown placeholder rather than guessing a value the normalizer accepts.
    fn sample(placeholder: &str) -> &'static str {
        match placeholder {
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
