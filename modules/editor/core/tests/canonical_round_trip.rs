//! The canonical writer, held against the whole committed corpus.
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

use editor_core::canonical::write_draft;
use editor_core::draft::ProjectDraft;
use shared_metadata::project::ProjectRaw;

/// Set to rewrite the corpus instead of asserting against it.
const REGENERATE: &str = "CANONICALIZE_PROJECT_FILES";

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

#[test]
fn the_corpus_is_the_whole_published_set() {
    assert_eq!(
        project_files().len(),
        85,
        "the published set changed size. If that was deliberate, update this count in the same commit."
    );
}
