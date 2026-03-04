use leptos::prelude::*;
use leptos_router::hooks::use_query;
use mosaic_tiles::card::{Card, CardBody, CardHeader, CardVariant};

use super::filter_checkbox_group::FilterCheckboxGroup;
use crate::domain::ProjectQuery;

// Regular component for filters and search - uses simple links that reload the page
#[component]
pub fn ProjectFilters() -> impl IntoView {
    // Use Leptos query for reading URL query parameters
    let query = use_query::<ProjectQuery>();
    let current_query = query.get().unwrap_or_default();

    let ongoing = current_query.ongoing();
    let finished = current_query.finished();
    let search = current_query.search();

    // Helper function to build URL with one parameter toggled
    let build_url = |toggle_param: &str| {
        let new_query = ProjectQuery {
            ongoing: Some(if toggle_param == "ongoing" { !ongoing } else { ongoing }),
            finished: Some(if toggle_param == "finished" {
                !finished
            } else {
                finished
            }),
            search: if search.is_empty() { None } else { Some(search.clone()) },
            page: Some(1),
        };
        format!("/projects{}", new_query.to_query_string())
    };

    // Compute filter items eagerly so no borrows are captured in the view
    let filter_items: Vec<(String, bool, String)> =
        [("ongoing", "Ongoing", ongoing), ("finished", "Finished", finished)]
            .iter()
            .map(|(param, label, checked)| (label.to_string(), *checked, build_url(param)))
            .collect();

    view! {
        <Card variant=CardVariant::Bordered class="w-72">
            <CardHeader>
                <h4 class="dpe-title">"Filters"</h4>
            </CardHeader>
            <CardBody>
                <div class="space-y-4">
                    <FilterCheckboxGroup title="Status" items=filter_items />
                    <div class="border-t border-neutral-200"></div>

                    <div class="dpe-subtitle">"Other filters TODO"</div>
                </div>
            </CardBody>
        </Card>
    }
}
