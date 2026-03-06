use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconChevronDown, Info};

#[island]
pub fn FilterCheckboxGroup(
    title: String,
    items: Vec<(String, bool, String)>,
    #[prop(optional, into)] info_href: Option<String>,
    #[prop(optional, into)] info_tooltip: Option<String>,
) -> impl IntoView {
    let (open, set_open) = signal(true);

    view! {
        <div>
            <div class="flex items-center justify-between mb-2">
                <button
                    class="flex-1 flex items-center justify-between text-sm font-medium text-gray-700 hover:text-gray-900 cursor-pointer"
                    on:click=move |_| set_open.update(|v| *v = !*v)
                >
                    <span>{title}</span>
                    <div class="flex items-center gap-2">
                        {info_href.zip(info_tooltip).map(|(href, tooltip)| {
                            view! {
                                <div class="group relative">
                                    <a
                                        href=href
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        class="text-gray-400 hover:text-primary transition-colors"
                                        aria-label="More information"
                                        on:click=|e| e.stop_propagation()
                                    >
                                        <Icon icon=Info class="w-4 h-4" />
                                    </a>
                                    <div class="invisible group-hover:visible absolute right-0 top-full mt-1 w-64 p-2 bg-gray-900 text-white text-xs rounded-lg shadow-lg z-10 pointer-events-none">
                                        {tooltip}
                                    </div>
                                </div>
                            }
                        })}
                        <span
                            style=move || if open.get() { "" } else { "transform: rotate(-90deg)" }
                            class="transition-transform inline-flex"
                        >
                            <Icon icon=IconChevronDown class="w-4 h-4" />
                        </span>
                    </div>
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
