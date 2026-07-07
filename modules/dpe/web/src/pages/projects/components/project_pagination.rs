use leptos::prelude::*;
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{Icon, IconChevronLeft, IconChevronRight};
use mosaic_tiles::link::Link;

use crate::domain::ProjectQuery;

#[component]
pub fn ProjectPagination(nr_pages: i32, query: ProjectQuery) -> impl IntoView {
    let current_page = query.page();

    // Pre-compute disabled states
    let is_first_page = current_page <= 1;
    let is_last_page = current_page >= nr_pages;

    // Helper closure to build page URLs
    let build_page_url = |page: i32| format!("/dpe/projects{}", query.clone().with_page(page).to_query_string());

    // Pre-compute navigation URLs
    let prev_url = build_page_url(current_page - 1);
    let next_url = build_page_url(current_page + 1);

    view! {
        <nav role="navigation" aria-label="Pagination">
            <div class="flex items-center justify-center gap-2">
                <Link href=prev_url as_button=ButtonVariant::Outline disabled=is_first_page>
                    <Icon icon=IconChevronLeft class="w-3 h-3" />
                </Link>
                {(1..=nr_pages)
                    .map(|page| {
                        let page_url = build_page_url(page);
                        let is_current = page == current_page;
                        let variant = if is_current {
                            ButtonVariant::Primary
                        } else {
                            ButtonVariant::Ghost
                        };
                        view! {
                            <Link href=page_url as_button=variant>
                                {page.to_string()}
                            </Link>
                        }
                    })
                    .collect_view()}
                <Link href=next_url as_button=ButtonVariant::Outline disabled=is_last_page>
                    <Icon icon=IconChevronRight class="w-3 h-3" />
                </Link>
            </div>
        </nav>
    }
}
