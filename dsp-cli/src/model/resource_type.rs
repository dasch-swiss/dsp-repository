//! Resource-type domain shape — shared across the client → action boundary.
//!
//! Types here are the dsp-cli vocabulary for resource-type detail data,
//! surfaced by `dsp vre resource-type describe`. DSP-API wire types
//! live inside `src/client/http.rs` and are never exposed above the client
//! layer. See ADR-0001 and ADR-0008.
//!
//! Key CONTEXT.md vocabulary: resource-type, field, value-type, cardinality,
//! representation. Wire deserialization (DSP-API `owl:Restriction`,
//! `owl:onProperty`, `knora-api:objectType`, `rdfs:subClassOf`, etc.) stays
//! inside `src/client/http.rs`.

use std::fmt;

/// The full detail of a resource-type, as surfaced by `dsp vre resource-type describe`.
///
/// The describe projection: identity (`name` + `iri`), server-supplied `label`,
/// the resource-type's own `data_model` name (the baseline for cross-DM field
/// tagging and the prose `Data-model:` header), an optional `representation`
/// kind (for asset types), the project/external superclass local names
/// (`super_types`), and the full field list. No `serde` derive: wire
/// deserialization stays in `src/client/http.rs`. See ADR-0001 / ADR-0008
/// and the CONTEXT.md "Resource Type" / "Field" / "Cardinality" /
/// "Representation" entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceTypeDetail {
    /// Short name of the resource-type (e.g. `manuscript`), derived from the IRI fragment.
    pub name: String,
    /// Full IRI of the resource-type
    /// (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2#manuscript`).
    pub iri: String,
    /// Server-supplied human label (`rdfs:label`), if any.
    pub label: Option<String>,
    /// Short name of the resource-type's own data-model (e.g. `beol`). Used as
    /// the prose `Data-model:` header line and as the baseline when the renderer
    /// tags cross-DM fields (`[from <dm>]`). Derived from the data-model IRI at
    /// the client boundary — not a wire field.
    pub data_model: String,
    /// Representation kind, if the resource-type is a file representation
    /// (still-image, moving-image, audio, document, archive, text). Detected from
    /// the presence of the corresponding `knora-api` file-value restriction in the
    /// class's flattened `owl:Restriction` set (Decision 5 — transitive-safe;
    /// see the plan). `None` for non-asset resource-types.
    pub representation: Option<Representation>,
    /// Project and external (non-system) superclass local names. Derived from the
    /// non-Restriction `{"@id":…}` entries in `rdfs:subClassOf` (Decision 6).
    /// Excludes `knora-api` / system supers. Empty if the type extends only system
    /// classes or has no explicit non-system superclass.
    pub super_types: Vec<String>,
    /// Full field list. Includes both project-defined and built-in (system)
    /// fields; the action layer filters built-ins unless `--include-builtins` is
    /// set. Sorted by `salsah-gui:guiOrder` then by name at the client boundary.
    pub fields: Vec<Field>,
    /// Instance count from the v3 `resourcesPerOntology` route, populated only
    /// when `--count` is passed to `resource-type describe`. `None` when the
    /// flag was not used, or when the class was absent from the v3 payload
    /// (e.g. a built-in with `--include-builtins`).
    pub count: Option<u64>,
}

/// A field belonging to a resource-type, as surfaced by `dsp vre resource-type describe`.
///
/// Carries identity (`name` + `iri`), server-supplied `label`, the field's
/// `value_type` (see `ValueType`), an optional `link_target` (the target
/// resource-type local name, `Some` iff `value_type == ValueType::Link`),
/// `cardinality`, a flag marking system (built-in) fields, and the source
/// `data_model` name (for cross-DM tagging). No `serde` derive: wire
/// deserialization stays in `src/client/http.rs`. See ADR-0001 / ADR-0008
/// and the CONTEXT.md "Field" / "Value Type" / "Cardinality" entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Short name of the field (e.g. `hasTitle`), derived from the property IRI.
    pub name: String,
    /// Full IRI of the field's property
    /// (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2#hasTitle`).
    pub iri: String,
    /// Server-supplied human label (`rdfs:label`), if any. `None` for system
    /// built-in fields whose property node is not fetched, and for fields left
    /// best-effort after a failed sibling-ontology fetch.
    pub label: Option<String>,
    /// The kind of value this field holds (text, integer, link, …).
    pub value_type: ValueType,
    /// The local name of the link target resource-type (e.g. `Book`, `person`).
    /// `Some` iff `value_type == ValueType::Link`; `None` for all other value
    /// types. This invariant is asserted in unit tests — the type does not
    /// enforce it structurally.
    pub link_target: Option<String>,
    /// Cardinality constraint: how many values the field may / must carry.
    pub cardinality: Cardinality,
    /// `true` iff the field's property CURIE prefix is a system namespace
    /// (`knora-api`, `knora-base`, `rdf`, `rdfs`, `owl`, `salsah-gui`,
    /// `standoff`, `xsd`). System fields are hidden by default; revealed with
    /// `--include-builtins` (Decision 3).
    pub is_builtin: bool,
    /// Source data-model name for the field's property. `Some` with the
    /// data-model short name for project-defined and cross-DM fields
    /// (e.g. `Some("biblio")` for a `biblio:` property on a `beol` class);
    /// `None` for system built-ins (system-namespace prefix). The renderer
    /// emits a `[from <dm>]` tag when `Some(x)` and `x` differs from
    /// `ResourceTypeDetail.data_model` (Decision 10).
    pub data_model: Option<String>,
}

/// The kind of value a field holds — the dsp-cli vocabulary for DSP-API's
/// `knora-api:objectType`.
///
/// Named variants cover all 16 value types from the CONTEXT.md "Value Type"
/// entry. `Other(String)` provides graceful degradation for any `objectType`
/// outside this set (e.g. `GeomValue`, `IntervalValue`, `TextFileValue`) — the
/// string is a kebab-cased local name derived by the client at the ADR-0001
/// boundary. `Display` writes the kebab string; `Other(s)` writes `s` verbatim
/// (the client builds the kebab form). Does NOT derive `Copy` (has `Other(String)`).
/// No `serde` derive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueType {
    /// Plain text (`knora-api:TextValue`).
    Text,
    /// Integer number (`knora-api:IntValue`).
    Integer,
    /// Decimal number (`knora-api:DecimalValue`).
    Decimal,
    /// Boolean (`knora-api:BooleanValue`).
    Boolean,
    /// Calendar date (`knora-api:DateValue`).
    Date,
    /// Point in time (`knora-api:TimeValue`).
    Time,
    /// URI (`knora-api:UriValue`).
    Uri,
    /// Color value (`knora-api:ColorValue`).
    Color,
    /// Geonames location reference (`knora-api:GeonameValue`).
    Geoname,
    /// Reference to a list node (`knora-api:ListValue`).
    VocabularyItem,
    /// Link to another resource (`knora-api:isLinkProperty`; objectType is the
    /// target resource class, not a `…Value` type).
    Link,
    /// Still-image file (`knora-api:StillImageFileValue`).
    StillImage,
    /// Moving-image file (`knora-api:MovingImageFileValue`).
    MovingImage,
    /// Audio file (`knora-api:AudioFileValue`).
    Audio,
    /// Document file (`knora-api:DocumentFileValue`).
    Document,
    /// Archive file (`knora-api:ArchiveFileValue`).
    Archive,
    /// Any objectType not covered by the 16 named variants (e.g. `geom`,
    /// `interval`, `text-file`). The string is already in kebab form — `Display`
    /// writes it verbatim.
    Other(String),
}

/// Cardinality constraint on a field — how many values may / must be supplied.
///
/// Maps directly to DSP-API's `owl:cardinality` / `owl:minCardinality` /
/// `owl:maxCardinality` triple (Decision 1 — only `0`/`1` bounds are emitted).
/// Derives `Copy` (fieldless). `Display` produces the CONTEXT.md canonical
/// notation (`1`, `0-1`, `0-n`, `1-n`), matching dsp-tools' native data-model
/// format. No `serde` derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    /// Exactly one value required (`owl:cardinality 1`). Display: `"1"`.
    One,
    /// At most one value, may be absent (`owl:maxCardinality 1`). Display: `"0-1"`.
    ZeroOrOne,
    /// Any number of values, may be absent (`owl:minCardinality 0`). Display: `"0-n"`.
    ZeroOrMore,
    /// At least one value required (`owl:minCardinality 1`). Display: `"1-n"`.
    OneOrMore,
}

/// Representation kind of a resource-type — what kind of file it holds.
///
/// Detected from the presence of the corresponding `knora-api` file-value
/// property restriction in the class's flattened `owl:Restriction` set (Decision
/// 5 — transitive-safe). Only present on resource-types that are file
/// representations; non-asset types carry `None` on `ResourceTypeDetail`.
/// Derives `Copy` (fieldless). `Display` produces a kebab string. No `serde`
/// derive. `ValueType` and `Representation` overlap semantically (both carry
/// still-image / … variants) but are independent types — field-level vs
/// resource-type-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Representation {
    /// Still-image representation (`knora-api:hasStillImageFileValue`). Display: `"still-image"`.
    StillImage,
    /// Moving-image representation (`knora-api:hasMovingImageFileValue`). Display:
    /// `"moving-image"`.
    MovingImage,
    /// Audio representation (`knora-api:hasAudioFileValue`). Display: `"audio"`.
    Audio,
    /// Document representation (`knora-api:hasDocumentFileValue`). Display: `"document"`.
    Document,
    /// Archive representation (`knora-api:hasArchiveFileValue`). Display: `"archive"`.
    Archive,
    /// Text representation (`knora-api:hasTextFileValue`). Display: `"text"`.
    Text,
}

impl fmt::Display for Cardinality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cardinality::One => f.write_str("1"),
            Cardinality::ZeroOrOne => f.write_str("0-1"),
            Cardinality::ZeroOrMore => f.write_str("0-n"),
            Cardinality::OneOrMore => f.write_str("1-n"),
        }
    }
}

impl fmt::Display for Representation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Representation::StillImage => f.write_str("still-image"),
            Representation::MovingImage => f.write_str("moving-image"),
            Representation::Audio => f.write_str("audio"),
            Representation::Document => f.write_str("document"),
            Representation::Archive => f.write_str("archive"),
            Representation::Text => f.write_str("text"),
        }
    }
}

impl ValueType {
    /// Returns the canonical dsp-cli kebab token for this value type.
    ///
    /// Named variants return a `&'static str` literal. `Other(s)` borrows `s`
    /// directly — the client already built the kebab form. Matches `Display`
    /// output; prefer `as_token` when you need a `&str` without allocating.
    pub fn as_token(&self) -> &str {
        match self {
            ValueType::Text => "text",
            ValueType::Integer => "integer",
            ValueType::Decimal => "decimal",
            ValueType::Boolean => "boolean",
            ValueType::Date => "date",
            ValueType::Time => "time",
            ValueType::Uri => "uri",
            ValueType::Color => "color",
            ValueType::Geoname => "geoname",
            ValueType::VocabularyItem => "vocabulary-item",
            ValueType::Link => "link",
            ValueType::StillImage => "still-image",
            ValueType::MovingImage => "moving-image",
            ValueType::Audio => "audio",
            ValueType::Document => "document",
            ValueType::Archive => "archive",
            ValueType::Other(s) => s.as_str(),
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Route through `as_token` so the two remain in sync.
        f.write_str(self.as_token())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Construction, equality, clone round-trip ---

    #[test]
    fn resource_type_detail_full_construction_and_equality() {
        let detail = ResourceTypeDetail {
            name: "manuscript".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript".into(),
            label: Some("Manuscript".into()),
            data_model: "beol".into(),
            representation: Some(Representation::StillImage),
            super_types: vec!["writtenSource".into()],
            fields: vec![Field {
                name: "hasTitle".into(),
                iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasTitle".into(),
                label: Some("Title".into()),
                value_type: ValueType::Text,
                link_target: None,
                cardinality: Cardinality::OneOrMore,
                is_builtin: false,
                data_model: Some("beol".into()),
            }],
            count: None,
        };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert_eq!(detail.name, "manuscript");
        assert_eq!(detail.iri, "http://api.dasch.swiss/ontology/0801/beol/v2#manuscript");
        assert_eq!(detail.label.as_deref(), Some("Manuscript"));
        assert_eq!(detail.data_model, "beol");
        assert_eq!(detail.representation, Some(Representation::StillImage));
        assert_eq!(detail.super_types, vec!["writtenSource"]);
        assert_eq!(detail.fields.len(), 1);
    }

    #[test]
    fn resource_type_detail_minimal_none_variants() {
        let detail = ResourceTypeDetail {
            name: "Thing".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#Thing".into(),
            label: None,
            data_model: "minimal".into(),
            representation: None,
            super_types: vec![],
            fields: vec![],
            count: None,
        };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert_eq!(detail.label, None);
        assert_eq!(detail.representation, None);
        assert!(detail.super_types.is_empty());
        assert!(detail.fields.is_empty());
    }

    #[test]
    fn field_full_construction_and_equality() {
        let field = Field {
            name: "hasAuthor".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasAuthor".into(),
            label: Some("Author".into()),
            value_type: ValueType::Link,
            link_target: Some("person".into()),
            cardinality: Cardinality::ZeroOrMore,
            is_builtin: false,
            data_model: Some("beol".into()),
        };
        let cloned = field.clone();
        assert_eq!(field, cloned);
        assert_eq!(field.name, "hasAuthor");
        assert_eq!(field.iri, "http://api.dasch.swiss/ontology/0801/beol/v2#hasAuthor");
        assert_eq!(field.label.as_deref(), Some("Author"));
        assert_eq!(field.value_type, ValueType::Link);
        assert_eq!(field.link_target.as_deref(), Some("person"));
        assert_eq!(field.cardinality, Cardinality::ZeroOrMore);
        assert!(!field.is_builtin);
        assert_eq!(field.data_model.as_deref(), Some("beol"));
    }

    #[test]
    fn field_minimal_none_variants() {
        let field = Field {
            name: "arkUrl".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2#arkUrl".into(),
            label: None,
            value_type: ValueType::Uri,
            link_target: None,
            cardinality: Cardinality::One,
            is_builtin: true,
            data_model: None,
        };
        let cloned = field.clone();
        assert_eq!(field, cloned);
        assert_eq!(field.label, None);
        assert_eq!(field.link_target, None);
        assert!(field.is_builtin);
        assert_eq!(field.data_model, None);
    }

    // --- Display assertions for Cardinality (all 4 variants) ---

    #[test]
    fn cardinality_display_one() {
        assert_eq!(Cardinality::One.to_string(), "1");
    }

    #[test]
    fn cardinality_display_zero_or_one() {
        assert_eq!(Cardinality::ZeroOrOne.to_string(), "0-1");
    }

    #[test]
    fn cardinality_display_zero_or_more() {
        assert_eq!(Cardinality::ZeroOrMore.to_string(), "0-n");
    }

    #[test]
    fn cardinality_display_one_or_more() {
        assert_eq!(Cardinality::OneOrMore.to_string(), "1-n");
    }

    // --- Display assertions for Representation (all 6 variants) ---

    #[test]
    fn representation_display_still_image() {
        assert_eq!(Representation::StillImage.to_string(), "still-image");
    }

    #[test]
    fn representation_display_moving_image() {
        assert_eq!(Representation::MovingImage.to_string(), "moving-image");
    }

    #[test]
    fn representation_display_audio() {
        assert_eq!(Representation::Audio.to_string(), "audio");
    }

    #[test]
    fn representation_display_document() {
        assert_eq!(Representation::Document.to_string(), "document");
    }

    #[test]
    fn representation_display_archive() {
        assert_eq!(Representation::Archive.to_string(), "archive");
    }

    #[test]
    fn representation_display_text() {
        assert_eq!(Representation::Text.to_string(), "text");
    }

    // --- Display assertions for ValueType (all 16 named + Other) ---

    #[test]
    fn value_type_display_text() {
        assert_eq!(ValueType::Text.to_string(), "text");
    }

    #[test]
    fn value_type_display_integer() {
        assert_eq!(ValueType::Integer.to_string(), "integer");
    }

    #[test]
    fn value_type_display_decimal() {
        assert_eq!(ValueType::Decimal.to_string(), "decimal");
    }

    #[test]
    fn value_type_display_boolean() {
        assert_eq!(ValueType::Boolean.to_string(), "boolean");
    }

    #[test]
    fn value_type_display_date() {
        assert_eq!(ValueType::Date.to_string(), "date");
    }

    #[test]
    fn value_type_display_time() {
        assert_eq!(ValueType::Time.to_string(), "time");
    }

    #[test]
    fn value_type_display_uri() {
        assert_eq!(ValueType::Uri.to_string(), "uri");
    }

    #[test]
    fn value_type_display_color() {
        assert_eq!(ValueType::Color.to_string(), "color");
    }

    #[test]
    fn value_type_display_geoname() {
        assert_eq!(ValueType::Geoname.to_string(), "geoname");
    }

    #[test]
    fn value_type_display_vocabulary_item() {
        assert_eq!(ValueType::VocabularyItem.to_string(), "vocabulary-item");
    }

    #[test]
    fn value_type_display_link() {
        assert_eq!(ValueType::Link.to_string(), "link");
    }

    #[test]
    fn value_type_display_still_image() {
        assert_eq!(ValueType::StillImage.to_string(), "still-image");
    }

    #[test]
    fn value_type_display_moving_image() {
        assert_eq!(ValueType::MovingImage.to_string(), "moving-image");
    }

    #[test]
    fn value_type_display_audio() {
        assert_eq!(ValueType::Audio.to_string(), "audio");
    }

    #[test]
    fn value_type_display_document() {
        assert_eq!(ValueType::Document.to_string(), "document");
    }

    #[test]
    fn value_type_display_archive() {
        assert_eq!(ValueType::Archive.to_string(), "archive");
    }

    #[test]
    fn value_type_display_other_verbatim() {
        // Other(s) writes s verbatim — the client builds the kebab form.
        assert_eq!(ValueType::Other("text-file".into()).to_string(), "text-file");
    }

    // --- ValueType::as_token matches Display for all variants ---

    /// `as_token` returns the correct kebab string for a representative of each
    /// named variant, and matches `Display` output exactly.
    #[test]
    fn value_type_as_token_matches_display_named_variants() {
        let cases = [
            ValueType::Text,
            ValueType::Integer,
            ValueType::Decimal,
            ValueType::Boolean,
            ValueType::Date,
            ValueType::Time,
            ValueType::Uri,
            ValueType::Color,
            ValueType::Geoname,
            ValueType::VocabularyItem,
            ValueType::Link,
            ValueType::StillImage,
            ValueType::MovingImage,
            ValueType::Audio,
            ValueType::Document,
            ValueType::Archive,
        ];
        for vt in &cases {
            assert_eq!(vt.as_token(), vt.to_string(), "as_token must match Display for {:?}", vt);
        }
    }

    #[test]
    fn value_type_as_token_still_image() {
        assert_eq!(ValueType::StillImage.as_token(), "still-image");
    }

    #[test]
    fn value_type_as_token_vocabulary_item() {
        assert_eq!(ValueType::VocabularyItem.as_token(), "vocabulary-item");
    }

    #[test]
    fn value_type_as_token_moving_image() {
        assert_eq!(ValueType::MovingImage.as_token(), "moving-image");
    }

    #[test]
    fn value_type_as_token_other_borrows_string() {
        let vt = ValueType::Other("geom".into());
        // as_token borrows from the inner String; matches Display.
        assert_eq!(vt.as_token(), "geom");
        assert_eq!(vt.as_token(), vt.to_string());
    }

    // --- Link ⇔ link_target invariant, both directions ---

    #[test]
    fn link_field_has_link_target_some() {
        // A Field with value_type == Link MUST carry link_target == Some(...).
        let field = Field {
            name: "hasAuthor".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasAuthor".into(),
            label: Some("Author".into()),
            value_type: ValueType::Link,
            link_target: Some("person".into()),
            cardinality: Cardinality::ZeroOrMore,
            is_builtin: false,
            data_model: Some("beol".into()),
        };
        assert_eq!(field.value_type, ValueType::Link);
        assert!(field.link_target.is_some(), "a Link field must have link_target == Some(...)");
    }

    #[test]
    fn non_link_field_has_link_target_none() {
        // A Field with value_type != Link MUST carry link_target == None.
        let field = Field {
            name: "hasTitle".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#hasTitle".into(),
            label: Some("Title".into()),
            value_type: ValueType::Text,
            link_target: None,
            cardinality: Cardinality::OneOrMore,
            is_builtin: false,
            data_model: Some("beol".into()),
        };
        assert_ne!(field.value_type, ValueType::Link);
        assert!(field.link_target.is_none(), "a non-Link field must have link_target == None");
    }
}
