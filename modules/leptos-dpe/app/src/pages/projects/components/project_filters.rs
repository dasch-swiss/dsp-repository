use leptos::prelude::*;

use super::project_filters_content::ProjectFiltersContent;

// Regular component for filters and search - uses simple links that reload the page
#[component]
pub fn ProjectFilters(
    status_items: Vec<(String, bool, String)>,
    type_of_data_items: Vec<(String, bool, String)>,
    data_language_items: Vec<(String, bool, String)>,
    access_rights_items: Vec<(String, bool, String)>,
) -> impl IntoView {
    view! {
        <div class="dpe-card w-full">
            <ProjectFiltersContent
                status_items=status_items
                type_of_data_items=type_of_data_items
                data_language_items=data_language_items
                access_rights_items=access_rights_items
            />
        </div>
    }
}
