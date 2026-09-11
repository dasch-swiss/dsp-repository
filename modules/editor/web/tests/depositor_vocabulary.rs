//! REQ-2.2: the words a depositor must never be shown.
//!
//! "The editor shall not expose the words *export*, *JSON*, *transfer*,
//! *commit* or *pull request* to a depositor." Every one of them names a
//! mechanism rather than an outcome — a depositor is publishing their project's
//! metadata, and how it gets there is the editor's problem.
//!
//! ## Rendered text, not source strings
//!
//! The assertion runs against **rendered markup with its tags stripped**, which
//! is what a depositor actually reads. A grep over the source would both miss
//! and over-report: it cannot see a word assembled from a constant and a
//! `format!`, and it fires on every doc comment and identifier that mentions
//! the mechanism deliberately — this file among them.
//!
//! Attribute values are dropped with the tags. That is correct here and
//! deliberate: `href="/projects/0801d"` and `class="..."` are not prose, and a
//! word inside one is not a word the reader is shown. The one attribute that
//! *is* read aloud is `aria-label`, so those are kept; see [`visible_text`].
//!
//! The browser-level half of this requirement lives in the E2E suite, which
//! drives the real pages including the ones assembled by `editor-server`.

use editor_core::draft::ProjectDraft;
use editor_core::published::{ProjectSummary, PublishedProjects};
use editor_core::records::ReviewOutcome;
use editor_core::status::ProjectState;
use editor_web::form::registry::{self, Audience};
use editor_web::pages::projects::AssignedProject;
use editor_web::pages::{projects, section, states};

/// The words REQ-2.2 closes out, lowercased for a case-folded search.
///
/// `pull request` is two words on purpose: the requirement names the phrase,
/// and "request" alone is ordinary English the editor needs ("request changes").
const FORBIDDEN: &[&str] = &["export", "json", "transfer", "commit", "pull request"];

/// The text a reader is actually shown: markup with tags removed, plus the
/// `aria-label` values a screen reader announces.
///
/// Crude on purpose — a real HTML parser would be a dependency bought to make a
/// string search marginally tidier, and the inputs are this crate's own markup
/// rather than arbitrary documents.
fn visible_text(markup: &str) -> String {
    let mut text = String::with_capacity(markup.len());
    let mut rest = markup;
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        text.push(' ');
        let Some(close) = rest[open..].find('>') else { break };
        let tag = &rest[open..open + close];
        // Announced to a screen reader, so it is read text even though it lives
        // in an attribute.
        if let Some(label) = tag.split("aria-label=\"").nth(1).and_then(|rest| rest.split('"').next()) {
            text.push_str(label);
            text.push(' ');
        }
        rest = &rest[open + close + 1..];
    }
    text.push_str(rest);
    text.to_lowercase()
}

/// Every word found in `markup`, so a failure names all of them at once.
fn forbidden_words_in(markup: &str) -> Vec<&'static str> {
    let text = visible_text(markup);
    FORBIDDEN.iter().copied().filter(|word| text.contains(word)).collect()
}

fn published() -> PublishedProjects {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dpe/server/data/projects"));
    let (published, errors) = PublishedProjects::load_from(dir);
    assert!(errors.is_empty(), "{errors:?}");
    published
}

#[test]
fn the_state_vocabulary_itself_carries_none_of_the_forbidden_words() {
    // The labels and explanations are the normative strings REQ-2.1 closes, and
    // they are rendered on three different surfaces. Checking them directly
    // means a new state cannot slip a forbidden word past the page tests.
    for state in ProjectState::ALL {
        assert!(
            forbidden_words_in(state.label()).is_empty(),
            "{}: {:?}",
            state.label(),
            forbidden_words_in(state.label())
        );
        assert!(
            forbidden_words_in(state.explanation()).is_empty(),
            "{}: {:?}",
            state.label(),
            forbidden_words_in(state.explanation())
        );
    }
}

#[test]
fn the_state_explanation_page_carries_none_of_the_forbidden_words() {
    let markup = states::explanation().into_string();
    assert!(
        forbidden_words_in(&markup).is_empty(),
        "{:?} in:\n{markup}",
        forbidden_words_in(&markup)
    );
}

#[test]
fn the_project_list_carries_none_of_the_forbidden_words_in_any_state() {
    // Every state, because the column's text is the state's own label and a
    // list rendered in one state says nothing about the other four.
    for state in ProjectState::ALL {
        let rows = [AssignedProject {
            summary: ProjectSummary { shortcode: "0801d", name: "A Project", status: "ongoing" },
            state,
        }];
        let markup = projects::assigned(&rows, 1).into_string();
        assert!(
            forbidden_words_in(&markup).is_empty(),
            "{:?} in the list for {}:\n{markup}",
            forbidden_words_in(&markup),
            state.label()
        );
    }
    // And the two empty states, which are prose rather than a table.
    for assignments in [0, 2] {
        let markup = projects::assigned(&[], assignments).into_string();
        assert!(forbidden_words_in(&markup).is_empty(), "{:?}", forbidden_words_in(&markup));
    }
}

#[test]
fn every_form_section_a_depositor_sees_carries_none_of_the_forbidden_words() {
    // The largest depositor-facing surface by far: field labels, hints and the
    // obligation badges, over a real committed project so the rendered values
    // are the ones a depositor reads rather than placeholders.
    let published = published();
    let raw = published.get("0801d").expect("the fixture project");
    let draft = ProjectDraft::from_raw(raw);

    // Every review outcome, not just the absent one. The round summary is
    // depositor-facing prose that renders only in this slot, so a `None` here
    // leaves REQ-2.2 unchecked over the whole of it.
    let rounds = [
        None,
        Some(ReviewOutcome::Approved),
        Some(ReviewOutcome::ChangesRequested),
        Some(ReviewOutcome::Rejected),
        Some(ReviewOutcome::Withdrawn),
    ];
    for (section_def, outcome) in registry::sections_for(Audience::Everyone)
        .flat_map(|section| rounds.iter().map(move |outcome| (section, *outcome)))
    {
        let round = outcome.map(|outcome| section::RoundSummary {
            outcome,
            note: Some("A reviewer's note."),
            at: "1 January 2026",
            substitutions: &[],
        });
        let view = section::SectionView {
            shortcode: "0801d",
            project_name: Some("A Project"),
            section: section_def,
            audience: Audience::Everyone,
            draft: &draft,
            locked: None,
            accepted_fields: &[],
            may_withdraw: false,
            may_discard: true,
            confirming: None,
            errors: &[],
            proposal_findings: &[],
            proposals: &[],
            round,
            last_editor: None,
            signed_out_at: None,
            baseline: None,
            saved_at: None,
            notice: None,
            posted: None,
            adding_row: None,
            agents: None,
            rows_action: format!("/projects/0801d/sections/{}/fields", section_def.id),
            // REQ-2.5's notice is depositor-facing text too, and it is only
            // rendered in this state.
            awaiting_release: true,
        };
        let markup = section::page(&view).into_string();
        assert!(
            forbidden_words_in(&markup).is_empty(),
            "{:?} in section {} with round {outcome:?}:\n{markup}",
            forbidden_words_in(&markup),
            section_def.id
        );
    }
}

#[test]
fn the_check_would_notice_a_forbidden_word_that_was_actually_rendered() {
    // The canary. Every assertion above is an absence, and an absence passes
    // just as happily when the check is broken — a `visible_text` that returned
    // an empty string would make the whole file green.
    let markup = "<p class=\"json\">We will commit your JSON export in a pull request.</p>";
    let found = forbidden_words_in(markup);
    assert_eq!(
        found,
        ["export", "json", "commit", "pull request"],
        "the check must see rendered prose"
    );

    // And the complement: a forbidden word in an attribute that is not read out
    // must not trip it, or the check is unusable on real markup.
    assert!(
        forbidden_words_in("<a href=\"/export.json\" class=\"commit\">Your projects</a>").is_empty(),
        "a word inside a non-announced attribute is not text a depositor reads"
    );

    // An `aria-label` is announced, so it counts.
    assert_eq!(forbidden_words_in("<button aria-label=\"Export draft\"></button>"), ["export"]);
}
