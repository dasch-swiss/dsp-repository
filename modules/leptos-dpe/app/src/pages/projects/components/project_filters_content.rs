use leptos::prelude::*;

use super::filter_checkbox_group::FilterCheckboxGroup;

#[component]
pub fn ProjectFiltersContent(items: Vec<(String, bool, String)>) -> impl IntoView {
    view! {
        <h4 class="dpe-title">"Filters"</h4>
        <div class="space-y-4">
            <FilterCheckboxGroup title="Project Status".to_string() items=items />
            <div class="border-t border-neutral-200"></div>
            <div class="dpe-subtitle">"Other filters TODO"</div>
        </div>
    }
}
