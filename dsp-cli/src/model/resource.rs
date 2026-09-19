//! Domain model for resource instances — `ResourceSummary`, `ResourcePage`,
//! `ResourceDetail`, `ResourceVisibility`, `ResourceAccess`, `FieldValues`,
//! `Value`, `ValueContent`, `DateValue`, `DatePoint`, and `FileValue`.
//!
//! These types cross the client/action boundary: `ResourcePage` is the raw
//! per-page result returned by [`DspClient::list_resources`]; `ResourceSummary`
//! is the per-row projection that flows through the action into the renderer.
//! `ResourceDetail` carries the full envelope metadata returned by
//! [`DspClient::describe_resource`]. `FieldValues` and `ValueContent` carry
//! the parsed field values (instance side), emitted when `--values` is set.

/// A single resource instance, as returned by the list endpoint.
///
/// Fields are the envelope metadata only (label, IRI, ARK URL, creation date,
/// last-modification date, resource type). Values are NOT fetched at list time
/// — see D4 in the plan.
///
/// Uses the detailed representation: `ark_url`, `creation_date`, and
/// `last_modified` are `Option<String>` for robustness; the list endpoint uses
/// the complex schema so both dates populate (verified live on `dev` 2026-06-17).
/// `last_modified` is additionally server-side optional — a resource that has
/// never been modified will have none. The live test (Layer 5) makes hard
/// assertions on the load-bearing fields.
#[derive(Debug, Clone)]
pub struct ResourceSummary {
    /// Human-readable label assigned to this resource instance.
    pub label: String,
    /// Absolute IRI of this resource instance (its `@id`).
    pub iri: String,
    /// ARK URL for permanent citation, if present in the response.
    pub ark_url: Option<String>,
    /// RFC 3339 creation timestamp, if present in the response.
    pub creation_date: Option<String>,
    /// RFC 3339 last-modification timestamp, if present in the response.
    /// Server-side optional — absent for resources that have never been modified.
    pub last_modified: Option<String>,
    /// Local name of the resource type (derived from `@type`).
    pub resource_type: String,
}

/// Who can see a resource — derived from the resource's full access-control list.
///
/// Answers "is this resource public?". Classified by the highest access level
/// granted to anonymous visitors or any logged-in user. The value is a
/// translated dsp-cli domain term; raw permission codes never appear above the
/// client boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceVisibility {
    /// The resource is publicly readable without authentication.
    Public,
    /// The resource is publicly accessible but only in a restricted view
    /// (e.g. watermarked images). Full content requires authentication.
    PublicRestricted,
    /// The resource is visible to any logged-in user, but not to anonymous visitors.
    LoggedInUsers,
    /// The resource is only accessible to members of the project (or higher).
    ProjectMembers,
}

impl ResourceVisibility {
    /// Display string used uniformly across all output formats.
    ///
    /// Note: some variants include spaces and parentheses (e.g.
    /// `"public (restricted view)"`), so this is not a bare lowercase token.
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceVisibility::Public => "public",
            ResourceVisibility::PublicRestricted => "public (restricted view)",
            ResourceVisibility::LoggedInUsers => "logged-in users",
            ResourceVisibility::ProjectMembers => "project members only",
        }
    }
}

/// What the requesting caller can do with a resource.
///
/// Derived from the server's per-caller effective permission code. Answers
/// "what can I do with this resource?" The value is a translated dsp-cli
/// domain term; raw permission codes never appear above the client boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceAccess {
    /// The caller can view the resource in a restricted form only (e.g. a
    /// low-resolution image). They cannot see the full content.
    RestrictedView,
    /// The caller can view the resource in full.
    View,
    /// The caller can view and modify the resource's values.
    Edit,
    /// The caller can view, modify, and delete the resource.
    Delete,
    /// The caller has full control over the resource (view, modify, delete,
    /// and change permissions).
    Manage,
}

impl ResourceAccess {
    /// Display string used uniformly across all output formats.
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceAccess::RestrictedView => "restricted view",
            ResourceAccess::View => "view",
            ResourceAccess::Edit => "edit",
            ResourceAccess::Delete => "delete",
            ResourceAccess::Manage => "manage",
        }
    }
}

/// Full envelope metadata for a single resource instance.
///
/// Returned by [`DspClient::describe_resource`]. Contains the resource's
/// identity, audit timestamps, ownership, and two translated permission facets
/// (visibility and the caller's access level). The `values` field is populated
/// only when `--values` is requested (Phase 8c); it is `None` for the default
/// metadata-only mode (Phase 8b).
#[derive(Debug, Clone)]
pub struct ResourceDetail {
    /// Human-readable label assigned to this resource instance.
    pub label: String,
    /// Absolute IRI of this resource instance.
    pub iri: String,
    /// Local name of the resource type (e.g. "Page").
    pub resource_type: String,
    /// ARK URL for permanent citation, if present in the response.
    pub ark_url: Option<String>,
    /// RFC 3339 creation timestamp, if present in the response.
    pub creation_date: Option<String>,
    /// RFC 3339 last-modification timestamp, if present in the response.
    /// Absent for resources that have never been modified.
    pub last_modified: Option<String>,
    /// IRI of the project this resource belongs to, if present in the response.
    pub attached_project: Option<String>,
    /// IRI of the user who owns this resource, if present in the response.
    pub owner: Option<String>,
    /// Who can see this resource, derived from the resource's access-control list.
    /// `None` when the ACL is absent or unparseable.
    pub visibility: Option<ResourceVisibility>,
    /// What the requesting caller can do with this resource.
    /// `None` when the caller's effective permission is absent or unknown.
    pub your_access: Option<ResourceAccess>,
    /// Parsed field values for this resource instance.
    ///
    /// `None` when `--values` is not set (metadata-only mode — the default).
    /// `Some` when `--values` is set; the vec may still be empty if the resource
    /// has no readable user fields.
    pub values: Option<Vec<FieldValues>>,
}

/// One value on a field, plus its optional per-value comment
/// (`knora-api:valueHasComment`). The comment is free-text server-supplied
/// annotation; `None` when the value has no comment (the common case).
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub content: ValueContent,
    pub comment: Option<String>,
}

impl From<ValueContent> for Value {
    fn from(content: ValueContent) -> Self {
        Value {
            content,
            comment: None,
        }
    }
}

/// One field on a resource and the value(s) it holds (instance side).
///
/// `name` is the field's local name (the `Value`-suffix is stripped on link
/// properties, as in DSP-API property names like `isPartOfBookValue` →
/// `isPartOfBook`). `label` is the server-supplied `rdfs:label` resolved from
/// the defining data-model's ontology; `None` when the fetch failed or the
/// field is a built-in. `values` is the list of parsed values for this field
/// (multi-value fields have more than one entry; the list is never empty — a
/// field with no readable values is omitted from the parent
/// `ResourceDetail.values` vec).
#[derive(Debug, Clone, PartialEq)]
pub struct FieldValues {
    /// Local field name (Value-suffix stripped for link properties).
    pub name: String,
    /// Human label from the defining data-model's ontology; `None` if unresolved
    /// or the field is a system built-in.
    pub label: Option<String>,
    /// Parsed values carried by this field — at least one entry. Each entry
    /// pairs a [`ValueContent`] with an optional per-value comment.
    pub values: Vec<Value>,
}

/// The typed content of a single value (instance side).
///
/// Each variant corresponds to one entry in the value-type rendering matrix
/// (ADR-0013). Scalar arms hold a single extracted datum. `VocabularyItem` and `Link`
/// additionally carry a resolved label (falling back to `None` on failure). `File`
/// covers all file-representation types. `Raw` is the long-tail fallback for any
/// value-type not in the named set — it never causes a hard error.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueContent {
    /// Plain or standoff text — standoff XML stripped by the client boundary.
    Text(String),
    /// Integer number.
    Integer(i64),
    /// Decimal number — stored as a string to preserve precision.
    Decimal(String),
    /// Boolean value.
    Boolean(bool),
    /// Calendar-aware date, possibly a range.
    Date(DateValue),
    /// Point-in-time timestamp string (ISO 8601).
    Time(String),
    /// URI string.
    Uri(String),
    /// Hex colour string (e.g. `"#ff0000"`).
    Color(String),
    /// GeoNames location code string.
    Geoname(String),
    /// Reference to a controlled-vocabulary list node.
    VocabularyItem {
        /// IRI of the list node.
        node_iri: String,
        /// Human label resolved from `/v2/node`; `None` when the fetch failed.
        label: Option<String>,
    },
    /// Link to another resource instance.
    Link {
        /// IRI of the target resource.
        target_iri: String,
        /// `rdfs:label` of the target resource, embedded in the complex-schema
        /// link value; `None` when absent.
        target_label: Option<String>,
    },
    /// File representation value (still-image, moving-image, audio, document,
    /// archive).
    File(FileValue),
    /// Long-tail fallback for value-types not covered by the named variants
    /// (e.g. interval, geometry). Never a hard error.
    Raw {
        /// dsp-cli token for the value type, derived from the DSP-API `@type`
        /// local name (lower-kebab form, e.g. `"interval"`).
        value_type: String,
        /// Best-effort text representation of the value datum.
        text: String,
    },
}

impl ValueContent {
    /// Returns the dsp-cli kebab token for this value's type.
    ///
    /// Scalar arms return a `&'static str` literal. `File` borrows from the
    /// inner `FileValue.value_type`. `Raw` borrows from the stored `value_type`
    /// string. No allocation in any arm.
    pub fn value_type_token(&self) -> &str {
        match self {
            ValueContent::Text(_) => "text",
            ValueContent::Integer(_) => "integer",
            ValueContent::Decimal(_) => "decimal",
            ValueContent::Boolean(_) => "boolean",
            ValueContent::Date(_) => "date",
            ValueContent::Time(_) => "time",
            ValueContent::Uri(_) => "uri",
            ValueContent::Color(_) => "color",
            ValueContent::Geoname(_) => "geoname",
            ValueContent::VocabularyItem { .. } => "vocabulary-item",
            ValueContent::Link { .. } => "link",
            ValueContent::File(fv) => fv.value_type.as_token(),
            ValueContent::Raw { value_type, .. } => value_type.as_str(),
        }
    }
}

/// Calendar-aware date value, optionally a range.
///
/// `start` and `end` are both present on every `DateValue`; a single-point date
/// has `start == end` (field-by-field). The renderer collapses equal start/end to
/// a single point display (`<point> (<Calendar>)`).
#[derive(Debug, Clone, PartialEq)]
pub struct DateValue {
    /// Calendar system (e.g. `"GREGORIAN"`, `"JULIAN"`, `"ISLAMIC"`).
    pub calendar: String,
    /// Start of the date range (inclusive).
    pub start: DatePoint,
    /// End of the date range (inclusive). Equal to `start` for a single-point date.
    pub end: DatePoint,
}

/// One endpoint of a date range, with optional precision.
///
/// Precision follows the fields present: year only → year precision; year + month
/// → month precision; all three → day precision.
#[derive(Debug, Clone, PartialEq)]
pub struct DatePoint {
    /// Year component. Negative for BCE years.
    pub year: Option<i32>,
    /// Month component (1–12), if present.
    pub month: Option<u32>,
    /// Day component (1–31), if present.
    pub day: Option<u32>,
    /// Era string (e.g. `"CE"`, `"BCE"`), if present in the response.
    pub era: Option<String>,
}

/// File-representation value, covering all representation kinds.
///
/// `value_type` restricts to the five file kinds (`StillImage`, `MovingImage`,
/// `Audio`, `Document`, `Archive`). `width` and `height` are only populated for
/// `StillImage`; they are `None` for all other kinds.
#[derive(Debug, Clone, PartialEq)]
pub struct FileValue {
    /// Representation kind — one of the five file variants of `ValueType`.
    pub value_type: crate::model::resource_type::ValueType,
    /// Original filename as stored by the server (e.g. `"image.jp2"`).
    pub filename: String,
    /// IIIF or download URL for the file.
    pub url: String,
    /// Image width in pixels; `Some` for `StillImage`, `None` otherwise.
    pub width: Option<u32>,
    /// Image height in pixels; `Some` for `StillImage`, `None` otherwise.
    pub height: Option<u32>,
}

/// A single page of resource-list results from the DSP-API.
///
/// Returned by [`DspClient::list_resources`]. The action accumulates pages
/// for `--all` mode; for single-page mode the action reads exactly one.
#[derive(Debug, Clone)]
pub struct ResourcePage {
    /// The resources on this page (may be empty — the empty-final-page case is normal).
    pub resources: Vec<ResourceSummary>,
    /// Whether the server reports more pages after this one.
    ///
    /// `false` when the field is absent or false in the response — there are no
    /// further pages. `true` means the caller should fetch the next page.
    pub may_have_more_results: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::resource_type::ValueType;

    // ── ValueContent::value_type_token ───────────────────────────────────────

    #[test]
    fn value_type_token_text() {
        assert_eq!(
            ValueContent::Text("hello".into()).value_type_token(),
            "text"
        );
    }

    #[test]
    fn value_type_token_integer() {
        assert_eq!(ValueContent::Integer(42).value_type_token(), "integer");
    }

    #[test]
    fn value_type_token_decimal() {
        assert_eq!(
            ValueContent::Decimal("3.14".into()).value_type_token(),
            "decimal"
        );
    }

    #[test]
    fn value_type_token_boolean() {
        assert_eq!(ValueContent::Boolean(true).value_type_token(), "boolean");
    }

    #[test]
    fn value_type_token_date() {
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
            end: DatePoint {
                year: Some(1489),
                month: None,
                day: None,
                era: Some("CE".into()),
            },
        };
        assert_eq!(ValueContent::Date(dv).value_type_token(), "date");
    }

    #[test]
    fn value_type_token_time() {
        assert_eq!(
            ValueContent::Time("2021-01-01T00:00:00Z".into()).value_type_token(),
            "time"
        );
    }

    #[test]
    fn value_type_token_uri() {
        assert_eq!(
            ValueContent::Uri("https://example.com".into()).value_type_token(),
            "uri"
        );
    }

    #[test]
    fn value_type_token_color() {
        assert_eq!(
            ValueContent::Color("#ff0000".into()).value_type_token(),
            "color"
        );
    }

    #[test]
    fn value_type_token_geoname() {
        assert_eq!(
            ValueContent::Geoname("2661552".into()).value_type_token(),
            "geoname"
        );
    }

    #[test]
    fn value_type_token_vocabulary_item() {
        assert_eq!(
            ValueContent::VocabularyItem {
                node_iri: "http://rdfh.ch/lists/0001/node1".into(),
                label: Some("Leaf node".into()),
            }
            .value_type_token(),
            "vocabulary-item"
        );
    }

    #[test]
    fn value_type_token_link() {
        assert_eq!(
            ValueContent::Link {
                target_iri: "http://rdfh.ch/0803/res1".into(),
                target_label: None,
            }
            .value_type_token(),
            "link"
        );
    }

    #[test]
    fn value_type_token_file_still_image() {
        let fv = FileValue {
            value_type: ValueType::StillImage,
            filename: "image.jp2".into(),
            url: "https://iiif.example.com/image.jp2/full/max/0/default.jpg".into(),
            width: Some(1200),
            height: Some(800),
        };
        assert_eq!(ValueContent::File(fv).value_type_token(), "still-image");
    }

    #[test]
    fn value_type_token_file_moving_image() {
        let fv = FileValue {
            value_type: ValueType::MovingImage,
            filename: "video.mp4".into(),
            url: "https://example.com/video.mp4".into(),
            width: None,
            height: None,
        };
        assert_eq!(ValueContent::File(fv).value_type_token(), "moving-image");
    }

    #[test]
    fn value_type_token_file_audio() {
        let fv = FileValue {
            value_type: ValueType::Audio,
            filename: "sound.wav".into(),
            url: "https://example.com/sound.wav".into(),
            width: None,
            height: None,
        };
        assert_eq!(ValueContent::File(fv).value_type_token(), "audio");
    }

    #[test]
    fn value_type_token_file_document() {
        let fv = FileValue {
            value_type: ValueType::Document,
            filename: "doc.pdf".into(),
            url: "https://example.com/doc.pdf".into(),
            width: None,
            height: None,
        };
        assert_eq!(ValueContent::File(fv).value_type_token(), "document");
    }

    #[test]
    fn value_type_token_file_archive() {
        let fv = FileValue {
            value_type: ValueType::Archive,
            filename: "data.zip".into(),
            url: "https://example.com/data.zip".into(),
            width: None,
            height: None,
        };
        assert_eq!(ValueContent::File(fv).value_type_token(), "archive");
    }

    #[test]
    fn value_type_token_raw() {
        assert_eq!(
            ValueContent::Raw {
                value_type: "interval".into(),
                text: "PT10S".into(),
            }
            .value_type_token(),
            "interval"
        );
    }

    // ── FieldValues construction ──────────────────────────────────────────────

    #[test]
    fn field_values_construction_and_equality() {
        let fv = FieldValues {
            name: "hasTitle".into(),
            label: Some("Title".into()),
            values: vec![ValueContent::Text("Incunabula".into()).into()],
        };
        let cloned = fv.clone();
        assert_eq!(fv, cloned);
        assert_eq!(fv.name, "hasTitle");
        assert_eq!(fv.label.as_deref(), Some("Title"));
        assert_eq!(fv.values.len(), 1);
    }

    // ── DateValue / DatePoint construction ───────────────────────────────────

    #[test]
    fn date_value_point_equality() {
        let pt = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: pt.clone(),
            end: pt.clone(),
        };
        assert_eq!(dv.start, dv.end, "single-point date must have start == end");
    }

    #[test]
    fn date_value_range_not_equal() {
        let start = DatePoint {
            year: Some(1489),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        let end = DatePoint {
            year: Some(1490),
            month: None,
            day: None,
            era: Some("CE".into()),
        };
        let dv = DateValue {
            calendar: "GREGORIAN".into(),
            start: start.clone(),
            end: end.clone(),
        };
        assert_ne!(dv.start, dv.end, "range date must have start != end");
    }
}
