//! Data-model domain shape — shared across the client → action boundary.
//!
//! Types here are the dsp-cli vocabulary for data-model data. DSP-API wire types
//! live inside `src/client/http.rs` and are never exposed above the client
//! layer. See ADR-0001 and ADR-0008.

/// A data-model as shown by `dsp vre data-model list`.
///
/// The list projection: identity (`name` + `iri`) plus the server's `label`
/// and `last_modified` metadata, and whether it is a platform **built-in**
/// (inherited by every project) rather than project-defined. `name` is derived
/// from the IRI at the client boundary — not a wire field. No `serde` derive:
/// wire deserialization stays in `src/client/http.rs`. See ADR-0001 / ADR-0008
/// and the CONTEXT.md "Data Model" / "Built-in" entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataModel {
    /// Short name of the data-model (e.g. `beol`), derived from the IRI.
    pub name: String,
    /// Full IRI of the data-model (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2`).
    pub iri: String,
    /// Server-supplied human label (`rdfs:label`), if any.
    pub label: Option<String>,
    /// Last-modification timestamp as a raw RFC3339 string, if the server
    /// supplies one. Built-ins carry `None`.
    pub last_modified: Option<String>,
    /// `true` for platform built-ins (knora-api, standoff, salsah-gui);
    /// `false` for project-defined data-models.
    pub is_builtin: bool,
}

/// A lean reference to a child resource-type, as surfaced by
/// `data-model describe`.
///
/// The describe-summary projection: identity (`name` + `iri`) plus the server's
/// human `label`. Full resource-type detail (fields, value-types, cardinalities)
/// is surfaced by the `dsp vre resource-type describe` leaf command — see
/// `model::ResourceTypeDetail` / `Field`.
/// `name` is derived from the resource-type's IRI by the HTTP client layer at the
/// ADR-0001 boundary — it is NOT a server-supplied field. No `serde` derive: wire
/// deserialization stays in `src/client/http.rs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceTypeSummary {
    /// Short name of the resource-type (e.g. `letter`), derived from the IRI.
    pub name: String,
    /// Full IRI of the resource-type
    /// (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2#letter`).
    pub iri: String,
    /// Server-supplied human label (`rdfs:label`), if any.
    pub label: Option<String>,
}

/// A resource-type as shown by `dsp vre resource-type list` — the list projection.
///
/// Contrast `ResourceTypeSummary` (the lean describe-child inside `DataModelDetail`,
/// which has no `is_builtin` concept because `data-model describe` only ever shows a
/// project ontology's own classes). This type adds `is_builtin` so the list command
/// can mix project-defined and platform built-in resource-types (Region, AudioSegment,
/// VideoSegment, LinkObj) when `--include-builtins` is supplied, and callers can
/// discriminate the two kinds. `name` is derived from the IRI fragment at the client
/// boundary — not a wire field. No `serde` derive: wire deserialization stays in
/// `src/client/http.rs`. See ADR-0001 / ADR-0008 and the CONTEXT.md "Resource Type" /
/// "Built-in" entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceType {
    /// Short name of the resource-type (e.g. `letter`), derived from the IRI fragment.
    pub name: String,
    /// Full IRI of the resource-type
    /// (project: `http://api.dasch.swiss/ontology/0801/beol/v2#letter`;
    /// built-in: `http://api.knora.org/ontology/knora-api/v2#Region`).
    pub iri: String,
    /// Server-supplied human label (`rdfs:label`), if any. All 4 platform built-ins
    /// carry labels (Region, Audio Annotation, Video Annotation, Link Object).
    pub label: Option<String>,
    /// `true` for platform built-ins (knora-api: Region, AudioSegment, VideoSegment,
    /// LinkObj); `false` for project-defined resource-types.
    pub is_builtin: bool,
    /// Instance count from the v3 `resourcesPerOntology` route, populated only
    /// when `--count` is passed to `resource-type list`. `None` when the flag was
    /// not used, or when the class was absent from the v3 payload (e.g. a
    /// built-in with `--include-builtins`).
    pub count: Option<u64>,
}

/// A data-model as shown by `dsp vre data-model describe` — the rich projection.
///
/// Contrast `DataModel` (the lean `list` index projection). Carries identity
/// (`name` + `iri`), the server's `label` and `last_modified`, and a **summary**
/// (count + names) of the data-model's child resource-types — NOT their fields.
/// No `serde` derive: wire deserialization stays in `src/client/http.rs`.
/// See ADR-0001 / ADR-0008 and the CONTEXT.md "Data Model" / "Resource Type" entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataModelDetail {
    /// Short name of the data-model (e.g. `beol`), derived from the IRI.
    pub name: String,
    /// Full IRI of the data-model.
    pub iri: String,
    /// Server-supplied human label (`rdfs:label`), if any.
    pub label: Option<String>,
    /// Last-modification timestamp as a raw RFC3339 string, if supplied.
    pub last_modified: Option<String>,
    /// Lean references to the data-model's child resource-types, sorted by name.
    pub resource_types: Vec<ResourceTypeSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_model_full_construction_and_equality() {
        let dm = DataModel {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
            label: Some("The BEOL data-model".into()),
            last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
            is_builtin: false,
        };
        let cloned = dm.clone();
        assert_eq!(dm, cloned);
        assert_eq!(dm.name, "beol");
        assert_eq!(dm.iri, "http://api.dasch.swiss/ontology/0801/beol/v2");
        assert_eq!(dm.label.as_deref(), Some("The BEOL data-model"));
        assert_eq!(
            dm.last_modified.as_deref(),
            Some("2024-05-27T13:43:26.233048Z")
        );
        assert!(!dm.is_builtin);
    }

    #[test]
    fn data_model_all_none_builtin() {
        let dm = DataModel {
            name: "knora-api".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2".into(),
            label: None,
            last_modified: None,
            is_builtin: true,
        };
        let cloned = dm.clone();
        assert_eq!(dm, cloned);
        assert_eq!(dm.name, "knora-api");
        assert_eq!(dm.iri, "http://api.knora.org/ontology/knora-api/v2");
        assert_eq!(dm.label, None);
        assert_eq!(dm.last_modified, None);
        assert!(dm.is_builtin);
    }

    #[test]
    fn data_model_label_some_last_modified_none() {
        let dm = DataModel {
            name: "limc".into(),
            iri: "http://api.dasch.swiss/ontology/0603/limc/v2".into(),
            label: Some("LIMC data-model".into()),
            last_modified: None,
            is_builtin: false,
        };
        let cloned = dm.clone();
        assert_eq!(dm, cloned);
        assert_eq!(dm.label.as_deref(), Some("LIMC data-model"));
        assert_eq!(dm.last_modified, None);
        assert!(!dm.is_builtin);
    }

    // --- ResourceType tests ---

    #[test]
    fn resource_type_with_label_not_builtin() {
        let rt = ResourceType {
            name: "letter".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
            label: Some("Letter".into()),
            is_builtin: false,
            count: None,
        };
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
        assert_eq!(rt.name, "letter");
        assert_eq!(
            rt.iri,
            "http://api.dasch.swiss/ontology/0801/beol/v2#letter"
        );
        assert_eq!(rt.label.as_deref(), Some("Letter"));
        assert!(!rt.is_builtin);
    }

    #[test]
    fn resource_type_label_none_not_builtin() {
        let rt = ResourceType {
            name: "Archive".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
            label: None,
            is_builtin: false,
            count: None,
        };
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
        assert_eq!(rt.name, "Archive");
        assert_eq!(rt.label, None);
        assert!(!rt.is_builtin);
    }

    #[test]
    fn resource_type_with_label_builtin() {
        let rt = ResourceType {
            name: "Region".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2#Region".into(),
            label: Some("Region".into()),
            is_builtin: true,
            count: None,
        };
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
        assert_eq!(rt.name, "Region");
        assert_eq!(rt.iri, "http://api.knora.org/ontology/knora-api/v2#Region");
        assert_eq!(rt.label.as_deref(), Some("Region"));
        assert!(rt.is_builtin);
    }

    #[test]
    fn resource_type_label_none_builtin() {
        // Edge-case: built-in with no label (shouldn't happen in practice, but the type
        // must round-trip cleanly regardless).
        let rt = ResourceType {
            name: "LinkObj".into(),
            iri: "http://api.knora.org/ontology/knora-api/v2#LinkObj".into(),
            label: None,
            is_builtin: true,
            count: None,
        };
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
        assert_eq!(rt.name, "LinkObj");
        assert_eq!(rt.label, None);
        assert!(rt.is_builtin);
    }

    // --- ResourceTypeSummary tests ---

    #[test]
    fn resource_type_summary_with_label() {
        let rt = ResourceTypeSummary {
            name: "letter".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
            label: Some("Letter".into()),
        };
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
        assert_eq!(rt.name, "letter");
        assert_eq!(
            rt.iri,
            "http://api.dasch.swiss/ontology/0801/beol/v2#letter"
        );
        assert_eq!(rt.label.as_deref(), Some("Letter"));
    }

    #[test]
    fn resource_type_summary_without_label() {
        let rt = ResourceTypeSummary {
            name: "Archive".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
            label: None,
        };
        let cloned = rt.clone();
        assert_eq!(rt, cloned);
        assert_eq!(rt.name, "Archive");
        assert_eq!(
            rt.iri,
            "http://api.dasch.swiss/ontology/0801/beol/v2#Archive"
        );
        assert_eq!(rt.label, None);
    }

    // --- DataModelDetail tests ---

    #[test]
    fn data_model_detail_full_construction_and_equality() {
        let detail = DataModelDetail {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
            label: Some("The BEOL data-model".into()),
            last_modified: Some("2024-05-27T13:43:26.233048Z".into()),
            resource_types: vec![
                ResourceTypeSummary {
                    name: "Archive".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#Archive".into(),
                    label: Some("Archive".into()),
                },
                ResourceTypeSummary {
                    name: "letter".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2#letter".into(),
                    label: Some("Letter".into()),
                },
            ],
        };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert_eq!(detail.name, "beol");
        assert_eq!(detail.iri, "http://api.dasch.swiss/ontology/0801/beol/v2");
        assert_eq!(detail.label.as_deref(), Some("The BEOL data-model"));
        assert_eq!(
            detail.last_modified.as_deref(),
            Some("2024-05-27T13:43:26.233048Z")
        );
        assert_eq!(detail.resource_types.len(), 2);
    }

    #[test]
    fn data_model_detail_label_none_last_modified_none() {
        let detail = DataModelDetail {
            name: "minimal".into(),
            iri: "http://api.dasch.swiss/ontology/0000/minimal/v2".into(),
            label: None,
            last_modified: None,
            resource_types: vec![ResourceTypeSummary {
                name: "Thing".into(),
                iri: "http://api.dasch.swiss/ontology/0000/minimal/v2#Thing".into(),
                label: None,
            }],
        };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert_eq!(detail.label, None);
        assert_eq!(detail.last_modified, None);
        assert_eq!(detail.resource_types.len(), 1);
        assert_eq!(detail.resource_types[0].label, None);
    }

    #[test]
    fn data_model_detail_empty_resource_types() {
        let detail = DataModelDetail {
            name: "empty".into(),
            iri: "http://api.dasch.swiss/ontology/9999/empty/v2".into(),
            label: Some("Empty data-model".into()),
            last_modified: None,
            resource_types: vec![],
        };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert!(detail.resource_types.is_empty());
        assert_eq!(detail.label.as_deref(), Some("Empty data-model"));
        assert_eq!(detail.last_modified, None);
    }
}
