//! The canonical writer, held against the whole committed corpus.
//!
//! `load -> draft -> canonical write` must be byte-identical for every one of
//! the 85 project files. That is a hard oracle: it pins member order at every
//! depth, indentation, the trailing newline, null-stripping, language-key order
//! and `serde_json`'s string escaping all at once, which is why the writer needs
//! no hand-written escaping and no per-field order table.
//!
//! It also pins the claim `ProjectDraft::from_raw` rests on, that stripping null
//! members loses nothing: a field it drops has to come back as `None` for
//! `to_raw` to succeed here.
//!
//! ## Regenerating the corpus
//!
//! ```text
//! CANONICALIZE_PROJECT_FILES=1 cargo test -p editor-core --test canonical_round_trip
//! ```
//!
//! rewrites each file with what the writer produces instead of asserting. Use it
//! when a deliberate change to the canonical form lands, and commit the result
//! as its own commit so the reformat is reviewable apart from the code that
//! caused it. Generating the corpus *from* the writer is the point: a separate
//! script would have to agree with the writer by inspection, and a near-miss
//! there surfaces later as a failing round-trip that looks like a writer bug.
//!
//! ## Why this test reads DPE's data directory
//!
//! It is the published corpus, not DPE's private fixture: the editor's whole job
//! is to read and write these files, and asserting against the real 85 is the
//! only version of this test worth having. `platform-metadata` is the crate that
//! must not reach for a service's data directory, and it does not.

use std::path::{Path, PathBuf};

use editor_core::canonical::write_draft;
use editor_core::draft::ProjectDraft;
use platform_metadata::project::ProjectRaw;

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

/// The corpus is what makes the round-trip assertion meaningful, so a silent
/// drop to a handful of files would hollow it out without failing anything.
///
/// Adding or removing a project is expected to fail this: bump the number in the
/// same commit as the data change. The runbook in `modules/dpe/CLAUDE.md` says so
/// too.
#[test]
fn the_corpus_is_the_whole_published_set() {
    assert_eq!(
        project_files().len(),
        85,
        "the published set changed size. If that was deliberate, update this count in the same commit."
    );
}
