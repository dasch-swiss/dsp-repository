//! The project list and the per-project page.
//!
//! The per-project page is still a placeholder for the editing surface, which is
//! the form's work. What is real here is the **scoping** — the list shows a
//! depositor exactly the projects assigned to them (REQ-1.2), and the
//! per-project page is reached only through the check that answers REQ-1.3's 403
//! otherwise — and, now that the editor reads the published set, the projects'
//! actual names.
//!
//! ## A row can be missing in two directions, and both are ordinary
//!
//! An assignment naming no published project is skipped rather than rendered as
//! a nameless row: a project assigned before it is published, and REQ-2.3's
//! project that exists only locally, are both real states, and a blank row would
//! read as data loss. What must not happen is a depositor with assignments
//! seeing an empty page with no explanation, so the two empty states say
//! different things — which is also which person the reader should go to.

use editor_core::published::ProjectSummary;
use maud::{html, Markup};
use mosaic_tiles::table::{table, table_cell, table_head_cell};

/// `GET /projects` for a depositor: the projects assigned to them.
///
/// `assignments` is how many shortcodes the account holds, which is not
/// `rows.len()`: a shortcode with no published project is not a row. The
/// difference is what separates "nobody has assigned you anything" from "your
/// projects are not published yet".
pub fn assigned(rows: &[ProjectSummary<'_>], assignments: usize) -> Markup {
    html! {
        div class="max-w-4xl py-8" {
            h1 class="font-display text-2xl mb-2" { "Your projects" }
            @if !rows.is_empty() {
                p class="text-gray-600 mb-6" { "These are the projects you may edit." }
                (project_table("The projects assigned to your account", rows))
            } @else if assignments == 0 {
                p class="text-gray-600" {
                    "No projects are assigned to your account yet. RDU assigns them; ask them to add yours."
                }
            } @else {
                p class="text-gray-600" {
                    "Your account is assigned "
                    (assignment_count(assignments))
                    ", but none of them is in the published set this deployment carries. Ask RDU to check the \
                     assignment."
                }
            }
        }
    }
}

/// `GET /projects` for an RDU member: every published project.
///
/// RDU access is role-based rather than per-project (REQ-4.2), so there is no
/// assignment set to list and the account's own `shortcodes` is empty by design.
/// The list is therefore the whole published set.
pub fn rdu_overview(rows: &[ProjectSummary<'_>]) -> Markup {
    html! {
        div class="max-w-4xl py-8" {
            h1 class="font-display text-2xl mb-2" { "Projects" }
            @if rows.is_empty() {
                p class="text-gray-600 mb-4" {
                    "This deployment carries no published project set, so there is nothing to list. A project \
                     is still reachable by shortcode at "
                    code class="font-mono" { "/projects/{shortcode}" }
                    "."
                }
            } @else {
                p class="text-gray-600 mb-6" {
                    "Your access is role-based rather than per-project, so every project here is open to you."
                }
                (project_table("Every published project", rows))
            }
            p class="mt-6" {
                a href="/depositors" class="underline" { "Manage depositor accounts" }
            }
        }
    }
}

/// `GET /projects/{shortcode}` — reached only by someone who may.
///
/// `name` is the published project's name, or `None` for a shortcode the
/// published set does not hold. That is not an error state (REQ-2.3): a project
/// may exist only locally, and its page has to open without reading as a
/// failure.
pub fn project(shortcode: &str, name: Option<&str>) -> Markup {
    html! {
        div class="max-w-2xl py-8" {
            @match name {
                Some(name) => {
                    h1 class="font-display text-2xl mb-1" { (name) }
                    p class="font-mono text-sm text-gray-600 mb-6" { (shortcode) }
                }
                None => {
                    h1 class="font-display text-2xl mb-1" { "Project " (shortcode) }
                    p class="text-gray-600 mb-6" {
                        "This project is not in the published set this deployment carries, so there is nothing \
                         to pre-fill yet."
                    }
                }
            }
            p class="text-gray-600 mb-6" {
                "The editing form for this project is not available yet."
            }
            p {
                a href="/projects" class="underline" { "Back to your projects" }
            }
        }
    }
}

/// The shared list rendering. `caption` is the table's accessible name.
fn project_table(caption: &str, rows: &[ProjectSummary<'_>]) -> Markup {
    let head = html! {
        tr { (table_head_cell("Shortcode")) (table_head_cell("Name")) (table_head_cell("Status")) }
    };
    let body = html! {
        @for row in rows {
            tr {
                (table_cell(project_link(row)))
                (table_cell(row.name))
                (table_cell(status_label(row.status)))
            }
        }
    };
    html! {
        (table(caption).head(head).body(body))
    }
}

/// One row's link to its project.
///
/// A named function rather than an `@let link = html! { … }` inside the loop:
/// `maudfmt` formats `html!` only at Rust statement position, so an in-macro
/// `@let` is skipped and then reformatted by `cargo fmt` as ordinary Rust —
/// which splits attributes across lines and puts spaces around `=`. It is not a
/// rendering bug, but it comes back on every `cargo fmt` run. See the
/// formatting note in `docs/src/mosaic/component-api-conventions.md`.
fn project_link(row: &ProjectSummary<'_>) -> Markup {
    html! {
        a href={ "/projects/" (row.shortcode) } class="underline font-mono font-bold" {
            (row.shortcode)
        }
    }
}

/// The contract stores `ongoing` / `finished`; a page shows them capitalised,
/// and anything else verbatim rather than silently as one of the two.
fn status_label(status: &str) -> &str {
    match status {
        "ongoing" => "Ongoing",
        "finished" => "Finished",
        other => other,
    }
}

/// "1 project" / "3 projects" — enough grammar to avoid "1 projects".
fn assignment_count(count: usize) -> String {
    if count == 1 {
        "1 project".to_string()
    } else {
        format!("{count} projects")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary<'a>(shortcode: &'a str, name: &'a str, status: &'a str) -> ProjectSummary<'a> {
        ProjectSummary { shortcode, name, status }
    }

    #[test]
    fn test_the_list_links_each_project_by_shortcode_and_names_it() {
        let rows = [
            summary("0801d", "Bernoulli-Euler Online", "ongoing"),
            summary("080C", "Anton Webern", "finished"),
        ];
        let out = assigned(&rows, 2).into_string();
        assert!(out.contains(r#"<a href="/projects/0801d""#), "{out}");
        assert!(out.contains("Bernoulli-Euler Online"), "{out}");
        assert!(out.contains(r#"<a href="/projects/080C""#), "{out}");
        assert!(out.contains("Anton Webern"), "{out}");
    }

    #[test]
    fn test_the_list_shows_nothing_that_was_not_assigned() {
        // The whole point of the page: it is the depositor's own scope
        // (REQ-1.2), not a directory of every project.
        let out = assigned(&[summary("0801d", "Bernoulli-Euler Online", "ongoing")], 1).into_string();
        assert!(!out.contains("0803"), "{out}");
    }

    #[test]
    fn test_no_assignments_says_who_fixes_it() {
        // A new account has none, so it must not read as an error page.
        let out = assigned(&[], 0).into_string();
        assert!(out.contains("RDU assigns them"), "{out}");
        assert!(!out.contains("<table"), "{out}");
    }

    #[test]
    fn test_assignments_that_are_all_unpublished_is_a_different_message() {
        // Told "nobody has assigned you anything", a depositor whose projects
        // are merely unpublished goes to the wrong person for help.
        let out = assigned(&[], 2).into_string();
        assert!(out.contains("2 projects"), "{out}");
        assert!(out.contains("none of them is in the published set"), "{out}");
        assert!(!out.contains("RDU assigns them"), "{out}");
    }

    #[test]
    fn test_one_unpublished_assignment_is_counted_in_the_singular() {
        let out = assigned(&[], 1).into_string();
        assert!(out.contains("1 project,"), "{out}");
        assert!(!out.contains("1 projects"), "{out}");
    }

    #[test]
    fn test_the_rdu_overview_lists_every_published_project() {
        let rows = [
            summary("0801d", "Bernoulli-Euler Online", "ongoing"),
            summary("080C", "Anton Webern", "finished"),
        ];
        let out = rdu_overview(&rows).into_string();
        assert!(out.contains("role-based"), "{out}");
        assert!(out.contains("Bernoulli-Euler Online"), "{out}");
        assert!(out.contains("Anton Webern"), "{out}");
        assert!(out.contains(r#"href="/depositors""#), "{out}");
    }

    #[test]
    fn test_the_rdu_overview_explains_an_absent_set_rather_than_showing_an_empty_table() {
        let out = rdu_overview(&[]).into_string();
        assert!(out.contains("carries no published project set"), "{out}");
        assert!(!out.contains("<table"), "{out}");
        // The way in by shortcode is still stated, since it still works.
        assert!(out.contains("/projects/{shortcode}"), "{out}");
    }

    #[test]
    fn test_status_is_capitalised_and_an_unknown_value_is_shown_verbatim() {
        let out = rdu_overview(&[summary("0801", "A", "ongoing")]).into_string();
        assert!(out.contains("Ongoing"), "{out}");
        let odd = rdu_overview(&[summary("0801", "A", "suspended")]).into_string();
        assert!(odd.contains("suspended"), "{odd}");
    }

    #[test]
    fn test_the_project_page_leads_with_the_name_and_keeps_the_shortcode() {
        let out = project("0801d", Some("Bernoulli-Euler Online")).into_string();
        assert!(out.contains("Bernoulli-Euler Online"), "{out}");
        assert!(out.contains("0801d"), "{out}");
        assert!(out.contains(r#"<a href="/projects""#), "{out}");
    }

    #[test]
    fn test_an_unpublished_project_opens_without_reading_as_an_error() {
        // REQ-2.3 allows a project that exists only locally, and REQ-1.1's
        // "current published metadata" is then empty.
        let out = project("0999", None).into_string();
        assert!(out.contains("Project 0999"), "{out}");
        assert!(out.contains("nothing to pre-fill"), "{out}");
    }

    #[test]
    fn test_a_shortcode_and_a_name_are_escaped_wherever_they_are_rendered() {
        // Both arrive from data: the shortcode from a path segment or a stored
        // assignment, the name from a project file.
        let hostile = "<script>alert(1)</script>";
        let rows = [summary(hostile, hostile, hostile)];
        for out in [
            project(hostile, Some(hostile)).into_string(),
            assigned(&rows, 1).into_string(),
            rdu_overview(&rows).into_string(),
        ] {
            assert!(!out.contains("<script>alert(1)</script>"), "{out}");
            assert!(out.contains("&lt;script&gt;"), "{out}");
        }
    }
}
