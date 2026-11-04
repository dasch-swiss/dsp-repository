//! What submit refuses that needs the field registry to decide.
//!
//! The gate sequence, which decides when each rule runs, is
//! [`crate::submission::check`]; a new rule is wired in there.
//! [`obligation::unsatisfied_required`](super::obligation::unsatisfied_required) stays in its own
//! module because the rail counts presence through the same function.

use serde_json::Value;
use shared_metadata::is_placeholder;

use super::registry::{sections_for, Audience, Field, Section};
use crate::agents::AgentScope;
use crate::draft::ProjectDraft;
use crate::form::{language_parts, ChoiceSet, FormBody, Shape, WhenCleared, MAX_VALUES_PER_FIELD};
use crate::languages::canonical_code;
use crate::multilingual::normalized_language_tag;

/// Every field whose stored value is a placeholder sentinel the editor could not
/// have written there, which leaves one way for it to have arrived: a depositor
/// typed it.
///
/// Such a value is a dead end: it renders as an empty control and survives every
/// later empty submit, because `apply_text` reads that as "not a clear", so the
/// field is only editable by typing some other value first.
///
/// The rule is read off the shape, not off the value. A sentinel in a draft is
/// usually correct: [`WhenCleared::Placeholder`] means the editor itself writes
/// one on clear, and the committed corpus is full of them. So the question is
/// "could clearing this field have produced one": yes for
/// [`WhenCleared::Placeholder`], no for [`WhenCleared::Drop`] (the cleared state
/// is an absent member) and no for [`Shape::Multilingual`] (an empty text drops
/// its tag). Derived from the shape rather than compared against the published
/// project because a local-only project has nothing to compare against.
#[must_use]
pub fn typed_sentinels(audience: Audience, draft: &ProjectDraft) -> Vec<&'static Field> {
    sections_for(audience)
        .flat_map(|section| section.fields_for(audience))
        .filter(|field| holds_typed_sentinel(field, draft))
        .collect()
}

/// Every stored language value that is not a listed code, with its field, unless `published`
/// holds that exact value in the same field.
///
/// A draft can hold an unknown typed name, so saving is never refused and submit is where it
/// stops. A published value is excused so a live project is not unsubmittable over data the
/// depositor did not write.
#[must_use]
pub fn unlisted_languages(
    audience: Audience,
    draft: &ProjectDraft,
    published: Option<&ProjectDraft>,
) -> Vec<(&'static Field, String)> {
    sections_for(audience)
        .flat_map(|section| section.fields_for(audience))
        .filter(|field| matches!(field.shape, Some(Shape::StringList(ChoiceSet::Languages(_)))))
        .flat_map(|field| {
            let held_by_published = strings_in(published, field.id);
            strings_in(Some(draft), field.id)
                .filter(|value| canonical_code(value).is_none())
                .filter(|value| !held_by_published.clone().any(|held| held == *value))
                .map(|value| (field, value.to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn strings_in<'a>(draft: Option<&'a ProjectDraft>, field: &str) -> impl Iterator<Item = &'a str> + Clone {
    draft
        .and_then(|draft| draft.get(field))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
}

/// Whether any string anywhere in `value` is a placeholder sentinel.
fn holds_placeholder(value: &Value) -> bool {
    match value {
        Value::String(text) => is_placeholder(text),
        Value::Array(items) => items.iter().any(holds_placeholder),
        Value::Object(members) => members.values().any(holds_placeholder),
        _ => false,
    }
}

/// Whether this field's stored value is a sentinel its shape cannot explain.
fn holds_typed_sentinel(field: &Field, draft: &ProjectDraft) -> bool {
    let Some(shape) = field.shape else {
        // No applier reads it, so the editor never wrote its value and a
        // sentinel in it came from the published data.
        return false;
    };
    let Some(value) = draft.get(field.id) else { return false };
    match shape {
        Shape::Text(WhenCleared::Placeholder) => false,
        Shape::Text(WhenCleared::Drop) => value.as_str().is_some_and(is_placeholder),
        // Plain strings a depositor may type, checked as the variant rows below
        // are; a reference url is excused for the same reason as Shape::Url.
        Shape::ReferenceRows(_) => value.as_array().is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.iter().filter(|(member, _)| *member != "url").map(|(_, text)| text))
                .filter_map(Value::as_str)
                .any(is_placeholder)
        }),
        // Every string is typed and no committed value is a sentinel.
        Shape::PublicationRows | Shape::FundingRows => holds_placeholder(value),
        // The label is typed and checked; the url is excused as Shape::Url is:
        // `0110_h-steiner` holds `{"type": "URL", "url": "MISSING"}`, and refusing
        // it would make a live project unsubmittable over data it did not write.
        Shape::TextOrReferenceRows(_) => value.as_array().is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.iter().filter(|(member, _)| *member != "url").map(|(_, text)| text))
                .filter_map(Value::as_str)
                .any(is_placeholder)
        }),
        // Each row's roles are plain strings a depositor may type.
        Shape::AttributionRows => value.as_array().is_some_and(|rows| {
            rows.iter()
                .filter_map(|row| row.get("contributorType"))
                .filter_map(Value::as_array)
                .flatten()
                .filter_map(Value::as_str)
                .any(is_placeholder)
        }),
        Shape::StringRows | Shape::AgentRows => value
            .as_array()
            .is_some_and(|rows| rows.iter().filter_map(Value::as_str).any(is_placeholder)),
        // A language map per row: an empty text drops its tag, so a sentinel
        // cannot have come from clearing.
        Shape::MultilingualRows => value.as_array().is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.values())
                .filter_map(Value::as_str)
                .any(is_placeholder)
        }),
        // An open list stores whatever a depositor types; a closed set stores
        // only an offered value, and no vocabulary offers a sentinel.
        Shape::StringList(_) => value
            .as_array()
            .is_some_and(|items| items.iter().filter_map(Value::as_str).any(is_placeholder)),
        // The published data's own empty state, which `apply_url` leaves alone.
        Shape::Url(_) => false,
        // `apply_choice` stores only a listed value, so a sentinel here came from
        // the published data.
        Shape::Choice(_) => false,
        Shape::Multilingual => value
            .as_object()
            .is_some_and(|texts| texts.values().filter_map(Value::as_str).any(is_placeholder)),
    }
}

/// Every field in `section` whose posted body carries more values than one field
/// may hold: the cap a depositor can see, where [`FormBody::entries`] stopping at
/// [`MAX_VALUES_PER_FIELD`] would silently discard the rest.
///
/// Checked before any applier runs, on a save as well as a submit: deferred to
/// submit alone, an over-cap save truncates and stores, and the submit that
/// follows sees a draft within the cap. One number rather than a cap per field:
/// input hygiene, not a product rule.
#[must_use]
pub fn over_cap(audience: Audience, section: &Section, body: &FormBody) -> Vec<&'static Field> {
    section
        .fields_for(audience)
        .filter(|field| match field.shape {
            // A scalar posts one value; `FormBody::get` takes the first, so there is
            // nothing to bound.
            None | Some(Shape::Text(_) | Shape::Choice(_) | Shape::Url(_)) => false,
            Some(Shape::Multilingual) => body.exceeds_entries(field.id, MAX_VALUES_PER_FIELD),
            // Counts the comma-split parts, as the applier reads them.
            Some(Shape::StringList(ChoiceSet::Languages(_))) => {
                body.all(field.id).flat_map(language_parts).nth(MAX_VALUES_PER_FIELD).is_some()
            }
            // Repeated under one name rather than suffixed.
            Some(Shape::StringList(_)) => body.exceeds_all(field.id, MAX_VALUES_PER_FIELD),
            // Lines are counted across every row too: render cost grows with them.
            Some(Shape::MultilingualRows) => {
                body.exceeds_all(&format!("{}.row", field.id), MAX_VALUES_PER_FIELD)
                    || body
                        .rows(field.id)
                        .iter()
                        .map(|key| body.all(&format!("{}.{key}.lang", field.id)).count())
                        .sum::<usize>()
                        > MAX_VALUES_PER_FIELD
            }
            // Rows are repeated `{field}.row` values.
            Some(
                Shape::StringRows
                | Shape::AgentRows
                | Shape::AttributionRows
                | Shape::TextOrReferenceRows(_)
                | Shape::ReferenceRows(_)
                | Shape::PublicationRows
                | Shape::FundingRows,
            ) => body.exceeds_all(&format!("{}.row", field.id), MAX_VALUES_PER_FIELD),
        })
        .collect()
}

/// One line of a [`Shape::MultilingualRows`] field that has text and no language.
#[derive(Debug, Clone)]
pub struct LineWithoutLanguage {
    /// The line's position within its row.
    pub line: usize,
    /// The name its language select posts under, which is where an error on the line renders.
    pub control: String,
}

impl LineWithoutLanguage {
    /// The hidden field a flagged line posts, its value the line's position: what keeps the flag
    /// through the next autosave, which re-renders the form after the save that set it.
    #[must_use]
    pub fn marker(&self) -> String {
        format!("{}-missing", self.control)
    }
}

/// Every line of `section`'s multilingual-row fields with non-blank text and no language code,
/// leaving out the `accepted` fields, which the applier skips.
///
/// Read from the body: the applier drops such a line, so the stored draft cannot say it was typed.
#[must_use]
pub fn lines_without_language(
    audience: Audience,
    section: &Section,
    accepted: &[String],
    body: &FormBody,
) -> Vec<LineWithoutLanguage> {
    lines_matching(audience, section, accepted, body, |_| true)
}

/// The lines of [`lines_without_language`] a save already flagged, by the
/// [`LineWithoutLanguage::marker`] posted with them: an autosave or a row action keeps the flag
/// until a language is chosen, and flags no line the depositor is still typing.
#[must_use]
pub fn lines_still_without_language(
    audience: Audience,
    section: &Section,
    accepted: &[String],
    body: &FormBody,
) -> Vec<LineWithoutLanguage> {
    lines_matching(audience, section, accepted, body, |line| {
        body.all(&line.marker()).any(|value| value == line.line.to_string())
    })
}

fn lines_matching(
    audience: Audience,
    section: &Section,
    accepted: &[String],
    body: &FormBody,
    keep: impl Fn(&LineWithoutLanguage) -> bool,
) -> Vec<LineWithoutLanguage> {
    let mut lines = Vec::new();
    for field in section
        .fields_for(audience)
        .filter(|field| matches!(field.shape, Some(Shape::MultilingualRows)))
        .filter(|field| !accepted.iter().any(|accepted| accepted == field.id))
    {
        for key in body.rows(field.id) {
            let control = format!("{}.{key}.lang", field.id);
            let tags = body.all(&control);
            let texts = body.all(&format!("{}.{key}.text", field.id));
            for (line, (tag, text)) in tags.zip(texts).enumerate() {
                let found = LineWithoutLanguage { line, control: control.clone() };
                if !text.trim().is_empty() && normalized_language_tag(tag).is_none() && keep(&found) {
                    lines.push(found);
                }
            }
        }
    }
    lines
}

/// Every agent-reference field holding an id that resolves to nobody, with the
/// offending ids: the submit-time half of `Shape::AgentRows`, since the applier
/// stores whatever arrives. A dangling reference stops here rather than
/// rendering as a bare `person-001` on the public project page.
///
/// A dangling reference in a field the depositor did not touch is still refused:
/// every committed reference resolves, so a dangling one can only have arrived
/// after the fact, which is a thing to report rather than publish.
///
/// `funding[].funders` is a fourth agent field and must stay in this filter: it
/// declares [`Shape::FundingRows`], and `checks::contributor_refs` does not
/// report funders either, so this gate is the only one that does.
#[must_use]
pub fn unresolved_agents(
    audience: Audience,
    draft: &ProjectDraft,
    agents: &AgentScope,
) -> Vec<(&'static Field, String)> {
    sections_for(audience)
        .flat_map(|section| section.fields_for(audience))
        .filter(|field| field.shape.is_some_and(Shape::has_agent_picker))
        .flat_map(|field| {
            let rows = draft
                .get(field.id)
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            rows.iter()
                // The three shapes spell a reference differently; read in one place
                // so a new agent field cannot be added to one and forgotten in another.
                .flat_map(agent_ids_in_row)
                .filter(|id| !agents.has(id))
                .map(move |id| (field, id.to_string()))
                .collect::<Vec<(&'static Field, String)>>()
        })
        .collect()
}

/// Removes every reference to agent `id` from the draft and returns how many it removed: an id in
/// `AgentRows`, an `AttributionRows` row naming it as `contributor` (the row goes), a
/// `FundingRows` grant listing it in `funders` (the grant stays, even with no funder left).
///
/// A field left empty is removed rather than stored as `[]`.
pub fn drop_agent(draft: &mut ProjectDraft, id: &str) -> usize {
    let mut removed = 0;
    for field in sections_for(Audience::RduOnly)
        .flat_map(|section| section.fields_for(Audience::RduOnly))
        .filter(|field| field.shape.is_some_and(Shape::has_agent_picker))
    {
        let Some(Value::Array(rows)) = draft.get(field.id) else {
            continue;
        };
        let mut rows = rows.clone();
        let before = removed;
        rows.retain_mut(|row| match row {
            Value::String(held) => {
                let keep = held != id;
                removed += usize::from(!keep);
                keep
            }
            Value::Object(object) if object.contains_key("contributor") => {
                let keep = object.get("contributor").and_then(Value::as_str) != Some(id);
                removed += usize::from(!keep);
                keep
            }
            Value::Object(object) => {
                if let Some(Value::Array(funders)) = object.get_mut("funders") {
                    let held = funders.len();
                    funders.retain(|funder| funder.as_str() != Some(id));
                    removed += held - funders.len();
                }
                true
            }
            _ => true,
        });
        if removed == before {
            continue;
        }
        if rows.is_empty() {
            draft.remove(field.id);
        } else {
            draft.set(field.id, Value::Array(rows));
        }
    }
    removed
}

/// Every agent id one row of an agent-bearing field holds: a bare string for
/// `AgentRows`, `contributor` for `AttributionRows`, the `funders` array for
/// `FundingRows`, which is why this returns a list.
fn agent_ids_in_row(row: &Value) -> Vec<&str> {
    if let Some(id) = row.as_str() {
        return vec![id];
    }
    if let Some(id) = row.get("contributor").and_then(Value::as_str) {
        return vec![id];
    }
    row.get("funders")
        .and_then(Value::as_array)
        .map(|funders| funders.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::form::registry::field;

    fn draft() -> ProjectDraft {
        let dir = crate::checkout_dpe_data_dir().join("projects");
        let (published, errors) = crate::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    /// The dead end this module closes, reproduced through the real applier.
    #[test]
    fn a_typed_sentinel_in_an_optional_field_cannot_be_cleared_again() {
        use crate::form::{apply, FormBody};

        let shape = field("provenance").expect("provenance").shape.expect("a declared shape");
        let mut draft = draft();

        // The depositor types the word.
        apply(
            shape,
            &FormBody::from_pairs(vec![("provenance".to_string(), "MISSING".to_string())]),
            &mut draft,
            "provenance",
        );
        assert_eq!(draft.get("provenance"), Some(&json!("MISSING")), "stored verbatim");

        // The control now renders empty, so an untouched form posts empty — and
        // that leaves the sentinel alone rather than dropping the member.
        apply(
            shape,
            &FormBody::from_pairs(vec![("provenance".to_string(), String::new())]),
            &mut draft,
            "provenance",
        );
        assert_eq!(
            draft.get("provenance"),
            Some(&json!("MISSING")),
            "an empty submit does not clear a stored sentinel, which is what strands the field"
        );

        // Which is exactly what this module reports.
        assert_eq!(
            typed_sentinels(Audience::Everyone, &draft)
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            ["provenance"]
        );
    }

    #[test]
    fn a_published_project_holds_no_typed_sentinel() {
        // The committed corpus is full of sentinels, and every one must be legitimate, or submit
        // refuses a live project for data it did not write.
        let dir = crate::checkout_dpe_data_dir().join("projects");
        let (published, errors) = crate::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let mut refused: Vec<(String, Vec<&str>)> = Vec::new();
        for shortcode in published.summaries().map(|s| s.shortcode.to_string()) {
            let raw = published.get(&shortcode).expect("a summary names a loaded project");
            let draft = ProjectDraft::from_raw(raw);
            for audience in [Audience::Everyone, Audience::RduOnly] {
                let named: Vec<&str> = typed_sentinels(audience, &draft).iter().map(|f| f.id).collect();
                if !named.is_empty() {
                    refused.push((shortcode.clone(), named));
                }
            }
        }
        assert!(
            refused.is_empty(),
            "published projects refused for inherited sentinels: {refused:?}"
        );
    }

    #[test]
    fn the_corpus_really_does_carry_the_sentinels_this_module_must_not_refuse() {
        // The canary for the absence above.
        let draft = draft();
        let held: Vec<&str> = draft
            .fields()
            .filter(|id| draft.get(id).and_then(Value::as_str).is_some_and(is_placeholder))
            .collect();
        assert!(!held.is_empty(), "0801d should carry at least one legitimate sentinel");
    }

    #[test]
    fn a_sentinel_is_allowed_where_clearing_the_field_writes_one() {
        // endDate is WhenCleared::Placeholder, so MISSING is what clearing writes.
        let mut draft = draft();
        draft.set("endDate", json!("MISSING"));
        assert!(typed_sentinels(Audience::Everyone, &draft).is_empty());
    }

    #[test]
    fn a_sentinel_typed_into_one_language_of_a_map_is_caught() {
        // Unlike a scalar, a sentinel in a language map renders visibly, so the
        // refusal has to name the field.
        let mut draft = draft();
        draft.set("abstract", json!({"en": "A real abstract", "de": "CALCULATED"}));
        assert_eq!(
            typed_sentinels(Audience::Everyone, &draft)
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            ["abstract"]
        );
    }

    #[test]
    fn both_sentinel_words_are_refused_and_a_lookalike_is_not() {
        // Both sentinel words, and an exact compare so a real value containing the
        // word passes.
        let mut draft = draft();
        for word in ["MISSING", "CALCULATED"] {
            draft.set("provenance", json!(word));
            assert_eq!(typed_sentinels(Audience::Everyone, &draft).len(), 1, "{word} should be refused");
        }
        for allowed in ["Missing", "MISSING pages", "The data is missing", "CALCULATED_BY_HAND"] {
            draft.set("provenance", json!(allowed));
            assert!(
                typed_sentinels(Audience::Everyone, &draft).is_empty(),
                "{allowed:?} is a real value and must pass"
            );
        }
    }

    /// The most values any list in the committed corpus holds, so the cap and the
    /// data cannot silently invert: a project over the cap would be refused on
    /// save, so this fails first and somebody raises the cap deliberately.
    #[test]
    fn the_cap_is_above_anything_the_committed_corpus_holds() {
        use crate::form::MAX_VALUES_PER_FIELD;

        let dir = crate::checkout_dpe_data_dir().join("projects");
        let (published, errors) = crate::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let mut widest_map = 0;
        let mut longest_list = ("", 0);
        for shortcode in published.summaries().map(|s| s.shortcode.to_string()) {
            let raw = published.get(&shortcode).expect("a summary names a loaded project");
            let draft = ProjectDraft::from_raw(raw);
            for id in draft.fields().map(str::to_string).collect::<Vec<_>>() {
                match draft.get(&id) {
                    Some(Value::Object(map)) if crate::multilingual::is_language_map(&Value::Object(map.clone())) => {
                        widest_map = widest_map.max(map.len());
                    }
                    Some(Value::Array(items)) if items.len() > longest_list.1 => {
                        longest_list = (field(&id).map_or("", |f| f.id), items.len());
                    }
                    _ => {}
                }
            }
        }
        assert!(widest_map > 0, "the corpus should carry language maps at all");
        assert!(longest_list.1 > 0, "the corpus should carry lists at all");
        assert!(
            widest_map < MAX_VALUES_PER_FIELD,
            "the widest committed language map is {widest_map} tags, at or above the cap of \
             {MAX_VALUES_PER_FIELD} — raise the cap rather than refusing a published project"
        );
        assert!(
            longest_list.1 < MAX_VALUES_PER_FIELD,
            "the longest committed list is {} at {} entries, at or above the cap of \
             {MAX_VALUES_PER_FIELD} — raise the cap rather than refusing a published project",
            longest_list.0,
            longest_list.1
        );
    }

    #[test]
    fn a_body_over_the_language_cap_is_reported_and_one_at_the_cap_is_not() {
        use crate::form::{FormBody, MAX_VALUES_PER_FIELD};

        let section = crate::form::registry::section("overview").expect("overview");
        let at_cap: Vec<(String, String)> = (0..MAX_VALUES_PER_FIELD)
            .map(|n| (format!("description.l{n}"), "text".to_string()))
            .collect();
        assert!(
            over_cap(Audience::Everyone, section, &FormBody::from_pairs(at_cap.clone())).is_empty(),
            "exactly at the cap is allowed"
        );

        let mut over = at_cap;
        over.push(("description.zz".to_string(), "one too many".to_string()));
        assert_eq!(
            over_cap(Audience::Everyone, section, &FormBody::from_pairs(over))
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            ["description"],
            "one past the cap is refused, and named"
        );
    }

    #[test]
    fn comma_separated_data_languages_count_as_separate_values_towards_the_cap() {
        use crate::form::{FormBody, MAX_VALUES_PER_FIELD};

        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let parts = |count: usize| (0..count).map(|n| format!("x{n}")).collect::<Vec<_>>().join(", ");
        let over_for = |count: usize| {
            let body = FormBody::from_pairs(vec![("dataLanguage".to_string(), parts(count))]);
            over_cap(Audience::Everyone, dataset, &body)
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>()
        };
        assert!(over_for(MAX_VALUES_PER_FIELD).is_empty(), "exactly at the cap is allowed");
        assert_eq!(over_for(MAX_VALUES_PER_FIELD + 1), ["dataLanguage"]);
    }

    #[test]
    fn a_language_list_is_capped_on_its_lines_not_only_its_rows() {
        use crate::form::{FormBody, MAX_VALUES_PER_FIELD};

        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let mut body = vec![("keywords.row".to_string(), "r0".to_string())];
        for _ in 0..=MAX_VALUES_PER_FIELD {
            body.push(("keywords.r0.lang".to_string(), "en".to_string()));
            body.push(("keywords.r0.text".to_string(), "x".to_string()));
        }
        assert_eq!(
            over_cap(Audience::Everyone, dataset, &FormBody::from_pairs(body))
                .iter()
                .map(|f| f.id)
                .collect::<Vec<_>>(),
            ["keywords"]
        );
    }

    #[test]
    fn a_line_with_text_and_no_language_is_flagged_and_a_blank_or_tagged_one_is_not() {
        use crate::form::FormBody;

        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let pairs = |lines: &[(&str, &str)]| {
            let mut body = vec![("keywords.row".to_string(), "r0".to_string())];
            for (tag, text) in lines {
                body.push(("keywords.r0.lang".to_string(), (*tag).to_string()));
                body.push(("keywords.r0.text".to_string(), (*text).to_string()));
            }
            FormBody::from_pairs(body)
        };
        let flagged = |lines: &[(&str, &str)]| {
            lines_without_language(Audience::Everyone, dataset, &[], &pairs(lines))
                .iter()
                .map(|line| (line.control.clone(), line.line))
                .collect::<Vec<_>>()
        };

        assert_eq!(
            flagged(&[("en", "manuscripts"), ("", "Handschriften"), ("EN1", " x ")]),
            [("keywords.r0.lang".to_string(), 1), ("keywords.r0.lang".to_string(), 2)]
        );
        assert!(flagged(&[("", ""), ("", "   "), (" EN ", "manuscripts")]).is_empty());
    }

    #[test]
    fn after_a_save_only_a_marked_line_still_without_a_language_stays_flagged() {
        use crate::form::FormBody;

        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let body = |marker: &str, second_tag: &str| {
            FormBody::from_pairs(
                [
                    ("keywords.row", "r0"),
                    ("keywords.r0.lang", ""),
                    ("keywords.r0.text", "typed, language not reached yet"),
                    ("keywords.r0.lang", second_tag),
                    ("keywords.r0.text", "flagged by the save"),
                    ("keywords.r0.lang-missing", marker),
                ]
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .to_vec(),
            )
        };
        let still = |body: &FormBody| {
            lines_still_without_language(Audience::Everyone, dataset, &[], body)
                .iter()
                .map(|line| (line.marker(), line.line))
                .collect::<Vec<_>>()
        };

        assert_eq!(still(&body("1", "")), [("keywords.r0.lang-missing".to_string(), 1)]);
        assert!(still(&body("1", "de")).is_empty(), "a language was chosen");
        assert!(still(&body("", "")).is_empty(), "nothing was flagged yet");
    }

    #[test]
    fn a_tag_the_applier_stores_is_never_reported_and_one_it_drops_always_is() {
        use crate::form::{apply_multilingual_rows, FormBody};

        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let lines = [
            ("en", "a"),
            (" DE ", "b"),
            ("Fr", "c"),
            ("", "d"),
            ("en1", "e"),
            ("toolong", "f"),
        ];
        let mut body = vec![("keywords.row".to_string(), "r0".to_string())];
        for (tag, text) in lines {
            body.push(("keywords.r0.lang".to_string(), tag.to_string()));
            body.push(("keywords.r0.text".to_string(), text.to_string()));
        }
        let body = FormBody::from_pairs(body);

        let mut stored = ProjectDraft::default();
        apply_multilingual_rows(&body, &mut stored, "keywords");
        let stored_text = serde_json::to_string(&stored.get("keywords")).expect("serialises");
        let reported: Vec<usize> = lines_without_language(Audience::Everyone, dataset, &[], &body)
            .iter()
            .map(|line| line.line)
            .collect();

        for (position, (_, text)) in lines.iter().enumerate() {
            let is_stored = stored_text.contains(&format!("\"{text}\""));
            assert_eq!(is_stored, !reported.contains(&position), "line {position} ({text})");
        }
    }

    #[test]
    fn a_repeated_tag_does_not_count_twice_towards_the_cap() {
        use crate::form::{FormBody, MAX_VALUES_PER_FIELD};

        // The cap counts distinct tags, which is what the stored map holds.
        let section = crate::form::registry::section("overview").expect("overview");
        let repeated: Vec<(String, String)> = (0..MAX_VALUES_PER_FIELD * 4)
            .map(|_| ("description.en".to_string(), "text".to_string()))
            .collect();
        assert!(over_cap(Audience::Everyone, section, &FormBody::from_pairs(repeated)).is_empty());
    }

    #[test]
    fn the_cap_never_names_a_field_the_section_does_not_show() {
        use crate::form::FormBody;

        // Section-scoped because a POST carries one section; a refusal naming a
        // field the reader is not posting would be unactionable.
        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let over: Vec<(String, String)> = (0..500).map(|n| (format!("description.l{n}"), "text".to_string())).collect();
        assert!(over_cap(Audience::Everyone, dataset, &FormBody::from_pairs(over)).is_empty());
    }

    #[test]
    fn an_unlisted_language_is_refused_naming_field_and_value() {
        let mut draft = draft();
        draft.set("dataLanguage", json!(["hu", "Belgian"]));
        let named: Vec<(&str, String)> = unlisted_languages(Audience::Everyone, &draft, None)
            .into_iter()
            .map(|(field, value)| (field.id, value))
            .collect();
        assert_eq!(named, [("dataLanguage", "Belgian".to_string())]);
    }

    #[test]
    fn an_unlisted_language_the_published_project_already_holds_passes() {
        let mut published = draft();
        published.set("dataLanguage", json!(["Belgian"]));
        let mut submitted = draft();
        submitted.set("dataLanguage", json!(["hu", "Belgian", "Flemish"]));
        let named: Vec<String> = unlisted_languages(Audience::Everyone, &submitted, Some(&published))
            .into_iter()
            .map(|(_, value)| value)
            .collect();
        assert_eq!(named, ["Flemish".to_string()], "only the value published does not hold");
    }

    #[test]
    fn listed_language_codes_pass() {
        let mut draft = draft();
        draft.set("dataLanguage", json!(["hu", "de", "la"]));
        assert!(unlisted_languages(Audience::Everyone, &draft, None).is_empty());
    }

    fn agents() -> crate::agents::Agents {
        let dir = crate::checkout_dpe_data_dir();
        let (agents, errors) = crate::agents::Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
        assert!(errors.is_empty(), "{errors:?}");
        agents
    }

    #[test]
    fn no_published_project_is_refused_for_an_unresolvable_agent() {
        // Every committed agent id resolves, so submit refuses no live project.
        let agents = agents();
        let scope = AgentScope::published_only(&agents);
        let dir = crate::checkout_dpe_data_dir().join("projects");
        let (published, errors) = crate::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "{errors:?}");

        let mut refused: Vec<String> = Vec::new();
        for summary in published.summaries() {
            let raw = published.get(summary.shortcode).expect("a summary names a loaded project");
            let draft = ProjectDraft::from_raw(raw);
            for audience in [Audience::Everyone, Audience::RduOnly] {
                for (field, id) in unresolved_agents(audience, &draft, &scope) {
                    refused.push(format!("{}: {}={id}", summary.shortcode, field.id));
                }
            }
        }
        assert!(refused.is_empty(), "published projects refused for a reference: {refused:?}");
    }

    #[test]
    fn a_grant_funder_that_resolves_to_nobody_is_refused() {
        // `funding` declares Shape::FundingRows, so a filter over AgentRows |
        // AttributionRows walks past it and a dangling funder reaches a published
        // file.
        let agents = agents();
        let mut draft = draft();
        draft.set(
            "funding",
            json!([{ "funders": ["organization-002", "organization-99999"], "number": "1" }]),
        );
        let scope = AgentScope::published_only(&agents);
        let named: Vec<(&str, String)> = unresolved_agents(Audience::Everyone, &draft, &scope)
            .into_iter()
            .map(|(field, id)| (field.id, id))
            .collect();
        assert_eq!(named, [("funding", "organization-99999".to_string())]);
    }

    #[test]
    fn every_funder_of_a_grant_is_checked_not_only_the_first() {
        // One row can hold several references.
        let agents = agents();
        let mut draft = draft();
        draft.set(
            "funding",
            json!([{ "funders": ["organization-002", "person-99998", "organization-99999"] }]),
        );
        let scope = AgentScope::published_only(&agents);
        let ids: Vec<String> = unresolved_agents(Audience::Everyone, &draft, &scope)
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        assert_eq!(ids, ["person-99998".to_string(), "organization-99999".to_string()]);
    }

    #[test]
    fn free_text_funding_holds_no_references_to_check() {
        // Free-text funding has no funders, and reading one out of it would report
        // the prose as a dangling id.
        let agents = agents();
        let mut draft = draft();
        draft.set("funding", json!("No funding"));
        assert!(unresolved_agents(Audience::Everyone, &draft, &AgentScope::published_only(&agents)).is_empty());
    }

    #[test]
    fn an_id_that_resolves_to_nobody_is_named_with_its_field() {
        // The id as well as the field: attributions reaches dozens of rows.
        let agents = agents();
        let mut draft = draft();
        draft.set("contactPoint", json!(["organization-008", "person-99999"]));
        let scope = AgentScope::published_only(&agents);
        let named: Vec<(&str, String)> = unresolved_agents(Audience::Everyone, &draft, &scope)
            .into_iter()
            .map(|(field, id)| (field.id, id))
            .collect();
        assert_eq!(named, [("contactPoint", "person-99999".to_string())]);
    }

    #[test]
    fn an_empty_agent_set_refuses_every_reference_rather_than_accepting_them() {
        // A deployment with no data directory: accepting references nothing can
        // resolve would publish bare ids.
        let mut draft = draft();
        draft.set("contactPoint", json!(["organization-008"]));
        let empty = crate::agents::Agents::default();
        let scope = AgentScope::published_only(&empty);
        let refused: Vec<(&str, String)> = unresolved_agents(Audience::Everyone, &draft, &scope)
            .into_iter()
            .map(|(field, id)| (field.id, id))
            .collect();
        assert!(
            refused.contains(&("contactPoint", "organization-008".to_string())),
            "{refused:?}"
        );
        // 0801d's own contributors are refused too, which is the point: every
        // field that refers to an agent is checked, not just the one edited.
        assert!(refused.iter().any(|(field, _)| *field == "attributions"), "{refused:?}");
    }

    #[test]
    fn drop_agent_removes_every_reference_and_keeps_what_else_the_fields_hold() {
        let mut draft = draft();
        draft.set(
            "attributions",
            json!([
                { "contributor": "person-417", "roles": ["Researcher"] },
                { "contributor": "person-001", "roles": ["Researcher"] },
            ]),
        );
        draft.set("contactPoint", json!(["person-417"]));
        draft.set(
            "funding",
            json!([{ "funders": ["person-417", "organization-002"], "number": "1" }]),
        );

        assert_eq!(drop_agent(&mut draft, "person-417"), 3);

        assert_eq!(
            draft.get("attributions"),
            Some(&json!([{ "contributor": "person-001", "roles": ["Researcher"] }]))
        );
        assert_eq!(draft.get("contactPoint"), None, "an emptied field is removed, not stored as []");
        assert_eq!(
            draft.get("funding"),
            Some(&json!([{ "funders": ["organization-002"], "number": "1" }]))
        );
    }

    #[test]
    fn drop_agent_keeps_a_grant_whose_only_funder_it_removes() {
        let mut draft = draft();
        draft.set("funding", json!([{ "funders": ["person-417"], "number": "1" }]));
        assert_eq!(drop_agent(&mut draft, "person-417"), 1);
        assert_eq!(draft.get("funding"), Some(&json!([{ "funders": [], "number": "1" }])));
    }

    #[test]
    fn drop_agent_on_an_id_held_nowhere_changes_nothing() {
        let mut draft = draft();
        let before = serde_json::to_string(&draft.to_raw().expect("the corpus draft is valid")).expect("serialize");
        assert_eq!(drop_agent(&mut draft, "person-99999"), 0);
        let after = serde_json::to_string(&draft.to_raw().expect("still valid")).expect("serialize");
        assert_eq!(before, after);
    }

    #[test]
    fn a_field_the_form_does_not_read_is_never_refused() {
        // No applier touches a field with no shape, so a sentinel in it came from
        // the published data; `legalInfo` is maintained by RDU elsewhere.
        let mut draft = draft();
        draft.set("legalInfo", json!(["MISSING"]));
        assert!(
            field("legalInfo").expect("legalInfo").shape.is_none(),
            "the premise of this test"
        );
        assert!(typed_sentinels(Audience::RduOnly, &draft).is_empty());
    }
}
