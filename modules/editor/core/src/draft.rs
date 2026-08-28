//! The permissive draft representation (REQ-1.9).
//!
//! A draft has to hold what `ProjectRaw` cannot: a field the depositor has not
//! filled in yet, and a value that is present but invalid. It also has to carry
//! every field the editor does not manage, unchanged (REQ-1.7), and to survive a
//! field being added to `ProjectRaw` without an editor change (REQ-1.8).
//!
//! Those three pull in the same direction, so a draft is the project's JSON
//! members rather than a struct mirroring `ProjectRaw` with 36 `Option` fields.
//! An absent key is a missing field, any `Value` is an accepted value whether it
//! validates or not, and a key the editor has never heard of rides through
//! untouched. Validity is decided once, at [`ProjectDraft::to_raw`], which is
//! the submission boundary.
//!
//! ## Why the untagged variants need no separate tag
//!
//! `TemporalCoverage`, `Discipline` and `Funding` are `#[serde(untagged)]`, and
//! untagged deserialization takes the first variant that fits. The risk the
//! issue names is a project whose `funding` is free text being forced into the
//! grant shape. That cannot happen here: the value keeps its JSON kind verbatim
//! in [`Self::members`], so a `Value::String` can only ever deserialize as
//! `Funding::Text` (a string is not an array, so `Grants` cannot fit).
//!
//! The variant is therefore *derived* rather than stored, by
//! [`Self::funding_shape`] and friends, each of which asks the question in
//! serde's own attempt order. A stored tag would be a second source of truth
//! able to drift from the value it describes, and it is the value that the
//! written file is built from.
//!
//! ## `url`
//!
//! Zero of the 85 committed files use the structured object form: 36 hold a
//! one-element string array, 38 a two-element array, 11 omit `url` entirely.
//! Writing the object form would rewrite 74 files, so the editor writes back
//! whatever form it read, and uses the object form only where there was no
//! prior value. [`Self::url_shape`] reports the form and [`Self::set_url`]
//! honours it.

use platform_metadata::project::ProjectRaw;
use platform_metadata::utils::Multilingual;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::json::strip_null_members;
use crate::multilingual::DraftMultilingual;

/// A project while it is being edited.
///
/// Serializes as the project object itself, so `drafts.payload` holds readable
/// JSON rather than a wrapper.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectDraft {
    members: Map<String, Value>,
}

/// Why a draft could not be turned into a publishable project.
#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    /// The draft is missing a required field, or holds a value of the wrong
    /// shape for one. Carries `serde_json`'s message, which names the field.
    ///
    /// Per-field error paths for the form are a separate concern (DEV-7045
    /// extracts `validate`'s rules with paths); this is the type-level gate.
    #[error("draft is not a publishable project: {0}")]
    NotPublishable(String),

    /// The project could not be serialized. Not the depositor's problem, and
    /// separate from [`Self::NotPublishable`] so a writer bug is not reported as
    /// an invalid field, sending them to hunt a problem that is not there.
    #[error("could not serialize the project: {0}")]
    Serialization(String),
}

/// Which variant of an `#[serde(untagged)]` coverage entry a value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextOrReference {
    /// An authority-file reference: `{"type": …, "url": …, "text": …}`.
    Reference,
    /// A free-text multilingual value.
    Text,
}

/// Which variant of `Funding` a value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundingShape {
    /// A list of structured grants.
    Grants,
    /// A single free-text string.
    Text,
}

/// The on-disk form of `url`, which the editor writes back unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrlShape {
    /// No `url` member. A first value is written as [`Self::Object`].
    Absent,
    /// The legacy form: a string array, element 1 being the secondary URL.
    StringArray,
    /// The structured `AuthorityFileReference` object.
    Object,
}

impl ProjectDraft {
    /// Builds a draft from a project as loaded, losslessly (REQ-1.7).
    ///
    /// Null members are stripped so that "absent" has one meaning in a draft.
    /// Nothing is lost: every nullable field on `ProjectRaw` is an `Option`,
    /// and serde reads a missing `Option` as `None` (asserted over all 85
    /// committed files by the round-trip test).
    ///
    /// Member order is `ProjectRaw`'s field declaration order, because
    /// `serde_json::to_value` follows the serializer under the workspace's
    /// `preserve_order` feature. That is also the order the canonical writer
    /// emits, so a draft nobody edited writes back byte-identically.
    #[must_use]
    pub fn from_raw(raw: &ProjectRaw) -> Self {
        // Both panics are unreachable for the contract as it stands: every field
        // is a `String`, an `Option`, a `Vec`, a `BTreeMap<String, _>`, a `Value`
        // or a struct of those, none of which can fail to serialize, and a struct
        // always serializes to an object. They are loud rather than degrading
        // because the degraded value would be an *empty* draft, which is
        // indistinguishable from a project with no fields: the form would render
        // blank and a save would write `{}` over the depositor's project. A
        // future field with a fallible `Serialize` has to fail visibly, and the
        // round-trip test fails first, in CI.
        let mut value = serde_json::to_value(raw).expect("ProjectRaw serializes");
        strip_null_members(&mut value);
        let Value::Object(members) = value else {
            panic!("ProjectRaw serializes to a JSON object");
        };
        Self { members }
    }

    /// The publishable project, or why the draft is not one yet.
    ///
    /// This is the submission gate (REQ-1.12's type-level half): a draft that
    /// omits a required field or holds an invalid value fails here.
    pub fn to_raw(&self) -> Result<ProjectRaw, DraftError> {
        serde_json::from_value(Value::Object(self.members.clone()))
            .map_err(|err| DraftError::NotPublishable(err.to_string()))
    }

    /// One field's raw value, or `None` when the field is not set.
    #[must_use]
    pub fn get(&self, field: &str) -> Option<&Value> {
        self.members.get(field)
    }

    /// Sets one field's raw value, valid or not.
    ///
    /// A `Value::Null` removes the field instead of storing a null, so a draft
    /// never holds the ambiguity `from_raw` strips out.
    pub fn set(&mut self, field: &str, value: Value) {
        if value.is_null() {
            self.members.shift_remove(field);
        } else {
            self.members.insert(field.to_string(), value);
        }
    }

    /// Drops one field. Returns whether it was set.
    pub fn remove(&mut self, field: &str) -> bool {
        self.members.shift_remove(field).is_some()
    }

    /// The fields currently set, in declaration order.
    pub fn fields(&self) -> impl Iterator<Item = &str> {
        self.members.keys().map(String::as_str)
    }

    /// One multilingual field as an editing view. An unset or non-object field
    /// reads as empty, which is what a form needs from a draft.
    #[must_use]
    pub fn multilingual(&self, field: &str) -> DraftMultilingual {
        let contract = self
            .get(field)
            .and_then(|value| serde_json::from_value::<Multilingual>(value.clone()).ok())
            .unwrap_or_default();
        DraftMultilingual::from_contract(&contract)
    }

    /// Writes one multilingual field back. An empty value removes the field
    /// rather than writing `{}`, so clearing a field in the form is the same
    /// state as never having filled it.
    pub fn set_multilingual(&mut self, field: &str, value: &DraftMultilingual) {
        if value.is_empty() {
            self.remove(field);
        } else {
            self.set(field, serde_json::to_value(value.to_contract()).unwrap_or(Value::Null));
        }
    }

    /// Which variant each `disciplines` entry is. Empty when the field is unset
    /// or is not a list.
    #[must_use]
    pub fn discipline_shapes(&self) -> Vec<TextOrReference> {
        self.coverage_shapes("disciplines")
    }

    /// Which variant each `temporalCoverage` entry is. Empty when the field is
    /// unset or is not a list.
    #[must_use]
    pub fn temporal_coverage_shapes(&self) -> Vec<TextOrReference> {
        self.coverage_shapes("temporalCoverage")
    }

    fn coverage_shapes(&self, field: &str) -> Vec<TextOrReference> {
        self.get(field)
            .and_then(Value::as_array)
            .map(|entries| entries.iter().map(coverage_shape).collect())
            .unwrap_or_default()
    }

    /// Which variant `funding` is, or `None` when the field is unset or holds
    /// neither an array nor a string.
    #[must_use]
    pub fn funding_shape(&self) -> Option<FundingShape> {
        match self.get("funding")? {
            // Serde tries `Grants(Vec<Grant>)` before `Text(String)`, and only
            // an array can satisfy it.
            Value::Array(_) => Some(FundingShape::Grants),
            Value::String(_) => Some(FundingShape::Text),
            _ => None,
        }
    }

    /// The form `url` is held in, and therefore the form a write-back uses.
    #[must_use]
    pub fn url_shape(&self) -> UrlShape {
        match self.get("url") {
            None => UrlShape::Absent,
            Some(Value::Array(_)) => UrlShape::StringArray,
            Some(Value::Object(_)) => UrlShape::Object,
            // A `url` of some other kind is invalid data the editor did not
            // write. Treat it as legacy so an edit replaces it with the form
            // its neighbours use rather than introducing a second one.
            Some(_) => UrlShape::StringArray,
        }
    }

    /// Sets `url`, and `secondaryUrl` where the form keeps it separately.
    ///
    /// The form is whatever was read ([`Self::url_shape`]); a field that had no
    /// prior value gets the structured object, which is the form new projects
    /// use. `primary: None` clears both.
    pub fn set_url(&mut self, primary: Option<&str>, secondary: Option<&str>) {
        let Some(primary) = primary else {
            self.remove("url");
            self.remove("secondaryUrl");
            return;
        };
        match self.url_shape() {
            UrlShape::StringArray => {
                let mut array = vec![Value::String(primary.to_string())];
                array.extend(secondary.map(|url| Value::String(url.to_string())));
                self.set("url", Value::Array(array));
                // The legacy form carries the secondary URL as element 1, so a
                // separate member would be a second, contradictory home for it.
                self.remove("secondaryUrl");
            }
            UrlShape::Object | UrlShape::Absent => {
                self.set("url", authority_file_reference(primary));
                match secondary {
                    Some(url) => self.set("secondaryUrl", authority_file_reference(url)),
                    None => {
                        self.remove("secondaryUrl");
                    }
                }
            }
        }
    }
}

/// The variant an untagged coverage entry deserializes to, asked in serde's own
/// attempt order: `Reference` is declared first, so a value that satisfies
/// `AuthorityFileReference` is one whatever else it might also fit.
fn coverage_shape(entry: &Value) -> TextOrReference {
    if serde_json::from_value::<platform_metadata::AuthorityFileReference>(entry.clone()).is_ok() {
        TextOrReference::Reference
    } else {
        TextOrReference::Text
    }
}

fn authority_file_reference(url: &str) -> Value {
    let mut object = Map::new();
    object.insert("type".to_string(), Value::String("URL".to_string()));
    object.insert("url".to_string(), Value::String(url.to_string()));
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::test_support::sample_raw;

    #[test]
    fn a_draft_may_omit_a_field_that_the_contract_requires() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        assert!(draft.remove("name"));
        assert!(draft.get("name").is_none());
        let err = draft.to_raw().expect_err("a nameless draft is not publishable");
        assert!(err.to_string().contains("name"), "{err}");
    }

    #[test]
    fn a_draft_retains_a_value_the_contract_rejects() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("status", json!("onging"));
        assert_eq!(draft.get("status"), Some(&json!("onging")));
        assert!(draft.to_raw().is_err());
    }

    /// REQ-1.8: the draft carries a field it has never heard of, so adding one
    /// to `ProjectRaw` needs no editor change.
    #[test]
    fn an_unknown_field_survives_the_draft_round_trip() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("fieldAddedNextYear", json!({"nested": ["value"]}));
        let payload = serde_json::to_string(&draft).expect("a draft serializes");
        let reloaded: ProjectDraft = serde_json::from_str(&payload).expect("a draft deserializes");
        assert_eq!(reloaded.get("fieldAddedNextYear"), Some(&json!({"nested": ["value"]})));
    }

    #[test]
    fn from_raw_holds_members_in_declaration_order() {
        let draft = ProjectDraft::from_raw(&sample_raw());
        let first: Vec<&str> = draft.fields().take(5).collect();
        assert_eq!(first, ["id", "pid", "name", "shortcode", "officialName"]);
    }

    #[test]
    fn from_raw_strips_nulls_so_absent_has_one_meaning() {
        let draft = ProjectDraft::from_raw(&sample_raw());
        assert!(
            draft.fields().all(|field| !draft.get(field).unwrap().is_null()),
            "no member should be null"
        );
        // `imageCredit` is absent from every committed file, so it is the null
        // that `to_value` would otherwise have produced.
        assert!(draft.get("imageCredit").is_none());
    }

    #[test]
    fn set_null_removes_rather_than_storing_a_null() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("provenance", Value::Null);
        assert!(draft.get("provenance").is_none());
    }

    /// The failure the issue names: free-text funding must not be forced into
    /// the grant shape.
    #[test]
    fn free_text_funding_stays_free_text_through_the_draft() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("funding", json!("Funded by a person, not a grant"));
        assert_eq!(draft.funding_shape(), Some(FundingShape::Text));
        let raw = draft.to_raw().expect("publishable");
        assert!(matches!(raw.funding, platform_metadata::Funding::Text(_)));
    }

    #[test]
    fn grant_funding_reads_as_grants() {
        let draft = ProjectDraft::from_raw(&sample_raw());
        assert_eq!(draft.funding_shape(), Some(FundingShape::Grants));
    }

    #[test]
    fn coverage_shapes_distinguish_references_from_free_text() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set(
            "temporalCoverage",
            json!([
                {"type": "Chronontology", "url": "https://chronontology.dainst.org/period/x", "text": "Trajanic"},
                {"en": "11th-15th centuries"},
            ]),
        );
        assert_eq!(
            draft.temporal_coverage_shapes(),
            [TextOrReference::Reference, TextOrReference::Text]
        );
    }

    #[test]
    fn multilingual_view_round_trips_through_the_draft() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        let mut description = draft.multilingual("description");
        description.set("ar", "مرحبا");
        draft.set_multilingual("description", &description);
        assert_eq!(draft.multilingual("description").get("ar"), Some("مرحبا"));
        // Written back alphabetically, whatever the editing order.
        let keys: Vec<&String> = draft.get("description").unwrap().as_object().unwrap().keys().collect();
        assert_eq!(keys.first(), Some(&&"ar".to_string()));
    }

    #[test]
    fn emptying_a_multilingual_field_removes_it_rather_than_writing_an_empty_object() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set_multilingual("abstract", &DraftMultilingual::new());
        assert!(draft.get("abstract").is_none());
    }

    #[test]
    fn a_missing_multilingual_field_reads_as_empty() {
        let draft = ProjectDraft::from_raw(&sample_raw());
        assert!(draft.multilingual("noSuchField").is_empty());
    }

    /// Finding 2: an array stays an array. 74 of the 85 files would otherwise
    /// be rewritten into the object form.
    #[test]
    fn set_url_keeps_the_legacy_array_form() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("url", json!(["https://old.example.org"]));
        assert_eq!(draft.url_shape(), UrlShape::StringArray);
        draft.set_url(Some("https://new.example.org"), Some("https://secondary.example.org"));
        assert_eq!(
            draft.get("url"),
            Some(&json!(["https://new.example.org", "https://secondary.example.org"]))
        );
        assert!(draft.get("secondaryUrl").is_none());
    }

    #[test]
    fn set_url_keeps_the_object_form() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("url", json!({"type": "URL", "url": "https://old.example.org"}));
        assert_eq!(draft.url_shape(), UrlShape::Object);
        draft.set_url(Some("https://new.example.org"), None);
        assert_eq!(
            draft.get("url"),
            Some(&json!({"type": "URL", "url": "https://new.example.org"}))
        );
    }

    /// The 11 files that omit `url`, and every new project: no prior value, so
    /// the structured form is the one to introduce.
    #[test]
    fn set_url_uses_the_object_form_where_there_was_no_prior_value() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.remove("url");
        assert_eq!(draft.url_shape(), UrlShape::Absent);
        draft.set_url(Some("https://new.example.org"), Some("https://secondary.example.org"));
        assert_eq!(
            draft.get("url"),
            Some(&json!({"type": "URL", "url": "https://new.example.org"}))
        );
        assert_eq!(
            draft.get("secondaryUrl"),
            Some(&json!({"type": "URL", "url": "https://secondary.example.org"}))
        );
    }

    #[test]
    fn set_url_with_no_primary_clears_both_members() {
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set("url", json!({"type": "URL", "url": "https://old.example.org"}));
        draft.set("secondaryUrl", json!({"type": "URL", "url": "https://secondary.example.org"}));
        draft.set_url(None, None);
        assert!(draft.get("url").is_none());
        assert!(draft.get("secondaryUrl").is_none());
    }

    #[test]
    fn a_default_draft_is_empty_and_not_publishable() {
        let draft = ProjectDraft::default();
        assert_eq!(draft.fields().count(), 0);
        assert!(draft.to_raw().is_err());
        assert_eq!(draft.url_shape(), UrlShape::Absent);
        assert_eq!(draft.funding_shape(), None);
    }
}
