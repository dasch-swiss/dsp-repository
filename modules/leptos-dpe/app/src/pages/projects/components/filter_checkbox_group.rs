use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconChevronDown};

#[island]
pub fn FilterCheckboxGroup(title: String, items: Vec<(String, bool, String)>) -> impl IntoView {
    let (open, set_open) = signal(true);

    view! {
        <div>
            <div class="flex items-center justify-between mb-2">
                <button
                    class="flex-1 flex items-center justify-between text-sm font-medium text-gray-700 hover:text-gray-900 cursor-pointer"
                    on:click=move |_| set_open.update(|v| *v = !*v)
                >
                    <span>{title}</span>
                    <span
                        style=move || if open.get() { "" } else { "transform: rotate(-90deg)" }
                        class="transition-transform inline-flex"
                    >
                        <Icon icon=IconChevronDown class="w-4 h-4" />
                    </span>
                </button>
            </div>
            <Show when=move || {
                open.get()
            }>
                {items
                    .clone()
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
            </Show>
        </div>
    }
}
