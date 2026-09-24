//! The RDU collection surface: every approved record and where its collection
//! stands, and the confirmation for force-discarding one.
//!
//! [`list`] renders [`RecordClassification`], the same classification the
//! startup pass computes ([`editor_core::status::classify_record`]), so this
//! page and the startup decision cannot disagree.
//!
//! RDU-facing rather than depositor-facing, so the forbidden-word list that
//! binds a depositor's screens does not apply here: a reviewer needs the
//! mechanism named, so this page says "pull request" and "merged" plainly.

use editor_core::status::RecordClassification;
use maud::{html, Markup};
use mosaic_tiles::badge::{badge, BadgeVariant};
use mosaic_tiles::button::{button, ButtonType, ButtonVariant};
use mosaic_tiles::link::link;
use mosaic_tiles::table::{table, table_cell, table_head_cell};

/// One approved record, as the list renders it.
pub struct CollectionRow<'a> {
    pub id: &'a str,
    pub shortcode: &'a str,
    pub project_name: Option<&'a str>,
    pub approved_at: &'a str,
    /// `None` when no collection run has ever reported on this record.
    pub reported_at: Option<&'a str>,
    pub pull_request: Option<&'a str>,
    pub state: &'a RecordClassification,
}

/// `GET /collection`: every approved record and where its collection stands.
pub fn list(rows: &[CollectionRow<'_>]) -> Markup {
    html! {
        div class="py-8" {
            h1 class="font-display text-2xl mb-2" { "Collection" }
            p class="text-gray-600 mb-6" {
                "A reported state is only as fresh as its last collection run. The trigger is manual, so a pull \
                 request merged since the last run can still read as open here until the next one."
            }
            @if rows.is_empty() {
                p class="text-gray-600" { "No approved records are waiting on collection." }
            } @else { (collection_table(rows)) }
        }
    }
}

fn collection_table(rows: &[CollectionRow<'_>]) -> Markup {
    let actions = html! {
        span class="sr-only" { "Actions" }
    };
    let head = html! {
        tr {
            (table_head_cell("Shortcode"))
            (table_head_cell("Project"))
            (table_head_cell("Approved"))
            (table_head_cell("Status"))
            (table_head_cell("Last reported"))
            (table_head_cell(actions))
        }
    };
    let body = html! {
        @for row in rows { (collection_row(row)) }
    };
    html! {
        (table("Approved records").head(head).body(body))
    }
}

fn collection_row(row: &CollectionRow<'_>) -> Markup {
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
            (table_cell(last_reported_cell(row)))
            (table_cell(discard_control(row)))
        }
    }
}

/// The badge naming where a record's collection stands. Exhaustive with no
/// wildcard arm, so a variant added later fails to compile here rather than
/// falling through to a mislabelled row.
fn status_label_and_variant(state: &RecordClassification) -> (&'static str, BadgeVariant) {
    match state {
        // Unreachable in practice: the startup pass discards a matching record
        // before this page ever reads it. Named rather than folded into a
        // catch-all, for the same reason the whole match has none.
        RecordClassification::Published => ("Published", BadgeVariant::Success),
        RecordClassification::AwaitingCollection { .. } => ("Awaiting collection", BadgeVariant::Info),
        RecordClassification::PullRequestOpen { .. } => ("Pull request open", BadgeVariant::Info),
        RecordClassification::PullRequestClosed { .. } => ("Closed, will retry", BadgeVariant::Warning),
        RecordClassification::Stranded { .. } => ("Merged, still differs", BadgeVariant::Danger),
        RecordClassification::CollectionFailed { .. } => ("Collection failed", BadgeVariant::Danger),
        RecordClassification::RemovedUpstream => ("Project dropped upstream", BadgeVariant::Warning),
        RecordClassification::Unreadable { .. } => ("Payload unreadable", BadgeVariant::Danger),
        RecordClassification::Anomalous => ("Unexpected comparison", BadgeVariant::Danger),
    }
}

/// The badge, plus the explanatory line and pull-request link that state
/// carries. A failure's reason must render, not only its badge; the same holds
/// for a payload that failed to parse.
fn status_cell(row: &CollectionRow<'_>) -> Markup {
    let (label, variant) = status_label_and_variant(row.state);
    html! {
        div class="flex flex-col gap-1" {
            (badge(label).variant(variant))
            (status_detail(row))
            @if let Some(pull_request) = row.pull_request {
                p class="text-sm" {
                    ({
                        link("View pull request", pull_request)
                            .aria_label(
                                format!("View the pull request for {}", row.shortcode),
                            )
                    })
                }
            }
        }
    }
}

fn status_detail(row: &CollectionRow<'_>) -> Markup {
    html! {
        @match row.state {
            RecordClassification::PullRequestClosed { .. } => {
                p class="text-sm text-neutral-600" {
                    "Needs no action — the next collection run reopens it."
                }
            }
            RecordClassification::Stranded { changed } => {
                p class="text-sm text-neutral-600" {
                    "The pull request merged, but the published data still differs from what was approved: \
                     reviewer edits landed instead of the depositor's own. Differs in: "
                    (changed.join(", "))
                    "."
                }
            }
            RecordClassification::CollectionFailed { reason, .. } => {
                p class="text-sm text-neutral-600" { "Reason: " (reason) }
            }
            RecordClassification::Unreadable { problem } => {
                p class="text-sm text-neutral-600" { "Reason: " (problem) }
            }
            _ => {}
        }
    }
}

/// A record that has never been reported on has to read as visibly
/// unreported, not as a blank cell indistinguishable from a quiet row.
fn last_reported_cell(row: &CollectionRow<'_>) -> Markup {
    html! {
        @match row.reported_at {
            Some(at) => span class="text-sm" { (at) }
            None => span class="italic text-neutral-600" { "Never reported on" }
        }
    }
}

/// Offered only on a [`RecordClassification::Stranded`] row — the case it
/// exists for, since every other state is still waiting on its own. A link,
/// not a button: it opens the confirmation, which is a `GET`, so the flow
/// still works with JavaScript off.
fn discard_control(row: &CollectionRow<'_>) -> Markup {
    html! {
        @match row.state {
            RecordClassification::Stranded { .. } => {
                ({
                    link("Discard", format!("/collection/{}/discard", row.id))
                        .aria_label(
                            format!("Discard the approved record for {}", row.shortcode),
                        )
                })
            }
            _ => {}
        }
    }
}

/// What a discard destroys, for [`confirm_discard`].
pub struct DiscardImpact<'a> {
    pub shortcode: &'a str,
    pub project_name: Option<&'a str>,
    pub approved_at: &'a str,
    pub pull_request: Option<&'a str>,
    /// Shown so the confirmation says what is being discarded, not just which
    /// record. The list offers this control on a stranded row only, but the URL
    /// is reachable directly and record ids are public, so a reader arriving
    /// out of band would otherwise see the same page for every state.
    pub state: &'a RecordClassification,
}

/// `GET /collection/{id}/discard`: the confirmation. The approved record is
/// the only surviving copy of what was approved, so this asks once, plainly,
/// before the `POST` that deletes it.
pub fn confirm_discard(id: &str, impact: &DiscardImpact<'_>) -> Markup {
    html! {
        div class="max-w-lg py-8" {
            h1 class="font-display text-2xl mb-2" { "Discard this record?" }
            p class="text-gray-600 mb-4" {
                "This deletes the only copy of what was approved. It cannot be undone."
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
                @if let Some(pull_request) = impact.pull_request {
                    p class="text-sm mt-1" { (link("View the pull request", pull_request)) }
                }
            }
            form
                method="post"
                action={ "/collection/" (id) "/discard" }
                class="flex items-center gap-3"
            {
                ({
                    button("Discard permanently")
                        .button_type(ButtonType::Submit)
                        .variant(ButtonVariant::Secondary)
                })
                (link("Cancel", "/collection"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(
        id: &'a str,
        shortcode: &'a str,
        reported_at: Option<&'a str>,
        pull_request: Option<&'a str>,
        state: &'a RecordClassification,
    ) -> CollectionRow<'a> {
        CollectionRow {
            id,
            shortcode,
            project_name: Some("Bernoulli-Euler Online"),
            approved_at: "2026-09-01 08:00 UTC",
            reported_at,
            pull_request,
            state,
        }
    }

    #[test]
    fn a_collection_failed_row_renders_its_reason() {
        let state = RecordClassification::CollectionFailed {
            changed: vec!["name".to_string()],
            reason: "GitHub API returned 500".to_string(),
        };
        let out = list(&[row("1", "0801", None, None, &state)]).into_string();
        assert!(out.contains("GitHub API returned 500"), "{out}");
    }

    #[test]
    fn a_never_reported_record_says_so_and_a_reported_one_shows_its_timestamp() {
        let state = RecordClassification::AwaitingCollection { changed: vec!["name".to_string()] };
        let never = list(&[row("1", "0801", None, None, &state)]).into_string();
        assert!(never.contains("Never reported on"), "{never}");

        let reported = list(&[row("1", "0801", Some("2026-09-02 09:00 UTC"), None, &state)]).into_string();
        assert!(reported.contains("2026-09-02 09:00 UTC"), "{reported}");
        assert!(!reported.contains("Never reported on"), "{reported}");
    }

    #[test]
    fn the_discard_control_appears_only_on_a_stranded_row_and_is_a_link() {
        let stranded = RecordClassification::Stranded { changed: vec!["name".to_string()] };
        let out = list(&[row("42", "0801", None, None, &stranded)]).into_string();
        assert!(out.contains(r#"<a href="/collection/42/discard""#), "{out}");

        for state in [
            RecordClassification::Published,
            RecordClassification::AwaitingCollection { changed: vec!["name".to_string()] },
            RecordClassification::PullRequestOpen { changed: vec!["name".to_string()] },
            RecordClassification::PullRequestClosed { changed: vec!["name".to_string()] },
            RecordClassification::CollectionFailed {
                changed: vec!["name".to_string()],
                reason: "boom".to_string(),
            },
            RecordClassification::RemovedUpstream,
            RecordClassification::Unreadable { problem: "boom".to_string() },
            RecordClassification::Anomalous,
        ] {
            let out = list(&[row("42", "0801", None, None, &state)]).into_string();
            assert!(!out.contains("/collection/42/discard"), "{state:?}: {out}");
        }
    }

    #[test]
    fn confirm_discard_posts_and_offers_a_cancel_link_back_to_the_list() {
        let stranded = RecordClassification::Stranded { changed: vec!["name".to_string()] };
        let impact = DiscardImpact {
            shortcode: "0801",
            project_name: Some("Bernoulli-Euler Online"),
            approved_at: "2026-09-01 08:00 UTC",
            pull_request: Some("https://github.com/dasch-swiss/dasch-specs/pull/123"),
            state: &stranded,
        };
        let out = confirm_discard("42", &impact).into_string();
        assert!(out.contains(r#"<form method="post" action="/collection/42/discard""#), "{out}");
        assert!(
            out.contains("Merged, still differs"),
            "the confirmation must name the state it is discarding: {out}"
        );
        assert!(out.contains(r#"href="/collection""#), "{out}");
        assert!(out.contains("cannot be undone"), "{out}");
        assert!(out.contains("https://github.com/dasch-swiss/dasch-specs/pull/123"), "{out}");
    }

    #[test]
    fn snapshot_the_collection_list_over_every_classification() {
        let published = RecordClassification::Published;
        let awaiting = RecordClassification::AwaitingCollection { changed: vec!["name".to_string()] };
        let pr_open = RecordClassification::PullRequestOpen { changed: vec!["name".to_string()] };
        let pr_closed = RecordClassification::PullRequestClosed { changed: vec!["name".to_string()] };
        let stranded = RecordClassification::Stranded { changed: vec!["name".to_string(), "abstract".to_string()] };
        let failed = RecordClassification::CollectionFailed {
            changed: vec!["name".to_string()],
            reason: "GitHub API returned 500".to_string(),
        };
        let removed = RecordClassification::RemovedUpstream;
        let unreadable = RecordClassification::Unreadable { problem: "invalid type: string, expected map".to_string() };
        let anomalous = RecordClassification::Anomalous;

        let pr = "https://github.com/dasch-swiss/dasch-specs/pull/123";
        let rows = vec![
            row("1", "0801", Some("2026-09-02 09:00 UTC"), None, &published),
            row("2", "0802", None, None, &awaiting),
            row("3", "0803", Some("2026-09-02 09:00 UTC"), Some(pr), &pr_open),
            row("4", "0804", Some("2026-09-02 09:00 UTC"), Some(pr), &pr_closed),
            row("5", "0805", Some("2026-09-02 09:00 UTC"), Some(pr), &stranded),
            row("6", "0806", Some("2026-09-02 09:00 UTC"), None, &failed),
            row("7", "0807", Some("2026-09-02 09:00 UTC"), None, &removed),
            row("8", "0808", Some("2026-09-02 09:00 UTC"), None, &unreadable),
            row("9", "0809", Some("2026-09-02 09:00 UTC"), None, &anomalous),
        ];
        insta::assert_snapshot!("collection_list", list(&rows).into_string());
    }
}
