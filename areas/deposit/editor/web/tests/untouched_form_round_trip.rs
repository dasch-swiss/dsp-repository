//! Saving a form nobody edited must not change the project, for every committed
//! file. `editor-core`'s `canonical_round_trip` pins `load -> draft -> write`;
//! this one puts the form in the middle, with the submit carrying exactly what
//! an untouched control would post.
//!
//! In `editor-web` because it derives its table from
//! [`registry::Field`](editor_web::form::registry::Field)'s declared shapes, so a
//! field whose shape is declared is covered automatically. Asserted against the
//! committed bytes rather than a fixture because every way this fails is silent:
//! the only symptom is a pull request touching dozens of projects nobody edited.

use std::path::PathBuf;

use editor_core::canonical::{write_draft, write_project};
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody, Shape};
use editor_core::multilingual::{DraftMultilingual, UI_LANGUAGES};
use editor_web::form::registry::{Field, FIELDS};
use serde_json::Value;
use shared_metadata::project::ProjectRaw;

/// Every field whose shape the registry declares, which is exactly the set an
/// applier reads and therefore exactly the set that can rewrite a file.
fn read_fields() -> Vec<(&'static Field, Shape)> {
    let fields: Vec<(&Field, Shape)> = FIELDS.iter().filter_map(|field| Some((field, field.shape?))).collect();
    assert!(
        !fields.is_empty(),
        "no field declares a shape, so this test would pass over an empty submit"
    );
    fields
}

fn projects_dir() -> PathBuf {
    editor_core::checkout_dpe_data_dir().join("projects")
}

fn project_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(projects_dir())
        .expect("the projects data directory should be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no project files were found");
    files
}

/// What an untouched form would post for `draft`: the stored value per control,
/// except that a placeholder renders as empty. Every field with a declared shape
/// is present, because a section posts its own fields whether or not they hold
/// anything.
fn untouched_submit(draft: &ProjectDraft) -> FormBody {
    let mut pairs: Vec<(String, String)> = Vec::new();

    for (field, shape) in read_fields() {
        match shape {
            Shape::Text(_) => {
                let rendered = match draft.get(field.id).and_then(Value::as_str) {
                    // The sentinel is not shown to a reader, so the control is
                    // empty.
                    Some(value) if shared_metadata::is_placeholder(value) => String::new(),
                    Some(value) => value.to_string(),
                    // An absent field renders an empty control, which posts
                    // empty.
                    None => String::new(),
                };
                pairs.push((field.id.to_string(), rendered));
            }
            Shape::ReferenceRows(_) => {
                let rows = draft.get(field.id).and_then(Value::as_array).cloned().unwrap_or_default();
                if rows.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, row) in rows.iter().enumerate() {
                    let key = format!("r{position}");
                    let prefix = format!("{}.{key}", field.id);
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    for (member, name) in [("type", "type"), ("url", "url"), ("text", "label")] {
                        pairs.push((
                            format!("{prefix}.ref.{name}"),
                            row.get(member).and_then(Value::as_str).unwrap_or_default().to_string(),
                        ));
                    }
                }
            }
            Shape::PublicationRows => {
                let rows = draft.get(field.id).and_then(Value::as_array).cloned().unwrap_or_default();
                if rows.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, row) in rows.iter().enumerate() {
                    let key = format!("r{position}");
                    let prefix = format!("{}.{key}", field.id);
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    pairs.push((
                        format!("{prefix}.text"),
                        row.get("text").and_then(Value::as_str).unwrap_or_default().to_string(),
                    ));
                    pairs.push((
                        format!("{prefix}.pid"),
                        row.get("pid")
                            .and_then(|pid| pid.get("url"))
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    ));
                }
            }
            Shape::FundingRows => {
                // The discriminant is on the field, so both branches post once
                // rather than per row.
                let held = draft.get(field.id);
                let is_grants = !held.is_some_and(Value::is_string);
                pairs.push((
                    format!("{}.kind", field.id),
                    if is_grants { "grants" } else { "text" }.to_string(),
                ));
                pairs.push((
                    format!("{}.text", field.id),
                    held.and_then(Value::as_str).unwrap_or_default().to_string(),
                ));
                let grants = held.and_then(Value::as_array).cloned().unwrap_or_default();
                if grants.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, grant) in grants.iter().enumerate() {
                    let key = format!("r{position}");
                    let prefix = format!("{}.{key}", field.id);
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    for id in grant
                        .get("funders")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                    {
                        pairs.push((format!("{prefix}.funder"), id.to_string()));
                    }
                    // The trailing blank funder control.
                    pairs.push((format!("{prefix}.funder"), String::new()));
                    for member in ["number", "name", "url"] {
                        pairs.push((
                            format!("{prefix}.{member}"),
                            grant.get(member).and_then(Value::as_str).unwrap_or_default().to_string(),
                        ));
                    }
                }
            }
            Shape::TextOrReferenceRows(_) => {
                // Both branches are in the DOM, the inactive one only hidden, so an
                // untouched form posts both candidates plus the discriminant.
                let rows = draft.get(field.id).and_then(Value::as_array).cloned().unwrap_or_default();
                if rows.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, row) in rows.iter().enumerate() {
                    let key = format!("r{position}");
                    let prefix = format!("{}.{key}", field.id);
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    let is_reference = row.get("url").is_some();
                    pairs.push((
                        format!("{prefix}.kind"),
                        if is_reference { "reference" } else { "text" }.to_string(),
                    ));
                    // The reference branch, empty on a text row.
                    for (member, name) in [("type", "type"), ("url", "url"), ("text", "label")] {
                        pairs.push((
                            format!("{prefix}.ref.{name}"),
                            row.get(member).and_then(Value::as_str).unwrap_or_default().to_string(),
                        ));
                    }
                    // The text branch, empty on a reference row. A control per
                    // offered language plus whatever tags the value carries.
                    let texts = if is_reference {
                        DraftMultilingual::default()
                    } else {
                        editor_web::form::widgets::as_multilingual(row)
                    };
                    for tag in UI_LANGUAGES.iter().copied().chain(texts.extra_tags()) {
                        pairs.push((format!("{prefix}.text.{tag}"), texts.get(tag).unwrap_or_default().to_string()));
                    }
                }
            }
            Shape::AttributionRows => {
                // The role group posts one value per ticked box: the roles the project
                // holds, in its order.
                let rows = draft.get(field.id).and_then(Value::as_array).cloned().unwrap_or_default();
                if rows.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, row) in rows.iter().enumerate() {
                    let key = format!("r{position}");
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    pairs.push((
                        format!("{}.{key}.contributor", field.id),
                        row.get("contributor").and_then(Value::as_str).unwrap_or_default().to_string(),
                    ));
                    for role in row
                        .get("contributorType")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                    {
                        pairs.push((format!("{}.{key}.role", field.id), role.to_string()));
                    }
                    // The "add another role" input, empty on an untouched form.
                    pairs.push((format!("{}.{key}.role", field.id), String::new()));
                }
            }
            Shape::AgentRows | Shape::StringRows => {
                // The row protocol, with one text per row.
                let rows = draft.get(field.id).and_then(Value::as_array).cloned().unwrap_or_default();
                if rows.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, row) in rows.iter().enumerate() {
                    let key = format!("r{position}");
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    pairs.push((format!("{}.{key}", field.id), row.as_str().unwrap_or_default().to_string()));
                }
            }
            Shape::MultilingualRows => {
                // A hidden `{field}.row` per row, and one empty marker when the list
                // has none, so the last row can be cleared. Keys are positional on a
                // fresh render.
                let rows = draft.get(field.id).and_then(Value::as_array).cloned().unwrap_or_default();
                if rows.is_empty() {
                    pairs.push((format!("{}.row", field.id), String::new()));
                }
                for (position, row) in rows.iter().enumerate() {
                    let key = format!("r{position}");
                    pairs.push((format!("{}.row", field.id), key.clone()));
                    let stored = editor_web::form::widgets::as_multilingual(row);
                    for tag in UI_LANGUAGES.iter().copied().chain(stored.extra_tags()) {
                        pairs.push((
                            format!("{}.{key}.{tag}", field.id),
                            stored.get(tag).unwrap_or_default().to_string(),
                        ));
                    }
                }
            }
            Shape::StringList(_) => {
                // A checkbox group posts nothing when none is ticked, which leaves the
                // field alone; several committed files are in that state for typeOfData.
                for value in draft
                    .get(field.id)
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    pairs.push((field.id.to_string(), value.to_string()));
                }
            }
            Shape::Url(slot) => {
                // A placeholder renders empty; two projects hold url: ["MISSING"].
                let rendered = draft
                    .url_slot(slot)
                    .filter(|text| !shared_metadata::is_placeholder(text))
                    .unwrap_or_default();
                pairs.push((field.id.to_string(), rendered.to_string()));
            }
            Shape::Choice(_) => {
                // A project holding nothing posts nothing: no radio is checked, so the
                // field is left alone rather than cleared.
                if let Some(value) = draft.get(field.id).and_then(Value::as_str) {
                    pairs.push((field.id.to_string(), value.to_string()));
                }
            }
            Shape::Multilingual => {
                // A control per offered language whether or not the value has the tag,
                // so an untouched submit carries empty texts for the missing ones.
                let stored = draft.multilingual(field.id);
                for tag in UI_LANGUAGES {
                    pairs.push((format!("{}.{tag}", field.id), stored.get(tag).unwrap_or_default().to_string()));
                }
                for (tag, text) in stored.iter().filter(|(tag, _)| !UI_LANGUAGES.contains(tag)) {
                    pairs.push((format!("{}.{tag}", field.id), text.to_string()));
                }
            }
        }
    }

    FormBody::from_pairs(pairs)
}

/// The same draft after that body is applied back.
fn resubmit(draft: &ProjectDraft) -> ProjectDraft {
    let body = untouched_submit(draft);
    let mut resubmitted = draft.clone();
    for (field, shape) in read_fields() {
        apply(shape, &body, &mut resubmitted, field.id);
    }
    resubmitted
}

/// The first line that differs, so a failure names a place rather than dumping
/// two files.
fn first_difference(committed: &str, written: &str) -> String {
    for (line, (before, after)) in committed.lines().zip(written.lines()).enumerate() {
        if before != after {
            return format!("line {}:\n  committed: {before}\n  written:   {after}", line + 1);
        }
    }
    format!(
        "every shared line matches; the files differ in length: committed {} bytes, written {} bytes",
        committed.len(),
        written.len()
    )
}

#[test]
fn saving_an_untouched_form_leaves_every_committed_project_byte_identical() {
    let files = project_files();
    let mut differing = Vec::new();

    for path in &files {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let committed = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let raw: ProjectRaw = serde_json::from_str(&committed).unwrap_or_else(|e| panic!("parsing {name}: {e}"));
        let written = write_draft(&resubmit(&ProjectDraft::from_raw(&raw)))
            .unwrap_or_else(|e| panic!("writing {name} after an untouched submit: {e}"));

        if written != committed {
            differing.push(format!("{name}\n{}", first_difference(&committed, &written)));
        }
    }

    assert!(
        differing.is_empty(),
        "{} of {} project files changed when an untouched form was saved:\n\n{}",
        differing.len(),
        files.len(),
        differing.join("\n\n")
    );
}

#[test]
fn the_corpus_really_does_carry_the_placeholders_this_test_is_about() {
    // A positive canary: the test above asserts an absence of change and would
    // pass over a corpus with no sentinel. The other two traps are pinned by unit
    // tests in editor_core::form.
    let mut sentinels = 0;
    let mut end_date_sentinels = 0;
    for path in project_files() {
        let raw: ProjectRaw = serde_json::from_str(&std::fs::read_to_string(&path).expect("readable")).expect("parses");
        let value = serde_json::to_value(&raw).expect("serializes");
        sentinels += count_placeholders(&value);
        if shared_metadata::is_placeholder(&raw.end_date) {
            end_date_sentinels += 1;
        }
    }
    assert!(
        end_date_sentinels >= 20,
        "expected `endDate` to be a placeholder in a substantial part of the corpus, found {end_date_sentinels}"
    );
    assert!(
        sentinels >= 100,
        "expected the corpus to carry many placeholders, found {sentinels}"
    );
}

#[test]
fn an_untouched_submit_preserves_a_row_whose_text_carries_surrounding_whitespace() {
    // A positive canary for the row arm: only a row with surrounding whitespace
    // proves the trimming-preserve rule is live, and no committed row carries
    // one, so this builds one instead of scanning for it, then drives it
    // through the same `resubmit` + `write_draft` path
    // `saving_an_untouched_form_leaves_every_committed_project_byte_identical`
    // uses.
    let mut raw: ProjectRaw =
        serde_json::from_str(&std::fs::read_to_string(projects_dir().join("0801_bebb.json")).expect("readable"))
            .expect("parses");
    raw.keywords[0].insert("en".to_string(), " Bernoulli ".to_string());

    let committed = write_project(&raw).expect("the synthetic project should serialize");
    let written = write_draft(&resubmit(&ProjectDraft::from_raw(&raw))).expect("the draft should write");
    assert_eq!(
        written, committed,
        "an untouched row's surrounding whitespace should survive the submit"
    );
}

#[test]
fn the_body_this_test_submits_carries_every_field_a_shape_is_declared_for() {
    // A field whose shape is declared but which untouched_submit forgets to
    // render is one the round trip silently skips.
    let draft = ProjectDraft::from_raw(
        &serde_json::from_str::<ProjectRaw>(
            &std::fs::read_to_string(projects_dir().join("0801_bebb.json")).expect("readable"),
        )
        .expect("parses"),
    );
    let body = untouched_submit(&draft);
    for (field, shape) in read_fields() {
        match shape {
            Shape::Text(_) => assert!(body.has(field.id), "{} is declared but never posted", field.id),
            // Funding's discriminant is on the field, so that is what always
            // posts - the row marker only appears on the grants branch.
            Shape::FundingRows => assert!(
                body.has(&format!("{}.kind", field.id)),
                "{} is declared but never posts its discriminant",
                field.id
            ),
            Shape::Url(_) => assert!(body.has(field.id), "{} is declared but never posted", field.id),
            Shape::AgentRows
            | Shape::StringRows
            | Shape::MultilingualRows
            | Shape::AttributionRows
            | Shape::TextOrReferenceRows(_)
            | Shape::ReferenceRows(_)
            | Shape::PublicationRows => assert!(
                body.has(&format!("{}.row", field.id)),
                "{} is declared but never posts its row marker",
                field.id
            ),
            Shape::StringList(_) => assert!(
                draft.get(field.id).is_none() || body.has(field.id),
                "{} is declared but never posted",
                field.id
            ),
            Shape::Choice(_) => assert!(
                draft.get(field.id).is_none() || body.has(field.id),
                "{} is declared but never posted",
                field.id
            ),
            Shape::Multilingual => {
                assert!(!body.entries(field.id).is_empty(), "{} is declared but never posted", field.id)
            }
        }
    }
}

fn count_placeholders(value: &Value) -> usize {
    match value {
        Value::String(text) => usize::from(shared_metadata::is_placeholder(text)),
        Value::Array(items) => items.iter().map(count_placeholders).sum(),
        Value::Object(members) => members.values().map(count_placeholders).sum(),
        _ => 0,
    }
}
