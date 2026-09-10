//! What submit refuses that needs the field registry to decide.
//!
//! The two other submit gates do not: `ProjectDraft::to_raw` is a contract question and
//! `unresolved_temporal_coverage` is a data question, so both live in `editor-core` beside what
//! they read. These need a field's *declared shape* and its audience, which is registry knowledge,
//! and `server -> web -> core` puts that here.
//!
//! [`obligation::unsatisfied_required`](super::obligation::unsatisfied_required) is the third such
//! gate and stays in its own module, because presence is what the section rail counts with and the
//! two must read it through one function.

use editor_core::agents::AgentScope;
use editor_core::draft::ProjectDraft;
use editor_core::form::{FormBody, Shape, WhenCleared, MAX_VALUES_PER_FIELD};
use platform_metadata::is_placeholder;
use serde_json::Value;

use super::registry::{sections_for, Audience, Field, Section};

/// Every field whose stored value is a placeholder sentinel the editor could not legitimately have
/// written there, which leaves one way for it to have arrived: a depositor typed it.
///
/// Such a value is a dead end. A submitted value is stored verbatim, so typing `MISSING` stores
/// what DPE and OAI-PMH filter out, renders as an *empty* control, and then survives every later
/// empty submit because `apply_text` reads that as "not a clear" — the field reads empty, will not
/// clear, and is only editable by typing some other value first.
///
/// ## The rule is read off the shape, not off the value
///
/// A sentinel in a draft is usually **correct**: [`WhenCleared::Placeholder`] means the editor
/// itself writes one when a depositor clears the field, and the committed corpus is full of them.
/// Refusing every stored sentinel would refuse those, so the question is not "is this a sentinel"
/// but "could clearing this field have produced one":
///
/// - [`WhenCleared::Placeholder`] — a sentinel *is* the cleared state, so typing one is
///   indistinguishable from clearing and has the same effect. Allowed, and nothing is lost by
///   allowing it.
/// - [`WhenCleared::Drop`] — the cleared state is an absent member, so a stored sentinel cannot
///   have come from clearing. Refused.
/// - [`Shape::Multilingual`] — an empty text drops its tag (`DraftMultilingual::to_contract`), so
///   the same argument applies per tag. Refused.
///
/// That is why the check is derived from the registry's declared shape rather than from a
/// comparison against the published project: a project with no published counterpart has nothing to
/// compare against, and the shape answers for it anyway.
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
        // A row of plain strings, so the same argument as an open list applies:
        // a depositor can type anything, including a sentinel.
        // Same rule as the variant rows below: everything a depositor types is
        // checked, and a reference `url` is excused because the published data
        // spells that member with a sentinel of its own.
        Shape::ReferenceRows(_) => value.as_array().is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.iter().filter(|(member, _)| *member != "url").map(|(_, text)| text))
                .filter_map(Value::as_str)
                .any(is_placeholder)
        }),
        // A citation, an identifier, a grant number and a funding note are all
        // typed, and none of the committed values is a sentinel, so every
        // string in them is checked.
        Shape::PublicationRows | Shape::FundingRows => holds_placeholder(value),
        // A text row is a language map and a reference row's label is typed, so
        // both are checked — but a reference's **`url` is excused**, for the
        // same reason `Shape::Url` is entirely: it is a member the published
        // data spells with a sentinel of its own.
        // `0110_h-steiner` holds `{"type": "URL", "url": "MISSING"}`, and
        // refusing it would make a live project unsubmittable over data it did
        // not write. It is also the only committed sentinel anywhere in either
        // field, so nothing else here has to be given up.
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
        // Each row is a language map, so the same argument as `Multilingual`
        // applies per row: an empty text drops its tag, so a sentinel cannot
        // have come from clearing one.
        Shape::MultilingualRows => value.as_array().is_some_and(|rows| {
            rows.iter()
                .filter_map(Value::as_object)
                .flat_map(|row| row.values())
                .filter_map(Value::as_str)
                .any(is_placeholder)
        }),
        // An open list can hold anything a depositor types, so a sentinel
        // typed as a language tag reaches the file and the platform then reads
        // that entry as no value. A closed set cannot: `apply_string_list`
        // stores only an offered value, and no vocabulary offers a sentinel.
        Shape::StringList(_) => value
            .as_array()
            .is_some_and(|items| items.iter().filter_map(Value::as_str).any(is_placeholder)),
        // A URL slot's sentinel is the published data's own empty state —
        // two projects hold `url: ["MISSING"]` — and `apply_url` leaves it
        // alone rather than writing over it, exactly as
        // `WhenCleared::Placeholder` does.
        Shape::Url(_) => false,
        // A closed set cannot hold a sentinel: `apply_choice` stores only a
        // listed value, and `registry`'s contract test pins that every
        // committed value is one too. So a sentinel here came from the
        // published data, not from a depositor.
        Shape::Choice(_) => false,
        Shape::Multilingual => value
            .as_object()
            .is_some_and(|texts| texts.values().filter_map(Value::as_str).any(is_placeholder)),
    }
}

/// Every field in `section` whose posted body carries more values than one field
/// may hold.
///
/// The cap a depositor can *see*: [`FormBody::entries`] stops at [`MAX_VALUES_PER_FIELD`] on its
/// own, which silently discards everything past it, so the value stored is quietly not what was
/// sent.
///
/// Checked **before any applier runs**, and on a save as well as a submit — deferred to submit
/// alone, an over-cap save truncates and stores, and the submit that follows sees a draft already
/// within the cap and passes it.
///
/// One number rather than a cap per field: this is input hygiene, not a product rule about how many
/// languages a description may have.
#[must_use]
pub fn over_cap(audience: Audience, section: &Section, body: &FormBody) -> Vec<&'static Field> {
    section
        .fields_for(audience)
        .filter(|field| match field.shape {
            // A scalar posts one value under its own name. A body repeating it
            // is a hand-built one, and `FormBody::get` takes the first — there
            // is no accumulation to bound.
            None | Some(Shape::Text(_) | Shape::Choice(_) | Shape::Url(_)) => false,
            Some(Shape::Multilingual) => body.exceeds_entries(field.id, MAX_VALUES_PER_FIELD),
            // Repeated under one name rather than suffixed, so it needs the
            // other counter — a checkbox group and an "add another" input both
            // post under the field's own name.
            Some(Shape::StringList(_)) => body.exceeds_all(field.id, MAX_VALUES_PER_FIELD),
            // Rows arrive as repeated `{field}.row` values, so the count to
            // bound is the number of rows — each row's own languages are
            // bounded by the same cap under its own prefix, which is why this
            // does not have to walk them.
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
/// offending ids.
///
/// The submit-time half of `Shape::AgentRows`. The applier stores whatever arrives, because a draft
/// is allowed to hold a value that does not validate and deciding that is submit's job — so this is
/// what stops an unresolvable reference reaching a published file, where it would render as a bare
/// `person-001` on the public project page.
///
/// **A dangling reference in a field the depositor did not touch is still refused**, deliberately:
/// every committed reference resolves
/// (`agents::tests::every_id_the_committed_projects_refer_to_resolves`), so a dangling one can only
/// have arrived after the fact — an agent file removed from under a project, which is a thing to
/// report rather than to publish.
///
/// **`funding[].funders` is a fourth agent field and must stay in this filter.** It declares
/// [`Shape::FundingRows`], so a filter naming only `AgentRows | AttributionRows` misses its 125
/// references and lets a dangling funder reach a published file — where the project page renders a
/// bare `organization-008`. `checks::contributor_refs` does not report funders either, so this
/// gate is the only one that does.
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
                // An `AgentRows` row *is* the id; an `AttributionRows` row
                // holds it under `contributor` beside its roles; a
                // `FundingRows` row holds a whole list of them under `funders`.
                // Read here rather than in three functions, so a field that
                // refers to an agent cannot be added to one and forgotten in
                // the others.
                .flat_map(agent_ids_in_row)
                .filter(|id| !agents.has(id))
                .map(move |id| (field, id.to_string()))
                .collect::<Vec<(&'static Field, String)>>()
        })
        .collect()
}

/// Every agent id one row of an agent-bearing field holds.
///
/// The three shapes spell a reference differently, and this is the single place
/// that knows how: a bare string for `AgentRows`, `contributor` for
/// `AttributionRows`, and a `funders` array for `FundingRows`. A grant with
/// several funders yields several ids, which is why this returns a list rather
/// than an `Option`.
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
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
        let (published, errors) = editor_core::published::PublishedProjects::load_from(&dir);
        assert!(errors.is_empty(), "the committed corpus should load: {errors:?}");
        ProjectDraft::from_raw(published.get("0801d").expect("0801d is in the committed corpus"))
    }

    /// The dead end this module exists to close, reproduced end to end through
    /// the real applier rather than asserted about it.
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
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
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
        // The canary for the test above, which asserts an absence: with no
        // sentinel anywhere in the corpus it would pass while proving nothing.
        let draft = draft();
        let held: Vec<&str> = draft
            .fields()
            .filter(|id| draft.get(id).and_then(Value::as_str).is_some_and(is_placeholder))
            .collect();
        assert!(!held.is_empty(), "0801d should carry at least one legitimate sentinel");
    }

    #[test]
    fn a_sentinel_is_allowed_where_clearing_the_field_writes_one() {
        // `endDate` is `WhenCleared::Placeholder`, so `"MISSING"` is what the editor writes when a
        // depositor clears it, and the committed projects hold it. Refusing it would make
        // every ongoing project unsubmittable.
        let mut draft = draft();
        draft.set("endDate", json!("MISSING"));
        assert!(typed_sentinels(Audience::Everyone, &draft).is_empty());
    }

    #[test]
    fn a_sentinel_typed_into_one_language_of_a_map_is_caught() {
        // An empty text drops its tag, so a sentinel cannot have come from
        // clearing — and unlike a scalar it renders *visibly* in the textarea,
        // which is why the refusal has to name the field rather than trusting
        // the depositor to notice.
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
        // `is_placeholder` matches `CALCULATED` as well as `MISSING`, so a fix
        // naming only the one the bug report mentioned would leave half of it
        // open. An exact compare is also what keeps a real value that merely
        // contains the word out of the refusal.
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

    /// The most values any single list in the committed corpus holds, measured
    /// rather than assumed: `attributions` reaches 56 and `publications` 50,
    /// while the widest language map is 3 tags.
    ///
    /// Here so the cap and the data cannot silently invert. A project arriving
    /// with more values than the cap allows would be refused on a save, which
    /// is a depositor blocked by input hygiene — so this fails first, and
    /// somebody raises the cap deliberately.
    #[test]
    fn the_cap_is_above_anything_the_committed_corpus_holds() {
        use editor_core::form::MAX_VALUES_PER_FIELD;

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
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

        // The cap counts distinct tags, because that is what the stored map
        // holds — `apply_multilingual` folds case and keeps the first. A body
        // repeating one tag many times is odd, but it is not a large value, and
        // refusing it would refuse something that stores as a single entry.
        let section = crate::form::registry::section("overview").expect("overview");
        let repeated: Vec<(String, String)> = (0..MAX_VALUES_PER_FIELD * 4)
            .map(|_| ("description.en".to_string(), "text".to_string()))
            .collect();
        assert!(over_cap(Audience::Everyone, section, &FormBody::from_pairs(repeated)).is_empty());
    }

    #[test]
    fn the_cap_never_names_a_field_the_section_does_not_show() {
        use editor_core::form::FormBody;

        // The check is section-scoped because a POST carries one section, and a
        // refusal naming a field the reader is not posting would be
        // unactionable. `description` is in overview, so a body posting it
        // against the dataset section is reported by neither.
        let dataset = crate::form::registry::section("dataset").expect("dataset");
        let over: Vec<(String, String)> = (0..500).map(|n| (format!("description.l{n}"), "text".to_string())).collect();
        assert!(over_cap(Audience::Everyone, dataset, &FormBody::from_pairs(over)).is_empty());
    }

    fn agents() -> editor_core::agents::Agents {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data");
        let (agents, errors) = editor_core::agents::Agents::load_from(&dir.join("persons"), &dir.join("organizations"));
        assert!(errors.is_empty(), "{errors:?}");
        agents
    }

    #[test]
    fn no_published_project_is_refused_for_an_unresolvable_agent() {
        // The same guarantee the required tier has, for references: every committed agent id
        // resolves, so submit refuses no project already live. A failure here means the
        // agent store lost a file or a project gained a reference to nothing.
        let agents = agents();
        let scope = AgentScope::published_only(&agents);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dpe/server/data/projects");
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
        // `funding` declares `Shape::FundingRows`, so a filter over
        // `AgentRows | AttributionRows` walked past it: the form warned in the
        // funder's label and submit then accepted the project anyway, and
        // `checks::contributor_refs` does not report funders either. A dangling
        // funder reached a published file and rendered as a bare
        // `organization-99999` on the public project page.
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
        // A grant carries a list, so one row can hold several references. Taking
        // only the first would let a second dangling funder through behind a
        // good one.
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
        // `Funding` is `#[serde(untagged)]`:
        // a project whose funding is a single string has no funders at all, and
        // reading one out of it would report the prose as a dangling id.
        let agents = agents();
        let mut draft = draft();
        draft.set("funding", json!("No funding"));
        assert!(unresolved_agents(Audience::Everyone, &draft, &AgentScope::published_only(&agents)).is_empty());
    }

    #[test]
    fn an_id_that_resolves_to_nobody_is_named_with_its_field() {
        // The refusal has to name the id as well as the field: `attributions`
        // reaches 56 rows on one committed project, so "one of these is wrong"
        // is not actionable.
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
        // What a deployment with no data directory looks like. The fail-safe
        // direction, and the same one the temporal tables take: accepting
        // references nothing can resolve would publish a page rendering bare
        // ids.
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
        // No applier touches it, so its value came from the published data and rides through
        // unchanged. Refusing it would strand a project on a field that renders no control
        // at all.
        let mut draft = draft();
        // Every editable field has a control now, so the fields with no shape
        // are exactly the display-only ones — which is the case that still
        // matters: `legalInfo` is maintained by RDU elsewhere and rides through
        // a save untouched, so a sentinel in it came from the
        // published data.
        draft.set("legalInfo", json!(["MISSING"]));
        assert!(
            field("legalInfo").expect("legalInfo").shape.is_none(),
            "the premise of this test"
        );
        assert!(typed_sentinels(Audience::RduOnly, &draft).is_empty());
    }
}
