use maud::{html, Markup};

use super::project_filters_content::{project_filters_content, FILTER_FORM_ID};
use crate::domain::ProjectQuery;
use crate::RenderContext;

/// The `id` of the filter panel, which the mobile trigger controls.
pub const FILTER_PANEL_ID: &str = "project-filters-panel";

/// The one container of the facet form: the static sidebar card at `lg`, a fixed slide-in
/// panel with a backdrop below `lg`. Below `lg` the panel is hidden unless `dialog=true` is
/// in the request. The panel and the backdrop are always rendered, so the JavaScript path
/// can show them without a request. The panel is a non-modal disclosure: no focus trap,
/// the backdrop only dims and takes a click.
pub fn project_filters(query: &ProjectQuery, ctx: &RenderContext) -> Markup {
    let open = query.dialog == Some(true);
    let backdrop_class = if open {
        "fixed inset-0 bg-black/40 z-40 lg:hidden"
    } else {
        "fixed inset-0 bg-black/40 z-40 lg:hidden max-lg:hidden"
    };
    let panel_class = if open {
        "max-lg:fixed max-lg:right-0 max-lg:top-0 max-lg:bottom-0 max-lg:w-full md:max-lg:w-96 max-lg:z-50 max-lg:overflow-y-auto bg-white lg:w-72 2xl:w-80 lg:shrink-0"
    } else {
        "max-lg:hidden max-lg:fixed max-lg:right-0 max-lg:top-0 max-lg:bottom-0 max-lg:w-full md:max-lg:w-96 max-lg:z-50 max-lg:overflow-y-auto bg-white lg:w-72 2xl:w-80 lg:shrink-0"
    };
    html! {
        div id="project-filters-backdrop" aria-hidden="true" class=(backdrop_class) {}
        div id=(FILTER_PANEL_ID)
            role="region"
            aria-labelledby="project-filters-heading"
            tabindex="-1"
            class=(panel_class)
        {
            div class="relative lg:rounded-lg lg:border lg:border-neutral-200 dpe-small p-4 text-gray-700 w-full" {
                button
                    type="submit"
                    form=(FILTER_FORM_ID)
                    aria-label="Close filters"
                    autofocus[open]
                    class="btn btn-ghost size-8 justify-center rounded-full p-0 absolute right-2 top-2 cursor-pointer lg:hidden"
                { "✕" }
                (project_filters_content(query, ctx))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_render_context;

    fn render(raw: &str) -> String {
        project_filters(&ProjectQuery::from_query_str(raw), &test_render_context(false)).into_string()
    }

    #[test]
    fn closed_panel_and_backdrop_render_hidden_below_lg() {
        let out = render("");
        assert!(out.contains(r#"<div id="project-filters-backdrop" aria-hidden="true" class="fixed inset-0 bg-black/40 z-40 lg:hidden max-lg:hidden">"#), "{out}");
        assert!(out.contains(r#"tabindex="-1" class="max-lg:hidden "#), "{out}");
        assert!(!out.contains("autofocus"), "{out}");
    }

    #[test]
    fn dialog_true_renders_the_panel_open_with_close_focused() {
        let out = render("dialog=true");
        assert!(out.contains(r#"<div id="project-filters-backdrop" aria-hidden="true" class="fixed inset-0 bg-black/40 z-40 lg:hidden">"#), "{out}");
        assert!(out.contains(r#"id="project-filters-panel" role="region" aria-labelledby="project-filters-heading" tabindex="-1" class="max-lg:fixed "#), "{out}");
        assert!(out.contains(r#"<button type="submit" form="project-filters" aria-label="Close filters" autofocus"#), "{out}");
    }

    #[test]
    fn renders_the_form_once_and_no_link_to_a_filtered_url() {
        let out = render("dialog=true&type_of_data=T");
        assert_eq!(out.matches(r#"<form id="project-filters""#).count(), 1, "{out}");
        assert!(!out.contains(r#"href="/dpe/projects?"#), "{out}");
        assert!(!out.contains("outline-none"), "{out}");
    }
}
