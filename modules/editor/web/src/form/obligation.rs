//! How much of a section is filled in, for the section rail.
//!
//! The state is a pair, required fields and how many the draft answers, because
//! a rail showing only what is outstanding renders "done" and "empty" the same.
//! Only the required tier is counted: counting `Recommended` would show a
//! permanently incomplete section for a complete project. Satisfied means
//! present, not valid: validity is submit's job, and a rail stricter than the
//! pill beside the field would disagree with it. A placeholder sentinel is not
//! present. Presence is read off the stored value rather than a shape, so a
//! field whose control has not landed still counts honestly.

use editor_core::draft::ProjectDraft;
use platform_metadata::is_placeholder;
use serde_json::Value;

use super::registry::{sections_for, Audience, Field, Obligation, Section};

/// A section's required-field state, as the rail shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionProgress {
    /// Required fields this audience sees in the section.
    pub required: usize,
    /// How many of them the draft answers.
    pub satisfied: usize,
}

impl SectionProgress {
    /// Whether every required field is answered. A section with no required
    /// fields is complete, not empty: nothing is outstanding in it.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.satisfied >= self.required
    }

    /// Whether the section has any required field at all.
    #[must_use]
    pub const fn has_requirements(&self) -> bool {
        self.required > 0
    }

    /// "3 of 5 required" — both numbers, always, so a complete section and an
    /// empty one cannot render the same.
    #[must_use]
    pub fn summary(&self) -> String {
        format!("{} of {} required", self.satisfied, self.required)
    }
}

/// One section's state for `audience`.
#[must_use]
pub fn section_progress(section: &Section, audience: Audience, draft: &ProjectDraft) -> SectionProgress {
    let required: Vec<&Field> = section
        .fields_for(audience)
        .filter(|field| field.obligation == Some(Obligation::Required))
        .collect();
    SectionProgress {
        required: required.len(),
        satisfied: required.iter().filter(|field| is_satisfied(field, draft)).count(),
    }
}

/// Whether the draft answers this field. See the module docs for what counts.
#[must_use]
pub fn is_satisfied(field: &Field, draft: &ProjectDraft) -> bool {
    draft.get(field.id).is_some_and(has_value)
}

/// Every required field this audience sees that the draft does not answer, in
/// the order the form shows them.
///
/// The submit gate, reading presence through the same [`is_satisfied`] the rail
/// does: a gate stricter than the rail would refuse a submission the rail had
/// just counted complete. Complementary to `ProjectDraft::to_raw`: every
/// `Required` field is a non-`Option` member of `ProjectRaw`, so an absent one
/// already fails there; this catches present-and-empty (`[]`, `{}`, `""`, a
/// sentinel). Iterated through [`sections_for`] and [`Section::fields_for`], so
/// the audience gate and the order are the form's own.
#[must_use]
pub fn unsatisfied_required(audience: Audience, draft: &ProjectDraft) -> Vec<&'static Field> {
    sections_for(audience)
        .flat_map(|section| section.fields_for(audience))
        .filter(|field| field.obligation == Some(Obligation::Required))
        .filter(|field| !is_satisfied(field, draft))
        .collect()
}

/// Whether a stored value is an answer rather than an empty state.
fn has_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty() && !is_placeholder(text),
        Value::Array(items) => items.iter().any(has_value),
        // `{}` and a map of empty texts both fall out of the recursion.
        Value::Object(members) => members.values().any(has_value),
        // No number or boolean is empty.
        Value::Bool(_) | Value::Number(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::form::registry::{section, SECTIONS};

    /// A draft over a real committed project, so a published project can be
    /// shown to come out complete, which a hand-written fixture cannot claim.
    fn draft() -> ProjectDraft {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    #[test]
    fn a_placeholder_sentinel_is_not_an_answer() {
        // Counting a sentinel would call a section complete while the field the
        // platform reads as empty sat in it.
        let mut draft = draft();
        let field = section("overview")
            .expect("overview")
            .fields_for(Audience::Everyone)
            .find(|field| field.id == "name")
            .expect("name is in the overview section");

        draft.set("name", json!("A Project"));
        assert!(is_satisfied(field, &draft));
        draft.set("name", json!("MISSING"));
        assert!(!is_satisfied(field, &draft));
        draft.set("name", json!("CALCULATED"));
        assert!(!is_satisfied(field, &draft));
        draft.set("name", json!("   "));
        assert!(!is_satisfied(field, &draft));
        draft.remove("name");
        assert!(!is_satisfied(field, &draft));
    }

    #[test]
    fn an_empty_language_map_is_not_an_answer_however_it_is_empty() {
        // Both are the state a depositor sees as a blank field.
        let mut draft = draft();
        let field = crate::form::registry::field("description").expect("description");

        draft.set("description", json!({"en": "A description"}));
        assert!(is_satisfied(field, &draft));
        draft.set("description", json!({"en": "", "de": "  "}));
        assert!(!is_satisfied(field, &draft));
        draft.set("description", json!({}));
        assert!(!is_satisfied(field, &draft));
    }

    #[test]
    fn an_empty_list_is_not_an_answer_and_a_list_of_empties_is_not_either() {
        let mut draft = draft();
        let field = crate::form::registry::field("keywords").expect("keywords");

        draft.set("keywords", json!([{"en": "manuscripts"}]));
        assert!(is_satisfied(field, &draft));
        draft.set("keywords", json!([]));
        assert!(!is_satisfied(field, &draft));
        draft.set("keywords", json!([{"en": ""}]));
        assert!(!is_satisfied(field, &draft));
    }

    #[test]
    fn a_section_reports_both_numbers_so_complete_and_empty_cannot_look_alike() {
        let overview = section("overview").expect("overview");
        let full = section_progress(overview, Audience::Everyone, &draft());
        assert!(full.required > 0, "the overview section should have required fields");
        assert_eq!(full.summary(), format!("{} of {} required", full.satisfied, full.required));

        let empty = section_progress(overview, Audience::Everyone, &ProjectDraft::default());
        assert_eq!(empty.satisfied, 0);
        assert_eq!(empty.required, full.required);
        assert!(!empty.is_complete());
        assert_ne!(empty.summary(), full.summary());
    }

    /// Every required field the committed corpus does not answer, and how many
    /// projects it is missing from. Empty, and that is an invariant:
    /// `Obligation::Required` is a submit gate, so a field tiered that way which
    /// the corpus does not answer makes every one of those projects
    /// unsubmittable. Fix the tier or the data, never this constant.
    const UNANSWERED_BY_THE_CORPUS: &[(&str, usize)] = &[];

    /// How many committed projects answer every required field a depositor
    /// sees: all of them, the per-project form of [`UNANSWERED_BY_THE_CORPUS`].
    const COMPLETE_FOR_A_DEPOSITOR: usize = 85;

    #[test]
    fn the_required_fields_the_committed_corpus_does_not_answer_are_the_measured_ones() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let shortcodes: Vec<String> = published.summaries().map(|s| s.shortcode.to_string()).collect();
        assert_eq!(shortcodes.len(), 85, "the corpus should be all 85 committed projects");

        let mut unanswered: Vec<(&str, usize)> = crate::form::registry::FIELDS
            .iter()
            .filter(|field| field.obligation == Some(Obligation::Required))
            .filter_map(|field| {
                let missing = shortcodes
                    .iter()
                    .filter(|shortcode| {
                        let raw = published.get(shortcode).expect("a summary names a loaded project");
                        !is_satisfied(field, &ProjectDraft::from_raw(raw))
                    })
                    .count();
                (missing > 0).then_some((field.id, missing))
            })
            .collect();
        unanswered.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));

        assert_eq!(
            unanswered, UNANSWERED_BY_THE_CORPUS,
            "the set of required fields the published corpus does not answer changed; see \
             UNANSWERED_BY_THE_CORPUS for why that is a submit-gate decision and not a rail bug"
        );
    }

    #[test]
    fn every_published_project_opens_complete_for_a_depositor() {
        // A live project must not open showing a depositor work they did not
        // create; this is also the submit gate's guarantee, one project at a time.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, _) = editor_core::published::PublishedProjects::load_from(&dir);
        let shortcodes: Vec<String> = published.summaries().map(|s| s.shortcode.to_string()).collect();

        let complete = shortcodes
            .iter()
            .filter(|shortcode| {
                let raw = published.get(shortcode).expect("a summary names a loaded project");
                let draft = ProjectDraft::from_raw(raw);
                SECTIONS
                    .iter()
                    .all(|section| section_progress(section, Audience::Everyone, &draft).is_complete())
            })
            .count();
        assert_eq!(complete, COMPLETE_FOR_A_DEPOSITOR, "of {} projects", shortcodes.len());
    }

    #[test]
    fn an_rdu_reader_sees_a_published_project_as_complete_too() {
        // RDU sees strictly more fields, so this stops the next Required on an
        // RDU-only field slipping past the depositor count.
        let draft = draft();
        for section in sections_for(Audience::RduOnly) {
            let progress = section_progress(section, Audience::RduOnly, &draft);
            assert!(
                progress.is_complete(),
                "section {} reads incomplete for RDU on a published project: {}",
                section.id,
                progress.summary()
            );
        }
    }

    #[test]
    fn no_published_project_is_refused_by_the_submit_gate() {
        // Through the function the route calls rather than the rail, over both
        // audiences, since RDU submits through the same handler.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let mut refused: Vec<(String, Vec<&str>)> = Vec::new();
        for shortcode in published.summaries().map(|s| s.shortcode.to_string()) {
            let raw = published.get(&shortcode).expect("a summary names a loaded project");
            let draft = ProjectDraft::from_raw(raw);
            for audience in [Audience::Everyone, Audience::RduOnly] {
                let unsatisfied: Vec<&str> =
                    unsatisfied_required(audience, &draft).iter().map(|field| field.id).collect();
                if !unsatisfied.is_empty() {
                    refused.push((shortcode.clone(), unsatisfied));
                }
            }
        }
        assert!(
            refused.is_empty(),
            "the submit gate would refuse published projects: {refused:?}"
        );
    }

    #[test]
    fn the_submit_gate_names_a_required_field_emptied_rather_than_removed() {
        // The state to_raw cannot see: [], {}, "" and a sentinel all deserialize
        // and none is an answer.
        let mut draft = draft();
        assert!(unsatisfied_required(Audience::Everyone, &draft).is_empty());

        draft.set("keywords", json!([]));
        draft.set("name", json!("MISSING"));
        let named: Vec<&str> = unsatisfied_required(Audience::Everyone, &draft)
            .iter()
            .map(|field| field.id)
            .collect();
        assert_eq!(named, ["name", "keywords"], "in the order the form shows them");

        // Still a complete `ProjectRaw`, which is what makes the gate necessary.
        assert!(draft.to_raw().is_ok(), "an emptied required field is still a valid contract");
    }

    #[test]
    fn the_submit_gate_never_names_a_field_the_submitter_cannot_see() {
        // A refusal naming an RDU-only field to a depositor is an instruction they
        // cannot follow. No required field is RDU-only today; this fires if one
        // becomes so.
        let empty = ProjectDraft::default();
        for field in unsatisfied_required(Audience::Everyone, &empty) {
            assert_eq!(
                field.audience,
                Audience::Everyone,
                "{} is RDU-only but gates a depositor's submission",
                field.id
            );
        }
        // The canary: an empty draft must actually reach the gate, or the loop
        // above passes over nothing.
        assert!(!unsatisfied_required(Audience::Everyone, &empty).is_empty());
    }

    #[test]
    fn a_dotted_required_field_is_counted_from_the_member_it_names() {
        // `accessRights.accessRights` is both required and dotted, so the count
        // has to follow the path `ProjectDraft::get` follows.
        let mut draft = draft();
        let field = crate::form::registry::field("accessRights.accessRights").expect("the access-rights choice");
        assert!(field.id.contains('.'), "the premise of this test");
        assert_eq!(field.obligation, Some(Obligation::Required));
        assert!(is_satisfied(field, &draft), "a published project answers it");

        draft.set("accessRights", serde_json::json!({ "embargoDate": "2030-01-01" }));
        assert!(
            !is_satisfied(field, &draft),
            "the choice is gone even though its parent object is not"
        );
    }

    #[test]
    fn a_section_with_no_required_field_reads_as_complete_rather_than_unfinished() {
        let image = section("image").expect("image");
        let progress = section_progress(image, Audience::Everyone, &ProjectDraft::default());
        assert_eq!(progress.required, 0);
        assert!(progress.is_complete());
        assert!(!progress.has_requirements());
    }
}
