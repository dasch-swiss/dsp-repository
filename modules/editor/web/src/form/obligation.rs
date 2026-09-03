//! How much of a section is filled in, for the section rail.
//!
//! The state is a **pair** — required fields, and how many the draft answers —
//! because a rail showing only what is outstanding renders "done" and "empty"
//! identically.
//!
//! Only the required tier is counted: a rail counting `Recommended` would show a
//! permanently incomplete section for a complete project.
//!
//! Satisfied means **present**, not valid. A draft holds values that do not
//! validate (REQ-1.9) and deciding that is submit's job (REQ-1.12); a rail
//! stricter than the pill beside the field would disagree with it. A placeholder
//! sentinel is not present — counting one would call a section complete while
//! `endDate` held `"MISSING"`.
//!
//! Presence is read off the stored value rather than off a shape, so a field
//! whose control has not landed still contributes an honest count.

use editor_core::draft::ProjectDraft;
use platform_metadata::is_placeholder;
use serde_json::Value;

use super::registry::{Audience, Field, Obligation, Section};

/// A section's required-field state, as the rail shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SectionProgress {
    /// Required fields this audience sees in the section.
    pub required: usize,
    /// How many of them the draft answers.
    pub satisfied: usize,
}

impl SectionProgress {
    /// Whether every required field in the section is answered.
    ///
    /// A section with no required fields is complete, not empty: there is
    /// nothing outstanding in it, and rendering it as unfinished would send a
    /// depositor looking for something that is not there.
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

/// Whether a stored value is an answer rather than an empty state.
fn has_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty() && !is_placeholder(text),
        Value::Array(items) => items.iter().any(has_value),
        // A language map of nothing but empty texts is not an answer, and
        // neither is `{}` — `has_value` recurses so both fall out of the same
        // rule rather than needing the map to be recognised as one.
        Value::Object(members) => members.values().any(has_value),
        // A number or a boolean is a value whatever it is; there is no empty
        // `0` or empty `false`.
        Value::Bool(_) | Value::Number(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::form::registry::{section, SECTIONS};

    /// A draft over a real committed project.
    ///
    /// Read off the corpus rather than from a fixture, and rather than
    /// `editor_core`'s own `sample_raw` (which is `#[cfg(test)]` and so private
    /// to that crate): the point of
    /// [`the_committed_sample_project_answers_every_required_field_it_can`] is
    /// that a *published* project comes out complete, which a hand-written
    /// fixture cannot claim. `0801d` is the project the registry's own contract
    /// test reads, for the same reason.
    fn draft() -> ProjectDraft {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    #[test]
    fn a_placeholder_sentinel_is_not_an_answer() {
        // 24 of the 85 committed files hold `"MISSING"` in `endDate`. Counting
        // one would tell a depositor a section was complete while the field the
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
        // `{}`, and a map whose every text is empty, are both the state a
        // depositor sees as a blank field.
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

    /// Every required field the committed corpus does **not** answer, and how
    /// many of the 85 projects it is missing from.
    ///
    /// Measured, not asserted from the requirements — and the measurement is the
    /// point. `Obligation::Required` means "must be present to submit"
    /// (REQ-1.12), so a submit gate applied literally against this tier would
    /// refuse every one of these projects: all 85 lack `documentationMaterial`,
    /// 13 lack `url`, 9 lack `contactPoint`. That is a live surface being made
    /// unsubmittable by a tier nobody checked against the data, and it is the
    /// same shape of failure `WhenCleared` was documented against. Submit
    /// validation is where it gets decided; this is the baseline it decides
    /// from, and the test below is what makes an obligation change say so.
    const UNANSWERED_BY_THE_CORPUS: &[(&str, usize)] = &[
        ("documentationMaterial", 85),
        ("url", 13),
        ("contactPoint", 9),
        ("dataLanguage", 1),
        ("typeOfData", 1),
    ];

    /// How many of the 85 committed projects answer every required field a
    /// *depositor* sees. The ten that do not are nine missing `contactPoint`
    /// (the `0801*` family, `0805`, `080C`, `080E`, `081C`) and `082C`, which
    /// has neither `typeOfData` nor `dataLanguage`.
    const COMPLETE_FOR_A_DEPOSITOR: usize = 75;

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
    fn most_published_projects_open_complete_for_a_depositor() {
        // The rail must not invent outstanding work: a project that is already
        // live should not open showing a depositor a list of things to fix that
        // they did not create. Pinned as a count rather than as "all of them",
        // because it is not all of them — ten projects genuinely lack a
        // depositor-visible required field, and the honest number is the one
        // worth watching. A rail bug moves this sharply; a data fix moves it by
        // one.
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
    fn an_rdu_reader_sees_the_field_the_corpus_never_answers() {
        // The other side of the same fact, pinned where a reader of this module
        // will find it: the rail is honest rather than flattering, so a section
        // holding an unanswered required field says so even when the project is
        // published.
        let progress = section_progress(section("dataset").expect("dataset"), Audience::RduOnly, &draft());
        assert!(!progress.is_complete(), "{}", progress.summary());
        assert_eq!(progress.satisfied + 1, progress.required, "{}", progress.summary());
    }

    #[test]
    fn a_required_field_is_never_addressed_by_a_dotted_path() {
        // `ProjectDraft::get` reads a top-level member, so a dotted id would
        // count as unsatisfied whatever the project holds — silently, and only
        // in the rail. No required field is dotted today
        // (`accessRights.embargoDate` is `Optional`); this fires the day one is,
        // rather than the rail quietly under-counting.
        for field in crate::form::registry::FIELDS {
            if field.obligation == Some(Obligation::Required) {
                assert!(
                    !field.id.contains('.'),
                    "{} is required and dotted: `is_satisfied` needs a path-aware read first",
                    field.id
                );
            }
        }
    }

    #[test]
    fn a_section_with_no_required_field_reads_as_complete_rather_than_unfinished() {
        // Nothing is outstanding in it, so a mark saying otherwise sends the
        // reader looking for something that is not there.
        let image = section("image").expect("image");
        let progress = section_progress(image, Audience::Everyone, &ProjectDraft::default());
        assert_eq!(progress.required, 0);
        assert!(progress.is_complete());
        assert!(!progress.has_requirements());
    }
}
