use maud::{html, Markup};

use crate::domain::{get_contributors, get_project};
use crate::pages::project::components::project_details::project_details;

/// Load a project by shortcode and render its detail view, or a "not found"
/// message. Looked up synchronously from the in-process project + contributor
/// caches. `active_tab` selects the initially-rendered tab.
pub fn project_loader(shortcode: &str, active_tab: &str) -> Markup {
    match get_project(shortcode) {
        Some(project) => {
            let contributors = get_contributors(project.attributions.clone());
            project_details(&project, &contributors, active_tab)
        }
        None => html! {
            div class="text-center py-12" {
                h1 class="font-display text-3xl font-bold mb-4" { "Project Not Found" }
                p class="text-lg" { "The project with shortcode " (shortcode) " could not be found." }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_shortcode_renders_not_found() {
        // No project cache is populated in the unit-test environment.
        let out = project_loader("zzzz", "overview").into_string();
        assert!(out.contains("Project Not Found"), "{out}");
        assert!(out.contains("zzzz"), "{out}");
    }
}
