//! The canonical writers, held against the whole committed corpus.
//!
//! `load -> draft -> canonical write` must be byte-identical for every
//! committed project file. That pins member order at every depth, indentation,
//! the trailing newline, null-stripping, language-key order and `serde_json`'s
//! escaping at once, and it pins the claim `ProjectDraft::from_raw` rests on:
//! stripping null members loses nothing.
//!
//! ```text
//! CANONICALIZE_PROJECT_FILES=1 cargo test -p editor-core --test canonical_round_trip
//! ```
//!
//! rewrites each file with what the writer produces instead of asserting. Commit
//! the result as its own commit, so the reformat is reviewable apart from the
//! code that caused it.

use std::path::{Path, PathBuf};

use editor_core::canonical::{write_draft, write_entity};
use editor_core::draft::ProjectDraft;
use shared_metadata::project::ProjectRaw;

/// Set to rewrite the corpus instead of asserting against it.
const REGENERATE: &str = "CANONICALIZE_PROJECT_FILES";

fn projects_dir() -> PathBuf {
    editor_core::checkout_dpe_data_dir().join("projects")
}

fn data_dir() -> PathBuf {
    editor_core::checkout_dpe_data_dir()
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

/// The first line that differs, so a failure names a place rather than dumping
/// two files.
fn first_difference(committed: &str, written: &str) -> String {
    for (line, (before, after)) in committed.lines().zip(written.lines()).enumerate() {
        if before != after {
            return format!("line {}:\n  committed: {before}\n  written:   {after}", line + 1);
        }
    }
    // Reached when one is a prefix of the other, the trailing-newline-only case
    // included, which `lines()` cannot show. Report bytes as well as lines: a
    // missing trailing newline leaves the line counts equal.
    format!(
        "every shared line matches; the files differ in length: committed {} bytes / {} lines, \
         written {} bytes / {} lines",
        committed.len(),
        committed.lines().count(),
        written.len(),
        written.lines().count()
    )
}

#[test]
fn every_committed_project_file_round_trips_byte_identically() {
    let regenerate = std::env::var_os(REGENERATE).is_some();
    let mut differing = Vec::new();
    let mut rewritten = 0;
    let files = project_files();

    for path in &files {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let committed = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {name}: {e}"));
        let raw: ProjectRaw = serde_json::from_str(&committed).unwrap_or_else(|e| panic!("parsing {name}: {e}"));
        let written = write_draft(&ProjectDraft::from_raw(&raw)).unwrap_or_else(|e| panic!("writing {name}: {e}"));

        if written == committed {
            continue;
        }
        if regenerate {
            std::fs::write(path, &written).unwrap_or_else(|e| panic!("rewriting {name}: {e}"));
            rewritten += 1;
        } else {
            differing.push(format!("{name}\n{}", first_difference(&committed, &written)));
        }
    }

    if regenerate {
        println!("{REGENERATE}: rewrote {rewritten} of {} files", files.len());
        return;
    }
    assert!(
        differing.is_empty(),
        "{} of {} project files do not round-trip. Review the change, then regenerate with \
         `{REGENERATE}=1 cargo test -p editor-core --test canonical_round_trip`:\n\n{}",
        differing.len(),
        files.len(),
        differing.join("\n\n")
    );
}

/// The one place the corpus's size is pinned to a stored number.
///
/// Every other count assertion in the tree compares a loader against a live
/// directory listing, so both sides move together and adding a file needs no
/// edit. That catches a loader which drops a file, but it cannot catch the
/// directory itself changing — a project deleted by accident would simply make
/// every such assertion agree on a smaller number. This test is what notices,
/// and `corpus-manifest.json` is the stored number it reads.
///
/// So a deliberate change to the published set is one line in a data file, and
/// an accidental one is a failing test.
#[test]
fn the_corpus_is_the_whole_published_set() {
    let manifest = data_dir().join("corpus-manifest.json");
    let json = std::fs::read_to_string(&manifest).expect("the corpus manifest should be readable");
    let counts: serde_json::Value = serde_json::from_str(&json).expect("the corpus manifest should parse");

    for (key, dir) in [
        ("projects", "projects"),
        ("persons", "persons"),
        ("organizations", "organizations"),
    ] {
        let recorded = counts[key]
            .as_u64()
            .unwrap_or_else(|| panic!("corpus-manifest.json should record a number for {key}"))
            as usize;
        assert_eq!(
            json_files_in(&data_dir().join(dir)),
            recorded,
            "the committed {key} changed size. If that was deliberate, update corpus-manifest.json \
             in the same commit."
        );
    }
}

/// The `*.json` files directly under `dir`.
fn json_files_in(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .expect("a data directory should be readable")
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
        .count()
}

/// The same claim for entity files, which `write_entity` produces: collecting
/// one entity must not reformat its neighbours.
///
/// Unlike the project side there is no draft detour — `write_entity` takes the
/// payload as the editor sent it, because the corpus carries keys no struct
/// declares and distinguishes an absent `affiliations` from an empty one.
///
/// Twelve committed files end without a newline and the writer always emits
/// one. They are compared with that difference normalised away, and counted, so
/// that a new file entering the corpus in non-canonical form fails here rather
/// than joining the exception unnoticed. The count cannot be derived from the
/// directory: both sides would move together.
#[test]
fn every_committed_entity_file_round_trips_byte_identically() {
    let mut missing_newline = Vec::new();

    for dir in ["persons", "organizations"] {
        let mut files: Vec<PathBuf> = std::fs::read_dir(data_dir().join(dir))
            .expect("an entity data directory should be readable")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no {dir} files were found");

        for path in files {
            let committed = std::fs::read_to_string(&path).expect("an entity file should be readable");
            let parsed: serde_json::Value = serde_json::from_str(&committed).expect("an entity file should parse");

            let written = write_entity(&parsed).expect("the writer should serialize an entity");

            if committed.ends_with('\n') {
                assert_eq!(written, committed, "{} does not round-trip", path.display());
            } else {
                missing_newline.push(path.file_name().expect("a file name").to_string_lossy().into_owned());
                assert_eq!(
                    written,
                    format!("{committed}\n"),
                    "{} differs beyond its missing newline",
                    path.display()
                );
            }
        }
    }

    assert_eq!(
        missing_newline.len(),
        12,
        "12 committed entity files end without a newline. A new one joining them is a file that \
         should have been written canonically: {missing_newline:?}"
    );
}
