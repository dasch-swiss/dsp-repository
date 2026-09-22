//! Where a project or entity's canonical file lives under `data_dir`.
//!
//! A project's filename stem is not derivable from its shortcode — the five
//! committed `projects/0801_*.json` files hold shortcodes `0801a` through
//! `0801e` — so an existing project has to be found by reading every file's
//! `shortcode` member, not by building a path and checking whether it exists.

use std::path::{Path, PathBuf};

/// The path an existing project's file lives at, or the fresh path a new one
/// should be written to.
///
/// Every `*.json` file directly under `data_dir/projects/` is read in sorted
/// path order and checked against its `"shortcode"` member, folding ASCII
/// case; a file that will not read or parse is skipped rather than treated as
/// an error. Sorting first keeps the answer independent of directory order.
///
/// When no file's `shortcode` matches, the project is new: the path is built
/// from `shortcode` and [`slug(name)`](slug), deconflicted with a `-2`,
/// `-3`, … suffix on the slug against any file that already sits at that
/// stem — which can only happen when that file's own `shortcode` differs.
#[must_use]
pub fn project_path(data_dir: &Path, shortcode: &str, name: &str) -> PathBuf {
    let projects_dir = data_dir.join("projects");
    if let Some(existing) = find_by_shortcode(&projects_dir, shortcode) {
        return existing;
    }

    let base_slug = slug(name);
    let mut candidate = projects_dir.join(format!("{shortcode}_{base_slug}.json"));
    let mut suffix = 2;
    while candidate.exists() {
        candidate = projects_dir.join(format!("{shortcode}_{base_slug}-{suffix}.json"));
        suffix += 1;
    }
    candidate
}

/// The path an entity's file lives at.
///
/// `"person"` and `"organization"` are the two kinds the editor knows about
/// and map to `persons/` and `organizations/`; any other `kind` still yields
/// a path, pluralised, so the function stays total and the caller is the one
/// that validates `kind`.
#[must_use]
pub fn entity_path(data_dir: &Path, kind: &str, id: &str) -> PathBuf {
    let dir = match kind {
        "person" => "persons".to_string(),
        "organization" => "organizations".to_string(),
        other => format!("{other}s"),
    };
    data_dir.join(dir).join(format!("{id}.json"))
}

/// The first `*.json` file directly under `projects_dir` whose `"shortcode"`
/// member matches `shortcode`, ignoring ASCII case.
fn find_by_shortcode(projects_dir: &Path, shortcode: &str) -> Option<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(projects_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();

    paths
        .into_iter()
        .find(|path| read_shortcode(path).is_some_and(|found| found.eq_ignore_ascii_case(shortcode)))
}

fn read_shortcode(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value.get("shortcode")?.as_str().map(str::to_string)
}

/// A filesystem- and URL-safe slug for `name`: lowercase ASCII alphanumerics
/// separated by single `-`s.
///
/// Common accented Latin-1 letters are folded to their plain ASCII
/// counterpart before the rest of the rule runs, so `"Université"` slugs the
/// same way an ASCII `"Universite"` would. Every run of characters that are
/// not ASCII alphanumeric — accented or not — becomes one `-`; leading and
/// trailing `-`s are trimmed, the result is capped at 40 characters, and a
/// trailing `-` exposed by that cut is trimmed again. A name that folds away
/// entirely becomes `"project"`, never an empty stem.
#[must_use]
pub fn slug(name: &str) -> String {
    let mut folded = String::with_capacity(name.len());
    for ch in name.chars() {
        folded.push_str(&fold_accent(ch));
    }

    let mut result = String::with_capacity(folded.len());
    let mut last_was_dash = false;
    for ch in folded.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !result.is_empty() && !last_was_dash {
            result.push('-');
            last_was_dash = true;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }

    let mut truncated: String = result.chars().take(40).collect();
    while truncated.ends_with('-') {
        truncated.pop();
    }

    if truncated.is_empty() {
        "project".to_string()
    } else {
        truncated
    }
}

/// The ASCII fold for one character, as a one-character string for anything
/// unmatched; `ß` is the one case that expands to two characters.
fn fold_accent(ch: char) -> String {
    match ch {
        'ä' | 'à' | 'á' | 'â' | 'Ä' | 'À' | 'Á' | 'Â' => "a".to_string(),
        'ö' | 'ò' | 'ó' | 'ô' | 'Ö' | 'Ò' | 'Ó' | 'Ô' => "o".to_string(),
        'ü' | 'ù' | 'ú' | 'û' | 'Ü' | 'Ù' | 'Ú' | 'Û' => "u".to_string(),
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => "e".to_string(),
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => "i".to_string(),
        'ç' | 'Ç' => "c".to_string(),
        'ñ' | 'Ñ' => "n".to_string(),
        'ß' => "ss".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// A `data_dir` holding `project_files` as `projects/<name>.json`, removed
    /// by the caller.
    fn data_dir_with(name: &str, project_files: &[(&str, String)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("editor-collector-layout-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let projects_dir = dir.join("projects");
        std::fs::create_dir_all(&projects_dir).expect("temp dir");
        for (file, body) in project_files {
            std::fs::write(projects_dir.join(format!("{file}.json")), body).expect("write");
        }
        dir
    }

    fn project_json(shortcode: &str) -> String {
        serde_json::json!({ "shortcode": shortcode }).to_string()
    }

    #[test]
    fn a_shortcode_is_matched_by_the_file_s_shortcode_field_not_its_stem() {
        let dir = data_dir_with("field-match", &[("0801_bebb", project_json("0801d"))]);
        let path = project_path(&dir, "0801d", "Whatever Name");
        assert_eq!(path, dir.join("projects/0801_bebb.json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_shortcode_match_folds_case() {
        let dir = data_dir_with("case-fold", &[("0801_bebb", project_json("0801D"))]);
        let path = project_path(&dir, "0801d", "Whatever Name");
        assert_eq!(path, dir.join("projects/0801_bebb.json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unknown_shortcode_gets_a_fresh_path_from_shortcode_and_slug() {
        let dir = data_dir_with("unknown", &[]);
        let path = project_path(&dir, "0999", "A New Project");
        assert_eq!(path, dir.join("projects/0999_a-new-project.json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stem_collision_from_a_project_with_a_different_shortcode_gets_a_dash_2_suffix() {
        let dir = data_dir_with("collision", &[("0999_a-new-project", project_json("0111"))]);
        let path = project_path(&dir, "0999", "A New Project");
        assert_eq!(path, dir.join("projects/0999_a-new-project-2.json"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entity_path_uses_persons_and_organizations_directories() {
        let dir = PathBuf::from("/data");
        assert_eq!(entity_path(&dir, "person", "person-001"), dir.join("persons/person-001.json"));
        assert_eq!(
            entity_path(&dir, "organization", "organization-002"),
            dir.join("organizations/organization-002.json")
        );
    }

    #[test]
    fn entity_path_pluralizes_an_unknown_kind_as_a_fallback() {
        let dir = PathBuf::from("/data");
        assert_eq!(
            entity_path(&dir, "cluster", "cluster-001"),
            dir.join("clusters/cluster-001.json")
        );
    }

    #[test]
    fn slug_lowercases_and_collapses_punctuation_runs_to_one_dash() {
        assert_eq!(slug("Hello,  World!!"), "hello-world");
    }

    #[test]
    fn slug_folds_common_latin_1_accents_to_ascii() {
        assert_eq!(slug("Université de Genève"), "universite-de-geneve");
        assert_eq!(slug("Straße"), "strasse");
    }

    #[test]
    fn slug_trims_leading_and_trailing_dashes() {
        assert_eq!(slug("  -Wrapped-  "), "wrapped");
    }

    #[test]
    fn slug_truncates_at_forty_characters() {
        let name = "b".repeat(50);
        assert_eq!(slug(&name), "b".repeat(40));
    }

    #[test]
    fn slug_trims_a_trailing_dash_exposed_by_truncation() {
        // The cut lands exactly on the separator: 39 `a`s then a run of
        // non-alphanumerics that folds to one dash at position 40.
        let name = format!("{} {}", "a".repeat(39), "b".repeat(10));
        assert_eq!(slug(&name), "a".repeat(39));
    }

    #[test]
    fn slug_of_only_punctuation_falls_back_to_project() {
        assert_eq!(slug("!!!"), "project");
        assert_eq!(slug(""), "project");
    }
}
