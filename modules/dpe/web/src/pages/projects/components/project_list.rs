use dpe_core::Page;
use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::card::{card, card_body, CardVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::ComponentBuilder;

use super::card::project_card;
use super::project_pagination::project_pagination;
use crate::domain::projects::filter_and_paginate;
use crate::domain::ProjectQuery;

/// Filtered + paginated project list, resolved synchronously from the in-process
/// project cache. Renders an empty-state card when nothing matches, otherwise a
/// responsive grid of project cards plus pagination.
pub fn project_list(query: &ProjectQuery) -> Markup {
    render_project_list(filter_and_paginate(dpe_core::all_projects(), query, None), query)
}

/// Render a resolved [`Page`] of projects. Separated from the cache lookup so it
/// can be unit-tested with a synthetic page.
fn render_project_list(page: Page, query: &ProjectQuery) -> Markup {
    if page.total_items == 0 {
        let empty_state = html! {
            div class="text-center" {
                h3 class="mb-4" { "No projects found matching your criteria" }
                ({
                    link("Clear your filters", "/dpe/projects")
                        .as_button(ButtonVariant::Ghost)
                })
            }
        };
        return card(card_body(empty_state)).variant(CardVariant::Bordered).build();
    }

    html! {
        div {
            div class="mb-4 text-sm text-gray-600" { (format!("{} projects", page.total_items)) }
            div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4" {
                @for project in &page.items {
                    @let keywords: Vec<String> = project
                        .keywords
                        .iter()
                        .filter_map(|m| dpe_core::lang_value(m).cloned())
                        .collect();
                    (project_card(project, &keywords))
                }
            }
        }
        div class="flex justify-center" { (project_pagination(page.nr_pages, query)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_project;

    #[test]
    fn empty_page_renders_clear_filters_card() {
        let page = Page { items: vec![], nr_pages: 1, total_items: 0 };
        let out = render_project_list(page, &ProjectQuery::default()).into_string();
        assert!(out.contains("No projects found matching your criteria"), "{out}");
        assert!(out.contains("Clear your filters"), "{out}");
        assert!(out.contains(r#"href="/dpe/projects""#), "{out}");
    }

    #[test]
    fn non_empty_page_renders_count_cards_and_pagination() {
        let page = Page { items: vec![sample_project()], nr_pages: 2, total_items: 1 };
        let out = render_project_list(page, &ProjectQuery::default()).into_string();
        assert!(out.contains("1 projects"), "{out}");
        assert!(out.contains("Sample Research Project"), "card rendered: {out}");
        assert!(out.contains(r#"aria-label="Pagination""#), "pagination rendered: {out}");
    }
}
