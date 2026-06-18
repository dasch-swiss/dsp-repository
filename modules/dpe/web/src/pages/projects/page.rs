use maud::{html, Markup};
use mosaic_tiles::card::{card, card_body, CardProps, CardVariant};

use super::components::mobile_filters_button::mobile_filters_button;
use super::components::project_filters::project_filters;
use super::components::project_list::project_list;
use super::components::project_search_input::project_search_input;
use crate::domain::{list_data_languages, list_type_of_data, ProjectQuery};

/// Projects index page.
///
/// Resolved synchronously from the in-memory project cache. The `query` is
/// parsed from the URL query string by the Axum route handler and carries the
/// active search/filter/pagination state.
pub fn projects_page(query: &ProjectQuery) -> Markup {
    let status_items = query.status_filter_items();
    let access_rights_items = query.access_rights_filter_items();

    let dialog_open = query.dialog.unwrap_or(false);
    let open_dialog_href = format!("/dpe/projects{}", query.clone().with_dialog(true).to_query_string());
    let close_dialog_href = format!("/dpe/projects{}", query.clone().with_dialog(false).to_query_string());

    let type_of_data_selected = query.type_of_data();
    let type_of_data_items: Vec<(String, bool, String)> = list_type_of_data()
        .into_iter()
        .map(|t| {
            let checked = type_of_data_selected.contains(&t);
            let href = format!("/dpe/projects{}", query.with_type_of_data_toggled(&t).to_query_string());
            (t, checked, href)
        })
        .collect();

    let data_language_selected = query.data_language();
    let data_language_items: Vec<(String, bool, String)> = list_data_languages()
        .into_iter()
        .map(|(code, display)| {
            let checked = data_language_selected.contains(&code);
            let href = format!("/dpe/projects{}", query.with_data_language_toggled(&code).to_query_string());
            (display, checked, href)
        })
        .collect();

    html! {
        div class="flex gap-4" {
            div class="hidden lg:block lg:w-72 2xl:w-80 flex-shrink-0" {
                (project_filters(&status_items, &type_of_data_items, &data_language_items, &access_rights_items))
            }
            div class="flex-1 flex flex-col gap-2" {
                (card(
                    CardProps { variant: CardVariant::Bordered, class: "overflow-visible" },
                    card_body("", html! {
                        div class="flex gap-4" {
                            div class="flex-1" { (project_search_input()) }
                            div class="lg:hidden" {
                                (mobile_filters_button(
                                    &status_items, &type_of_data_items, &data_language_items, &access_rights_items,
                                    dialog_open, &open_dialog_href, &close_dialog_href,
                                ))
                            }
                        }
                    }),
                ))
                (project_list(query))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_filter_sidebar_and_search() {
        // Facets read the in-memory cache (empty in the test environment); the
        // static page structure renders regardless.
        let out = projects_page(&ProjectQuery::default()).into_string();
        assert!(out.contains("Filters"), "{out}");
        assert!(
            out.contains(r#"<form method="get" action="/dpe/projects">"#),
            "search form: {out}"
        );
        assert!(out.contains("Access Rights"), "{out}");
    }
}
