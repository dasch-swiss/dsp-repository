use maud::{html, Markup};
use shared_datastar as datastar;
use shared_metadata::ACCESS_RIGHTS_VALUES;

use super::filter_checkbox_group::{filter_checkbox_group, FilterOption};
use crate::domain::{list_data_languages, list_type_of_data, ProjectQuery};
use crate::RenderContext;

/// The `id` of the facet form; the mobile trigger and Close submit it through `form=`.
pub const FILTER_FORM_ID: &str = "project-filters";

/// On a failed results request, submit the form natively (`submit()` skips the `submit`
/// listener), so the visitor gets the full page instead of a dead control. Datastar fires
/// `datastar-fetch` on `document` for every fetch on the page, hence the `el` guard.
const FETCH_FALLBACK: &str =
    "evt.detail.el === el && (evt.detail.type === 'error' || evt.detail.type === 'retries-failed') && el.submit()";

/// The facet form: a "Filters" heading with the "Clear all" slot, the access-rights,
/// status, type-of-data and data-language groups, the applied search as a hidden field and
/// an Apply button. It is a GET form on `/dpe/projects`, so it works without JavaScript
/// and gives a crawler no link per option. It has no `page` field, so a change starts at
/// page 1. `autocomplete="off"` keeps a Back navigation from restoring stale boxes.
///
/// With JavaScript, every change (and a submit) fetches `/dpe/projects/results` with the
/// form's fields; the server patches the results and writes the address bar. The Apply
/// button is hidden then.
pub fn project_filters_content(query: &ProjectQuery, ctx: &RenderContext) -> Markup {
    let results = datastar::get("/dpe/projects/results").form().retry_never();
    let access_rights = query.access_rights();
    let access_rights_options: Vec<FilterOption> = ACCESS_RIGHTS_VALUES
        .iter()
        .map(|&v| FilterOption {
            label: v.to_string(),
            value: v.to_string(),
            checked: access_rights.iter().any(|a| a == v),
        })
        .collect();
    let ongoing = [FilterOption {
        label: "Ongoing".to_string(),
        value: "true".to_string(),
        checked: query.ongoing(),
    }];
    let finished = [FilterOption {
        label: "Finished".to_string(),
        value: "true".to_string(),
        checked: query.finished(),
    }];
    let type_of_data = query.type_of_data();
    let type_of_data_options: Vec<FilterOption> = list_type_of_data(ctx.corpus)
        .into_iter()
        .map(|t| FilterOption {
            checked: type_of_data.contains(&t),
            label: t.clone(),
            value: t,
        })
        .collect();
    let data_language = query.data_language();
    let data_language_options: Vec<FilterOption> = list_data_languages(ctx.corpus)
        .into_iter()
        .map(|(code, display)| FilterOption {
            checked: data_language.contains(&code),
            label: display,
            value: code,
        })
        .collect();
    let search = query.search();

    html! {
        form
            id=(FILTER_FORM_ID)
            method="get"
            action="/dpe/projects"
            autocomplete="off"
            class="space-y-4"
            data-on:change=(results)
            data-on:submit__prevent=(results)
            data-on:datastar-fetch=(FETCH_FALLBACK)
        {
            div class="flex items-center justify-between pr-10 lg:pr-0" {
                h4 id="project-filters-heading" class="dpe-title" { "Filters" }
                (filters_clear_slot(query))
            }
            ({
                filter_checkbox_group(
                    "Access Rights",
                    "access_rights",
                    &access_rights_options,
                    Some("https://dasch.swiss/knowledge-hub/fundamentals-access-rights"),
                    Some(
                        "Access rights define how openly the data can be accessed. Learn more here.",
                    ),
                )
            })
            div class="border-t border-neutral-200" {}
            fieldset {
                legend class="dpe-subtitle mb-2" { "Project Status" }
                (status_rows(&ongoing, "ongoing"))
                (status_rows(&finished, "finished"))
            }
            div class="border-t border-neutral-200" {}
            ({
                filter_checkbox_group(
                    "Type of Data",
                    "type_of_data",
                    &type_of_data_options,
                    None,
                    None,
                )
            })
            div class="border-t border-neutral-200" {}
            ({
                filter_checkbox_group(
                    "Data Language",
                    "data_language",
                    &data_language_options,
                    None,
                    None,
                )
            })
            @if !search.is_empty() {
                input type="hidden" name="search" value=(search);
            }
            button type="submit" class="btn btn-primary w-full" data-show="false" {
                "Apply filters"
            }
        }
    }
}

/// The status rows: unlike the other groups, each status is its own parameter.
fn status_rows(options: &[FilterOption], name: &str) -> Markup {
    html! {
        @for option in options {
            @let id = format!("filter-{name}");
            label for=(id) class="flex items-center gap-2 min-h-6 cursor-pointer" {
                input
                    id=(id)
                    type="checkbox"
                    name=(name)
                    value=(option.value)
                    checked[option.checked]
                    class="w-4 h-4 shrink-0";
                span { (option.label) }
            }
        }
    }
}

/// The "Clear all" slot: always rendered, so a patch can fill or empty it; it holds the
/// link only when a facet or status is set.
pub fn filters_clear_slot(query: &ProjectQuery) -> Markup {
    let any_facet = query.ongoing()
        || query.finished()
        || query.type_of_data.is_some()
        || query.data_language.is_some()
        || query.access_rights.is_some();
    html! {
        span id="project-filters-clear" {
            @if any_facet {
                a href="/dpe/projects" class="text-xs text-primary hover:underline" { "Clear all" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_render_context;

    fn render(raw: &str) -> String {
        project_filters_content(&ProjectQuery::from_query_str(raw), &test_render_context(false)).into_string()
    }

    #[test]
    fn is_a_get_form_on_the_projects_page() {
        let out = render("");
        assert!(
            out.contains(r#"<form id="project-filters" method="get" action="/dpe/projects" autocomplete="off""#),
            "{out}"
        );
        assert_eq!(out.matches("<fieldset").count(), 4, "{out}");
        assert_eq!(out.matches("<legend").count(), 4, "{out}");
        assert!(
            out.contains(
                r#"<button type="submit" class="btn btn-primary w-full" data-show="false">Apply filters</button>"#
            ),
            "{out}"
        );
    }

    #[test]
    fn fetches_the_results_fragment_on_change_and_submit() {
        let out = render("");
        let action = "@get('/dpe/projects/results', {contentType: 'form', retry: 'never', retryMaxCount: 0})";
        assert!(out.contains(&format!(r#"data-on:change="{action}""#)), "{out}");
        assert!(out.contains(&format!(r#"data-on:submit__prevent="{action}""#)), "{out}");
    }

    #[test]
    fn falls_back_to_a_native_submit_only_for_its_own_failed_fetch() {
        let out = render("");
        assert!(
            out.contains(r#"data-on:datastar-fetch="evt.detail.el === el &amp;&amp; (evt.detail.type === 'error' || evt.detail.type === 'retries-failed') &amp;&amp; el.submit()""#),
            "{out}"
        );
    }

    #[test]
    fn holds_no_link_to_a_filtered_url_and_no_page_field() {
        let out = render("type_of_data=Text&ongoing=true&page=3&search=x");
        assert!(!out.contains(r#"href="/dpe/projects?"#), "{out}");
        assert!(!out.contains(r#"name="page""#), "{out}");
    }

    #[test]
    fn status_checkboxes_are_their_own_parameters() {
        let out = render("ongoing=true");
        assert!(
            out.contains(r#"id="filter-ongoing" type="checkbox" name="ongoing" value="true" checked"#),
            "{out}"
        );
        assert!(
            out.contains(r#"id="filter-finished" type="checkbox" name="finished" value="true" class"#),
            "{out}"
        );
    }

    #[test]
    fn data_language_values_are_codes_and_labels_are_display_names() {
        let ctx = test_render_context(false);
        let (code, display) = list_data_languages(ctx.corpus)
            .into_iter()
            .next()
            .expect("the corpus has languages");
        let out = project_filters_content(&ProjectQuery::from_query_str(&format!("data_language={code}")), &ctx)
            .into_string();
        assert!(
            out.contains(&format!(r#"name="data_language" value="{code}" checked"#)),
            "{out}"
        );
        assert!(out.contains(&format!("<span>{display}</span>")), "{out}");
    }

    #[test]
    fn checks_the_access_rights_set_in_the_query() {
        let value = ACCESS_RIGHTS_VALUES[0];
        let out = render(&format!("access_rights={}", urlencoding::encode(value)));
        assert!(
            out.contains(&format!(r#"name="access_rights" value="{value}" checked"#)),
            "{out}"
        );
    }

    #[test]
    fn carries_the_applied_search_only_when_set() {
        assert!(!render("").contains(r#"name="search""#));
        assert!(render("search=a+b").contains(r#"<input type="hidden" name="search" value="a b">"#));
    }

    #[test]
    fn keeps_the_access_rights_info_link() {
        let out = render("");
        assert!(out.contains(r#"aria-label="More information""#), "{out}");
        assert!(out.contains(r#"target="_blank""#), "{out}");
    }

    #[test]
    fn clear_slot_is_empty_without_a_facet() {
        assert_eq!(
            filters_clear_slot(&ProjectQuery::default()).into_string(),
            r#"<span id="project-filters-clear"></span>"#
        );
        assert_eq!(
            filters_clear_slot(&ProjectQuery::from_query_str("search=x&page=2")).into_string(),
            r#"<span id="project-filters-clear"></span>"#
        );
    }

    #[test]
    fn clear_slot_holds_one_clear_all_link_with_a_facet() {
        for raw in [
            "ongoing=true",
            "finished=true",
            "type_of_data=T",
            "data_language=en",
            "access_rights=A",
        ] {
            let out = filters_clear_slot(&ProjectQuery::from_query_str(raw)).into_string();
            assert_eq!(out.matches("<a ").count(), 1, "{raw}: {out}");
            assert!(out.contains(r#"href="/dpe/projects""#), "{raw}: {out}");
            assert!(out.contains(">Clear all</a>"), "{raw}: {out}");
        }
    }
}
