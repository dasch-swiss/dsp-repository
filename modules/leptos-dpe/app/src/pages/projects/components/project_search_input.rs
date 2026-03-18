use std::time::Duration;

use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, IconSearch};

use crate::domain::list_projects;

const DEBOUNCE_MS: u64 = 300;

#[island]
pub fn ProjectSearchInput() -> impl IntoView {
    let (value, set_value) = signal(String::new());
    let (debounced_value, set_debounced_value) = signal(String::new());
    let (focused, set_focused) = signal(false);
    let (selected_index, set_selected_index) = signal::<Option<usize>>(None);
    let show_dropdown = Memo::new(move |_| !debounced_value.get().is_empty() && focused.get());

    let results = Resource::new(
        move || debounced_value.get(),
        |search| async move {
            let search_opt = if search.is_empty() { None } else { Some(search) };
            list_projects(None, None, search_opt, None, Some(5), None, None, None).await
        },
    );

    view! {
        <form method="get" action="/projects">
            <div class="relative flex-1">
                <label class="input w-full">
                    <Icon icon=IconSearch class="w-4 h-4 opacity-50 shrink-0" />
                    <input
                        type="search"
                        placeholder="Search projects..."
                        class="grow"
                        prop:value=move || value.get()
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            set_value.set(v.clone());
                            set_selected_index.set(None);
                            set_timeout(
                                move || set_debounced_value.set(v),
                                Duration::from_millis(DEBOUNCE_MS),
                            );
                        }
                        on:focus=move |_| set_focused.set(true)
                        on:blur=move |_| set_focused.set(false)
                        on:keydown=move |ev| {
                            if !show_dropdown.get() {
                                return;
                            }
                            let key = ev.key();
                            let item_count = results
                                .get()
                                .and_then(|res| res.ok())
                                .map(|page| {
                                    if page.items.is_empty() { 0 } else { page.items.len() + 1 }
                                })
                                .unwrap_or(0);
                            if item_count == 0 {
                                return;
                            }
                            match key.as_str() {
                                "ArrowDown" => {
                                    ev.prevent_default();
                                    set_selected_index
                                        .update(|idx| {
                                            *idx = Some(
                                                match *idx {
                                                    None => 0,
                                                    Some(i) if i + 1 >= item_count => 0,
                                                    Some(i) => i + 1,
                                                },
                                            );
                                        });
                                }
                                "ArrowUp" => {
                                    ev.prevent_default();
                                    set_selected_index
                                        .update(|idx| {
                                            *idx = Some(
                                                match *idx {
                                                    None | Some(0) => item_count - 1,
                                                    Some(i) => i - 1,
                                                },
                                            );
                                        });
                                }
                                "Enter" => {
                                    if let Some(i) = selected_index.get() {
                                        ev.prevent_default();
                                        if let Some(Ok(page)) = results.get() {
                                            let url = if i < page.items.len() {
                                                format!("/projects/{}", page.items[i].shortcode)
                                            } else {
                                                let query = debounced_value.get();
                                                format!("/projects?search={}", urlencoding::encode(&query))
                                            };
                                            if let Some(window) = web_sys::window() {
                                                let _ = window.location().set_href(&url);
                                            }
                                        }
                                    }
                                }
                                "Escape" => {
                                    set_focused.set(false);
                                }
                                _ => {}
                            }
                        }
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
                                let query = debounced_value.get();
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
                                            let item_count = page.items.len();
                                            let search_url = format!(
                                                "/projects?search={}",
                                                urlencoding::encode(&query),
                                            );
                                            view! {
                                                <ul>
                                                    {page
                                                        .items
                                                        .into_iter()
                                                        .enumerate()
                                                        .map(|(i, p)| {
                                                            view! {
                                                                <li>
                                                                    <a
                                                                        href=format!("/projects/{}", p.shortcode)
                                                                        class=move || {
                                                                            if selected_index.get() == Some(i) {
                                                                                "block px-4 py-3 bg-base-200 transition-colors text-sm"
                                                                            } else {
                                                                                "block px-4 py-3 hover:bg-base-200 transition-colors text-sm"
                                                                            }
                                                                        }
                                                                    >
                                                                        <div class="font-medium text-base-content">{p.name}</div>
                                                                        <div class="text-sm text-base-content/60 truncate mt-0.5">
                                                                            {p.short_description}
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
                                                        class=move || {
                                                            if selected_index.get() == Some(item_count) {
                                                                "flex items-center gap-2 px-2 py-1 bg-base-200 rounded text-sm text-base-content/70"
                                                            } else {
                                                                "flex items-center gap-2 px-2 py-1 hover:bg-base-200 rounded text-sm text-base-content/70"
                                                            }
                                                        }
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
