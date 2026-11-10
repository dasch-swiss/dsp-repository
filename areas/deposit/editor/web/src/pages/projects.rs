//! The project list.
//!
//! This page owns the scoping: a depositor sees exactly the projects assigned to
//! them, named from the published set. The editing surface is
//! [`crate::pages::section`], which `/projects/{shortcode}` redirects into, so
//! exactly one place decides where a project link lands.
//!
//! An assigned shortcode matching no published project is skipped rather than rendered as
//! a nameless row: a project assigned before it is published, or one that
//! exists only locally, is a real state. The two empty states say different
//! things, because they send the reader to different people.

use deposit_shell::form_card;
use editor_core::published::ProjectSummary;
use editor_core::status::ProjectState;
use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::table::{table, table_cell, table_head_cell};
use mosaic_tiles::text_field::text_field;

use crate::state_badge::state_badge;

const TEXT_LINK: &str = "text-primary-600 underline underline-offset-2";

const LIST_TABLE: &str = "bg-white border border-neutral-200 [&_th]:bg-neutral-50 [&_th]:border-b \
                          [&_th]:border-neutral-200 [&_th]:text-xs [&_th]:uppercase [&_th]:tracking-wider \
                          [&_th]:text-neutral-600 [&_td]:border-neutral-100";

/// One row of a depositor's list: the published project, plus where that
/// depositor's own changes have got to. A pair rather than two parallel slices,
/// which can go out of step by one.
#[derive(Debug, Clone, Copy)]
pub struct AssignedProject<'a> {
    pub summary: ProjectSummary<'a>,
    pub state: ProjectState,
}

/// The `status` a created project's row shows, where a published one shows `ongoing` or
/// `finished`.
pub const NOT_PUBLISHED_YET: &str = "Not published yet";

/// `GET /projects` for a depositor: their published and created projects, and every assigned
/// shortcode that names neither, so a mistyped assignment is visible rather than silently missing.
pub fn assigned(rows: &[AssignedProject<'_>], unknown: &[&str]) -> Markup {
    html! {
        div class="max-w-4xl py-8" {
            h1 class="font-display font-semibold text-2xl text-neutral-900 mb-2" { "Your projects" }
            @if !rows.is_empty() {
                p class="text-neutral-600 mb-6" { "These are the projects you may edit." }
                (assigned_cards("The projects assigned to your account", rows))
                p class="mt-4" {
                    a href="/states" class=(TEXT_LINK) { "What do these states mean?" }
                }
            } @else if unknown.is_empty() {
                p class="text-neutral-600" {
                    "No projects are assigned to your account yet. RDU assigns them; ask them to add yours."
                }
            }
            @if !unknown.is_empty() {
                p class="text-neutral-600 mt-4" {
                    "DaSCH has no project with the shortcode "
                    @for (index, shortcode) in unknown.iter().enumerate() {
                        @if index > 0 { ", " }
                        code class="font-mono" { (shortcode) }
                    }
                    ". Ask RDU to check the shortcodes assigned to your account."
                }
            }
        }
    }
}

/// A project RDU created that the published set does not carry yet, as the RDU overview lists it.
#[derive(Debug, Clone)]
pub struct CreatedRow {
    /// The draft's spelling where there is a draft, the registration's key otherwise.
    pub shortcode: String,
    /// The draft's current name, falling back to the one entered at creation.
    pub name: String,
    pub created_at: String,
    /// No submission and no approved record: removing it deletes nobody's pending work.
    pub removable: bool,
}

/// `GET /projects` for an RDU member: the created projects, then the whole published set, since
/// RDU access is role-based and the account's `assigned_shortcodes` is empty by design.
///
/// `removed` is the shortcode of a created project just removed, for the line that confirms it.
pub fn rdu_overview(rows: &[ProjectSummary<'_>], created: &[CreatedRow], removed: Option<&str>) -> Markup {
    html! {
        div class="max-w-4xl py-8" {
            h1 class="font-display font-semibold text-2xl text-neutral-900 mb-2" { "Projects" }
            @if let Some(shortcode) = removed {
                div class="mb-4" {
                    ({
                        alert(format!("Project {shortcode} removed."))
                            .variant(AlertVariant::Success)
                    })
                }
            }
            p class="mb-6" {
                ({
                    link("Create a project", "/projects/new")
                        .as_button(ButtonVariant::Primary)
                })
            }
            @if !created.is_empty() {
                h2 class="font-display font-semibold text-xl text-neutral-900 mb-2" {
                    "New projects"
                }
                p class="text-neutral-600 mb-4" {
                    "Created in the editor and not published yet. A project with nothing submitted or approved \
                     can be removed."
                }
                (created_table(created))
                h2 class="font-display font-semibold text-xl text-neutral-900 mt-8 mb-2" {
                    "Published projects"
                }
            }
            @if rows.is_empty() {
                p class="text-neutral-600 mb-4" {
                    "This deployment carries no published project set, so there is nothing to list. A project \
                     is still reachable by shortcode at "
                    code class="font-mono" { "/projects/{shortcode}" }
                    "."
                }
            } @else {
                p class="text-neutral-600 mb-6" {
                    "Your access is role-based rather than per-project, so every project here is open to you."
                }
                (project_table("Every published project", rows))
            }
        }
    }
}

fn created_table(rows: &[CreatedRow]) -> Markup {
    let head = html! {
        tr {
            (table_head_cell("Shortcode"))
            (table_head_cell("Name"))
            (table_head_cell("Created"))
            (table_head_cell("Actions"))
        }
    };
    let body = html! {
        @for row in rows {
            tr {
                (table_cell(shortcode_link(&row.shortcode)))
                (table_cell(row.name.as_str()))
                (table_cell(row.created_at.as_str()))
                (table_cell(removal_cell(row)))
            }
        }
    };
    html! {
        ({
            table("Projects created in the editor")
                .head(head)
                .body(body)
                .class(LIST_TABLE)
        })
    }
}

/// The remove link, named for its project, or why there is none.
fn removal_cell(row: &CreatedRow) -> Markup {
    html! {
        @if row.removable {
            a   href={ "/projects/" (row.shortcode) "/remove" }
                class=(TEXT_LINK)
                aria-label={ "Remove project " (row.shortcode) }
            { "Remove" }
        } @else {
            span class="text-neutral-600" { "Submitted or approved" }
        }
    }
}

/// `GET /projects/{shortcode}/remove`: the confirmation before the `POST` that deletes a created
/// project and its draft. `refusal` says why a `POST` did not remove it.
pub fn confirm_removal(shortcode: &str, name: &str, refusal: Option<&str>) -> Markup {
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Remove project " (shortcode) "?" }
            @if let Some(refusal) = refusal {
                p class="mb-4" { (refusal) }
                (link("Back to the projects", "/projects"))
            } @else {
                p class="text-gray-600 mb-4" {
                    "This deletes the project "
                    (name)
                    " with its draft, including anything a depositor has entered, its review history and \
                     its unpublished person and organisation proposals. It cannot be undone. The \
                     shortcode can then be created again."
                }
                form
                    method="post"
                    action={ "/projects/" (shortcode) "/remove" }
                    class="flex items-center gap-3"
                {
                    ({
                        button("Remove project")
                            .button_type(ButtonType::Submit)
                            .variant(ButtonVariant::Secondary)
                    })
                    (link("Cancel", "/projects"))
                }
            }
        }
    }
}

/// What RDU typed into the create form, kept on a refused attempt so it is not typed again.
#[derive(Debug, Default, Clone, Copy)]
pub struct NewProjectFields<'a> {
    pub shortcode: &'a str,
    pub name: &'a str,
}

/// Why a create attempt was refused, per field.
#[derive(Debug, Default, Clone)]
pub struct NewProjectErrors {
    pub shortcode: Option<String>,
    pub name: Option<String>,
}

/// `GET /projects/new`, and a refused `POST` to it: RDU enters the shortcode and the name, and
/// everything else is either derived or the depositor's to fill in.
pub fn new_project(fields: NewProjectFields<'_>, errors: &NewProjectErrors) -> Markup {
    let shortcode_hint = html! {
        "Letters and digits, for example "
        code class="font-mono" { "08AB" }
        ". It cannot be changed later. The PID is derived from it."
    };
    let mut shortcode = text_field("shortcode", "Shortcode")
        .value(fields.shortcode)
        .hint(shortcode_hint)
        .autocomplete("off")
        .required();
    // Focus on the first refused field, so a screen reader announces its error on the re-render.
    if let Some(message) = &errors.shortcode {
        shortcode = shortcode.error(message.as_str()).autofocus();
    }
    let mut name = text_field("name", "Name").value(fields.name).required();
    if let Some(message) = &errors.name {
        name = name.error(message.as_str());
        if errors.shortcode.is_none() {
            name = name.autofocus();
        }
    }
    let body = html! {
        p class="text-neutral-600" {
            "The project is held in the editor until it is published. Assign it to its depositors on "
            a href="/depositors" class=(TEXT_LINK) { "Manage depositor accounts" }
            "."
        }
        form method="post" action="/projects/new" class="flex flex-col gap-4" {
            (shortcode)
            (name)
            div class="flex items-center gap-3" {
                (button("Create project").button_type(ButtonType::Submit))
                a href="/projects" class=(TEXT_LINK) { "Cancel" }
            }
        }
    };
    html! {
        div class="py-8" { (form_card("Create a project", body)) }
    }
}

/// A depositor's list: one card per project, still a table with a visually hidden header row.
/// The state column is headed "Your changes", since "Status" is the research project's.
///
/// Keep the explicit `role`s: Safari drops table semantics once rows are `display: grid`, and axe
/// cannot see it; `test_the_card_list_keeps_explicit_table_roles_on_every_level` pins them.
fn assigned_cards(caption: &str, rows: &[AssignedProject<'_>]) -> Markup {
    html! {
        table role="table" class="block w-full" {
            caption class="sr-only" { (caption) }
            thead role="rowgroup" class="sr-only" {
                tr role="row" {
                    th role="columnheader" scope="col" { "Shortcode" }
                    th role="columnheader" scope="col" { "Name" }
                    th role="columnheader" scope="col" { "Status" }
                    th role="columnheader" scope="col" { "Your changes" }
                }
            }
            tbody
                role="rowgroup"
                class="grid grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-3"
            {
                @for row in rows {
                    tr  role="row"
                        class="grid grid-cols-[1fr_auto] grid-rows-[auto_auto_1fr_auto] overflow-hidden rounded-md \
                               border border-neutral-200 bg-white before:col-span-2 before:h-2 before:bg-primary-600 \
                               before:content-['']"
                    {
                        td role="cell" class="col-span-2 px-3 pt-3" {
                            (shortcode_link(row.summary.shortcode))
                        }
                        td  role="cell"
                            class="col-span-2 px-3 pt-1 pb-3 font-display font-semibold text-lg text-neutral-900"
                        { (row.summary.name) }
                        td  role="cell"
                            class="border-t border-neutral-100 px-3 py-2 text-sm text-neutral-600"
                        { "Status: " (status_label(row.summary.status)) }
                        td role="cell" class="border-t border-neutral-100 px-3 py-2 text-right" {
                            (state_badge(row.state))
                        }
                    }
                }
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
        (table(caption).head(head).body(body).class(LIST_TABLE))
    }
}

/// One row's link. A named function rather than an in-macro `@let`: `maudfmt`
/// formats `html!` only at statement position, so the `@let` form is mangled by
/// `cargo fmt` on every run; see `docs/src/mosaic/component-api-conventions.md`.
fn project_link(row: &ProjectSummary<'_>) -> Markup {
    shortcode_link(row.shortcode)
}

fn shortcode_link(shortcode: &str) -> Markup {
    html! {
        a href={ "/projects/" (shortcode) } class={ (TEXT_LINK) " font-mono font-bold" } {
            (shortcode)
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
        let out = assigned(&rows, &[]).into_string();
        assert!(out.contains(r#"<a href="/projects/0801d""#), "{out}");
        assert!(out.contains("Bernoulli-Euler Online"), "{out}");
        assert!(out.contains(r#"<a href="/projects/080C""#), "{out}");
        assert!(out.contains("Anton Webern"), "{out}");
    }

    #[test]
    fn test_the_card_list_keeps_explicit_table_roles_on_every_level() {
        let out = assigned(&[row("0801d", "Bernoulli-Euler Online", "ongoing")], &[]).into_string();
        for (tag, role) in [
            ("table", "table"),
            ("thead", "rowgroup"),
            ("tbody", "rowgroup"),
            ("tr", "row"),
            ("th", "columnheader"),
            ("td", "cell"),
        ] {
            let tags = out.matches(&format!("<{tag} ")).count() + out.matches(&format!("<{tag}>")).count();
            let with_role = out.matches(&format!(r#"<{tag} role="{role}""#)).count();
            assert!(tags > 0 && tags == with_role, "<{tag}> without role={role} in {out}");
        }
    }

    #[test]
    fn test_the_list_shows_nothing_that_was_not_assigned() {
        let out = assigned(&[row("0801d", "Bernoulli-Euler Online", "ongoing")], &[]).into_string();
        assert!(!out.contains("0803"), "{out}");
    }

    #[test]
    fn test_no_assigned_projects_says_who_fixes_it() {
        // A new account has none, so it must not read as an error page.
        let out = assigned(&[], &[]).into_string();
        assert!(out.contains("RDU assigns them"), "{out}");
        assert!(!out.contains("<table"), "{out}");
    }

    #[test]
    fn test_an_assigned_shortcode_naming_no_project_is_named_in_plain_words() {
        // Told "nobody has assigned you anything", a depositor whose assignment is mistyped goes
        // to the wrong person for help.
        let out = assigned(&[], &["9999", "08XY"]).into_string();
        assert!(out.contains("DaSCH has no project with the shortcode"), "{out}");
        assert!(out.contains(">9999<") && out.contains(">08XY<"), "{out}");
        assert!(!out.contains("RDU assigns them"), "{out}");
        assert!(!out.contains("published set"), "{out}");
    }

    #[test]
    fn test_an_unknown_shortcode_is_named_beside_the_projects_that_are_listed() {
        let out = assigned(&[row("0801d", "Bernoulli-Euler Online", "ongoing")], &["9999"]).into_string();
        assert!(out.contains("<table"), "{out}");
        assert!(out.contains(">9999<"), "{out}");
    }

    #[test]
    fn test_the_rdu_overview_lists_every_published_project() {
        let rows = [
            summary("0801d", "Bernoulli-Euler Online", "ongoing"),
            summary("080C", "Anton Webern", "finished"),
        ];
        let out = rdu_overview(&rows, &[], None).into_string();
        assert!(out.contains("role-based"), "{out}");
        assert!(out.contains("Bernoulli-Euler Online"), "{out}");
        assert!(out.contains("Anton Webern"), "{out}");
        // The header menu carries these; the overview body does not repeat them.
        for href in ["/review", "/approved", "/depositors"] {
            assert!(!out.contains(&format!(r#"href="{href}""#)), "{href} in {out}");
        }
    }

    #[test]
    fn test_the_rdu_overview_explains_an_absent_set_rather_than_showing_an_empty_table() {
        let out = rdu_overview(&[], &[], None).into_string();
        assert!(out.contains("carries no published project set"), "{out}");
        assert!(!out.contains("<table"), "{out}");
        // The way in by shortcode is still stated, since it still works.
        assert!(out.contains("/projects/{shortcode}"), "{out}");
    }

    #[test]
    fn test_status_is_capitalised_and_an_unknown_value_is_shown_verbatim() {
        let out = rdu_overview(&[summary("0801", "A", "ongoing")], &[], None).into_string();
        assert!(out.contains("Ongoing"), "{out}");
        let odd = rdu_overview(&[summary("0801", "A", "suspended")], &[], None).into_string();
        assert!(odd.contains("suspended"), "{odd}");
    }

    #[test]
    fn test_a_shortcode_and_a_name_are_escaped_wherever_they_are_rendered() {
        // Both arrive from data: the shortcode from a path segment or a stored
        // assigned project, the name from a project file.
        let hostile = "<script>alert(1)</script>";
        let summaries = [summary(hostile, hostile, hostile)];
        let assigned_rows = [row(hostile, hostile, hostile)];
        for out in [
            assigned(&assigned_rows, &[hostile]).into_string(),
            rdu_overview(&summaries, &[], None).into_string(),
        ] {
            assert!(!out.contains("<script>alert(1)</script>"), "{out}");
            assert!(out.contains("&lt;script&gt;"), "{out}");
        }
    }

    #[test]
    fn a_refused_create_form_focuses_the_first_refused_field() {
        let focused = |errors: NewProjectErrors| {
            let out = new_project(NewProjectFields::default(), &errors).into_string();
            let at = out.find("autofocus").expect("a focused field");
            assert_eq!(out.matches("autofocus").count(), 1, "{out}");
            out[..at]
                .rfind(r#"name=""#)
                .map(|name| out[name..at].to_string())
                .unwrap_or_default()
        };
        let error = || Some("refused".to_string());
        assert!(focused(NewProjectErrors { shortcode: error(), name: error() }).starts_with(r#"name="shortcode""#));
        assert!(focused(NewProjectErrors { shortcode: None, name: error() }).starts_with(r#"name="name""#));
        let out = new_project(NewProjectFields::default(), &NewProjectErrors::default()).into_string();
        assert!(!out.contains("autofocus"), "{out}");
    }
}
