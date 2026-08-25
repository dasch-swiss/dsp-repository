//! The RDU screens for depositor accounts (US-7): the list, the create and edit
//! forms, and the removal confirmation.
//!
//! Every one of these is behind the RDU role, which is why they are the only
//! pages in the editor that render an email address. That is not in tension with
//! REQ-6.10: the requirement is about logs and traces, and RDU has to know which
//! address an account signs in with — it is the only channel to its owner.
//!
//! ## View types rather than the domain record
//!
//! The page functions take small `&str`-shaped structs, not
//! `editor_core::records::User`. Two reasons, and neither is layering
//! ceremony:
//!
//! - A `User` carries `failed_logins` and `failed_login_at`. Those are authentication internals,
//!   and a template that can see them is a template that can render them by accident.
//! - Whether a row may be edited is a **rule** — RDU accounts come from configuration (REQ-7.2) —
//!   and putting it in the struct makes the caller state it once instead of every template
//!   re-deriving it from a role string.
//!
//! Named fields rather than positional `&str` arguments for the same reason
//! `Viewer` is a struct: three adjacent strings are silently interchangeable,
//! and a swap here would put an address where a name belongs.
//!
//! Form primitives are hand-styled, as on the login screens. Text inputs and
//! tables are `mosaic-tiles`' next primitives and get proper tiles with
//! playground showcases then, rather than being guessed at from one surface.

use maud::{html, Markup};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;

/// Shared field styling, matching the login screens.
const FIELD_CLASS: &str =
    "border border-gray-300 rounded px-3 py-2 focus:outline-none focus:ring-2 focus:ring-primary-500";

const CELL_CLASS: &str = "px-3 py-2 align-top border-b border-gray-200";
const HEAD_CLASS: &str = "px-3 py-2 text-left font-bold border-b-2 border-gray-300";

/// A form-level error, above the fields it is about.
fn error_banner(message: &str) -> Markup {
    html! {
        p   role="alert"
            class="border border-danger-300 bg-danger-50 text-danger-800 rounded px-3 py-2 mb-4"
        { (message) }
    }
}

/// One account, as the list renders it.
pub struct DepositorRow<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub email: &'a str,
    /// The stored role, for display.
    pub role: &'a str,
    pub shortcodes: &'a [String],
    /// When a code was last successfully handed to the relay, already formatted,
    /// or `None` if none ever was.
    pub last_code_at: Option<&'a str>,
    /// Whether this account may be edited and removed here.
    ///
    /// False for RDU accounts: they come from `EDITOR_RDU_EMAILS` (REQ-7.2), so
    /// a change made here would be undone by the next restart, or — for an
    /// address no longer listed — would diverge from configuration invisibly.
    pub manageable: bool,
}

/// `GET /depositors` — every account, with the controls US-7 describes.
pub fn list(rows: &[DepositorRow<'_>]) -> Markup {
    html! {
        div class="py-8" {
            div class="flex items-center gap-4 mb-2" {
                h1 class="font-display text-2xl flex-1" { "Accounts" }
                ({
                    link("Add depositor", "/depositors/new")
                        .as_button(ButtonVariant::Primary)
                })
            }
            p class="text-gray-600 mb-6" {
                "RDU members come from the "
                code class="font-mono" { "EDITOR_RDU_EMAILS" }
                " setting and cannot be changed here. Depositors are created and removed on this page."
            }
            @if rows.is_empty() {
                p class="text-gray-600" { "There are no accounts yet." }
            } @else {
                div class="overflow-x-auto" {
                    table class="w-full text-left" {
                        thead {
                            tr {
                                th class=(HEAD_CLASS) { "Name" }
                                th class=(HEAD_CLASS) { "Email" }
                                th class=(HEAD_CLASS) { "Role" }
                                th class=(HEAD_CLASS) { "Projects" }
                                th class=(HEAD_CLASS) { "Last code sent" }
                                th class=(HEAD_CLASS) {
                                    span class="sr-only" { "Actions" }
                                }
                            }
                        }
                        tbody {
                            @for row in rows { (list_row(row)) }
                        }
                    }
                }
            }
        }
    }
}

/// One `<tr>` of [`list`].
fn list_row(row: &DepositorRow<'_>) -> Markup {
    html! {
        tr {
            td class=(CELL_CLASS) { (row.name) }
            td class={ (CELL_CLASS) " font-mono text-sm" } { (row.email) }
            td class=(CELL_CLASS) { (row.role) }
            td class=(CELL_CLASS) {
                @if row.shortcodes.is_empty() {
                    span class="text-gray-500" { "—" }
                } @else { (row.shortcodes.join(", ")) }
            }
            td class=(CELL_CLASS) {
                // The answer to "I never got a code" that does not need an
                // address in a log (REQ-6.10). Absent means none was ever handed
                // to the relay, which is a different problem from one that was
                // sent and did not arrive.
                @match row.last_code_at {
                    Some(at) => span class="text-sm" { (at) }
                    None => span class="text-gray-500" { "never" }
                }
            }
            td class={ (CELL_CLASS) " whitespace-nowrap" } {
                @if row.manageable {
                    a href={ "/depositors/" (row.id) "/edit" } class="underline mr-3" { "Edit" }
                    a href={ "/depositors/" (row.id) "/remove" } class="underline" { "Remove" }
                } @else {
                    span class="text-gray-500 text-sm" { "from configuration" }
                }
            }
        }
    }
}

/// The three fields a depositor account is made of (REQ-7.3), as typed.
///
/// `shortcodes` is the raw text of the field rather than a parsed list, so a
/// rejected form comes back showing exactly what was entered — including the
/// entry that was wrong.
pub struct DepositorFields<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub shortcodes: &'a str,
}

/// `GET /depositors/new`.
pub fn create(fields: &DepositorFields<'_>, error: Option<&str>) -> Markup {
    let form = depositor_form("/depositors", "Create depositor", fields, error);
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Add a depositor" }
            p class="text-gray-600 mb-6" {
                "The address is how they sign in, so it has to be one they can read mail at."
            }
            (form)
        }
    }
}

/// `GET /depositors/{id}/edit`, which is also where it posts.
///
/// Form and action share one URL deliberately: a rejected submission re-renders
/// at the path it posted to, so that path has to answer `GET` or a reload lands
/// on a bare 405.
pub fn edit(id: &str, fields: &DepositorFields<'_>, error: Option<&str>) -> Markup {
    let form = depositor_form(&format!("/depositors/{id}/edit"), "Save changes", fields, error);
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Edit " (fields.name) }
            p class="text-gray-600 mb-6" {
                "Changing the address changes how this person signs in. Removing a project takes away their \
                 access to it; the project's own draft is unaffected."
            }
            (form)
        }
    }
}

/// The create and edit forms, which differ only in where they post and what the
/// submit button says.
fn depositor_form(action: &str, submit_label: &str, fields: &DepositorFields<'_>, error: Option<&str>) -> Markup {
    html! {
        @if let Some(message) = error { (error_banner(message)) }
        form method="post" action=(action) class="flex flex-col gap-4" {
            div class="flex flex-col gap-1" {
                label for="name" class="font-bold" { "Name" }
                input
                    id="name"
                    name="name"
                    type="text"
                    value=(fields.name)
                    required
                    class=(FIELD_CLASS);
            }
            div class="flex flex-col gap-1" {
                label for="email" class="font-bold" { "Email address" }
                input
                    id="email"
                    name="email"
                    type="email"
                    autocomplete="off"
                    value=(fields.email)
                    required
                    class=(FIELD_CLASS);
            }
            div class="flex flex-col gap-1" {
                label for="shortcodes" class="font-bold" { "Project shortcodes" }
                input
                    id="shortcodes"
                    name="shortcodes"
                    type="text"
                    value=(fields.shortcodes)
                    aria-describedby="shortcodes-help"
                    class=(FIELD_CLASS);
                p id="shortcodes-help" class="text-gray-600 text-sm" {
                    "Separated by commas, for example "
                    code class="font-mono" { "0801, 080C" }
                    ". Leave empty to assign none."
                }
            }
            div class="flex items-center gap-3" {
                (button(submit_label).button_type(ButtonType::Submit))
                a href="/depositors" class="underline" { "Cancel" }
            }
        }
    }
}

/// What a removal destroys and what it leaves behind.
pub struct RemovalImpact<'a> {
    pub name: &'a str,
    pub email: &'a str,
    /// Projects this account is the last editor of.
    pub draft_shortcodes: &'a [String],
    /// Projects with a submission this account made, still awaiting review.
    pub submission_shortcodes: &'a [String],
}

/// `GET /depositors/{id}/remove` — the confirmation.
///
/// REQ-7.5 deletes the account and its sessions unconditionally, so this page
/// does not offer to refuse. What it does is make the consequences visible
/// before the fact, because two of them are irreversible and neither is obvious:
///
/// - the address goes with the row, and it is RDU's only channel to ask this person about work they
///   left behind, so it is shown here to be copied;
/// - a submission they made stays pending with no author, which means it can be approved or
///   rejected but never *returned* to its depositor (REQ-4.5).
///
/// Their drafts and submissions survive — the schema nulls the author rather
/// than cascading — so the project's work is not destroyed along with the
/// account.
pub fn confirm_removal(id: &str, impact: &RemovalImpact<'_>) -> Markup {
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Remove " (impact.name) "?" }
            p class="text-gray-600 mb-4" {
                "This deletes the account and signs it out everywhere. It cannot be undone."
            }
            div class="border border-gray-300 rounded p-4 mb-4" {
                p class="font-bold mb-1" { "Their address, which is deleted with the account" }
                p class="font-mono text-sm" { (impact.email) }
                p class="text-gray-600 text-sm mt-1" {
                    "Copy it now if you may need to ask them about work in progress."
                }
            }
            (removal_impact(impact))
            form
                method="post"
                action={ "/depositors/" (id) "/remove" }
                class="flex items-center gap-3"
            {
                ({
                    button("Remove permanently")
                        .button_type(ButtonType::Submit)
                        .variant(ButtonVariant::Secondary)
                })
                a href="/depositors" class="underline" { "Cancel" }
            }
        }
    }
}

/// The "what this leaves behind" block of [`confirm_removal`].
fn removal_impact(impact: &RemovalImpact<'_>) -> Markup {
    html! {
        @if impact.draft_shortcodes.is_empty() && impact.submission_shortcodes.is_empty() {
            p class="mb-6 text-gray-600" {
                "This account has no drafts and no submission awaiting review."
            }
        } @else {
            div class="border border-warning-300 bg-warning-50 rounded p-4 mb-6" {
                p class="font-bold mb-2" { "What this leaves behind" }
                ul class="flex flex-col gap-2 list-disc pl-5" {
                    @if !impact.draft_shortcodes.is_empty() {
                        li {
                            "Drafts on "
                            (impact.draft_shortcodes.join(", "))
                            ". The work is kept; the last editor becomes unknown."
                        }
                    }
                    @if !impact.submission_shortcodes.is_empty() {
                        li {
                            "A submission awaiting review on "
                            (impact.submission_shortcodes.join(", "))
                            ". It can still be approved or rejected, but it can no longer be returned to its \
                             depositor for changes."
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    fn depositor_row<'a>(id: &'a str, shortcodes: &'a [String], last_code_at: Option<&'a str>) -> DepositorRow<'a> {
        DepositorRow {
            id,
            name: "A Depositor",
            email: "a.depositor@example.test",
            role: "depositor",
            shortcodes,
            last_code_at,
            manageable: true,
        }
    }

    #[test]
    fn test_the_list_offers_edit_and_remove_for_a_depositor() {
        let shortcodes = codes(&["0801"]);
        let out = list(&[depositor_row("abc", &shortcodes, None)]).into_string();
        assert!(out.contains(r#"href="/depositors/abc/edit""#), "{out}");
        assert!(out.contains(r#"href="/depositors/abc/remove""#), "{out}");
        assert!(out.contains(r#"href="/depositors/new""#), "{out}");
    }

    #[test]
    fn test_an_account_from_configuration_has_no_controls() {
        // REQ-7.2 makes configuration the source of truth for RDU membership. A
        // change made here would be undone by the next restart, or would diverge
        // from configuration with nothing to say so.
        let out = list(&[DepositorRow {
            id: "rdu-1",
            name: "An Admin",
            email: "rdu@dasch.swiss",
            role: "rdu",
            shortcodes: &[],
            last_code_at: None,
            manageable: false,
        }])
        .into_string();
        assert!(!out.contains("/depositors/rdu-1/edit"), "{out}");
        assert!(!out.contains("/depositors/rdu-1/remove"), "{out}");
        assert!(out.contains("from configuration"), "{out}");
    }

    #[test]
    fn test_the_list_distinguishes_a_code_never_sent_from_one_that_was() {
        // The support answer to "I never got a code": REQ-6.8 covers an
        // unconfigured relay and REQ-6.9 a failed send, but neither covers
        // accepted-then-undelivered, and REQ-6.10 forbids the address in a log.
        let none = list(&[depositor_row("a", &[], None)]).into_string();
        assert!(none.contains("never"), "{none}");

        let sent = list(&[depositor_row("a", &[], Some("2026-08-25 09:14 UTC"))]).into_string();
        assert!(sent.contains("2026-08-25 09:14 UTC"), "{sent}");
        assert!(!sent.contains(">never<"), "{sent}");
    }

    #[test]
    fn test_an_empty_list_says_so_rather_than_rendering_an_empty_table() {
        let out = list(&[]).into_string();
        assert!(out.contains("no accounts yet"), "{out}");
        assert!(!out.contains("<table"), "{out}");
    }

    #[test]
    fn test_the_create_form_posts_the_three_fields_req_7_3_names() {
        let fields = DepositorFields { name: "", email: "", shortcodes: "" };
        let out = create(&fields, None).into_string();
        assert!(out.contains(r#"<form method="post" action="/depositors""#), "{out}");
        assert!(out.contains(r#"name="name""#), "{out}");
        assert!(out.contains(r#"name="email""#), "{out}");
        assert!(out.contains(r#"name="shortcodes""#), "{out}");
    }

    #[test]
    fn test_a_rejected_form_comes_back_holding_what_was_typed() {
        // Otherwise every rejection costs the whole form, and the entry that was
        // wrong is the one thing the reader needs to see.
        let fields = DepositorFields {
            name: "A Depositor",
            email: "taken@example.test",
            shortcodes: "0801, nope!",
        };
        let out = create(&fields, Some("An account already uses that email address.")).into_string();
        assert!(out.contains(r#"value="A Depositor""#), "{out}");
        assert!(out.contains(r#"value="taken@example.test""#), "{out}");
        assert!(out.contains(r#"value="0801, nope!""#), "{out}");
        assert!(out.contains(r#"role="alert""#), "{out}");
        assert!(out.contains("An account already uses that email address."), "{out}");
    }

    #[test]
    fn test_the_edit_form_posts_to_the_url_it_is_served_from() {
        // Not merely "to the right account": the action must be the *same* URL
        // the form was fetched from, because a rejected submission re-renders
        // there. Posting to a path with no `GET` — which an earlier version did,
        // at `/depositors/{id}` — strands a reload on a bare 405.
        let fields = DepositorFields {
            name: "A Depositor",
            email: "a@example.test",
            shortcodes: "0801",
        };
        let out = edit("abc", &fields, None).into_string();
        assert!(out.contains(r#"<form method="post" action="/depositors/abc/edit""#), "{out}");
        assert!(out.contains("Save changes"), "{out}");
    }

    #[test]
    fn test_removal_shows_the_address_because_deleting_the_row_deletes_it() {
        // It is RDU's only channel to ask about work left behind, and REQ-7.5
        // takes it away.
        let impact = RemovalImpact {
            name: "A Depositor",
            email: "a.depositor@example.test",
            draft_shortcodes: &[],
            submission_shortcodes: &[],
        };
        let out = confirm_removal("abc", &impact).into_string();
        assert!(out.contains("a.depositor@example.test"), "{out}");
        assert!(out.contains(r#"action="/depositors/abc/remove""#), "{out}");
    }

    #[test]
    fn test_removal_names_the_submission_that_can_no_longer_be_returned() {
        // REQ-4.5 returns a submission to its depositor. With the account gone
        // there is no recipient, so it can be approved or rejected and nothing
        // else — which is the consequence worth seeing before the fact.
        let drafts = codes(&["0801", "080C"]);
        let submissions = codes(&["0801"]);
        let impact = RemovalImpact {
            name: "A Depositor",
            email: "a@example.test",
            draft_shortcodes: &drafts,
            submission_shortcodes: &submissions,
        };
        let out = confirm_removal("abc", &impact).into_string();
        assert!(out.contains("0801, 080C"), "{out}");
        assert!(out.contains("no longer be returned"), "{out}");
    }

    #[test]
    fn test_removal_with_nothing_in_flight_says_so_plainly() {
        let impact = RemovalImpact {
            name: "A Depositor",
            email: "a@example.test",
            draft_shortcodes: &[],
            submission_shortcodes: &[],
        };
        let out = confirm_removal("abc", &impact).into_string();
        assert!(out.contains("no drafts and no submission"), "{out}");
        assert!(!out.contains("What this leaves behind"), "{out}");
    }

    #[test]
    fn test_removal_is_a_post_and_never_a_link() {
        // A `GET` that deletes is the one shape the `Sec-Fetch-Site` CSRF
        // control cannot cover, because navigations are exempt from it by
        // necessity — any page could then delete an account with an `<img src>`.
        let impact = RemovalImpact {
            name: "A Depositor",
            email: "a@example.test",
            draft_shortcodes: &[],
            submission_shortcodes: &[],
        };
        let out = confirm_removal("abc", &impact).into_string();
        assert!(!out.contains(r#"<a href="/depositors/abc/remove""#), "{out}");
        assert!(out.contains(r#"<form method="post" action="/depositors/abc/remove""#), "{out}");
    }

    #[test]
    fn test_every_rendered_value_is_escaped() {
        let hostile = "<script>alert(1)</script>";
        let shortcodes = codes(&[hostile]);
        let row = DepositorRow {
            id: hostile,
            name: hostile,
            email: hostile,
            role: hostile,
            shortcodes: &shortcodes,
            last_code_at: Some(hostile),
            manageable: true,
        };
        let fields = DepositorFields { name: hostile, email: hostile, shortcodes: hostile };
        let impact = RemovalImpact {
            name: hostile,
            email: hostile,
            draft_shortcodes: &shortcodes,
            submission_shortcodes: &shortcodes,
        };
        for out in [
            list(&[row]).into_string(),
            create(&fields, Some(hostile)).into_string(),
            edit(hostile, &fields, None).into_string(),
            confirm_removal(hostile, &impact).into_string(),
        ] {
            assert!(!out.contains("<script>alert(1)</script>"), "{out}");
            assert!(out.contains("&lt;script&gt;"), "{out}");
        }
    }
}
