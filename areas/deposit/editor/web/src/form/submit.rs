//! What submit refuses that needs the field registry to decide.
//!
//! `ProjectDraft::to_raw` is a contract question and `unresolved_temporal_coverage`
//! a data question, so both live in `editor-core`; these need a field's declared
//! shape and audience, which is registry knowledge, and `server -> web -> core`
//! puts that here. [`obligation::unsatisfied_required`](super::obligation::unsatisfied_required)
//! stays in its own module because the rail counts presence through the same
//! function.

use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::{FormBody, Shape, WhenCleared, MAX_VALUES_PER_FIELD};
use serde_json::Value;
use shared_metadata::is_placeholder;

use super::registry::{sections_for, Audience, Field, Section};

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
            // Repeated under one name rather than suffixed.
            Some(Shape::StringList(_)) => body.exceeds_all(field.id, MAX_VALUES_PER_FIELD),
            // Rows are repeated `{field}.row` values, and each row's own languages
            // are bounded by the same cap under their own prefix.
            Some(
                Shape::MultilingualRows
                | Shape::StringRows
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
        .filter(|field| {
            matches!(
                field.shape,
                Some(Shape::AgentRows | Shape::AttributionRows | Shape::FundingRows)
            )
        })
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
        let dir = editor_core::checkout_dpe_data_dir().join("projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    /// The dead end this module closes, reproduced through the real applier.
    #[test]
    fn a_typed_sentinel_in_an_optional_field_cannot_be_cleared_again() {
        use editor_core::form::{apply, FormBody};

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
        let dir = editor_core::checkout_dpe_data_dir().join("projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
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
        use editor_core::form::MAX_VALUES_PER_FIELD;

        let dir = editor_core::checkout_dpe_data_dir().join("projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");

        let mut widest_map = 0;
        let mut longest_list = ("", 0);
        for shortcode in published.summaries().map(|s| s.shortcode.to_string()) {
            let raw = published.get(&shortcode).expect("a summary names a loaded project");
            let draft = ProjectDraft::from_raw(raw);
            for id in draft.fields().map(str::to_string).collect::<Vec<_>>() {
                match draft.get(&id) {
                    Some(Value::Object(map)) if crate::form::widgets::is_language_map(&Value::Object(map.clone())) => {
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
        use editor_core::form::{FormBody, MAX_VALUES_PER_FIELD};

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
    fn a_repeated_tag_does_not_count_twice_towards_the_cap() {
        use editor_core::form::{FormBody, MAX_VALUES_PER_FIELD};

        // The cap counts distinct tags, which is what the stored map holds.
        let section = crate::form::registry::section("overview").expect("overview");
        let repeated: Vec<(String, String)> = (0..MAX_VALUES_PER_FIELD * 4)
            .map(|_| ("description.en".to_string(), "text".to_string()))
            .collect();
        assert!(over_cap(Audience::Everyone, section, &FormBody::from_pairs(repeated)).is_empty());
    }

    #[test]
    fn the_cap_never_names_a_field_the_section_does_not_show() {
        use editor_core::form::FormBody;

        // Section-scoped because a POST carries one section; a refusal naming a
        // field the reader is not posting would be unactionable.
        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let over: Vec<(String, String)> = (0..500).map(|n| (format!("description.l{n}"), "text".to_string())).collect();
        assert!(over_cap(Audience::Everyone, dataset, &FormBody::from_pairs(over)).is_empty());
    }

    fn agents() -> editor_core::agents::Agents {
        let dir = editor_core::checkout_dpe_data_dir();
        let (agents, errors) = editor_core::agents::Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
        assert!(errors.is_empty(), "{errors:?}");
        agents
    }

    #[test]
    fn no_published_project_is_refused_for_an_unresolvable_agent() {
        // Every committed agent id resolves, so submit refuses no live project.
        let agents = agents();
        let scope = AgentScope::published_only(&agents);
        let dir = editor_core::checkout_dpe_data_dir().join("projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
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
        let empty = editor_core::agents::Agents::default();
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
