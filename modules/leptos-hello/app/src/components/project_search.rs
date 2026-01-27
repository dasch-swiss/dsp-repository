use crate::components::ProjectFilters;
use crate::domain::ProjectQuery;
use leptos::prelude::*;



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

