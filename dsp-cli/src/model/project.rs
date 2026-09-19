//! Project domain shapes — shared across the client → action boundary.
//!
//! Types here are the dsp-cli vocabulary for project data. DSP-API wire types
//! live inside `src/client/http.rs` and are never exposed above the client
//! layer. See ADR-0001 and ADR-0008.

/// Minimal project reference. Phase 4 expands this into the full Project model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRef {
    /// The project's IRI (e.g. `http://rdfh.ch/projects/0001`).
    pub iri: String,
    /// Four-hex-digit shortcode (e.g. `0001`).
    pub shortcode: String,
    /// Human-readable shortname (e.g. `anything`).
    pub shortname: String,
}

/// A project's activation status (dsp-cli vocabulary for DSP-API's `status` bool).
///
/// **Why an enum over `active: bool`:** mirrors the [`DumpStatus`] precedent and
/// keeps the `active`/`inactive` display words as a single source of truth — the
/// vocabulary ADR wants dsp-cli terms defined once. Wire (bool) → enum translation
/// happens at the client boundary (`http.rs`), never here. Deliberate — not
/// over-engineering. See ADR-0001 and ADR-0008.
///
/// [`DumpStatus`]: crate::model::DumpStatus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    Active,
    Inactive,
}

impl ProjectStatus {
    /// Return the canonical lower-case display string for this status.
    ///
    /// This is the single source of truth for the status word used in
    /// `project list` output (human prose, JSON, tabular formats). Wire
    /// (bool) → enum translation happens in the HTTP client layer (`http.rs`)
    /// and is intentionally NOT delegated here.
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectStatus::Active => "active",
            ProjectStatus::Inactive => "inactive",
        }
    }
}

/// One language-tagged description value (dsp-cli vocabulary for the API's
/// `description: [{value, language}]`). `language` is optional on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDescription {
    /// The description text (may contain HTML markup as returned by the server).
    pub value: String,
    /// BCP-47 language tag, if the server provides one (e.g. `"en"`).
    pub language: Option<String>,
}

/// A lean reference to a child data-model, as surfaced by `project describe`.
///
/// Full data-model detail belongs to the future `data-model list/describe`
/// commands. `name` is derived from the data-model's IRI by the HTTP client
/// layer at the ADR-0001 boundary — it is NOT a server-supplied field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataModelSummary {
    /// Short name of the data-model (e.g. `beol`), derived from the IRI.
    pub name: String,
    /// Full IRI of the data-model (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2`).
    pub iri: String,
}

/// A project as shown by `dsp vre project describe` — the rich projection.
///
/// Contrast `Project` (the lean `list` index projection). Carries identity,
/// status, description, keywords, and a data-models summary (count + names).
/// No `serde` derive: wire deserialization stays in `src/client/http.rs`.
/// See ADR-0001 and ADR-0008.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDetail {
    /// The project's IRI (e.g. `http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF`).
    pub iri: String,
    /// Four-hex-digit shortcode (e.g. `0801`).
    pub shortcode: String,
    /// Human-readable shortname (e.g. `beol`).
    pub shortname: String,
    /// Human-readable long name, if the server supplies one.
    pub longname: Option<String>,
    /// Whether the project is currently active on the server.
    pub status: ProjectStatus,
    /// Language-tagged description values (may contain HTML markup).
    pub description: Vec<ProjectDescription>,
    /// Free-form keywords associated with the project.
    pub keywords: Vec<String>,
    /// Lean references to the project's child data-models, sorted by name.
    pub data_models: Vec<DataModelSummary>,
}

/// A project as shown by `dsp vre project list` — the lean index projection.
///
/// Rich detail (description, keywords, licences, …) belongs to the future
/// `describe` model, not here. No `serde` derive: wire deserialization stays
/// in `src/client/http.rs` (the private `ProjectListItemDto`). See ADR-0001
/// and ADR-0008.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// The project's IRI (e.g. `http://rdfh.ch/projects/0001`).
    pub iri: String,
    /// Four-hex-digit shortcode (e.g. `0001`).
    pub shortcode: String,
    /// Human-readable shortname (e.g. `anything`).
    pub shortname: String,
    /// Human-readable long name, if the server supplies one.
    pub longname: Option<String>,
    /// Whether the project is currently active on the server.
    pub status: ProjectStatus,
    /// Number of data-models (DSP-API "ontologies") attached to the project.
    pub data_models: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_ref_construction_and_equality() {
        let a = ProjectRef {
            iri: "http://rdfh.ch/projects/0001".into(),
            shortcode: "0001".into(),
            shortname: "anything".into(),
        };
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(a.iri, "http://rdfh.ch/projects/0001");
        assert_eq!(a.shortcode, "0001");
        assert_eq!(a.shortname, "anything");
    }

    #[test]
    fn project_status_as_str() {
        assert_eq!(ProjectStatus::Active.as_str(), "active");
        assert_eq!(ProjectStatus::Inactive.as_str(), "inactive");
    }

    #[test]
    fn project_status_derives() {
        // Clone, Copy, PartialEq, Eq, Debug
        let s = ProjectStatus::Active;
        let t = s; // Copy
        assert_eq!(s, t);
        assert_ne!(ProjectStatus::Active, ProjectStatus::Inactive);
        let _ = format!("{s:?}"); // Debug
    }

    #[test]
    fn project_construction_and_equality() {
        let p = Project {
            iri: "http://rdfh.ch/projects/0001".into(),
            shortcode: "0001".into(),
            shortname: "anything".into(),
            longname: Some("Anything Project".into()),
            status: ProjectStatus::Active,
            data_models: 3,
        };
        let cloned = p.clone();
        assert_eq!(p, cloned);
        assert_eq!(p.iri, "http://rdfh.ch/projects/0001");
        assert_eq!(p.shortcode, "0001");
        assert_eq!(p.shortname, "anything");
        assert_eq!(p.longname.as_deref(), Some("Anything Project"));
        assert_eq!(p.status, ProjectStatus::Active);
        assert_eq!(p.data_models, 3);
    }

    #[test]
    fn project_longname_none() {
        let p = Project {
            iri: "http://rdfh.ch/projects/0002".into(),
            shortcode: "0002".into(),
            shortname: "images".into(),
            longname: None,
            status: ProjectStatus::Inactive,
            data_models: 0,
        };
        assert_eq!(p.longname, None);
        assert_eq!(p.status, ProjectStatus::Inactive);
        assert_eq!(p.data_models, 0);
    }

    #[test]
    fn project_description_with_language() {
        let d = ProjectDescription {
            value: "<b>Project Metadata</b>".into(),
            language: Some("en".into()),
        };
        let cloned = d.clone();
        assert_eq!(d, cloned);
        assert_eq!(d.value, "<b>Project Metadata</b>");
        assert_eq!(d.language.as_deref(), Some("en"));
    }

    #[test]
    fn project_description_without_language() {
        let d = ProjectDescription { value: "Beschreibung".into(), language: None };
        let cloned = d.clone();
        assert_eq!(d, cloned);
        assert_eq!(d.language, None);
    }

    #[test]
    fn data_model_summary_construction_and_equality() {
        let dm = DataModelSummary {
            name: "beol".into(),
            iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
        };
        let cloned = dm.clone();
        assert_eq!(dm, cloned);
        assert_eq!(dm.name, "beol");
        assert_eq!(dm.iri, "http://api.dasch.swiss/ontology/0801/beol/v2");
    }

    #[test]
    fn project_detail_construction_and_equality() {
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF".into(),
            shortcode: "0801".into(),
            shortname: "beol".into(),
            longname: Some("Bernoulli-Euler Online".into()),
            status: ProjectStatus::Active,
            description: vec![ProjectDescription {
                value: "<b>Project Metadata</b>".into(),
                language: Some("en".into()),
            }],
            keywords: vec!["Bernoulli".into(), "Condorcet".into(), "Euler".into()],
            data_models: vec![
                DataModelSummary {
                    name: "beol".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/beol/v2".into(),
                },
                DataModelSummary {
                    name: "biblio".into(),
                    iri: "http://api.dasch.swiss/ontology/0801/biblio/v2".into(),
                },
            ],
        };
        let cloned = detail.clone();
        assert_eq!(detail, cloned);
        assert_eq!(detail.iri, "http://rdfh.ch/projects/yTerZGyxjZVqFMNNKXCDPF");
        assert_eq!(detail.shortcode, "0801");
        assert_eq!(detail.shortname, "beol");
        assert_eq!(detail.longname.as_deref(), Some("Bernoulli-Euler Online"));
        assert_eq!(detail.status, ProjectStatus::Active);
        assert_eq!(detail.description.len(), 1);
        assert_eq!(detail.keywords.len(), 3);
        assert_eq!(detail.data_models.len(), 2);
    }

    #[test]
    fn project_detail_empty_case() {
        let detail = ProjectDetail {
            iri: "http://rdfh.ch/projects/0000".into(),
            shortcode: "0000".into(),
            shortname: "minimal".into(),
            longname: None,
            status: ProjectStatus::Inactive,
            description: vec![],
            keywords: vec![],
            data_models: vec![],
        };
        assert_eq!(detail.longname, None);
        assert!(detail.description.is_empty());
        assert!(detail.keywords.is_empty());
        assert!(detail.data_models.is_empty());
        assert_eq!(detail.status, ProjectStatus::Inactive);
    }
}
