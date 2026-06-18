use maud::{html, Markup};

use super::project_loader::project_loader;

/// Project detail page content. The page `<title>` ("Project {shortcode}") is
/// set by the route handler; `active_tab` comes from the `?tab=` query param.
pub fn project_page(shortcode: &str, active_tab: &str) -> Markup {
    html! {
        div class="min-h-100" {
            (project_loader(shortcode, active_tab))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_loader_in_min_height_container() {
        let out = project_page("zzzz", "overview").into_string();
        assert!(out.contains(r#"class="min-h-100""#), "{out}");
        assert!(out.contains("Project Not Found"), "loader rendered: {out}");
    }
}
