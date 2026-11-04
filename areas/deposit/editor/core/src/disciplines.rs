//! The bundled SNSF discipline list. `data/snsf-disciplines.json` was extracted once from
//! `SNSF_DOMAINS` in the metadata-editor prototype (`dsp-incubator`,
//! `metadata-editor-v2/src/workflows/store.jsx`) and is the list's source from then on.

use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SnsfDiscipline {
    pub code: String,
    pub name: String,
    pub group: String,
}

/// The form field that records disciplines.
pub const FIELD: &str = "disciplines";

/// The SNSF disciplines, in the order of the SNSF list.
pub fn snsf() -> &'static [SnsfDiscipline] {
    static SNSF: OnceLock<Vec<SnsfDiscipline>> = OnceLock::new();
    SNSF.get_or_init(|| {
        serde_json::from_str(include_str!("../data/snsf-disciplines.json"))
            .expect("data/snsf-disciplines.json is an array of {code, name, group}")
    })
}

/// The leading five-digit SNSF code of a recorded value such as
/// `"10404 Visual arts and Art history"`: exactly five ASCII digits followed by
/// whitespace. A longer digit run (`110302 …`) is not a code.
pub fn snsf_code(text: &str) -> Option<&str> {
    let (code, rest) = text.split_at_checked(5)?;
    (code.bytes().all(|byte| byte.is_ascii_digit()) && rest.starts_with(char::is_whitespace)).then_some(code)
}

/// The text a `disciplines` row stores for a pick: `"<code> <name>"`.
pub fn snsf_text(discipline: &SnsfDiscipline) -> String {
    format!("{} {}", discipline.code, discipline.name)
}

/// The listed SNSF discipline with this code, or `None` for a group code
/// (`10400`) or anything the list does not carry.
pub fn snsf_discipline(code: &str) -> Option<&'static SnsfDiscipline> {
    snsf().iter().find(|discipline| discipline.code == code)
}

/// The SNSF discipline a stored `disciplines` row records: a text row (no `url`
/// member) in which some string, in any language, leads with a listed code; the
/// first such string in map order decides.
pub fn snsf_row(row: &serde_json::Value) -> Option<&'static SnsfDiscipline> {
    let members = row.as_object()?;
    if members.contains_key("url") {
        return None;
    }
    members
        .values()
        .filter_map(serde_json::Value::as_str)
        .find_map(|text| snsf_discipline(snsf_code(text)?))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;

    use super::*;
    use crate::checkout_dpe_data_dir;

    #[test]
    fn snsf_code_reads_exactly_five_leading_digits() {
        assert_eq!(snsf_code("10404 Visual arts and Art history"), Some("10404"));
        assert_eq!(snsf_code("110302 Schweizer Geschichte"), None);
        assert_eq!(snsf_code("1040 Visual arts"), None);
        assert_eq!(snsf_code("abcde Visual arts"), None);
        assert_eq!(snsf_code("10404"), None);
    }

    #[test]
    fn snsf_row_recognises_a_row_by_its_code_in_any_language() {
        let french = serde_json::json!({"fr": "10404 Histoire de l'art"});
        assert_eq!(snsf_row(&french).map(|discipline| discipline.code.as_str()), Some("10404"));
        let later_member = serde_json::json!({"de": "Kunst", "en": "10404 Visual arts"});
        assert_eq!(
            snsf_row(&later_member).map(|discipline| discipline.code.as_str()),
            Some("10404")
        );
        assert_eq!(snsf_row(&serde_json::json!({"en": "10400 Art studies"})), None);
        assert_eq!(snsf_row(&serde_json::json!({"en": "110302 Schweizer Geschichte"})), None);
        let reference =
            serde_json::json!({"type": "Skos", "url": "http://skos.um.es/unesco6/5506", "text": "10404 Art"});
        assert_eq!(snsf_row(&reference), None);
    }

    #[test]
    fn every_well_formed_corpus_snsf_code_is_listed() {
        let listed: HashSet<&str> = snsf().iter().map(|discipline| discipline.code.as_str()).collect();
        let projects = checkout_dpe_data_dir().join("projects");
        let mut checked = 0;
        for entry in fs::read_dir(&projects).expect("the committed corpus is readable") {
            let path = entry.expect("directory entry").path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let text = fs::read_to_string(&path).expect("project file is readable");
            let project: serde_json::Value = serde_json::from_str(&text).expect("project file is JSON");
            let entries = project.get("disciplines").and_then(|value| value.as_array());
            for value in entries.into_iter().flatten().filter_map(|entry| entry.as_object()) {
                if value.contains_key("url") {
                    continue;
                }
                for text in value.values().filter_map(|text| text.as_str()) {
                    let Some(code) = snsf_code(text) else { continue };
                    // A group code (`10400`) is valid when its group has listed codes.
                    let group_listed = code.ends_with("00") && listed.iter().any(|listed| listed[..3] == code[..3]);
                    assert!(
                        listed.contains(code) || group_listed,
                        "SNSF code `{code}` in {} is not in the SNSF list",
                        path.display()
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "no SNSF codes found under {}", projects.display());
    }
}
