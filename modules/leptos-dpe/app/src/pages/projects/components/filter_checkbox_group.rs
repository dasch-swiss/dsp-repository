use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, Info};

#[component]
pub fn FilterCheckboxGroup(
    title: String,
    items: Vec<(String, bool, String)>,
    #[prop(optional, into)] info_href: Option<String>,
    #[prop(optional, into)] info_tooltip: Option<String>,
) -> impl IntoView {
    view! {
        <div>
            <div class="flex items-center justify-between mb-2">
                <span class="dpe-subtitle">{title}</span>
                {info_href
                    .zip(info_tooltip)
                    .map(|(href, tooltip)| {
                        view! {
                            <div class="group relative">
                                <a
                                    href=href
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="text-gray-400 hover:text-primary transition-colors"
                                    aria-label="More information"
                                >
                                    <Icon icon=Info class="w-4 h-4" />
                                </a>
                                <div class="invisible group-hover:visible absolute right-0 top-full mt-1 w-64 p-2 bg-gray-900 text-white text-xs rounded-lg shadow-lg z-10 pointer-events-none">
                                    {tooltip}
                                </div>
                            </div>
                        }
                    })}
            </div>
            <div class="space-y-2">
                {items
                    .into_iter()
                    .map(|(label, checked, href)| {
                        view! {
                            <a
                                href=href
                                class="flex items-center gap-2 cursor-pointer"
                                aria-current=if checked { Some("true") } else { None }
                            >
                                <input
                                    type="checkbox"
                                    // pointer-events-none makes the link handle the click
                                    class="w-4 h-4 pointer-events-none"
                                    checked=if checked { Some("") } else { None }
                                />
                                <span>{label}</span>
                            </a>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}
