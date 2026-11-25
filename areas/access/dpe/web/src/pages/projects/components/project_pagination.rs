use maud::{html, Markup};
use mosaic_tiles::button::ButtonVariant;
use mosaic_tiles::icon::{icon, IconChevronLeft, IconChevronRight};
use mosaic_tiles::link::{link, LinkBuilder};
use mosaic_tiles::ComponentBuilder;

use crate::domain::ProjectQuery;

/// A prev/next arrow: an outline-button link. The enabled arrow is an `<a href>`
/// (implicit `link` role) carrying an `aria-label` for its icon-only glyph. The
/// disabled arrow drops the `href` and is `tabindex="-1"` + `aria-disabled`, so
/// it is not a link and gets no `aria-label` — axe `aria-prohibited-attr` forbids
/// `aria-label` on an href-less `<a>`, and a non-focusable control needs no name.
fn nav_arrow(url: &str, glyph: Markup, disabled: bool, label: &str, rel: Option<&str>) -> Markup {
    let arrow = link(glyph, url).as_button(ButtonVariant::Outline);
    if disabled {
        arrow.disabled().build()
    } else {
        with_rel(arrow, rel).aria_label(label).build()
    }
}

fn with_rel(link: LinkBuilder, rel: Option<&str>) -> LinkBuilder {
    match rel {
        Some(rel) => link.rel(rel),
        None => link,
    }
}

/// Page navigation for the projects list. Each control is a link carrying the
/// current filter query with `page` adjusted; prev/next are disabled at the
/// ends, the current page is highlighted. On a filtered view every link is
/// `rel="nofollow"`: only an unfiltered page's links are for crawlers.
pub fn project_pagination(nr_pages: i32, query: &ProjectQuery) -> Markup {
    let rel = query.has_filter_params().then_some("nofollow");
    let current_page = query.page();
    let is_first_page = current_page <= 1;
    let is_last_page = current_page >= nr_pages;

    let build_page_url = |page: i32| format!("/dpe/projects{}", query.clone().with_page(page).to_query_string());
    let prev_url = build_page_url(current_page - 1);
    let next_url = build_page_url(current_page + 1);

    html! {
        nav role="navigation" aria-label="Pagination" {
            div class="flex items-center justify-center gap-2" {
                ({
                    nav_arrow(
                        &prev_url,
                        icon(IconChevronLeft, "w-3 h-3"),
                        is_first_page,
                        "Previous page",
                        rel,
                    )
                })
                @for page in 1..=nr_pages {
                    @let page_url = build_page_url(page);
                    @let is_current = page == current_page;
                    @let variant = if is_current {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Ghost
                    };
                    @let page_label = html! {
                        (page)
                    };
                    @let page_link = with_rel(
                        link(page_label, page_url.as_str()).as_button(variant),
                        rel,
                    );
                    @if is_current { (page_link.aria_current("page")) } @else { (page_link) }
                }
                ({
                    nav_arrow(
                        &next_url,
                        icon(IconChevronRight, "w-3 h-3"),
                        is_last_page,
                        "Next page",
                        rel,
                    )
                })
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
        assert!(
            out.contains(r#"aria-current="page""#),
            "current page missing aria-current: {out}"
        );
    }

    #[test]
    fn disables_prev_on_first_page() {
        let query = ProjectQuery { page: Some(1), ..Default::default() };
        let out = project_pagination(3, &query).into_string();
        assert!(out.contains(r#"aria-disabled="true""#), "first page disables prev: {out}");
        // The href-less disabled arrow must NOT carry aria-label (axe
        // aria-prohibited-attr forbids it on an <a> with no link role).
        assert!(
            !out.contains(r#"aria-label="Previous page""#),
            "disabled prev must not set aria-label: {out}"
        );
    }

    #[test]
    fn disables_next_on_last_page() {
        let query = ProjectQuery { page: Some(3), ..Default::default() };
        let out = project_pagination(3, &query).into_string();
        assert!(out.contains(r#"aria-disabled="true""#), "last page disables next: {out}");
        assert!(
            !out.contains(r#"aria-label="Next page""#),
            "disabled next must not set aria-label: {out}"
        );
    }

    #[test]
    fn unfiltered_links_carry_no_rel() {
        let query = ProjectQuery { page: Some(2), ..Default::default() };
        let out = project_pagination(3, &query).into_string();
        assert!(!out.contains("rel="), "{out}");
    }

    #[test]
    fn filtered_links_keep_the_filter_and_are_nofollow() {
        for raw in ["type_of_data=Text&page=2", "search=x&page=2", "dialog=true&page=2"] {
            let query = ProjectQuery::from_query_str(raw);
            let out = project_pagination(3, &query).into_string();
            assert_eq!(
                out.matches("<a ").count(),
                out.matches(r#"rel="nofollow""#).count(),
                "{raw}: {out}"
            );
            assert!(out.matches(r#"rel="nofollow""#).count() >= 4, "{raw}: {out}");
        }
        let out = project_pagination(3, &ProjectQuery::from_query_str("type_of_data=Text&page=2")).into_string();
        assert!(out.contains(r#"href="/dpe/projects?page=3&amp;type_of_data=Text""#), "{out}");
    }
}
