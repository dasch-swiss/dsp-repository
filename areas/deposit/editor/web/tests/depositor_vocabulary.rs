//! The words a depositor must never be shown: *export*, *JSON*, *transfer*,
//! *commit* and *pull request*. Each names a mechanism rather than an outcome.
//!
//! The assertion runs against rendered markup with its tags stripped, which is
//! what a depositor reads: a grep over the source cannot see a word assembled
//! from a constant and a `format!`, and fires on every doc comment that names
//! the mechanism deliberately. Attribute values are dropped with the tags, since
//! `href` and `class` are not prose; `aria-label` is read aloud and kept, see
//! [`visible_text`]. The browser-level half lives in the E2E suite.
//!
//! A second check over the same text rejects internal identifiers: an entity id
//! (`person-042`) or a setting name (`EDITOR_RDU_EMAILS`) tells a depositor nothing.

use editor_core::agents::{AgentScope, Agents};
use editor_core::draft::ProjectDraft;
use editor_core::form::format::ValueKind;
use editor_core::form::registry::{self, Audience};
use editor_core::proposals::{EntityProposal, ProposalKind, ProposalOperation, ProposalStatus};
use editor_core::published::{ProjectSummary, PublishedProjects};
use editor_core::records::ReviewOutcome;
use editor_core::status::ProjectState;
use editor_web::entity::{self, Confirmation, EntityView};
use editor_web::pages::projects::AssignedProject;
use editor_web::pages::{projects, section, states};

/// The forbidden words, lowercased for a case-folded search. `pull request` is
/// two words on purpose: "request" alone is ordinary English the editor needs.
const FORBIDDEN: &[&str] = &["export", "json", "transfer", "commit", "pull request"];

/// The text a reader is shown: markup with tags removed, plus the `aria-label`
/// values a screen reader announces. Crude on purpose: the inputs are this
/// crate's own markup, not arbitrary documents.
fn visible_text(markup: &str) -> String {
    read_text(markup).to_lowercase()
}

/// [`visible_text`] with its case kept, for the identifier check: a setting name is upper case.
fn read_text(markup: &str) -> String {
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
    text
}

/// Every word found in `markup`, so a failure names all of them at once.
fn forbidden_words_in(markup: &str) -> Vec<&'static str> {
    let text = visible_text(markup);
    FORBIDDEN.iter().copied().filter(|word| text.contains(word)).collect()
}

/// Every internal identifier found in what a depositor reads: `person-NNN` and
/// `organization-NNN` as whole words, and `EDITOR_` setting names. A hand-rolled scan: the
/// crate carries no regex dependency for one test file.
fn internal_identifiers_in(markup: &str) -> Vec<String> {
    let text = read_text(markup);
    let bytes = text.as_bytes();
    let is_word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let mut found = Vec::new();
    for (start, _) in text.char_indices() {
        if start > 0 && is_word(bytes[start - 1]) {
            continue;
        }
        let rest = &text[start..];
        let end = if let Some(after) = rest.strip_prefix("person-").or_else(|| rest.strip_prefix("organization-")) {
            let digits = after.bytes().take_while(u8::is_ascii_digit).count();
            let tail_is_boundary = after.as_bytes().get(digits).is_none_or(|byte| !is_word(*byte));
            (digits > 0 && tail_is_boundary).then(|| rest.len() - after.len() + digits)
        } else if let Some(after) = rest.strip_prefix("EDITOR_") {
            let name = after
                .bytes()
                .take_while(|byte| byte.is_ascii_uppercase() || *byte == b'_')
                .count();
            (name > 0).then(|| "EDITOR_".len() + name)
        } else {
            None
        };
        if let Some(end) = end {
            found.push(rest[..end].to_string());
        }
    }
    found
}

fn published() -> PublishedProjects {
    let dir = editor_core::checkout_dpe_data_dir().join("projects");
    let (published, errors) = PublishedProjects::load_from(&dir);
    assert!(errors.is_empty(), "{errors:?}");
    published
}

#[test]
fn the_state_vocabulary_itself_carries_none_of_the_forbidden_words() {
    // The normative strings, rendered on three surfaces; checking them directly
    // means a new state cannot slip a word past the page tests.
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
        let rows = [
            AssignedProject {
                summary: ProjectSummary { shortcode: "0801d", name: "A Project", status: "ongoing" },
                state,
            },
            AssignedProject {
                summary: ProjectSummary {
                    shortcode: "08AB",
                    name: "A Created Project",
                    status: projects::NOT_PUBLISHED_YET,
                },
                state,
            },
        ];
        let markup = projects::assigned(&rows, &["9999"]).into_string();
        assert!(
            forbidden_words_in(&markup).is_empty(),
            "{:?} in the list for {}:\n{markup}",
            forbidden_words_in(&markup),
            state.label()
        );
    }
    // And the two empty states, which are prose rather than a table.
    for unknown in [&[][..], &["9999"][..]] {
        let markup = projects::assigned(&[], unknown).into_string();
        assert!(forbidden_words_in(&markup).is_empty(), "{:?}", forbidden_words_in(&markup));
    }
}

#[test]
fn every_form_section_a_depositor_sees_carries_none_of_the_forbidden_words() {
    // The largest depositor-facing surface, over a real committed project so the
    // rendered values are the ones a depositor reads.
    let published = published();
    let raw = published.get("0801d").expect("the fixture project");
    let draft = ProjectDraft::from_raw(raw);
    // Resolving agents, so a contributor row shows a name: unresolved, the row falls back to
    // the id and the identifier check would be testing the fixture rather than the view.
    let agents = agents();
    let scope = AgentScope::published_only(&agents);
    // Every review outcome: the round summary is depositor-facing prose that
    // renders only in this slot.
    let rounds = [
        None,
        Some(ReviewOutcome::Approved),
        Some(ReviewOutcome::ChangesRequested),
        Some(ReviewOutcome::Rejected),
        Some(ReviewOutcome::Withdrawn),
    ];
    // A created project's discard confirmation and notice have their own copy.
    for ((section_def, outcome), created) in registry::sections_for(Audience::Everyone)
        .flat_map(|section| rounds.iter().map(move |outcome| (section, *outcome)))
        .flat_map(|pair| [(pair, false), (pair, true)])
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
            created,
            section: section_def,
            audience: Audience::Everyone,
            draft: &draft,
            locked: None,
            accepted_fields: &[],
            may_withdraw: false,
            may_discard: true,
            confirming: created.then_some(section::Confirmation::Discard),
            errors: &[],
            proposal_findings: &[],
            proposals: &[],
            round,
            last_editor: None,
            signed_out_at: None,
            baseline: None,
            saved_at: None,
            notice: created.then_some(section::Notice::Discarded),
            posted: None,
            format_findings: Vec::new(),
            findings_elsewhere: Vec::new(),
            submitted: None,
            adding_row: None,
            focus_finding: false,
            without_language: Vec::new(),
            agents: Some(&scope),
            rows_action: format!("/projects/0801d/sections/{}/fields", section_def.id),
            // The approved box, shown while a release is awaited, is depositor-facing text too.
            awaiting_release: true,
        };
        let markup = section::page(&view).into_string();
        assert!(
            forbidden_words_in(&markup).is_empty(),
            "{:?} in section {} with round {outcome:?}, created {created}:\n{markup}",
            forbidden_words_in(&markup),
            section_def.id
        );
        assert!(
            internal_identifiers_in(&markup).is_empty(),
            "{:?} in section {} with round {outcome:?}, created {created}:\n{markup}",
            internal_identifiers_in(&markup),
            section_def.id
        );
        if created {
            assert!(
                markup.contains("since the project was created"),
                "the created copy renders: {markup}"
            );
        }
    }
}

#[test]
fn no_format_message_or_hint_uses_a_forbidden_word() {
    // A message renders only beside a value that fails its rule, which no fixture above holds.
    for kind in [ValueKind::Link, ValueKind::DoiOrLink, ValueKind::LinkOrNotAccessible] {
        for text in [
            editor_web::form::widgets::format_message(kind),
            editor_web::form::widgets::format_hint(kind),
        ] {
            assert!(forbidden_words_in(text).is_empty(), "{kind:?}: {text}");
        }
    }
}

#[test]
fn the_check_would_notice_a_forbidden_word_that_was_actually_rendered() {
    // The canary: every assertion above is an absence, and an absence passes
    // when the check is broken.
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

#[test]
fn the_identifier_check_would_notice_an_id_or_a_setting_name_that_was_actually_rendered() {
    assert_eq!(
        internal_identifiers_in("<p>Ask person-042 or organization-008 about EDITOR_RDU_EMAILS.</p>"),
        ["person-042", "organization-008", "EDITOR_RDU_EMAILS"]
    );
    assert!(internal_identifiers_in("<p>A person-centred, non-person-42x view.</p>").is_empty());
    assert!(internal_identifiers_in("<a href=\"/entities/person-042\">Ada (person)</a>").is_empty());
}

fn agents() -> Agents {
    let dir = editor_core::checkout_dpe_data_dir();
    let (agents, errors) = Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
    assert!(errors.is_empty(), "{errors:?}");
    agents
}

fn a_change_proposal(agents: &Agents, entity_id: &str, kind: ProposalKind) -> EntityProposal {
    let payload = agents.seed_payload(entity_id).expect("a published agent");
    EntityProposal {
        id: Default::default(),
        shortcode: "0801d".to_string(),
        entity_id: entity_id.to_string(),
        kind,
        operation: ProposalOperation::Change,
        payload: payload.to_string(),
        status: ProposalStatus::Draft,
        decision: None,
        proposed_by: None,
        created_at: Default::default(),
        updated_at: Default::default(),
        decided_by: None,
        decided_at: None,
        retired_at: None,
    }
}

#[test]
fn the_entity_page_and_its_discard_confirmation_name_the_entity_instead_of_its_id() {
    // The fixture agents carry names, so an id on screen would be a fallback taken where a name
    // was available, not a record without one.
    let agents = agents();
    for (entity_id, kind) in [
        ("organization-008", ProposalKind::Organization),
        ("person-001", ProposalKind::Person),
    ] {
        let name = agents.get(entity_id).expect("a published agent").label.clone();
        assert!(
            !name.trim().is_empty() && name != entity_id,
            "the fixture needs a name: {name:?}"
        );
        let proposal = a_change_proposal(&agents, entity_id, kind);
        let scope = AgentScope::with_proposals(&agents, std::slice::from_ref(&proposal));
        let draft = serde_json::from_str::<ProjectDraft>(&proposal.payload).expect("a stored payload");
        for confirming in [None, Some(Confirmation::Discard)] {
            let view = EntityView {
                origin: None,
                shortcode: "0801d",
                proposal: &proposal,
                over: None,
                draft: &draft,
                confirming,
                notice: None,
                posted: None,
                adding_row: None,
                agents: Some(&scope),
                format_findings: Vec::new(),
                rows_action: format!("/projects/0801d/entities/{entity_id}/fields"),
            };
            let markup = entity::page(&view).into_string();
            assert!(
                markup.contains(&format!("({})", kind.label().to_lowercase())),
                "the heading must carry the resolved name and kind:\n{markup}"
            );
            assert!(
                forbidden_words_in(&markup).is_empty(),
                "{:?}:\n{markup}",
                forbidden_words_in(&markup)
            );
            // The form's own `person-NNN` hidden values are attributes, not read text.
            assert!(
                internal_identifiers_in(&markup).is_empty(),
                "{:?} in the entity page for {entity_id} ({confirming:?}):\n{markup}",
                internal_identifiers_in(&markup)
            );
        }
    }
}

#[test]
fn the_new_proposal_notice_names_no_id() {
    let published = published();
    let raw = published.get("0801d").expect("the fixture project");
    let draft = ProjectDraft::from_raw(raw);
    let agents = agents();
    let scope = AgentScope::published_only(&agents);
    let section_def = registry::sections_for(Audience::Everyone).next().expect("a section");
    for (kind, entity_id) in [
        (ProposalKind::Organization, "organization-900"),
        (ProposalKind::Person, "person-900"),
    ] {
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
            round: None,
            last_editor: None,
            signed_out_at: None,
            baseline: None,
            saved_at: None,
            notice: Some(section::Notice::Proposed { kind, entity_id }),
            posted: None,
            submitted: None,
            adding_row: None,
            agents: Some(&scope),
            created: false,
            focus_finding: false,
            without_language: Vec::new(),
            format_findings: Vec::new(),
            findings_elsewhere: Vec::new(),
            rows_action: format!("/projects/0801d/sections/{}/fields", section_def.id),
            awaiting_release: false,
        };
        let markup = section::page(&view).into_string();
        assert!(
            internal_identifiers_in(&markup).is_empty(),
            "{:?} in the {kind:?} notice:\n{markup}",
            internal_identifiers_in(&markup)
        );
    }
}

#[test]
fn the_proposals_box_and_its_findings_name_each_proposal_and_carry_no_id() {
    // `proposals_summary` renders only in a section with an agent field, and only for a live
    // proposal; `proposal_findings_list` only for a finding. Both slots are empty in the page
    // tests above, so a view that printed an id there would pass them.
    let published = published();
    let raw = published.get("0801d").expect("the fixture project");
    let draft = ProjectDraft::from_raw(raw);
    let agents = agents();
    let proposals: Vec<EntityProposal> = [
        ("organization-008", ProposalKind::Organization),
        ("person-001", ProposalKind::Person),
    ]
    .into_iter()
    .map(|(entity_id, kind)| a_change_proposal(&agents, entity_id, kind))
    .collect();
    let scope = AgentScope::with_proposals(&agents, &proposals);
    let findings: Vec<(String, ProposalKind, String)> = proposals
        .iter()
        .map(|proposal| (proposal.entity_id.clone(), proposal.kind, "Needs finishing.".to_string()))
        .collect();
    let section_def = registry::sections_for(Audience::Everyone)
        .find(|section| section.id == "contributors")
        .expect("the section holding the agent fields");
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
        proposal_findings: &findings,
        proposals: &proposals,
        round: None,
        last_editor: None,
        signed_out_at: None,
        baseline: None,
        saved_at: None,
        notice: None,
        posted: None,
        submitted: None,
        adding_row: None,
        agents: Some(&scope),
        created: false,
        focus_finding: false,
        without_language: Vec::new(),
        format_findings: Vec::new(),
        findings_elsewhere: Vec::new(),
        rows_action: format!("/projects/0801d/sections/{}/fields", section_def.id),
        awaiting_release: false,
    };
    let markup = section::page(&view).into_string();
    let text = read_text(&markup);
    assert!(
        text.contains("Proposed persons and organisations"),
        "the proposals box did not render:\n{markup}"
    );
    assert!(
        text.contains("A person or organisation you started needs finishing"),
        "the findings alert did not render:\n{markup}"
    );
    for proposal in &proposals {
        let name = &agents.get(&proposal.entity_id).expect("a published agent").label;
        let label = format!("{name} ({})", proposal.kind.label().to_lowercase());
        assert_eq!(
            text.matches(&label).count(),
            2,
            "{label:?} must be named once in the box and once in the findings:\n{markup}"
        );
    }
    assert!(
        internal_identifiers_in(&markup).is_empty(),
        "{:?} in the proposals box:\n{markup}",
        internal_identifiers_in(&markup)
    );
    assert!(
        forbidden_words_in(&markup).is_empty(),
        "{:?} in the proposals box:\n{markup}",
        forbidden_words_in(&markup)
    );
}
