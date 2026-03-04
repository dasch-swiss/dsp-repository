use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconSearch};

use crate::domain::list_projects;

#[island]
pub fn ProjectSearchInput() -> impl IntoView {
    let (value, set_value) = signal(String::new());
    let (focused, set_focused) = signal(false);
    let show_dropdown = Memo::new(move |_| !value.get().is_empty() && focused.get());

    let results = Resource::new(
        move || value.get(),
        |search| async move {
            let search_opt = if search.is_empty() { None } else { Some(search) };
            list_projects(None, None, search_opt, None, Some(5)).await
        },
    );

    view! {
        <form method="get" action="/projects">
            <div class="relative flex-1 mr-2">
                <label class="input w-full">
                    <Icon icon=IconSearch class="w-4 h-4 opacity-50 shrink-0" />
                    <input
                        type="search"
                        placeholder="Search projects..."
                        class="grow"
                        prop:value=move || value.get()
                        on:input=move |ev| {
                            set_value.set(event_target_value(&ev));
                        }
                        on:focus=move |_| set_focused.set(true)
                        on:blur=move |_| set_focused.set(false)
                    />
                </label>

                <Show when=move || show_dropdown.get()>
                    <div
                        class="absolute top-full left-0 right-0 mt-1 bg-base-100 border border-base-300 rounded-box shadow-lg z-[100] p-2"
                        on:mousedown=move |ev| ev.prevent_default()
                    >
                        <Suspense fallback=move || {
                            view! { <p class="text-sm px-2 py-1">"Loading..."</p> }
                        }>
                            {move || {
                                let query = value.get();
                                results
                                    .get()
                                    .map(|res| match res {
                                        Ok(page) if page.items.is_empty() => {
                                            view! {
                                                <p class="text-sm text-base-content/60 px-2 py-1">
                                                    "No results"
                                                </p>
                                            }
                                                .into_any()
                                        }
                                        Ok(page) => {
                                            let total_items = page.total_items;
                                            let search_url = format!(
                                                "/projects?search={}",
                                                urlencoding::encode(&query),
                                            );
                                            view! {
                                                <ul>
                                                    {page
                                                        .items
                                                        .into_iter()
                                                        .map(|p| {
                                                            view! {
                                                                <li>
                                                                    <a
                                                                        href=format!("/projects/{}", p.shortcode)
                                                                        class="block px-4 py-3 hover:bg-base-200 transition-colors text-sm"
                                                                    >
                                                                        <div class="font-medium text-base-content">
                                                                            {p.name.clone()}
                                                                        </div>
                                                                        <div class="text-sm text-base-content/60 truncate mt-0.5">
                                                                            {p.short_description.clone()}
                                                                        </div>
                                                                    </a>
                                                                </li>
                                                            }
                                                        })
                                                        .collect_view()}
                                                </ul>
                                                <div class="border-t border-base-300 mt-1 pt-1">
                                                    <a
                                                        href=search_url
                                                        class="flex items-center gap-2 px-2 py-1 hover:bg-base-200 rounded text-sm text-base-content/70"
                                                    >
                                                        <Icon icon=IconSearch class="w-4 h-4" />
                                                        {format!("Search for \"{query}\" ({total_items} results)")}
                                                    </a>
                                                </div>
                                            }
                                                .into_any()
                                        }
                                        Err(_) => {
                                            view! {
                                                <p class="text-sm text-red-500 px-2 py-1">
                                                    "Error loading results"
                                                </p>
                                            }
                                                .into_any()
                                        }
                                    })
                            }}
                        </Suspense>
                    </div>
                </Show>
            </div>

        </form>
    }
}
