//! The permissive draft representation.
//!
//! A draft holds what `ProjectRaw` cannot: a field not filled in yet, a value
//! present but invalid, every field the editor does not manage, and a field
//! added to `ProjectRaw` since this build. So a draft is the project's JSON
//! members: an absent key is a missing field, any `Value` is accepted whether it
//! validates or not, and an unknown key rides through untouched. Validity is
//! decided once, at [`ProjectDraft::to_raw`].
//!
//! `TemporalCoverage`, `Discipline` and `Funding` are `#[serde(untagged)]`, and
//! the variant is derived from the value's JSON kind (`ProjectDraft::funding_shape`
//! and friends) rather than stored: a stored tag would be a second source of
//! truth able to drift from the value the file is built from.
//!
//! `url` is written back in whatever form it was read, and the object form is
//! used only where there was no prior value; `ProjectDraft::url_shape` reports
//! the form and `ProjectDraft::set_url_slot` honours it.

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

/// Which of the two URLs a field owns.
///
/// Where the contract keeps the DaSCH address and the external one depends on
/// the project's vintage, so a field cannot simply name a member. `url` is
/// RDU-only and `secondaryUrl` is a depositor's, so the two are written
/// independently: a depositor must neither touch the DaSCH one nor lose it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlSlot {
    /// The DaSCH platform address — `url`, or element 0 of the legacy array.
    Primary,
    /// The project's own website — `secondaryUrl`, or element 1 of the legacy
    /// array.
    Secondary,
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
    /// Builds a draft from a project as loaded, losslessly.
    ///
    /// Null members are stripped so that "absent" has one meaning; nothing is
    /// lost, since every nullable field on `ProjectRaw` is an `Option`. Member
    /// order is `ProjectRaw`'s declaration order under `preserve_order`, which
    /// is also what the canonical writer emits.
    #[must_use]
    pub fn from_raw(raw: &ProjectRaw) -> Self {
        // Loud rather than degrading: the degraded value would be an empty draft,
        // indistinguishable from a project with no fields, and a save would write
        // `{}` over the depositor's project.
        let mut value = serde_json::to_value(raw).expect("ProjectRaw serializes");
        strip_null_members(&mut value);
        let Value::Object(members) = value else {
            panic!("ProjectRaw serializes to a JSON object");
        };
        Self { members }
    }

    /// The publishable project, or why the draft is not one yet.
    ///
    /// This is the submission gate's type-level half: a draft that
    /// omits a required field or holds an invalid value fails here.
    pub fn to_raw(&self) -> Result<ProjectRaw, DraftError> {
        serde_json::from_value(Value::Object(self.members.clone()))
            .map_err(|err| DraftError::NotPublishable(err.to_string()))
    }

    /// One field's raw value, or `None` when the field is not set.
    ///
    /// A dotted field is followed segment by segment, so `accessRights.embargoDate`
    /// reads the member inside `accessRights`. Every reader here does the same,
    /// which is what lets the field registry name a nested member.
    #[must_use]
    pub fn get(&self, field: &str) -> Option<&Value> {
        let (root, rest) = split_path(field);
        let mut current = self.members.get(root)?;
        for segment in rest {
            current = current.as_object()?.get(segment)?;
        }
        Some(current)
    }

    /// Sets one field's raw value, valid or not.
    ///
    /// A `Value::Null` removes the field instead of storing a null. A dotted
    /// field writes the nested member, creating the objects on the way down; a
    /// segment holding a non-object is replaced, because the alternative is a
    /// write that silently does nothing.
    pub fn set(&mut self, field: &str, value: Value) {
        let (root, rest) = split_path(field);
        if rest.is_empty() {
            if value.is_null() {
                self.members.shift_remove(root);
            } else {
                self.members.insert(root.to_string(), value);
            }
            return;
        }
        if value.is_null() {
            self.remove(field);
            return;
        }
        let mut current = self
            .members
            .entry(root.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        for segment in &rest[..rest.len() - 1] {
            if !current.is_object() {
                *current = Value::Object(Map::new());
            }
            current = current
                .as_object_mut()
                .expect("just made an object")
                .entry((*segment).to_string())
                .or_insert_with(|| Value::Object(Map::new()));
        }
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        current
            .as_object_mut()
            .expect("just made an object")
            .insert(rest[rest.len() - 1].to_string(), value);
    }

    /// Drops one field. Returns whether it was set.
    ///
    /// A dotted field drops only the nested member; the object holding it stays.
    /// Removing the parent would drop its siblings: clearing an embargo date
    /// would take the access-rights choice with it.
    pub fn remove(&mut self, field: &str) -> bool {
        let (root, rest) = split_path(field);
        if rest.is_empty() {
            return self.members.shift_remove(root).is_some();
        }
        let mut current = match self.members.get_mut(root) {
            Some(current) => current,
            None => return false,
        };
        for segment in &rest[..rest.len() - 1] {
            current = match current.as_object_mut().and_then(|object| object.get_mut(*segment)) {
                Some(next) => next,
                None => return false,
            };
        }
        current
            .as_object_mut()
            .is_some_and(|object| object.shift_remove(rest[rest.len() - 1]).is_some())
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

    /// One of the two URLs, as a plain string, or `None` where the project has
    /// none in that slot.
    ///
    /// Reads whichever representation the project uses: the pair positionally in
    /// a `url` array, the secondary in its own `secondaryUrl` member, or both.
    /// The array answers for the secondary only when it has a second element;
    /// returning early on its presence reads a member-held website as absent,
    /// and an untouched save then deletes it.
    #[must_use]
    pub fn url_slot(&self, slot: UrlSlot) -> Option<&str> {
        let array = self.get("url").and_then(Value::as_array);
        match slot {
            UrlSlot::Primary => match array {
                Some(array) => array.first().and_then(Value::as_str),
                // The structured form. No committed project uses it; it is what
                // a project starting from nothing is written as.
                None => self.get("url").and_then(|url| url.get("url")).and_then(Value::as_str),
            },
            UrlSlot::Secondary => array
                .and_then(|array| array.get(1))
                .and_then(Value::as_str)
                .or_else(|| self.get("secondaryUrl").and_then(|url| url.get("url")).and_then(Value::as_str)),
        }
    }

    /// Whether this project keeps its secondary URL positionally, and therefore
    /// whether a write to that slot goes into the `url` array.
    ///
    /// The current home wins, so a save preserves the representation rather than
    /// migrating it. Only when nothing is stored is there a choice, and then the
    /// array wins if there is one.
    fn secondary_in_array(&self) -> bool {
        match self.get("url").and_then(Value::as_array) {
            Some(array) => array.len() > 1 || self.get("secondaryUrl").is_none(),
            None => false,
        }
    }

    /// Writes one of the two URLs, leaving the other alone.
    ///
    /// Do not fold these into one setter taking both: writing the pair together
    /// clears both when the primary is `None`, and "no primary, has a secondary"
    /// is a state published projects are in. Each write touches only the slot's
    /// own home, so an untouched save is byte-identical. One forced exception:
    /// clearing the primary while a positional secondary exists cannot be written
    /// as an array, element 0 being the primary, so the secondary moves into its
    /// own member and `url` is dropped.
    pub fn set_url_slot(&mut self, slot: UrlSlot, value: Option<&str>) {
        match slot {
            UrlSlot::Secondary => {
                if self.secondary_in_array() {
                    let primary = self.url_slot(UrlSlot::Primary).map(str::to_string);
                    match (primary, value) {
                        (Some(primary), Some(secondary)) => {
                            self.set("url", Value::Array(vec![text(&primary), text(secondary)]));
                        }
                        (Some(primary), None) => self.set("url", Value::Array(vec![text(&primary)])),
                        // No primary to anchor the array, so the member is the
                        // only place left for a secondary.
                        (None, Some(secondary)) => {
                            self.remove("url");
                            self.set("secondaryUrl", authority_file_reference(secondary));
                        }
                        (None, None) => {
                            self.remove("url");
                        }
                    }
                } else {
                    match value {
                        Some(secondary) => self.set("secondaryUrl", authority_file_reference(secondary)),
                        None => {
                            self.remove("secondaryUrl");
                        }
                    }
                }
            }
            UrlSlot::Primary => match self.get("url").and_then(Value::as_array) {
                Some(array) => {
                    let positional = array.get(1).and_then(Value::as_str).map(str::to_string);
                    match (value, positional) {
                        (Some(primary), Some(secondary)) => {
                            self.set("url", Value::Array(vec![text(primary), text(&secondary)]));
                        }
                        (Some(primary), None) => self.set("url", Value::Array(vec![text(primary)])),
                        // Element 0 *is* the primary, so there is no positional
                        // way to say "no primary, has a secondary".
                        (None, Some(secondary)) => {
                            self.remove("url");
                            self.set("secondaryUrl", authority_file_reference(&secondary));
                        }
                        (None, None) => {
                            self.remove("url");
                        }
                    }
                }
                None => match value {
                    Some(primary) => self.set("url", authority_file_reference(primary)),
                    None => {
                        self.remove("url");
                    }
                },
            },
        }
    }
}

fn text(value: &str) -> Value {
    Value::String(value.to_string())
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

/// A field id as a root member plus the nested segments under it. One place, so
/// `get`, `set` and `remove` agree about what a dotted id points at.
fn split_path(field: &str) -> (&str, Vec<&str>) {
    let mut segments = field.split('.');
    let root = segments.next().unwrap_or(field);
    (root, segments.collect())
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

    /// A key the editor has never heard of survives being stored and reloaded,
    /// so a payload written by a newer schema is not truncated by an older
    /// binary. What reaches the published file is a separate question, decided
    /// by `ProjectRaw` in `canonical`.
    #[test]
    fn an_unknown_field_survives_the_draft_payload_round_trip() {
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

    #[test]
    fn a_dotted_field_reads_the_nested_member_rather_than_a_top_level_one() {
        // The registry's one dotted id; read as a top-level member it is always
        // absent, and nothing else about it looks wrong.
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set(
            "accessRights",
            json!({"accessRights": "Embargoed Access", "embargoDate": "2030-01-01"}),
        );
        assert_eq!(draft.get("accessRights.embargoDate"), Some(&json!("2030-01-01")));
        assert_eq!(draft.get("accessRights.accessRights"), Some(&json!("Embargoed Access")));
        assert_eq!(draft.get("accessRights.nothingHere"), None);
    }

    #[test]
    fn a_dotted_write_leaves_its_siblings_alone() {
        // The whole reason `remove` drops the member and not its parent: an
        // embargo date and the access-rights choice live in one object, so
        // clearing the date must not take the choice with it.
        let mut draft = ProjectDraft::from_raw(&sample_raw());
        draft.set(
            "accessRights",
            json!({"accessRights": "Embargoed Access", "embargoDate": "2030-01-01"}),
        );

        draft.set("accessRights.embargoDate", json!("2031-06-30"));
        assert_eq!(draft.get("accessRights.accessRights"), Some(&json!("Embargoed Access")));

        assert!(draft.remove("accessRights.embargoDate"));
        assert_eq!(draft.get("accessRights.embargoDate"), None);
        assert_eq!(
            draft.get("accessRights.accessRights"),
            Some(&json!("Embargoed Access")),
            "clearing the date must not clear the choice"
        );
        assert!(draft.get("accessRights").is_some(), "the object itself survives");
    }

    #[test]
    fn a_dotted_write_creates_the_object_it_needs() {
        // A project with no `accessRights` at all still has to be able to take
        // an embargo date, or the field is uneditable until some other write
        // happens to create the parent.
        let mut draft = ProjectDraft::default();
        draft.set("accessRights.embargoDate", json!("2030-01-01"));
        assert_eq!(draft.get("accessRights"), Some(&json!({"embargoDate": "2030-01-01"})));
    }

    #[test]
    fn a_dotted_set_of_null_removes_the_nested_member_only() {
        let mut draft = ProjectDraft::default();
        draft.set(
            "accessRights",
            json!({"accessRights": "Full Open Access", "embargoDate": "2030-01-01"}),
        );
        draft.set("accessRights.embargoDate", Value::Null);
        assert_eq!(draft.get("accessRights"), Some(&json!({"accessRights": "Full Open Access"})));
    }

    #[test]
    fn removing_a_dotted_field_that_is_not_there_reports_nothing_removed() {
        let mut draft = ProjectDraft::default();
        assert!(!draft.remove("accessRights.embargoDate"));
        draft.set("accessRights", json!("not an object"));
        assert!(!draft.remove("accessRights.embargoDate"), "a non-object parent holds no member");
    }

    #[test]
    fn a_plain_field_is_unaffected_by_the_path_split() {
        // The regression this could cause is broad and silent — every existing
        // caller passes an undotted id — so it is asserted rather than assumed.
        let mut draft = ProjectDraft::default();
        draft.set("name", json!("A Project"));
        assert_eq!(draft.get("name"), Some(&json!("A Project")));
        assert!(draft.remove("name"));
        assert_eq!(draft.get("name"), None);
        assert!(!draft.remove("name"));
    }

    #[test]
    fn a_slot_read_follows_whichever_form_the_project_uses() {
        let mut draft = ProjectDraft::default();
        draft.set(
            "url",
            json!(["https://app.dasch.swiss/project/0119", "https://external.example/"]),
        );
        assert_eq!(draft.url_slot(UrlSlot::Primary), Some("https://app.dasch.swiss/project/0119"));
        assert_eq!(draft.url_slot(UrlSlot::Secondary), Some("https://external.example/"));

        draft.set("url", json!(["https://app.dasch.swiss/project/0119"]));
        assert_eq!(draft.url_slot(UrlSlot::Secondary), None, "a one-element array has no secondary");

        let mut newer = ProjectDraft::default();
        newer.set("secondaryUrl", json!({"type": "URL", "url": "https://roud.unil.ch/"}));
        assert_eq!(newer.url_slot(UrlSlot::Primary), None);
        assert_eq!(newer.url_slot(UrlSlot::Secondary), Some("https://roud.unil.ch/"));
    }

    #[test]
    fn writing_one_slot_leaves_the_other_alone_in_the_array_form() {
        let mut draft = ProjectDraft::default();
        draft.set(
            "url",
            json!(["https://app.dasch.swiss/project/0119", "https://external.example/"]),
        );

        draft.set_url_slot(UrlSlot::Secondary, Some("https://moved.example/"));
        assert_eq!(
            draft.get("url"),
            Some(&json!(["https://app.dasch.swiss/project/0119", "https://moved.example/"])),
            "the DaSCH address is untouched"
        );

        draft.set_url_slot(UrlSlot::Primary, Some("https://app.dasch.swiss/project/9999"));
        assert_eq!(
            draft.get("url"),
            Some(&json!(["https://app.dasch.swiss/project/9999", "https://moved.example/"])),
            "the external site is untouched"
        );
    }

    #[test]
    fn clearing_the_secondary_leaves_a_one_element_array() {
        let mut draft = ProjectDraft::default();
        draft.set(
            "url",
            json!(["https://app.dasch.swiss/project/0119", "https://external.example/"]),
        );
        draft.set_url_slot(UrlSlot::Secondary, None);
        assert_eq!(draft.get("url"), Some(&json!(["https://app.dasch.swiss/project/0119"])));
        assert_eq!(draft.get("secondaryUrl"), None);
    }

    #[test]
    fn clearing_the_primary_moves_the_secondary_to_its_own_member() {
        let mut draft = ProjectDraft::default();
        draft.set(
            "url",
            json!(["https://app.dasch.swiss/project/0119", "https://external.example/"]),
        );

        draft.set_url_slot(UrlSlot::Primary, None);
        assert_eq!(draft.get("url"), None);
        assert_eq!(
            draft.get("secondaryUrl"),
            Some(&json!({"type": "URL", "url": "https://external.example/"})),
            "the depositor's value survives an RDU clear"
        );
        assert_eq!(draft.url_slot(UrlSlot::Secondary), Some("https://external.example/"));
    }

    #[test]
    fn a_secondary_survives_a_write_on_a_project_that_has_no_primary() {
        // Published projects exist with a secondary and no primary; editing the
        // external site on one must not wipe it.
        let mut draft = ProjectDraft::default();
        draft.set("secondaryUrl", json!({"type": "URL", "url": "https://roud.unil.ch/"}));

        draft.set_url_slot(UrlSlot::Secondary, Some("https://roud.unil.ch/about"));
        assert_eq!(
            draft.get("secondaryUrl"),
            Some(&json!({"type": "URL", "url": "https://roud.unil.ch/about"}))
        );
        assert_eq!(draft.get("url"), None, "no primary was invented");
    }

    #[test]
    fn a_first_url_on_a_project_with_none_takes_the_object_form() {
        // The form new projects use. Nothing in the corpus holds `url` as an
        // object, so this is only ever reached by a project starting from
        // nothing — which is why it must not be the form a *legacy* project is
        // migrated to.
        let mut draft = ProjectDraft::default();
        draft.set_url_slot(UrlSlot::Primary, Some("https://app.dasch.swiss/project/9999"));
        assert_eq!(
            draft.get("url"),
            Some(&json!({"type": "URL", "url": "https://app.dasch.swiss/project/9999"}))
        );
    }

    #[test]
    fn a_one_element_array_beside_a_member_reads_the_member_as_the_secondary() {
        // `0112_roud`: a one-element `url` array *and* a `secondaryUrl` member;
        // the member is the secondary.
        let mut draft = ProjectDraft::default();
        draft.set("url", json!(["https://app.ls-prod-server.dasch.swiss/project/0112"]));
        draft.set("secondaryUrl", json!({"type": "URL", "url": "https://roud.unil.ch/"}));

        assert_eq!(
            draft.url_slot(UrlSlot::Primary),
            Some("https://app.ls-prod-server.dasch.swiss/project/0112")
        );
        assert_eq!(draft.url_slot(UrlSlot::Secondary), Some("https://roud.unil.ch/"));
    }

    #[test]
    fn a_write_keeps_a_secondary_in_the_home_it_already_has() {
        // The representation survives the save: the secondary stays in the member
        // and the array stays one element, not two homes for one value.
        let mut draft = ProjectDraft::default();
        draft.set("url", json!(["https://app.ls-prod-server.dasch.swiss/project/0112"]));
        draft.set("secondaryUrl", json!({"type": "URL", "url": "https://roud.unil.ch/"}));

        draft.set_url_slot(UrlSlot::Secondary, Some("https://roud.unil.ch/about"));
        assert_eq!(
            draft.get("url"),
            Some(&json!(["https://app.ls-prod-server.dasch.swiss/project/0112"])),
            "the array gains no second element"
        );
        assert_eq!(
            draft.get("secondaryUrl"),
            Some(&json!({"type": "URL", "url": "https://roud.unil.ch/about"}))
        );
    }

    #[test]
    fn a_first_secondary_on_a_positional_project_goes_into_the_array() {
        // A one-element array and no member: the new secondary goes positional,
        // like the projects that already carry theirs that way.
        let mut draft = ProjectDraft::default();
        draft.set("url", json!(["https://app.dasch.swiss/project/0119"]));

        draft.set_url_slot(UrlSlot::Secondary, Some("https://external.example/"));
        assert_eq!(
            draft.get("url"),
            Some(&json!(["https://app.dasch.swiss/project/0119", "https://external.example/"]))
        );
        assert_eq!(draft.get("secondaryUrl"), None);
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
