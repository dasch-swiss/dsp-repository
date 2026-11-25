use maud::{html, Markup};
use mosaic_tiles::card::{card, card_body, CardVariant};

use super::components::mobile_filters_button::mobile_filters_button;
use super::components::project_filters::project_filters;
use super::components::project_list::project_list;
use super::components::project_search_input::project_search_input;
use crate::domain::ProjectQuery;
use crate::RenderContext;

/// Projects index page.
///
/// Resolved synchronously from the in-memory project cache. The `query` is
/// parsed from the URL query string by the route handler and carries the
/// active search/filter/pagination state. The facets are one GET form, rendered
/// once: the sidebar at `lg`, the slide-in panel below it.
pub fn projects_page(query: &ProjectQuery, ctx: &RenderContext) -> Markup {
    let search_bar_inner = html! {
        div class="flex gap-4" {
            div class="flex-1 min-w-0" { (project_search_input()) }
            div class="lg:hidden" { (mobile_filters_button(query.dialog == Some(true))) }
        }
    };

    html! {
        div class="flex gap-4" {
            (project_filters(query, ctx))
            div class="flex-1 min-w-0 flex flex-col gap-2" {
                ({
                    card(card_body(search_bar_inner))
                        .variant(CardVariant::Bordered)
                        .class("overflow-visible")
                })
                (project_list(query, ctx))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_render_context;

    #[test]
    fn renders_filter_sidebar_and_search() {
        // Facets read the committed test corpus; the static page structure
        // renders regardless.
        let out = projects_page(&ProjectQuery::default(), &test_render_context(false)).into_string();
        assert!(out.contains("Filters"), "{out}");
        assert!(
            out.contains(r#"<form method="get" action="/dpe/projects">"#),
            "search form: {out}"
        );
        assert!(out.contains("Access Rights"), "{out}");
    }

    /// Every `href` on the page that carries a facet parameter or `dialog`.
    fn filter_hrefs(out: &str) -> Vec<String> {
        out.match_indices(r#"href=""#)
            .map(|(i, _)| {
                let rest = &out[i + 6..];
                rest[..rest.find('"').unwrap()].to_string()
            })
            .filter(|href| {
                ["ongoing=", "finished=", "type_of_data=", "data_language=", "access_rights=", "dialog="]
                    .iter()
                    .any(|p| href.contains(p))
            })
            .collect()
    }

    #[test]
    fn unfiltered_page_has_no_link_carrying_a_facet_or_dialog() {
        for raw in ["", "dialog=true", "page=2"] {
            let out = projects_page(&ProjectQuery::from_query_str(raw), &test_render_context(false)).into_string();
            assert_eq!(filter_hrefs(&out), Vec::<String>::new(), "{raw}");
        }
    }

    #[test]
    fn filtered_page_links_carry_facets_only_in_nofollow_pagination() {
        let ctx = test_render_context(false);
        let out = projects_page(&ProjectQuery::from_query_str("ongoing=true&dialog=true"), &ctx).into_string();
        for (i, _) in out.match_indices(r#"href="/dpe/projects?"#) {
            let tag_start = out[..i].rfind('<').unwrap();
            let tag = &out[tag_start..i + out[i..].find('>').unwrap()];
            assert!(tag.contains(r#"rel="nofollow""#), "{tag}");
        }
    }

    #[test]
    fn renders_the_facet_form_once_with_the_trigger_and_panel() {
        let ctx = test_render_context(false);
        let out = projects_page(&ProjectQuery::from_query_str("dialog=true"), &ctx).into_string();
        assert_eq!(out.matches(r#"<form id="project-filters""#).count(), 1, "{out}");
        assert!(out.contains(r#"aria-controls="project-filters-panel" aria-expanded="true""#), "{out}");
        assert!(out.contains(r#"id="project-filters-panel""#), "{out}");
    }
}
