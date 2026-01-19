use crate::domain::ProjectQuery;
use leptos::prelude::*;

#[cfg(feature = "hydrate")]
use gloo_history::{BrowserHistory, History};

#[component]
pub fn ProjectSearch(nr_pages: i32, total_items: i32, query: ProjectQuery) -> impl IntoView {
    let current_page = query.page();
    let ongoing = query.ongoing();
    let finished = query.finished();
    let search = query.search();
    view! {
        <div class="flex flex-col gap-4">
            // Filters island - programmatic navigation
            <ProjectFilters ongoing=ongoing finished=finished search=search.clone() />

            // Pagination - plain HTML links (MPA style)
            <div class="flex items-center gap-4">
                <div class="btn-group">
                    {if current_page > 1 {
                        let prev_query = ProjectQuery {
                            page: Some(current_page - 1),
                            ongoing: query.ongoing,
                            finished: query.finished,
                            search: query.search.clone(),
                        };
                        let prev_url = format!("/projects{}", prev_query.to_query_string());
                        view! {
                            <a href=prev_url class="btn">
                                "«"
                            </a>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button class="btn" disabled>
                                "«"
                            </button>
                        }
                            .into_any()
                    }}
                    {(1..=nr_pages)
                        .map(|page| {
                            let page_query = ProjectQuery {
                                page: Some(page),
                                ongoing: query.ongoing,
                                finished: query.finished,
                                search: query.search.clone(),
                            };
                            let page_url = format!("/projects{}", page_query.to_query_string());
                            let btn_class = if page == current_page {
                                "btn btn-primary font-bold"
                            } else {
                                "btn"
                            };
                            view! {
                                <a href=page_url class=btn_class>
                                    {page.to_string()}
                                </a>
                            }
                        })
                        .collect_view()}
                    {if current_page < nr_pages {
                        let next_query = ProjectQuery {
                            page: Some(current_page + 1),
                            ongoing: query.ongoing,
                            finished: query.finished,
                            search: query.search.clone(),
                        };
                        let next_url = format!("/projects{}", next_query.to_query_string());
                        view! {
                            <a href=next_url class="btn">
                                "»"
                            </a>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button class="btn" disabled>
                                "»"
                            </button>
                        }
                            .into_any()
                    }}
                </div>
                <span class="text-sm text-gray-600">{format!("Total: {} items", total_items)}</span>
            </div>
        </div>
    }
}

// Island for filters and search - uses programmatic navigation
#[island]
fn ProjectFilters(ongoing: bool, finished: bool, search: String) -> impl IntoView {
    // Local state
    let (ongoing_checked, set_ongoing_checked) = signal(ongoing);
    let (finished_checked, set_finished_checked) = signal(finished);
    let (search_value, set_search_value) = signal(search);

    // Focus the search input on mount
    let search_input_ref = NodeRef::<leptos::html::Input>::new();
    Effect::new(move || {
        if let Some(input) = search_input_ref.get() {
            let _ = input.focus();
        }
    });

    // Navigate function
    let navigate = move |ongoing: bool, finished: bool, search: String| {
        #[cfg(feature = "hydrate")]
        {
            let query = ProjectQuery {
                page: Some(1),
                ongoing: Some(ongoing),
                finished: Some(finished),
                search: if search.is_empty() {
                    None
                } else {
                    Some(search)
                },
            };
            let url = format!("/projects{}", query.to_query_string());
            let history = BrowserHistory::new();
            history.push(&url);

            let window = web_sys::window().unwrap();
            let _ = window.location().set_href(&url);
        }

        #[cfg(not(feature = "hydrate"))]
        {
            let _ = (ongoing, finished, search);
        }
    };

    view! {
        <div class="flex flex-col gap-4">
            // Status filter checkboxes
            <div class="flex gap-4 items-center">
                <span class="font-semibold">"Filter by Status:"</span>
                <label class="flex items-center gap-2 cursor-pointer">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-primary"
                        prop:checked=move || ongoing_checked.get()
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_ongoing_checked.set(checked);
                            navigate(
                                checked,
                                finished_checked.get_untracked(),
                                search_value.get_untracked(),
                            );
                        }
                    />
                    <span>"Ongoing"</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-primary"
                        prop:checked=move || finished_checked.get()
                        on:change=move |ev| {
                            let checked = event_target_checked(&ev);
                            set_finished_checked.set(checked);
                            navigate(
                                ongoing_checked.get_untracked(),
                                checked,
                                search_value.get_untracked(),
                            );
                        }
                    />
                    <span>"Finished"</span>
                </label>
            </div>

            // Search form - uses form submission instead of instant navigation
            <form
                class="flex gap-4 items-center"
                on:submit=move |ev| {
                    ev.prevent_default();
                    navigate(
                        ongoing_checked.get_untracked(),
                        finished_checked.get_untracked(),
                        search_value.get_untracked(),
                    );
                }
            >
                <span class="font-semibold">"Search:"</span>
                <input
                    type="text"
                    class="input input-bordered input-primary w-full max-w-xs"
                    placeholder="Search projects..."
                    node_ref=search_input_ref
                    prop:value=move || search_value.get()
                    on:input=move |ev| {
                        let value = event_target_value(&ev);
                        set_search_value.set(value);
                    }
                />
                <button
                    type="submit"
                    class="btn btn-primary btn-sm"
                    disabled=move || search_value.get().trim().is_empty()
                >
                    "Search"
                </button>
            </form>

            // Reset link
            <div>
                <a href="/projects" class="btn btn-primary btn-sm">
                    "Reset Search and Filter"
                </a>
            </div>
        </div>
    }
}
