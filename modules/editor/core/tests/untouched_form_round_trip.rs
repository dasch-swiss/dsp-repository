//! Saving a form nobody edited must not change the project, for all 85 files.
//!
//! The sibling `canonical_round_trip` test pins `load -> draft -> write`. This
//! one puts the **form** in the middle: `load -> draft -> render -> submit ->
//! draft -> write`, with the submit carrying exactly what an untouched control
//! would post. A depositor opening a section to read it and pressing save must
//! get a byte-identical file.
//!
//! ## What it caught
//!
//! Three separate ways an untouched save rewrites a published file, all found by
//! running this test rather than by reasoning about the code:
//!
//! 1. **Placeholder sentinels.** `MISSING` and `CALCULATED` (`platform_metadata::is_placeholder`)
//!    are filtered out of DPE's UI and of OAI-PMH's output, so a control holding one renders
//!    **empty** — an untouched form posts an empty value, and the obvious decoder writes `""` back.
//!    All 85 files carry 131 sentinels across 8 paths, 24 of them `endDate`.
//! 2. **Trimming.** Four files carry a leading or trailing space in a field the form owns, and a
//!    control posts it back verbatim.
//! 3. **Newline encoding.** 26 files hold a newline in `description` or `abstract`, and 10 hold a
//!    bare `\r` that no `<textarea>` can represent.
//!
//! Enumerated rather than sampled, and asserted against the committed bytes
//! rather than against a fixture, because every one of these is silent by
//! construction: the unit tests pass, the form renders correctly, the draft is
//! valid, and the only symptom is a diff in a pull request against dozens of
//! projects nobody touched.

use std::path::{Path, PathBuf};

use editor_core::canonical::write_draft;
use editor_core::draft::ProjectDraft;
use editor_core::form::{apply_multilingual, apply_text, FormBody, WhenCleared};
use platform_metadata::project::ProjectRaw;
use serde_json::Value;

/// The scalar text fields the form owns, with what a clear means for each.
///
/// `Placeholder` is for a field the contract types as a required `String`, whose
/// empty state the data spells as a sentinel; `Drop` is for an `Option`, where
/// absent is unset.
const TEXT_FIELDS: &[(&str, WhenCleared)] = &[
    ("name", WhenCleared::Placeholder),
    ("officialName", WhenCleared::Placeholder),
    ("shortDescription", WhenCleared::Placeholder),
    ("startDate", WhenCleared::Placeholder),
    ("endDate", WhenCleared::Placeholder),
    ("provenance", WhenCleared::Drop),
    ("dataManagementPlan", WhenCleared::Drop),
    ("dataPublicationYear", WhenCleared::Drop),
    ("imageCredit", WhenCleared::Drop),
];

/// The language-map fields the form owns.
const MULTILINGUAL_FIELDS: &[&str] = &["description", "abstract"];

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
/// carries. Every field the form owns is present, because a section posts its
/// own fields whether or not they hold anything.
fn untouched_submit(draft: &ProjectDraft) -> FormBody {
    let mut pairs: Vec<(String, String)> = Vec::new();

    for (field, _) in TEXT_FIELDS {
        let rendered = match draft.get(field).and_then(Value::as_str) {
            // The sentinel is not shown to a reader, so the control is empty.
            Some(value) if platform_metadata::is_placeholder(value) => String::new(),
            Some(value) => value.to_string(),
            // An absent field renders an empty control, which posts empty.
            None => String::new(),
        };
        pairs.push(((*field).to_string(), rendered));
    }

    for field in MULTILINGUAL_FIELDS {
        for (tag, text) in draft.multilingual(field).iter() {
            pairs.push((format!("{field}.{tag}"), text.to_string()));
        }
    }

    FormBody::from_pairs(pairs)
}

/// The same draft after that body is applied back.
fn resubmit(draft: &ProjectDraft) -> ProjectDraft {
    let body = untouched_submit(draft);
    let mut resubmitted = draft.clone();
    for (field, when_cleared) in TEXT_FIELDS {
        apply_text(&body, &mut resubmitted, field, *when_cleared);
    }
    for field in MULTILINGUAL_FIELDS {
        apply_multilingual(&body, &mut resubmitted, field);
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

fn count_placeholders(value: &Value) -> usize {
    match value {
        Value::String(text) => usize::from(platform_metadata::is_placeholder(text)),
        Value::Array(items) => items.iter().map(count_placeholders).sum(),
        Value::Object(members) => members.values().map(count_placeholders).sum(),
        _ => 0,
    }
}
