use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{icon, IconChevronLeft, IconChevronRight};
use mosaic_tiles::link::{link, LinkProps};

use crate::domain::ProjectQuery;

/// Page navigation for the projects list. Each control is a link carrying the
/// current filter query with `page` adjusted; prev/next are disabled at the
/// ends, the current page is highlighted.
pub fn project_pagination(nr_pages: i32, query: &ProjectQuery) -> Markup {
    let current_page = query.page();
    let is_first_page = current_page <= 1;
    let is_last_page = current_page >= nr_pages;

    let build_page_url = |page: i32| format!("/dpe/projects{}", query.clone().with_page(page).to_query_string());
    let prev_url = build_page_url(current_page - 1);
    let next_url = build_page_url(current_page + 1);

    html! {
        nav role="navigation" aria-label="Pagination" {
            div class="flex items-center justify-center gap-2" {
                (link(
                    LinkProps {
                        href: &prev_url,
                        as_button: Some(ButtonVariant::Outline),
                        disabled: is_first_page,
                        // Only the interactive (href-bearing) link carries the name; a
                        // disabled link is a roleless <a>, where aria-label is prohibited.
                        aria_label: (!is_first_page).then_some("Previous page"),
                        ..Default::default()
                    },
                    icon(IconChevronLeft, "w-3 h-3"),
                ))
                @for page in 1..=nr_pages {
                    @let page_url = build_page_url(page);
                    @let variant = if page == current_page { ButtonVariant::Primary } else { ButtonVariant::Ghost };
                    (link(
                        LinkProps { href: &page_url, as_button: Some(variant), ..Default::default() },
                        html! { (page) },
                    ))
                }
                (link(
                    LinkProps {
                        href: &next_url,
                        as_button: Some(ButtonVariant::Outline),
                        disabled: is_last_page,
                        aria_label: (!is_last_page).then_some("Next page"),
                        ..Default::default()
                    },
                    icon(IconChevronRight, "w-3 h-3"),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlights_current_page_and_links_others() {
        let query = ProjectQuery { page: Some(2), ..Default::default() };
        let out = project_pagination(3, &query).into_string();
        // Page 2 is current → primary; pages 1 and 3 are ghost links.
        assert!(out.contains("btn btn-primary"), "{out}");
        assert!(out.contains(r#"href="/dpe/projects?page=3""#), "{out}");
        // Page 1 has no `page=` param (omitted when 1).
        assert!(out.contains(r#"href="/dpe/projects""#), "{out}");
        assert!(out.contains(r#"aria-label="Pagination""#), "{out}");
    }

    #[test]
    fn disables_prev_on_first_page() {
        let query = ProjectQuery { page: Some(1), ..Default::default() };
        let out = project_pagination(3, &query).into_string();
        assert!(out.contains(r#"aria-disabled="true""#), "first page disables prev: {out}");
    }

    #[test]
    fn disables_next_on_last_page() {
        let query = ProjectQuery { page: Some(3), ..Default::default() };
        let out = project_pagination(3, &query).into_string();
        // Next is disabled (aria-disabled) on the last page.
        assert!(out.contains(r#"aria-disabled="true""#), "last page disables next: {out}");
    }
}
