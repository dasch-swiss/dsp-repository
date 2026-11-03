//! The RDU screens for depositor accounts: the list, the create and edit
//! forms, and the removal confirmation.
//!
//! Behind the RDU role, which is why these are the only pages that render an
//! email address: RDU has to know which address an account signs in with. The
//! address-disclosure rule is about logs and traces.
//!
//! The page functions take small `&str` structs, not `identity_core::records::User`:
//! a `User` carries authentication internals a template could render by
//! accident, and whether a row may be edited is a rule (RDU accounts come from
//! configuration) the caller states once. Named fields rather than positional
//! `&str` arguments, since three adjacent strings are silently interchangeable.

use maud::{html, Markup};
use mosaic_tiles::alert::{alert, AlertVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::table::{table, table_cell, table_cell_with_class, table_head_cell};
use mosaic_tiles::text_field::{text_field, InputType};

/// One account, as the list renders it.
pub struct DepositorRow<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub email: &'a str,
    /// The stored role, for display.
    pub role: &'a str,
    pub shortcodes: &'a [String],
    /// Those of `shortcodes` that name a project neither published nor created.
    pub unknown_shortcodes: &'a [String],
    /// When a code was last successfully handed to the relay, already formatted,
    /// or `None` if none ever was.
    pub last_code_at: Option<&'a str>,
    /// Whether this account may be edited and removed here. False for RDU
    /// accounts: they come from `EDITOR_RDU_EMAILS`, so a change here would be
    /// undone by the next restart or diverge from configuration invisibly.
    pub manageable: bool,
}

/// Shown instead of the unknown-shortcode warnings when the projects could not be read: a page
/// without a warning would read as every shortcode being known.
pub const PROJECTS_UNCHECKED: &str = "Could not check the assigned projects.";

/// `GET /depositors` — every account, with the controls for managing one. `projects_checked` is
/// false when the unknown shortcodes could not be read, so the rows name none.
pub fn list(rows: &[DepositorRow<'_>], projects_checked: bool) -> Markup {
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
                "RDU members are set up by DaSCH and can't be changed here. Depositors are created and removed on this page."
            }
            @if !projects_checked { (unchecked_notice()) }
            @if rows.is_empty() {
                p class="text-gray-600" { "There are no accounts yet." }
            } @else { (accounts_table(rows)) }
        }
    }
}

/// The accounts table of [`list`]. "Actions" is present but visually hidden: an
/// unlabelled column is one a screen reader announces as nothing at all.
fn accounts_table(rows: &[DepositorRow<'_>]) -> Markup {
    let actions = html! {
        span class="sr-only" { "Actions" }
    };
    let head = html! {
        tr {
            (table_head_cell("Name"))
            (table_head_cell("Email"))
            (table_head_cell("Role"))
            (table_head_cell("Projects"))
            (table_head_cell("Last code sent"))
            (table_head_cell(actions))
        }
    };
    let body = html! {
        @for row in rows { (list_row(row)) }
    };
    html! {
        (table("Accounts").head(head).body(body))
    }
}

/// One `<tr>` of [`accounts_table`].
fn list_row(row: &DepositorRow<'_>) -> Markup {
    let projects = html! {
        @if row.shortcodes.is_empty() {
            span class="text-gray-500" { "—" }
        } @else {
            (row.shortcodes.join(", "))
            @if !row.unknown_shortcodes.is_empty() {
                span class="block text-sm" { (unknown_projects(row.unknown_shortcodes)) }
            }
        }
    };
    // Absent means no code was ever handed to the relay, a different problem
    // from one sent and not arrived.
    let last_code = html! {
        @match row.last_code_at {
            Some(at) => span class="text-sm" { (at) }
            None => span class="text-gray-500" { "never" }
        }
    };
    let controls = html! {
        @if row.manageable {
            a href={ "/depositors/" (row.id) "/edit" } class="underline mr-3" { "Edit" }
            a href={ "/depositors/" (row.id) "/remove" } class="underline" { "Remove" }
        } @else {
            span class="text-gray-500 text-sm" { "Set up by DaSCH" }
        }
    };
    html! {
        tr {
            (table_cell(row.name))
            (table_cell_with_class("font-mono text-sm", row.email))
            (table_cell(row.role))
            (table_cell(projects))
            (table_cell(last_code))
            (table_cell_with_class("whitespace-nowrap", controls))
        }
    }
}

/// The three fields a depositor account is made of, as typed. `shortcodes` is
/// the raw text, so a rejected form comes back showing what was entered.
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

/// `GET /depositors/{id}/edit`, which is also where it posts: a rejected
/// submission re-renders at the path it posted to, so that path has to answer
/// `GET` or a reload lands on a bare 405.
///
/// `unknown` are the stored shortcodes that name a project neither published nor created, `None`
/// when they could not be read; a re-rendered rejection passes none, since its fields are what
/// was typed, not what is stored.
pub fn edit(id: &str, fields: &DepositorFields<'_>, error: Option<&str>, unknown: Option<&[String]>) -> Markup {
    let form = depositor_form(&format!("/depositors/{id}/edit"), "Save changes", fields, error);
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Edit " (fields.name) }
            p class="text-gray-600 mb-6" {
                "Changing the address changes how this person signs in. Removing a project takes away their \
                 access to it; the project's own draft is unaffected."
            }
            @match unknown {
                Some(unknown) if !unknown.is_empty() => (unknown_notice(unknown))
                Some(_) => {}
                None => (unchecked_notice())
            }
            (form)
        }
    }
}

/// [`PROJECTS_UNCHECKED`], on the list and the edit page alike.
fn unchecked_notice() -> Markup {
    html! {
        div class="mb-4" { (alert(PROJECTS_UNCHECKED).variant(AlertVariant::Warning)) }
    }
}

/// The edit page's warning about assigned shortcodes no project carries. A warning, not a
/// refusal: the account saves either way, and the project may be created later.
fn unknown_notice(unknown: &[String]) -> Markup {
    let message = html! {
        (unknown_projects(unknown))
        " Check for a typo."
    };
    html! {
        div class="mb-4" { (alert(message).variant(AlertVariant::Warning)) }
    }
}

/// "Neither published nor created: `0801`, `080C`.", each shortcode in a span of its own so
/// the separators stay in the body font.
fn unknown_projects(unknown: &[String]) -> Markup {
    html! {
        "Neither published nor created: "
        @for (index, shortcode) in unknown.iter().enumerate() {
            @if index > 0 { ", " }
            code class="font-mono" { (shortcode) }
        }
        "."
    }
}

/// The create and edit forms, which differ only in where they post and what the
/// submit button says.
fn depositor_form(action: &str, submit_label: &str, fields: &DepositorFields<'_>, error: Option<&str>) -> Markup {
    // One `<code>` per shortcode: inside a monospace span the comma and space take a full
    // character width each, which reads as a gap.
    let shortcodes_hint = html! {
        "Separated by commas, for example "
        code class="font-mono" { "0801" }
        ", "
        code class="font-mono" { "080C" }
        ". Leave empty to assign none."
    };
    let name_field = text_field("name", "Name")
        .value(fields.name)
        .required()
        .hint("How the editor shows this person, for example in the page header after sign-in.");
    // `autocomplete("off")`: whoever fills this in is entering somebody else's
    // address, so offering their own is a wrong answer one keystroke away.
    let email_field = text_field("email", "Email address")
        .input_type(InputType::Email)
        .autocomplete("off")
        .value(fields.email)
        .required()
        .hint("The sign-in code is sent here, so use a mailbox the person reads. One account per address.");
    let shortcodes_field = text_field("shortcodes", "Project shortcodes")
        .value(fields.shortcodes)
        .hint(shortcodes_hint);
    html! {
        @if let Some(message) = error { (alert(message).variant(AlertVariant::Danger)) }
        form method="post" action=(action) class="flex flex-col gap-4" {
            (name_field)
            (email_field)
            (shortcodes_field)
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

/// `GET /depositors/{id}/remove`: the confirmation.
///
/// Removal deletes the account and its sessions unconditionally, so this page
/// makes the two irreversible consequences visible first: the address goes with
/// the row and is RDU's only channel to this person, so it is shown to be
/// copied; a submission they made stays pending with no author, so it can be
/// approved or rejected but never returned. Drafts and submissions survive,
/// since the editor's database keeps the author's id rather than the row.
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
        } @else { (leaves_behind(impact)) }
    }
}

/// The consequences block of [`removal_impact`].
fn leaves_behind(impact: &RemovalImpact<'_>) -> Markup {
    let items = html! {
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
                    ". It can still be approved or rejected, but it can no longer be returned to its depositor \
                     for changes."
                }
            }
        }
    };
    html! {
        (alert(items).variant(AlertVariant::Warning).title("What this leaves behind"))
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
            unknown_shortcodes: &[],
            last_code_at,
            manageable: true,
        }
    }

    #[test]
    fn test_the_list_offers_edit_and_remove_for_a_depositor() {
        let shortcodes = codes(&["0801"]);
        let out = list(&[depositor_row("abc", &shortcodes, None)], true).into_string();
        assert!(out.contains(r#"href="/depositors/abc/edit""#), "{out}");
        assert!(out.contains(r#"href="/depositors/abc/remove""#), "{out}");
        assert!(out.contains(r#"href="/depositors/new""#), "{out}");
    }

    #[test]
    fn test_an_account_from_configuration_has_no_controls() {
        let out = list(
            &[DepositorRow {
                id: "rdu-1",
                name: "An Admin",
                email: "rdu@dasch.swiss",
                role: "rdu",
                shortcodes: &[],
                unknown_shortcodes: &[],
                last_code_at: None,
                manageable: false,
            }],
            true,
        )
        .into_string();
        assert!(!out.contains("/depositors/rdu-1/edit"), "{out}");
        assert!(!out.contains("/depositors/rdu-1/remove"), "{out}");
        assert!(out.contains("Set up by DaSCH"), "{out}");
    }

    #[test]
    fn test_the_list_distinguishes_a_code_never_sent_from_one_that_was() {
        // The support answer to "I never got a code", without an address in a log.
        let none = list(&[depositor_row("a", &[], None)], true).into_string();
        assert!(none.contains("never"), "{none}");

        let sent = list(&[depositor_row("a", &[], Some("2026-08-25 09:14 UTC"))], true).into_string();
        assert!(sent.contains("2026-08-25 09:14 UTC"), "{sent}");
        assert!(!sent.contains(">never<"), "{sent}");
    }

    #[test]
    fn test_the_accounts_table_says_what_it_lists_and_what_its_columns_are() {
        // A name for the table and its scroll region, and headers that declare the
        // cells they govern.
        let out = list(&[depositor_row("abc", &[], None)], true).into_string();
        assert!(out.contains(r#"<caption class="sr-only">Accounts</caption>"#), "{out}");
        assert!(out.contains(r#"aria-label="Accounts""#), "{out}");
        assert!(out.contains(r#"scope="col""#), "{out}");
        // The controls column is headed for a screen reader and blank for everyone else.
        assert!(out.contains(r#"<span class="sr-only">Actions</span>"#), "{out}");
    }

    #[test]
    fn test_every_hint_is_announced_with_the_field_it_explains() {
        // Loose prose under an input is prose a screen reader reaches after the
        // field it was meant to explain, if at all.
        let fields = DepositorFields { name: "", email: "", shortcodes: "" };
        for out in [
            create(&fields, None).into_string(),
            edit("abc", &fields, None, Some(&[])).into_string(),
        ] {
            for field in ["name", "email", "shortcodes"] {
                assert!(out.contains(&format!(r#"aria-describedby="{field}-hint""#)), "{field}: {out}");
                assert!(out.contains(&format!(r#"id="{field}-hint""#)), "{field}: {out}");
            }
            assert!(out.contains("Leave empty to assign none."), "{out}");
        }
    }

    #[test]
    fn test_the_shortcodes_hint_keeps_the_separator_out_of_the_monospace_span() {
        let fields = DepositorFields { name: "", email: "", shortcodes: "" };
        let out = create(&fields, None).into_string();
        assert!(
            out.contains(r#"<code class="font-mono">0801</code>, <code class="font-mono">080C</code>"#),
            "{out}"
        );
    }

    #[test]
    fn test_the_list_names_each_unknown_shortcode_of_the_account_holding_it() {
        let shortcodes = codes(&["0801", "9999", "0ZZ0"]);
        let unknown = codes(&["9999", "0ZZ0"]);
        let row = DepositorRow {
            unknown_shortcodes: &unknown,
            ..depositor_row("abc", &shortcodes, None)
        };
        let out = list(&[row], true).into_string();
        assert!(
            out.contains(
                r#"Neither published nor created: <code class="font-mono">9999</code>, <code class="font-mono">0ZZ0</code>."#
            ),
            "{out}"
        );

        let known = list(&[depositor_row("abc", &shortcodes, None)], true).into_string();
        assert!(!known.contains("Neither published nor created"), "{known}");
    }

    #[test]
    fn test_the_edit_page_names_an_unknown_shortcode() {
        let fields = DepositorFields {
            name: "A Depositor",
            email: "a@example.test",
            shortcodes: "0801, 9999",
        };
        let out = edit("abc", &fields, None, Some(&codes(&["9999"]))).into_string();
        assert!(
            out.contains(r#"Neither published nor created: <code class="font-mono">9999</code>."#),
            "{out}"
        );

        let known = edit("abc", &fields, None, Some(&[])).into_string();
        assert!(!known.contains("Neither published nor created"), "{known}");
    }

    #[test]
    fn test_an_empty_list_says_so_rather_than_rendering_an_empty_table() {
        let out = list(&[], true).into_string();
        assert!(out.contains("no accounts yet"), "{out}");
        assert!(!out.contains("<table"), "{out}");
    }

    #[test]
    fn test_the_create_form_posts_name_email_and_shortcodes() {
        let fields = DepositorFields { name: "", email: "", shortcodes: "" };
        let out = create(&fields, None).into_string();
        assert!(out.contains(r#"<form method="post" action="/depositors""#), "{out}");
        assert!(out.contains(r#"name="name""#), "{out}");
        assert!(out.contains(r#"name="email""#), "{out}");
        assert!(out.contains(r#"name="shortcodes""#), "{out}");
    }

    #[test]
    fn test_a_rejected_form_comes_back_holding_what_was_typed() {
        // The entry that was wrong is the one thing the reader needs to see.
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
        // The action must be the URL the form was fetched from, because a rejected
        // submission re-renders there; a path with no GET strands a reload on a 405.
        let fields = DepositorFields {
            name: "A Depositor",
            email: "a@example.test",
            shortcodes: "0801",
        };
        let out = edit("abc", &fields, None, Some(&[])).into_string();
        assert!(out.contains(r#"<form method="post" action="/depositors/abc/edit""#), "{out}");
        assert!(out.contains("Save changes"), "{out}");
    }

    #[test]
    fn test_removal_shows_the_address_because_deleting_the_row_deletes_it() {
        // It is RDU's only channel to ask about work left behind, and removal takes it away.
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
        // With the account gone, request-changes has no recipient.
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
        // A GET that deletes is the one shape the Sec-Fetch-Site control cannot
        // cover, since navigations are exempt.
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
            unknown_shortcodes: &shortcodes,
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
            list(&[row], true).into_string(),
            create(&fields, Some(hostile)).into_string(),
            edit(hostile, &fields, None, Some(&shortcodes)).into_string(),
            confirm_removal(hostile, &impact).into_string(),
        ] {
            assert!(!out.contains("<script>alert(1)</script>"), "{out}");
            assert!(out.contains("&lt;script&gt;"), "{out}");
        }
    }
}
