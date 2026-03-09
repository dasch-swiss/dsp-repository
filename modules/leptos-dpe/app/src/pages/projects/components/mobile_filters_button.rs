use leptos::prelude::*;
use mosaic_tiles::icon::{Icon, Tune};

use super::project_filters_content::ProjectFiltersContent;
use crate::domain::ProjectQuery;

fn parse_query_from_window() -> ProjectQuery {
    #[cfg(feature = "hydrate")]
    {
        if let Some(window) = web_sys::window() {
            let search = window.location().search().unwrap_or_default();
            let search = search.trim_start_matches('?').to_string();
            let mut ongoing: Option<bool> = None;
            let mut finished: Option<bool> = None;
            let mut query_search: Option<String> = None;
            let mut page: Option<i32> = None;
            for pair in search.split('&') {
                let mut kv = pair.splitn(2, '=');
                let key = kv.next().unwrap_or("");
                let val = kv.next().unwrap_or("");
                match key {
                    "ongoing" => ongoing = Some(val != "false"),
                    "finished" => finished = Some(val != "false"),
                    "search" => query_search = Some(val.replace('+', " ")),
                    "page" => page = val.parse().ok(),
                    _ => {}
                }
            }
            return ProjectQuery { ongoing, finished, search: query_search, page, type_of_data: None, data_language: None, access_rights: None };
        }
    }
    ProjectQuery::default()
}

#[island]
pub fn MobileFiltersButton() -> impl IntoView {
    let current_query = parse_query_from_window();
    let ongoing = current_query.ongoing();
    let finished = current_query.finished();
    let search = current_query.search();

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
            type_of_data: None,
            data_language: None,
            access_rights: None,
        };
        format!("/projects{}", new_query.to_query_string())
    };

    let filter_items: Vec<(String, bool, String)> =
        [("ongoing", "Ongoing", ongoing), ("finished", "Finished", finished)]
            .iter()
            .map(|(param, label, checked)| (label.to_string(), *checked, build_url(param)))
            .collect();

    let open = RwSignal::new(false);

    view! {
        <button
            class="btn btn-outline flex items-center gap-2 cursor-pointer"
            on:click=move |_| open.set(true)
        >
            <Icon icon=Tune class="w-5 h-5" />
            <span class="text-sm font-medium">"Filters"</span>
        </button>

        <Show when=move || open.get()>
            // Backdrop
            <div
                class="fixed inset-0 bg-black/40 z-40 lg:hidden"
                on:click=move |_| open.set(false)
            ></div>
            // Panel
            <div class="fixed right-0 top-0 bottom-0 w-full md:w-96 bg-white z-50 overflow-y-auto lg:hidden">
                <div class="relative p-4">
                    <button
                        class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2 cursor-pointer"
                        on:click=move |_| open.set(false)
                    >
                        "✕"
                    </button>
                    <ProjectFiltersContent
                        status_items=filter_items.clone()
                        type_of_data_items=vec![]
                        data_language_items=vec![]
                        access_rights_items=vec![]
                        clear_href={
                            let q = ProjectQuery {
                                ongoing: None,
                                finished: None,
                                search: if search.is_empty() { None } else { Some(search.clone()) },
                                page: Some(1),
                                type_of_data: None,
                                data_language: None,
                                access_rights: None,
                            };
                            format!("/projects{}", q.to_query_string())
                        }
                    />
                </div>
            </div>
        </Show>
    }
}
