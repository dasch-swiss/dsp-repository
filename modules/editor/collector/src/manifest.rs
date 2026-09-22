//! The corpus manifest, which every collection has to keep true.
//!
//! `corpus-manifest.json` is the one place the published set's size is written
//! down, and `the_corpus_is_the_whole_published_set` fails the moment a data
//! file is added without it. A collection adds data files, so it updates the
//! manifest in the same commit or arrives red.

use std::path::Path;

pub const MANIFEST_FILE: &str = "corpus-manifest.json";

/// The indent the committed manifest uses.
const INDENT: &[u8] = b"    ";

/// Rewrites the manifest from what the data directory now holds.
///
/// Counted rather than incremented: a record that rewrites an existing entity
/// adds no file, and a run that partly failed must not leave the manifest
/// describing files it never wrote.
pub fn rewrite(data_dir: &Path) -> Result<(), String> {
    let mut counts = serde_json::Map::new();
    for directory in ["projects", "persons", "organizations"] {
        counts.insert(directory.to_string(), json_files_in(&data_dir.join(directory))?.into());
    }

    let rendered = crate::json::render(&counts, INDENT, MANIFEST_FILE)?;
    std::fs::write(data_dir.join(MANIFEST_FILE), rendered)
        .map_err(|error| format!("could not write {MANIFEST_FILE}: {error}"))
}

/// The `*.json` files directly under `dir`.
fn json_files_in(dir: &Path) -> Result<u64, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| format!("could not list {}: {error}", dir.display()))?;
    Ok(entries
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|extension| extension.to_str()) == Some("json"))
        .count() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The manifest the test writes must be what the pin reads back: same key
    /// set, same indent, same trailing newline.
    #[test]
    fn the_manifest_is_rewritten_in_the_committed_form() {
        let dir = std::env::temp_dir().join("collector-manifest-form");
        let _ = std::fs::remove_dir_all(&dir);
        for sub in ["projects", "persons", "organizations"] {
            std::fs::create_dir_all(dir.join(sub)).expect("a fixture directory");
        }
        std::fs::write(dir.join("projects/0803_x.json"), "{}").expect("a fixture file");
        std::fs::write(dir.join("persons/person-001.json"), "{}").expect("a fixture file");
        std::fs::write(dir.join("persons/notes.txt"), "ignored").expect("a fixture file");

        rewrite(&dir).expect("the manifest is rewritten");

        let written = std::fs::read_to_string(dir.join(MANIFEST_FILE)).expect("the manifest is readable");
        assert_eq!(
            written,
            "{\n    \"projects\": 1,\n    \"persons\": 1,\n    \"organizations\": 0\n}\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
