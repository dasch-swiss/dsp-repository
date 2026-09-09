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

/// Which of the two URLs a field owns.
///
/// The project contract keeps a DaSCH address and an external one, and *where*
/// it keeps them depends on the project's vintage — so a field cannot simply
/// name a member. The two slots are also owned by different audiences (`url` is
/// RDU-only, `secondaryUrl` is a depositor's), which is the reason they are
/// written independently rather than as a pair: a depositor editing the external
/// site must not be able to touch the DaSCH one, and must not lose it either.
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
    ///
    /// A **dotted** field is followed segment by segment, so
    /// `accessRights.embargoDate` reads the member inside `accessRights`. Every
    /// other reader here does the same, which is what lets the field registry
    /// name a nested member and have the form, the applier, the rail and the
    /// submit gate all agree about what it points at. Without it a dotted id
    /// looked up a top-level member that does not exist, so the field read as
    /// unset whatever the project held — silently, since nothing else about it
    /// would look wrong.
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
    /// A `Value::Null` removes the field instead of storing a null, so a draft
    /// never holds the ambiguity `from_raw` strips out.
    ///
    /// A dotted field writes the nested member, creating the objects on the way
    /// down where they are absent — a project whose `accessRights` is missing
    /// still has to be able to take an embargo date. A segment holding a
    /// non-object is replaced, because the alternative is a write that silently
    /// does nothing.
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
    /// A dotted field drops only the nested member; the object holding it stays,
    /// even when it is left empty. `canonical::write_draft` strips empty and
    /// null members on the way out, so the file is the same either way, and
    /// removing the parent here would drop its *siblings* — clearing an embargo
    /// date would take the access-rights choice with it.
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
    /// Reads whichever representation the project actually uses: most keep the pair positionally in
    /// a `url` array, some keep the secondary in its own `secondaryUrl` member, and one project
    /// holds a one-element array *and* a member.
    ///
    /// So the array is not simply "the form". For the secondary it answers only when it actually
    /// has a second element, and the member answers otherwise — returning early on the array's
    /// mere presence reads that project's external website as absent, and an untouched save
    /// then deletes it. No project has a two-element array and a member, so there is never a
    /// contradiction to resolve.
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
    /// Its current home wins, so a save preserves the representation rather than
    /// migrating it: the one project holding both a one-element array and a
    /// `secondaryUrl` member keeps using the member. Only when nothing is
    /// stored is there a choice, and then the array wins if there is one —
    /// which is what the 38 projects carrying a secondary positionally look
    /// like.
    fn secondary_in_array(&self) -> bool {
        match self.get("url").and_then(Value::as_array) {
            Some(array) => array.len() > 1 || self.get("secondaryUrl").is_none(),
            None => false,
        }
    }

    /// Writes one of the two URLs, leaving the other alone.
    ///
    /// **Do not fold these into one setter taking both.** Writing the pair together clears both
    /// when the primary is `None`, and "no primary, has a secondary" is a state published
    /// projects are already in — `url` is RDU-only and `secondaryUrl` is a depositor's, so the
    /// two must be writable independently.
    ///
    /// Each write touches only the slot's own home, so the stored representation survives and an
    /// untouched save is byte-identical. One forced exception: clearing the primary while a
    /// *positional* secondary exists cannot be written as an array, element 0 being the
    /// primary, so that case moves the secondary into its own member and drops `url` — the form
    /// other projects already use for this state.
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

/// A JSON string, so the array arms above read as data rather than as
/// conversions.
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

/// A field id as a root member plus the nested segments under it.
///
/// One place, because `get`, `set` and `remove` must agree about what a dotted
/// id points at; three copies of a two-line split is how they stop agreeing.
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

    /// The 11 files that omit `url`, and every new project: no prior value, so
    /// the structured form is the one to introduce.

    #[test]
    fn a_dotted_field_reads_the_nested_member_rather_than_a_top_level_one() {
        // `accessRights.embargoDate` is the registry's one dotted id. Read as a
        // top-level member it is always absent, so the field renders empty
        // whatever the project holds and the rail counts it unsatisfied — with
        // nothing else about it looking wrong.
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
        // `set(_, Null)` removes rather than storing a null at every depth, so a
        // draft never holds the ambiguity `from_raw` strips out.
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
        // Projects keep the pair positionally, or the secondary in its own member, or one project
        // both — so the array answers for the secondary only when it has a second element.
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
        // The property the old paired `set_url` could not offer: `url` is
        // RDU-only and `secondaryUrl` is a depositor's, so each write must be
        // confined to its own slot.
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
        // The one case the array form cannot express: element 0 *is* the primary, so "no primary,
        // has a secondary" has no positional writing. The member form does, and it is the
        // form committed projects already use.
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
        // The regression the old paired API had: `set_url(None, Some(x))`
        // cleared both members, and 11 published projects are in precisely the
        // "no primary, has a secondary" state — so editing the external site on
        // one of them would have wiped it.
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
        // `0112_roud` is the shape this got wrong: a one-element `url` array
        // *and* a `secondaryUrl` member. Returning early on the array's
        // presence read the secondary as absent, and the untouched-save round
        // trip then deleted it — caught only because that test runs over the
        // committed bytes rather than a fixture.
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
        // The representation survives the save: this project keeps its
        // secondary in the member, so a write goes there and the array is left
        // as the one element it is. Writing it positionally instead would give
        // the project two homes for one value.
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
        // 35 projects have a one-element array and no member at all; adding an
        // external site to one of those should make it look like the 38 that
        // already carry theirs positionally, not introduce a second form.
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
