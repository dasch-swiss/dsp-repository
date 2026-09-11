//! What a depositor is told about a project, and how the startup comparison
//! decides it.
//!
//! [`ProjectState`] is the five-value vocabulary REQ-2.1 makes normative — the
//! *only* states a depositor may see. [`Comparison`] is what reading the
//! published set against a local record yields at startup (REQ-2.3), and the
//! input to the Online decision (REQ-2.4).
//!
//! **The comparison is [`crate::review::diff`], not a second one.** It already
//! compares member by member and already models an absent published side as
//! `None`. A comparison written here would be a second definition of "changed"
//! from the one RDU reviews through, and the two would drift.
//!
//! **Nothing canonicalises, sorts or serialises before comparing, and nothing
//! should.** A draft holds members as `serde_json::Value`, whose `Map` equality
//! delegates to its backing map; both a `BTreeMap` and the `IndexMap` from
//! `preserve_order` compare by key rather than position. Language-map key order
//! therefore cannot register as a change, as a property of the types rather
//! than a rule a caller has to keep. Pinned by
//! [`tests::language_map_key_order_is_not_a_change`].
use platform_metadata::project::ProjectRaw;

use crate::draft::ProjectDraft;
use crate::records::SubmissionState;
use crate::review::diff;

/// The states REQ-2.1 permits a depositor to see, and no others.
///
/// A superset of [`SubmissionState`], which is only what the database stores:
/// `Draft` is a `drafts` row and `Online` is derived, so neither has a stored
/// form. [`Self::label`] and [`Self::explanation`] are the normative
/// depositor-facing strings REQ-2.2 bounds; three surfaces read them from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectState {
    /// Being edited: a `drafts` row exists, or the project is not published.
    Draft,
    /// Sent for review, not yet picked up.
    Submitted,
    /// A reviewer has it.
    InReview,
    /// Accepted, waiting for the repository release that carries it.
    Approved,
    /// The published set carries the change. The local record is gone.
    Online,
}

impl ProjectState {
    /// The depositor-facing spelling, checked against REQ-2.2's forbidden words.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Submitted => "Submitted",
            Self::InReview => "In review",
            Self::Approved => "Approved",
            Self::Online => "Online",
        }
    }

    /// Every state, so a legend or an assertion cannot miss one by hand.
    pub const ALL: [Self; 5] = [
        Self::Draft,
        Self::Submitted,
        Self::InReview,
        Self::Approved,
        Self::Online,
    ];

    /// What the state-explanation page (REQ-2.6) says this state means. Here
    /// rather than in a view because REQ-2.2 makes the wording normative.
    #[must_use]
    pub const fn explanation(self) -> &'static str {
        match self {
            Self::Draft => {
                "You are editing this project. Nothing has been sent to DaSCH yet, and you can \
                 change anything. Your work is saved as you go."
            }
            Self::Submitted => {
                "You have sent your changes to DaSCH. They are queued for a reviewer, and you \
                 cannot edit the project while it is waiting."
            }
            Self::InReview => {
                "A DaSCH reviewer is reading your changes. They may accept them, suggest \
                 different wording, or send the project back to you with a note."
            }
            Self::Approved => {
                "DaSCH has accepted your changes. They will appear on the public site with the \
                 next repository release — usually within a few weeks."
            }
            Self::Online => {
                "Your changes are live on the public site. The project now shows what you \
                 submitted, and you can start editing it again whenever you like."
            }
        }
    }
}

/// What the startup comparison found for one project (REQ-2.3).
///
/// REQ-2.3 names three branches and admits a fourth by omission: a project the
/// published set *dropped* while a local record survives. Presence alone cannot
/// tell that from "never published", so [`Self::classify`] separates them by
/// whether the record carries the `id`/`pid` assigned on first publication —
/// treating a deletion as a new project would offer to republish something
/// removed on purpose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    /// In the published set, with no local record: unchanged.
    Unchanged,
    /// Neither side has this project. Separate from [`Self::Unchanged`]
    /// because that one is the resting state read as Online, and "nothing
    /// anywhere" must never be read that way.
    Absent,
    /// Local only, and never published — a new project. Unreachable in v1,
    /// which edits existing projects only (no requirement allocates a project's
    /// `id`, `pid` or `shortcode`, and REQ-1.5 makes all three display-only).
    /// Kept because REQ-2.3 names the branch and a v2 that adds creation should
    /// find it here rather than invent it.
    NewAndUnpublished,
    /// In both, and publishing the local record would change nothing. The
    /// precondition for Online (REQ-2.4).
    Matches,
    /// In both, and they differ. Carries the member names that differ, so a
    /// collision can name them rather than saying only that something moved.
    Differs { changed: Vec<String> },
    /// A local record exists for a project the published set no longer holds.
    ///
    /// The fourth branch. Never resolved automatically: the local record is the
    /// only surviving copy of the depositor's work, and an upstream deletion is
    /// not evidence that it should be destroyed.
    RemovedUpstream,
}

impl Comparison {
    /// Read the published side against the local side. `local` is whichever
    /// record's draft the caller picked; this only compares.
    #[must_use]
    pub fn classify(published: Option<&ProjectRaw>, local: Option<&ProjectDraft>) -> Self {
        match (published, local) {
            (Some(_), None) => Self::Unchanged,
            (None, None) => Self::Absent,
            (None, Some(local)) => {
                if was_published(local) {
                    Self::RemovedUpstream
                } else {
                    Self::NewAndUnpublished
                }
            }
            (Some(published), Some(local)) => {
                let published = ProjectDraft::from_raw(published);
                let changed: Vec<String> = diff(Some(&published), local)
                    .into_iter()
                    .filter(|row| row.changed())
                    .map(|row| row.field)
                    .collect();
                if changed.is_empty() {
                    Self::Matches
                } else {
                    Self::Differs { changed }
                }
            }
        }
    }

    /// Whether this comparison lets an Approved record go Online (REQ-2.4).
    /// [`Self::Matches`] only — `Unchanged` says there was no record to
    /// compare, not that a change shipped.
    #[must_use]
    pub const fn permits_online(&self) -> bool {
        matches!(self, Self::Matches)
    }

    /// Whether the published set carries this project at all: what separates
    /// "published, nothing pending" (read as Online) from "not published".
    #[must_use]
    pub const fn is_published(&self) -> bool {
        matches!(self, Self::Unchanged | Self::Matches | Self::Differs { .. })
    }
}

/// Whether this local record describes a project that was published once.
///
/// `id` and `pid` are assigned on first publication and are display-only
/// (REQ-1.5), so a depositor can neither set nor clear them — which makes this
/// a property of the data rather than a flag a write path must remember.
fn was_published(local: &ProjectDraft) -> bool {
    ["id", "pid"].iter().any(|field| {
        local
            .get(field)
            .and_then(|value| value.as_str())
            .is_some_and(|text| !text.trim().is_empty())
    })
}

/// The state a depositor is shown for one project.
///
/// **Online is the resting state, not a moment, and has to be.** REQ-2.4
/// discards the local record once its data is published and
/// [`crate::records::ReviewRound`] does not keep the approved payload, so after
/// the discard nothing records that a change shipped. Were Online only the
/// instant of the transition, no depositor would ever see it. So Online is what
/// a published project with nothing pending *is*; it drops out the moment
/// something is pending again. This is also what makes the five states total
/// over every project, which REQ-2.1 needs.
///
/// A live submission outranks a draft: a project cannot be both under review
/// and editable.
#[must_use]
pub fn depositor_state(
    submission: Option<SubmissionState>,
    approved: bool,
    has_draft: bool,
    comparison: &Comparison,
) -> ProjectState {
    if let Some(state) = submission {
        return match state {
            SubmissionState::Submitted => ProjectState::Submitted,
            SubmissionState::InReview => ProjectState::InReview,
            // An `approved` submission row is the window between the reviewer's
            // decision and the record being written; the depositor is told the
            // same thing either way.
            SubmissionState::Approved => ProjectState::Approved,
        };
    }
    if approved {
        // REQ-2.4 as a rendering rule: the startup pass discards a matching
        // record, but a record approved *while this process is running* has not
        // met a startup yet, and reading it as Approved until the next restart
        // would be a stale label the page can avoid for free.
        return if comparison.permits_online() {
            ProjectState::Online
        } else {
            ProjectState::Approved
        };
    }
    if has_draft {
        return ProjectState::Draft;
    }
    // Nothing pending. Published means the public site is current (Online);
    // unpublished means there is nothing live to be current, so the project is
    // back at the start.
    if comparison.is_published() {
        ProjectState::Online
    } else {
        ProjectState::Draft
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn draft(value: serde_json::Value) -> ProjectDraft {
        serde_json::from_value(value).expect("an object deserializes as a draft")
    }

    fn published_raw() -> ProjectRaw {
        crate::test_support::sample_raw()
    }

    #[test]
    fn published_with_no_local_record_is_unchanged() {
        assert_eq!(Comparison::classify(Some(&published_raw()), None), Comparison::Unchanged);
    }

    #[test]
    fn an_identical_local_record_matches_and_permits_online() {
        let raw = published_raw();
        let local = ProjectDraft::from_raw(&raw);
        let comparison = Comparison::classify(Some(&raw), Some(&local));
        assert_eq!(comparison, Comparison::Matches);
        assert!(comparison.permits_online());
    }

    #[test]
    fn a_changed_member_is_named_rather_than_only_counted() {
        // A collision has to say which field moved; "something changed" sends
        // RDU to diff two files by hand.
        let raw = published_raw();
        let mut local = ProjectDraft::from_raw(&raw);
        local.set("name", json!("A Different Name"));
        let comparison = Comparison::classify(Some(&raw), Some(&local));
        let Comparison::Differs { changed } = &comparison else {
            panic!("expected Differs, got {comparison:?}");
        };
        assert_eq!(changed, &["name"]);
        assert!(!comparison.permits_online());
    }

    #[test]
    fn language_map_key_order_is_not_a_change() {
        // The requirement the issue calls out. It holds because `Value`'s
        // object equality is an `IndexMap` key lookup rather than a positional
        // walk — so this passes without anything sorting or canonicalising
        // first, and would fail loudly if that ever stopped being true.
        let mut raw = published_raw();
        raw.description = serde_json::from_value(json!({ "de": "Deutscher Text", "en": "English text" }))
            .expect("a multilingual map deserializes");

        // The published side goes through `Multilingual`, a `BTreeMap`, so it
        // is alphabetical: de, then en. The local side is set directly, in the
        // opposite order.
        let mut local = ProjectDraft::from_raw(&raw);
        local.set("description", json!({ "en": "English text", "de": "Deutscher Text" }));

        // Canary: the two sides must really be in different key order, or the
        // assertion below holds for a reason that has nothing to do with the
        // property being pinned. Serialising is how order becomes observable at
        // all — `PartialEq` is exactly what ignores it.
        let published_side = ProjectDraft::from_raw(&raw);
        assert_ne!(
            serde_json::to_string(published_side.get("description").expect("published description"))
                .expect("serializes"),
            serde_json::to_string(local.get("description").expect("local description")).expect("serializes"),
            "the fixture must hold the same tags in different order for this test to mean anything"
        );

        assert_eq!(
            Comparison::classify(Some(&raw), Some(&local)),
            Comparison::Matches,
            "key order must not register as a change"
        );
    }

    #[test]
    fn a_local_record_for_a_project_dropped_upstream_is_its_own_branch() {
        // Not "new and unpublished": the record carries the id and pid DaSCH
        // assigned, so this project was published and has been removed.
        let local = ProjectDraft::from_raw(&published_raw());
        assert_eq!(Comparison::classify(None, Some(&local)), Comparison::RemovedUpstream);
    }

    #[test]
    fn a_local_record_that_was_never_published_is_new_rather_than_removed() {
        let local = draft(json!({ "id": "", "pid": "", "name": "Brand New" }));
        assert_eq!(Comparison::classify(None, Some(&local)), Comparison::NewAndUnpublished);
    }

    #[test]
    fn a_record_with_no_id_members_at_all_is_new_rather_than_removed() {
        let local = draft(json!({ "name": "Brand New" }));
        assert_eq!(Comparison::classify(None, Some(&local)), Comparison::NewAndUnpublished);
    }

    #[test]
    fn approved_goes_online_only_once_the_published_data_matches() {
        assert_eq!(
            depositor_state(None, true, false, &Comparison::Matches),
            ProjectState::Online,
            "REQ-2.4: an approved record matching published data is Online"
        );
        assert_eq!(
            depositor_state(None, true, false, &Comparison::Differs { changed: vec!["name".into()] }),
            ProjectState::Approved,
            "REQ-2.5: still waiting for the release that carries it"
        );
    }

    #[test]
    fn a_live_submission_outranks_a_draft_row() {
        // A project cannot be both under review and editable; the submission is
        // what the depositor is waiting on.
        assert_eq!(
            depositor_state(Some(SubmissionState::InReview), false, true, &Comparison::Unchanged),
            ProjectState::InReview
        );
        assert_eq!(
            depositor_state(Some(SubmissionState::Submitted), false, true, &Comparison::Unchanged),
            ProjectState::Submitted
        );
    }

    #[test]
    fn a_published_project_with_nothing_pending_is_online() {
        // The resting state. If this were Draft, Online would be unobservable:
        // REQ-2.4 discards the record, and no depositor loads the page inside
        // the instant between the comparison and the delete.
        assert_eq!(
            depositor_state(None, false, false, &Comparison::Unchanged),
            ProjectState::Online
        );
    }

    #[test]
    fn a_draft_row_takes_a_published_project_back_out_of_online() {
        assert_eq!(depositor_state(None, false, true, &Comparison::Unchanged), ProjectState::Draft);
    }

    #[test]
    fn an_unpublished_project_with_nothing_pending_is_not_online() {
        // Nothing is live, so there is nothing for Online to be a claim about.
        assert_eq!(depositor_state(None, false, false, &Comparison::Absent), ProjectState::Draft);
        assert_eq!(
            depositor_state(None, false, false, &Comparison::NewAndUnpublished),
            ProjectState::Draft
        );
    }

    #[test]
    fn a_discarded_record_leaves_the_project_reading_online() {
        // The end-to-end shape of REQ-2.4 as a depositor sees it: the startup
        // pass deletes the record, so the very next page load has no submission,
        // no record and no draft — and must say the change is live rather than
        // silently reverting to Draft.
        assert_eq!(
            depositor_state(None, false, false, &Comparison::Unchanged),
            ProjectState::Online
        );
    }

    #[test]
    fn the_five_states_are_exactly_the_requirement() {
        // REQ-2.1 makes this list closed. A sixth state added without a
        // requirement change fails here.
        let labels: Vec<&str> = ProjectState::ALL.iter().map(|state| state.label()).collect();
        assert_eq!(labels, ["Draft", "Submitted", "In review", "Approved", "Online"]);
    }

    #[test]
    fn every_state_explains_itself_and_approved_states_the_wait() {
        // REQ-2.6 requires the expected wait, and Approved is the only state
        // that has one.
        for state in ProjectState::ALL {
            assert!(!state.explanation().is_empty(), "{state:?} needs an explanation");
        }
        assert!(
            ProjectState::Approved.explanation().contains("few weeks"),
            "REQ-2.6: the Approved explanation states the expected wait"
        );
    }
}
