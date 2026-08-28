//! The canonical `projects/*.json` writer.
//!
//! One function decides what a project file looks like, so an approved
//! submission is byte-comparable with what is committed and a review diff shows
//! only what the depositor actually changed. The form is the one the 85
//! committed files already hold:
//!
//! - members in `ProjectRaw`'s field declaration order, nested objects included
//! - `null` members dropped, recursively
//! - language keys alphabetical, which [`Multilingual`] gives for free
//! - four-space indent
//! - a trailing newline
//! - non-ASCII left unescaped
//!
//! Output goes through `ProjectRaw` and not through the draft's own members
//! (REQ-1.8): a field added to the contract is carried without an editor
//! change, and anything the contract does not declare is not written.
//!
//! `serde_json`'s string escaping already matches the committed files (`\n`,
//! `\r`, `\t`, `\"`, and non-ASCII left as-is), so there is no custom escaping
//! here. The 85-file round-trip test is what holds that claim up.
//!
//! [`Multilingual`]: platform_metadata::utils::Multilingual

use platform_metadata::project::ProjectRaw;
use serde::Serialize;
use serde_json::ser::PrettyFormatter;
use serde_json::Value;

use crate::draft::{DraftError, ProjectDraft};
use crate::json::strip_null_members;

/// The indent the committed files use, and therefore the only one that keeps a
/// reformat out of every future diff.
const INDENT: &[u8] = b"    ";

/// Serializes a project in the canonical form.
///
/// The `Value` detour exists to drop `null` members: `ProjectRaw` cannot carry
/// `skip_serializing_if`, because `dpe-server` serializes it through
/// `axum::Json` and that attribute would change DPE's API responses too. It is
/// order-safe because the workspace enables `serde_json`'s `preserve_order`;
/// without that feature `Value` is `BTreeMap`-backed and this would alphabetise
/// every key in the file.
pub fn write_project(project: &ProjectRaw) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(project)?;
    strip_null_members(&mut value);
    render(&value)
}

/// Serializes a draft in the canonical form, or reports why it is not
/// publishable yet.
pub fn write_draft(draft: &ProjectDraft) -> Result<String, DraftError> {
    let project = draft.to_raw()?;
    write_project(&project).map_err(|err| DraftError::Serialization(err.to_string()))
}

fn render(value: &Value) -> Result<String, serde_json::Error> {
    let mut buffer = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, PrettyFormatter::with_indent(INDENT));
    value.serialize(&mut serializer)?;
    let mut json = String::from_utf8(buffer).map_err(|err| {
        // Unreachable: `serde_json` only ever writes UTF-8. Mapped rather than
        // unwrapped so a writer bug cannot panic a request.
        <serde_json::Error as serde::ser::Error>::custom(err)
    })?;
    json.push('\n');
    Ok(json)
}

#[cfg(test)]
mod tests {
    use platform_metadata::utils::Multilingual;
    use serde_json::json;

    use super::*;
    use crate::test_support::sample_raw;

    #[test]
    fn emits_members_in_declaration_order_not_alphabetically() {
        let json = write_project(&sample_raw()).expect("writes");
        let keys: Vec<&str> = json
            .lines()
            .filter_map(|line| line.strip_prefix("    \""))
            .filter_map(|line| line.split('"').next())
            .collect();
        assert_eq!(&keys[..5], ["id", "pid", "name", "shortcode", "officialName"]);
    }

    #[test]
    fn indents_with_four_spaces_and_ends_with_one_newline() {
        let json = write_project(&sample_raw()).expect("writes");
        assert!(json.contains("\n    \"pid\""), "four-space indent at depth 1");
        assert!(json.ends_with("}\n"));
        assert!(!json.ends_with("}\n\n"));
    }

    #[test]
    fn drops_null_members() {
        let json = write_project(&sample_raw()).expect("writes");
        assert!(!json.contains("null"), "no null members in canonical output");
    }

    #[test]
    fn leaves_non_ascii_unescaped() {
        let mut project = sample_raw();
        project.image_credit = Some("© Sophie Müller, Zürich".to_string());
        let json = write_project(&project).expect("writes");
        assert!(json.contains("© Sophie Müller, Zürich"), "{json}");
        assert!(!json.contains("\\u00a9"));
    }

    #[test]
    fn escapes_control_characters_the_way_the_committed_files_do() {
        let mut project = sample_raw();
        project.image_credit = Some("a\nb\tc\"d".to_string());
        let json = write_project(&project).expect("writes");
        assert!(json.contains(r#""imageCredit": "a\nb\tc\"d""#), "{json}");
    }

    #[test]
    fn orders_language_keys_alphabetically() {
        let mut project = sample_raw();
        project.description = Multilingual::from([
            ("it".to_string(), "Ciao".to_string()),
            ("ar".to_string(), "مرحبا".to_string()),
            ("de".to_string(), "Hallo".to_string()),
        ]);
        let json = write_project(&project).expect("writes");
        let description = json
            .split("\"description\": {")
            .nth(1)
            .expect("a description member")
            .split('}')
            .next()
            .expect("its members");
        let order: Vec<&str> = description
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"'))
            .filter_map(|line| line.split('"').next())
            .collect();
        assert_eq!(order, ["ar", "de", "it"]);
    }

    /// Nested objects keep their own declaration order too. `AuthorityFileReference`
    /// is `type, url, text`, which alphabetising would turn into `text, type, url`
    /// in every file that has a `spatialCoverage` entry.
    #[test]
    fn keeps_nested_objects_in_declaration_order() {
        let json = write_project(&sample_raw()).expect("writes");
        let entry = json.split("\"spatialCoverage\": [").nth(1).expect("a spatialCoverage member");
        let type_at = entry.find("\"type\"").expect("a type key");
        let url_at = entry.find("\"url\"").expect("a url key");
        assert!(type_at < url_at, "type precedes url");
    }

    /// REQ-1.8: what the writer emits is decided by `ProjectRaw`, so a field
    /// added to the contract appears in the output with no change here. Checked
    /// through `imageCredit`, which is declared last and absent from every
    /// committed file: it must land at the end, not sorted into the `i`s.
    #[test]
    fn a_contract_field_is_written_in_its_declared_position() {
        let mut project = sample_raw();
        project.image_credit = Some("© Someone".to_string());
        let json = write_project(&project).expect("writes");
        let members: Vec<&str> = json
            .lines()
            .filter_map(|line| line.strip_prefix("    \""))
            .filter_map(|line| line.split('"').next())
            .collect();
        assert_eq!(members.last(), Some(&"imageCredit"), "{members:?}");
        assert!(members.len() > 2, "the sample has more than one member");
    }

    #[test]
    fn write_draft_refuses_an_incomplete_draft() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.remove("officialName");
        let err = write_draft(&draft).expect_err("an incomplete draft is not publishable");
        assert!(err.to_string().contains("officialName"), "{err}");
    }

    #[test]
    fn write_draft_matches_write_project_for_an_untouched_draft() {
        let project = sample_raw();
        let draft = ProjectDraft::from_raw(&project);
        assert_eq!(write_draft(&draft).expect("writes"), write_project(&project).expect("writes"));
    }

    /// A key the contract does not declare is not written. The draft carries it
    /// so a payload survives a schema the editor has not caught up with, but the
    /// published file is exactly what `ProjectRaw` describes.
    #[test]
    fn a_key_outside_the_contract_is_not_written() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("fieldTheContractDoesNotDeclare", json!("value"));
        let json = write_draft(&draft).expect("writes");
        assert!(!json.contains("fieldTheContractDoesNotDeclare"), "{json}");
    }
}
