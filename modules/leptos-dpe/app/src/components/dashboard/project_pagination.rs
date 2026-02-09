use leptos::prelude::*;
use crate::domain::ProjectQuery;

#[component]
pub fn ProjectPagination(nr_pages: i32, total_items: i32, query: ProjectQuery) -> impl IntoView {
    let current_page = query.page();

    // Pre-compute disabled states
    let is_first_page = current_page <= 1;
    let is_last_page = current_page >= nr_pages;

    // Helper closure to build page URLs
    let build_page_url = |page: i32| {
        format!("/projects{}", query.clone().with_page(page).to_query_string())
    };

    // Pre-compute navigation URLs
    let prev_url = if is_first_page {
        "#".to_string()
    } else {
        build_page_url(current_page - 1)
    };
    let next_url = if is_last_page {
        "#".to_string()
    } else {
        build_page_url(current_page + 1)
    };

    // Helper function to render navigation buttons (prev/next)
    let nav_button = |url: String, label: String, symbol: String, is_disabled: bool| {
        view! {
            <a
                href=url
                class="btn"
                class:btn-disabled=is_disabled
                class:pointer-events-none=is_disabled
                aria-label=label
                aria-disabled=is_disabled.to_string()
            >
                {symbol}
            </a>
        }
    };

    view! {
        <nav role="navigation" aria-label="Pagination">
            <div class="flex items-center gap-4">
                    <div class="btn-group">
                    {nav_button(prev_url, "Previous page".to_string(), "«".to_string(), is_first_page)}
                    {(1..=nr_pages)
                        .map(|page| {
                            let page_url = build_page_url(page);
                            let is_current = page == current_page;
                            view! {
                                <a
                                    href=page_url
                                    class="btn"
                                    class:btn-primary=is_current
                                    class:font-bold=is_current
                                    aria-label=format!("Page {}", page)
                                    aria-current=if is_current { "page" } else { "" }
                                >
                                    {page.to_string()}
                                </a>
                            }
                        })
                        .collect_view()}
                    {nav_button(next_url, "Next page".to_string(), "»".to_string(), is_last_page)}
                </div>
                <span class="text-sm text-gray-600">{format!("Total: {} items", total_items)}</span>
                </div>
        </nav>
    }
}