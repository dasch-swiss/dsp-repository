//! The RDU approved-records page: every approved record and how it compares with
//! the published set, and the confirmation for discarding one.
//!
//! [`list`] renders [`RecordClassification`], the same classification the
//! startup pass computes ([`editor_core::status::classify_record`]), so this
//! page and the startup decision cannot disagree. The editor does not know
//! whether a record's pull request is open, merged or closed: each row links to
//! GitHub's search for it instead.
//!
//! RDU-facing rather than depositor-facing, so the forbidden-word list that
//! binds a depositor's screens does not apply here: a reviewer needs the
//! mechanism named, so this page says "pull request" plainly.

use editor_core::status::RecordClassification;
use maud::{html, Markup};
use mosaic_tiles::badge::{badge, BadgeVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::table::{table, table_cell, table_head_cell};
use mosaic_tiles::ComponentBuilder;

/// One approved record, as the list renders it.
pub struct ApprovedRow<'a> {
    pub id: &'a str,
    pub shortcode: &'a str,
    pub project_name: Option<&'a str>,
    pub approved_at: &'a str,
    /// GitHub's search for this record's pull requests.
    pub pull_requests_url: &'a str,
    pub state: &'a RecordClassification,
}

/// `GET /approved`: every approved record and how it compares with the published set.
pub fn list(rows: &[ApprovedRow<'_>]) -> Markup {
    html! {
        div class="py-8" {
            h1 class="font-display text-2xl mb-2" { "Approved records" }
            p class="text-gray-600 mb-6" {
                "Status compares each approved record with the published set as of the last start. Whether its \
                 pull request is open, merged or closed is read on GitHub through the link."
            }
            @if rows.is_empty() {
                p class="text-gray-600" { "No approved records are waiting for release." }
            } @else { (approved_table(rows)) }
        }
    }
}

fn approved_table(rows: &[ApprovedRow<'_>]) -> Markup {
    let actions = html! {
        span class="sr-only" { "Actions" }
    };
    let head = html! {
        tr {
            (table_head_cell("Shortcode"))
            (table_head_cell("Project"))
            (table_head_cell("Approved"))
            (table_head_cell("Status"))
            (table_head_cell(actions))
        }
    };
    let body = html! {
        @for row in rows { (approved_row(row)) }
    };
    html! {
        (table("Approved records").head(head).body(body))
    }
}

fn approved_row(row: &ApprovedRow<'_>) -> Markup {
    let name = html! {
        @match row.project_name {
            Some(name) => (name)
            None => span class="italic text-neutral-600" { "Not in the published set" }
        }
    };
    html! {
        tr {
            ({
                table_cell(
                    html! {
                        span class = "font-mono text-sm" { (row.shortcode) }
                    },
                )
            })
            (table_cell(name))
            ({
                table_cell(
                    html! {
                        span class = "text-sm" { (row.approved_at) }
                    },
                )
            })
            (table_cell(status_cell(row)))
            (table_cell(discard_control(row)))
        }
    }
}

/// Exhaustive with no wildcard arm, so a variant added later fails to compile here rather than
/// falling through to a mislabelled row.
fn status_label_and_variant(state: &RecordClassification) -> (&'static str, BadgeVariant) {
    match state {
        // Unreachable in practice: the startup pass discards a matching record
        // before this page ever reads it. Named rather than folded into a
        // catch-all, for the same reason the whole match has none.
        RecordClassification::Published => ("Published", BadgeVariant::Success),
        RecordClassification::WaitingForRelease { .. } => ("Waiting for release", BadgeVariant::Info),
        RecordClassification::RemovedUpstream => ("Project dropped upstream", BadgeVariant::Warning),
        RecordClassification::Unreadable { .. } => ("Payload unreadable", BadgeVariant::Danger),
        RecordClassification::Anomalous => ("Unexpected comparison", BadgeVariant::Danger),
    }
}

fn status_cell(row: &ApprovedRow<'_>) -> Markup {
    let (label, variant) = status_label_and_variant(row.state);
    html! {
        div class="flex flex-col gap-1" {
            (badge(label).variant(variant))
            (status_detail(row.state))
            p class="text-sm" { (pull_requests_link(row.pull_requests_url, row.shortcode)) }
        }
    }
}

fn pull_requests_link(url: &str, shortcode: &str) -> Markup {
    link("Pull requests on GitHub", url)
        .aria_label(format!("Pull requests on GitHub for {shortcode}"))
        .build()
}

fn status_detail(state: &RecordClassification) -> Markup {
    html! {
        @match state {
            RecordClassification::WaitingForRelease { changed } => {
                p class="text-sm text-neutral-600" { "Differs in: " (changed.join(", ")) "." }
            }
            RecordClassification::Unreadable { problem } => {
                p class="text-sm text-neutral-600" { "Reason: " (problem) }
            }
            _ => {}
        }
    }
}

/// Offered on every row. A link, not a button: it opens the confirmation, which
/// is a `GET`, so the flow still works with JavaScript off.
fn discard_control(row: &ApprovedRow<'_>) -> Markup {
    link("Discard", format!("/approved/{}/discard", row.id))
        .aria_label(format!("Discard the approved record for {}", row.shortcode))
        .build()
}

/// What a discard deletes, for [`confirm_discard`].
pub struct DiscardImpact<'a> {
    pub shortcode: &'a str,
    pub project_name: Option<&'a str>,
    pub approved_at: &'a str,
    /// GitHub's search for the record's pull requests.
    pub pull_requests_url: &'a str,
    /// The URL is reachable directly, so the confirmation names the state instead of reading alike
    /// for every record.
    pub state: &'a RecordClassification,
}

/// `GET /approved/{id}/discard`: the confirmation before the `POST` that deletes the record.
pub fn confirm_discard(id: &str, impact: &DiscardImpact<'_>) -> Markup {
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Discard this record?" }
            p class="text-gray-600 mb-4" {
                "This deletes the only copy of what was approved. It cannot be undone. The pull request is not \
                 touched: close it on GitHub."
            }
            div class="border border-gray-300 rounded p-4 mb-4" {
                p class="font-bold mb-1" { "What is being destroyed" }
                @match impact.project_name {
                    Some(name) => p { (name) }
                    None => p class="italic text-neutral-600" { "Not in the published set" }
                }
                p class="font-mono text-sm" { (impact.shortcode) }
                p class="text-gray-600 text-sm mt-1" { "Approved " (impact.approved_at) "." }
                p class="mt-2" {
                    ({
                        badge(status_label_and_variant(impact.state).0)
                            .variant(status_label_and_variant(impact.state).1)
                    })
                }
                p class="text-sm mt-1" {
                    (pull_requests_link(impact.pull_requests_url, impact.shortcode))
                }
            }
            form
                method="post"
                action={ "/approved/" (id) "/discard" }
                class="flex items-center gap-3"
            {
                ({
                    button("Discard permanently")
                        .button_type(ButtonType::Submit)
                        .variant(ButtonVariant::Secondary)
                })
                (link("Cancel", "/approved"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::approved::pull_requests_url;

    fn row<'a>(
        id: &'a str,
        shortcode: &'a str,
        pull_requests_url: &'a str,
        state: &'a RecordClassification,
    ) -> ApprovedRow<'a> {
        ApprovedRow {
            id,
            shortcode,
            project_name: Some("Bernoulli-Euler Online"),
            approved_at: "2026-09-01 08:00 UTC",
            pull_requests_url,
            state,
        }
    }

    #[test]
    fn confirm_discard_posts_to_the_record_and_links_to_its_pull_requests() {
        let waiting = RecordClassification::WaitingForRelease { changed: vec!["name".to_string()] };
        let url = pull_requests_url("0801");
        let impact = DiscardImpact {
            shortcode: "0801",
            project_name: Some("Bernoulli-Euler Online"),
            approved_at: "2026-09-01 08:00 UTC",
            pull_requests_url: &url,
            state: &waiting,
        };
        let out = confirm_discard("42", &impact).into_string();
        assert!(out.contains(r#"<form method="post" action="/approved/42/discard""#), "{out}");
        assert!(out.contains(&url), "{out}");
        assert!(
            out.contains("Waiting for release"),
            "a confirmation reached by URL names the state: {out}"
        );
    }

    #[test]
    fn snapshot_the_approved_list_over_every_classification() {
        let published = RecordClassification::Published;
        let waiting =
            RecordClassification::WaitingForRelease { changed: vec!["name".to_string(), "abstract".to_string()] };
        let removed = RecordClassification::RemovedUpstream;
        let unreadable = RecordClassification::Unreadable { problem: "invalid type: string, expected map".to_string() };
        let anomalous = RecordClassification::Anomalous;

        let urls = ["0801", "0802", "0803", "0804", "0805"].map(pull_requests_url);
        let rows = vec![
            row("1", "0801", &urls[0], &published),
            row("2", "0802", &urls[1], &waiting),
            row("3", "0803", &urls[2], &removed),
            row("4", "0804", &urls[3], &unreadable),
            row("5", "0805", &urls[4], &anomalous),
        ];
        insta::assert_snapshot!("approved_list", list(&rows).into_string());
    }
}
