use maud::{html, Markup};

use super::project_filters_content::project_filters_content;

/// Desktop filter sidebar: the filter panel wrapped in a card. Uses plain links
/// that reload the page with the toggled filter query.
pub fn project_filters(
    status_items: &[(String, bool, String)],
    type_of_data_items: &[(String, bool, String)],
    data_language_items: &[(String, bool, String)],
    access_rights_items: &[(String, bool, String)],
) -> Markup {
    html! {
        div class="card card-bordered dpe-small p-4 space-y-4 text-gray-700 w-full" {
            (project_filters_content(status_items, type_of_data_items, data_language_items, access_rights_items, false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_filter_content_in_a_card() {
        let empty: Vec<(String, bool, String)> = vec![];
        let out = project_filters(&empty, &empty, &empty, &empty).into_string();
        assert!(
            out.contains(r#"class="card card-bordered dpe-small p-4 space-y-4 text-gray-700 w-full""#),
            "{out}"
        );
        assert!(out.contains("Filters"), "{out}");
    }
}
