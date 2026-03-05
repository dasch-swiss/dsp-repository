use leptos::prelude::*;

#[component]
pub fn FilterCheckboxGroup(title: &'static str, items: Vec<(String, bool, String)>) -> impl IntoView {
    view! {
        <div>
            <h5 class="dpe-subtitle">{title}</h5>
            {items
                .into_iter()
                .map(|(label, checked, href)| {
                    view! {
                        <a
                            href=href
                            class="filter-option"
                            aria-current=if checked { Some("true") } else { None }
                        >
                            <span class=if checked {
                                "filter-indicator filter-indicator-checked"
                            } else {
                                "filter-indicator"
                            } />
                            <span class="text-sm">{label}</span>
                        </a>
                    }
                })
                .collect_view()}
        </div>
    }
}
