//! Saving a form nobody edited must not change the project, for all 85 files.
//!
//! `editor-core`'s `canonical_round_trip` pins `load -> draft -> write`; this
//! one puts the **form** in the middle, with the submit carrying exactly what an
//! untouched control would post.
//!
//! In `editor-web` because it derives its table from
//! [`registry::Field`](editor_web::form::registry::Field)'s declared shapes, and
//! `server -> web -> core` puts the registry on this side. Deriving it is the
//! point: a field whose shape is declared is covered automatically, where a
//! hand-written table agrees with the registry only by inspection.
//!
//! Asserted against the committed bytes rather than a fixture, because every way
//! this fails is silent: the unit tests pass, the form renders, the draft is
//! valid, and the only symptom is a pull request touching dozens of projects
//! nobody edited.

use std::path::{Path, PathBuf};

use editor_core::canonical::write_draft;
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply, FormBody, Shape};
use editor_core::multilingual::UI_LANGUAGES;
use editor_web::form::registry::{Field, FIELDS};
use platform_metadata::project::ProjectRaw;
use serde_json::Value;

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
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../dpe/server/data/projects")).to_path_buf()
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

/// What an untouched form would post for `draft`.
///
/// A control renders the stored value, except that a placeholder renders as
/// empty — which is the whole point of the test — so that is what the body
/// carries. Every field with a declared shape is present, because a section
/// posts its own fields whether or not they hold anything.
fn untouched_submit(draft: &ProjectDraft) -> FormBody {
    let mut pairs: Vec<(String, String)> = Vec::new();

    for (field, shape) in read_fields() {
        match shape {
            Shape::Text(_) => {
                let rendered = match draft.get(field.id).and_then(Value::as_str) {
                    // The sentinel is not shown to a reader, so the control is
                    // empty.
                    Some(value) if platform_metadata::is_placeholder(value) => String::new(),
                    Some(value) => value.to_string(),
                    // An absent field renders an empty control, which posts
                    // empty.
                    None => String::new(),
                };
                pairs.push((field.id.to_string(), rendered));
            }
            Shape::Multilingual => {
                // The widget renders a control per offered language whether or
                // not the value has that tag, so an untouched submit carries
                // empty texts for the ones it does not — which is what an
                // earlier version of this helper left out, posting only the
                // stored tags and testing a body no browser would send.
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
    // A positive canary. The test above asserts an *absence* of change, so it
    // would pass just as well over a corpus with no sentinel in it — at which
    // point it is proving nothing and nobody can tell. This pins that the
    // opportunity for the failure is present, and how much of it there is.
    //
    // The other two traps are covered the same way, by unit tests in
    // `editor_core::form` that assert the rule directly rather than its effect.
    let mut sentinels = 0;
    let mut end_date_sentinels = 0;
    for path in project_files() {
        let raw: ProjectRaw = serde_json::from_str(&std::fs::read_to_string(&path).expect("readable")).expect("parses");
        let value = serde_json::to_value(&raw).expect("serializes");
        sentinels += count_placeholders(&value);
        if platform_metadata::is_placeholder(&raw.end_date) {
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
fn the_body_this_test_submits_carries_every_field_a_shape_is_declared_for() {
    // The other half of the canary, and the reason the table is derived rather
    // than written out: a field whose shape is declared but which
    // `untouched_submit` forgets to render is a field the round-trip check
    // silently skips, and the check would still pass. Asserted against the
    // registry so a newly shaped field cannot be missed.
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
            Shape::Multilingual => {
                assert!(!body.entries(field.id).is_empty(), "{} is declared but never posted", field.id)
            }
        }
    }
}

fn count_placeholders(value: &Value) -> usize {
    match value {
        Value::String(text) => usize::from(platform_metadata::is_placeholder(text)),
        Value::Array(items) => items.iter().map(count_placeholders).sum(),
        Value::Object(members) => members.values().map(count_placeholders).sum(),
        _ => 0,
    }
}
