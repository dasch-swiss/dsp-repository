use leptos::prelude::*;
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectPagination(nr_pages: i32, total_items: i32, query: ProjectQuery) -> impl IntoView {
    let current_page = query.page();

    // Helper closure to build page URLs
    let build_page_url = |page: i32| {
        format!("/projects{}", query.clone().with_page(page).to_query_string())
    };

    view! {
        <div class="flex items-center gap-4">
                <div class="btn-group">
                    {if current_page > 1 {
                        let prev_url = build_page_url(current_page - 1);
                        view! {
                            <a href=prev_url class="btn" aria-label="Previous page">
                                "«"
                            </a>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button class="btn" disabled aria-label="Previous page">
                                "«"
                            </button>
                        }
                            .into_any()
                    }}
                    {(1..=nr_pages)
                        .map(|page| {
                            let page_url = build_page_url(page);
                            let is_current = page == current_page;
                            let btn_class = if is_current {
                                "btn btn-primary font-bold"
                            } else {
                                "btn"
                            };
                            view! {
                                <a
                                    href=page_url
                                    class=btn_class
                                    aria-label=format!("Page {}", page)
                                    aria-current=if is_current { "page" } else { "" }
                                >
                                    {page.to_string()}
                                </a>
                            }
                        })
                        .collect_view()}
                    {if current_page < nr_pages {
                        let next_url = build_page_url(current_page + 1);
                        view! {
                            <a href=next_url class="btn" aria-label="Next page">
                                "»"
                            </a>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button class="btn" disabled aria-label="Next page">
                                "»"
                            </button>
                        }
                            .into_any()
                    }}
                </div>
                <span class="text-sm text-gray-600">{format!("Total: {} items", total_items)}</span>
            </div>
    }
}