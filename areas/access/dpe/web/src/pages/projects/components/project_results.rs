use maud::{html, Markup};

use super::project_list::project_list;
use crate::domain::projects::filter_and_paginate;
use crate::domain::ProjectQuery;
use crate::RenderContext;

/// The results region the facet form's fragment replaces: the project list and its
/// pagination. The page and `/dpe/projects/results` render it with this one function, so
/// the two cannot drift.
pub fn project_results(query: &ProjectQuery, ctx: &RenderContext) -> Markup {
    html! {
        div id="project-results" class="flex flex-col gap-2" { (project_list(query, ctx)) }
    }
}

/// The result count, a polite live region outside the results region. A fragment patches
/// its text (`results_count_text`), never the node, so a screen reader announces the new
/// count.
pub fn project_results_count(query: &ProjectQuery, ctx: &RenderContext) -> Markup {
    html! {
        p   id="project-results-count"
            role="status"
            aria-live="polite"
            class="text-sm text-gray-600"
        { (results_count_text(query, ctx)) }
    }
}

/// "N projects" for the query, whatever its page.
pub fn results_count_text(query: &ProjectQuery, ctx: &RenderContext) -> String {
    let total = filter_and_paginate(ctx.corpus.all_projects(), query, None).total_items;
    format!("{total} projects")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_render_context;

    #[test]
    fn results_region_wraps_the_list() {
        let out = project_results(&ProjectQuery::default(), &test_render_context(false)).into_string();
        assert!(out.starts_with(r#"<div id="project-results""#), "{out}");
        assert!(
            !out.contains("project-results-count"),
            "the count lives outside the region: {out}"
        );
    }

    #[test]
    fn count_is_a_polite_status_region() {
        let ctx = test_render_context(false);
        let out = project_results_count(&ProjectQuery::default(), &ctx).into_string();
        let total = ctx.corpus.all_projects().len();
        assert_eq!(
            out,
            format!(
                r#"<p id="project-results-count" role="status" aria-live="polite" class="text-sm text-gray-600">{total} projects</p>"#
            )
        );
    }

    #[test]
    fn count_ignores_the_page() {
        let ctx = test_render_context(false);
        let q = ProjectQuery { page: Some(3), ..Default::default() };
        assert_eq!(results_count_text(&q, &ctx), results_count_text(&ProjectQuery::default(), &ctx));
    }
}
