use leptos::prelude::*;

use super::filter_checkbox_group::FilterCheckboxGroup;

#[component]
pub fn ProjectFiltersContent(
    status_items: Vec<(String, bool, String)>,
    type_of_data_items: Vec<(String, bool, String)>,
    data_language_items: Vec<(String, bool, String)>,
    access_rights_items: Vec<(String, bool, String)>,
) -> impl IntoView {
    view! {
        <h4 class="dpe-title">"Filters"</h4>
        <div class="space-y-4">
            <div>
            <FilterCheckboxGroup
                title="Access Rights".to_string()
                items=access_rights_items
                info_href="https://dasch.swiss/knowledge-hub/fundamentals-copyright-licenses"
                info_tooltip="Access rights define how openly the data can be accessed. Learn more here."
            />
        </div>
            <div class="border-t border-neutral-200"></div>
            <div>
            <FilterCheckboxGroup title="Project Status".to_string() items=status_items />
        </div>
            <div class="border-t border-neutral-200"></div>
            <div>
            <FilterCheckboxGroup title="Type of Data".to_string() items=type_of_data_items />
        </div>
            <div class="border-t border-neutral-200"></div>
        <div>
            <FilterCheckboxGroup title="Data Language".to_string() items=data_language_items />
        </div>
        </div>
    }
}
