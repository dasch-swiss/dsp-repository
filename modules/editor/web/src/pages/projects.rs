//! The project list.
//!
//! What this page owns is the **scoping**: it shows a depositor exactly the
//! projects assigned to them, named from the published set. The
//! editing surface is [`crate::pages::section`], which `/projects/{shortcode}`
//! redirects into — there is no per-project landing page between the two, so
//! that exactly one place decides where a project link lands.
//!
//! ## A row can be missing in two directions, and both are ordinary
//!
//! An assignment naming no published project is skipped rather than rendered as
//! a nameless row: a project assigned before it is published, and a
//! project that exists only locally, are both real states, and a blank row would
//! read as data loss. What must not happen is a depositor with assignments
//! seeing an empty page with no explanation, so the two empty states say
//! different things — which is also which person the reader should go to.

use editor_core::published::ProjectSummary;
use editor_core::status::ProjectState;
use maud::{html, Markup};
use mosaic_tiles::table::{table, table_cell, table_head_cell};

/// One row of a depositor's list: the published project, plus where that
/// depositor's own changes to it have got to.
///
/// A pair rather than two parallel slices, because the row and its state are
/// only ever read together and two slices can go out of step by one.
#[derive(Debug, Clone, Copy)]
pub struct AssignedProject<'a> {
    pub summary: ProjectSummary<'a>,
    pub state: ProjectState,
}

/// `GET /projects` for a depositor: the projects assigned to them.
///
/// `assignments` is how many shortcodes the account holds, which is not
/// `rows.len()`: a shortcode with no published project is not a row. The
/// difference is what separates "nobody has assigned you anything" from "your
/// projects are not published yet".
pub fn assigned(rows: &[AssignedProject<'_>], assignments: usize) -> Markup {
    html! {
        div class="max-w-4xl py-8" {
            h1 class="font-display text-2xl mb-2" { "Your projects" }
            @if !rows.is_empty() {
                p class="text-gray-600 mb-6" { "These are the projects you may edit." }
                (assigned_table("The projects assigned to your account", rows))
                p class="text-gray-600 mt-4" {
                    a href="/states" class="underline" { "What do these states mean?" }
                }
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
/// RDU access is role-based rather than per-project, so there is no
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
            // The review queue is otherwise reachable only by typing its URL:
            // nothing else in the interface links to it, so an RDU member has
            // no way to find what is waiting for them.
            p class="mt-6 flex gap-6" {
                a href="/review" class="underline" { "Review queue" }
                a href="/depositors" class="underline" { "Manage depositor accounts" }
            }
        }
    }
}

/// A depositor's list. Same shape as [`project_table`] plus the state column.
///
/// The extra column is deliberately **not** folded into `project_table` behind
/// an `Option`: RDU's overview has no depositor whose changes a state could
/// describe, so the column would always be empty there, and a shared function
/// with a mode flag would invite exactly that.
///
/// "Your changes" rather than a second "Status": the published project has its
/// own Ongoing/Finished status in the neighbouring column, and two columns
/// headed Status would make a depositor read the wrong one.
fn assigned_table(caption: &str, rows: &[AssignedProject<'_>]) -> Markup {
    let head = html! {
        tr {
            (table_head_cell("Shortcode"))
            (table_head_cell("Name"))
            (table_head_cell("Status"))
            (table_head_cell("Your changes"))
        }
    };
    let body = html! {
        @for row in rows {
            tr {
                (table_cell(project_link(&row.summary)))
                (table_cell(row.summary.name))
                (table_cell(status_label(row.summary.status)))
                (table_cell(row.state.label()))
            }
        }
    };
    html! {
        (table(caption).head(head).body(body))
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

    /// An assigned row in the state the list shows for an untouched project.
    fn row<'a>(shortcode: &'a str, name: &'a str, status: &'a str) -> AssignedProject<'a> {
        AssignedProject {
            summary: summary(shortcode, name, status),
            state: ProjectState::Draft,
        }
    }

    #[test]
    fn test_the_list_links_each_project_by_shortcode_and_names_it() {
        let rows = [
            row("0801d", "Bernoulli-Euler Online", "ongoing"),
            row("080C", "Anton Webern", "finished"),
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
        //, not a directory of every project.
        let out = assigned(&[row("0801d", "Bernoulli-Euler Online", "ongoing")], 1).into_string();
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
        // Nothing else in the interface links to the review queue, so without
        // this an RDU member has no way to find what is waiting for them.
        assert!(out.contains(r#"href="/review""#), "{out}");
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
    fn test_a_shortcode_and_a_name_are_escaped_wherever_they_are_rendered() {
        // Both arrive from data: the shortcode from a path segment or a stored
        // assignment, the name from a project file.
        let hostile = "<script>alert(1)</script>";
        let summaries = [summary(hostile, hostile, hostile)];
        let assigned_rows = [row(hostile, hostile, hostile)];
        for out in [
            assigned(&assigned_rows, 1).into_string(),
            rdu_overview(&summaries).into_string(),
        ] {
            assert!(!out.contains("<script>alert(1)</script>"), "{out}");
            assert!(out.contains("&lt;script&gt;"), "{out}");
        }
    }
}
