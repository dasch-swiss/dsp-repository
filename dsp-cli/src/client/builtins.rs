//! Platform built-in data-models.
//!
//! The fixed set of DSP-API built-in data-models that every project inherits.
//! Their IRIs use the stable `api.knora.org` namespace (deployment-independent),
//! so hardcoding is safe.
//!
//! DSP-API knowledge: knowing *which* ontologies are platform builtins belongs
//! here (inside the client boundary, per ADR-0001). The **action** owns the
//! `--include-builtins` policy: `list_data_models` is a pure single-endpoint
//! fetch of the project's own ontologies and takes no `include_builtins`
//! parameter. The action calls `builtin_data_models()` and extends the result
//! when the flag is set.

use crate::model::{DataModel, ResourceType, ValueType};

/// The fixed set of platform built-in data-models, surfaced by
/// `data-model list --include-builtins`. These belong to no project and are not
/// returned by the project-scoped endpoint; their IRIs use the stable
/// `api.knora.org` namespace (deployment-independent). DSP-API knowledge, kept
/// inside the client boundary (ADR-0001). The action — not the client I/O
/// method — decides whether to include them.
pub(crate) fn builtin_data_models() -> Vec<DataModel> {
    ["knora-api", "standoff", "salsah-gui"]
        .into_iter()
        .map(|name| DataModel {
            name: name.to_string(),
            iri: format!("http://api.knora.org/ontology/{name}/v2"),
            label: None,
            last_modified: None,
            is_builtin: true,
        })
        .collect()
}

/// The fixed set of user-instantiable platform built-in resource-types, surfaced
/// by `resource-type list --include-builtins`. These are the four DSP base resources
/// a user can instantiate without defining them in a data-model. Their IRIs use the
/// stable `api.knora.org` namespace (deployment-independent). DSP-API knowledge,
/// kept inside the client boundary (ADR-0001).
///
/// The deprecated `knora-base:Annotation` class is intentionally excluded — it is
/// absent from dsp-tools' modern base resources, and what DaSCH domain language calls
/// "annotation" is realised by Region / AudioSegment / VideoSegment. The action —
/// not the client I/O method — decides whether to include built-ins.
pub(crate) fn builtin_resource_types() -> Vec<ResourceType> {
    [
        ("Region", "Region"),
        ("AudioSegment", "Audio Annotation"),
        ("VideoSegment", "Video Annotation"),
        ("LinkObj", "Link Object"),
    ]
    .into_iter()
    .map(|(name, label)| ResourceType {
        name: name.to_string(),
        iri: format!("http://api.knora.org/ontology/knora-api/v2#{name}"),
        label: Some(label.to_string()),
        is_builtin: true,
        count: None,
    })
    .collect()
}

/// Map the 6 `knora-api` file-value property local names to [`ValueType`].
///
/// Used for built-in file-value fields whose property node is absent from the
/// response (system properties are not chased — R3). Returns `None` for any
/// local name not in the 6-entry file-value map.
///
/// DSP-API knowledge, kept inside the client boundary (ADR-0001).
pub(crate) fn builtin_field_value_type(local: &str) -> Option<ValueType> {
    match local {
        "hasStillImageFileValue" => Some(ValueType::StillImage),
        "hasMovingImageFileValue" => Some(ValueType::MovingImage),
        "hasAudioFileValue" => Some(ValueType::Audio),
        "hasDocumentFileValue" => Some(ValueType::Document),
        "hasArchiveFileValue" => Some(ValueType::Archive),
        "hasTextFileValue" => Some(ValueType::Other("text-file".to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::http::data_model_name_from_iri;

    #[test]
    fn builtin_data_models_returns_three_entries() {
        let builtins = builtin_data_models();
        assert_eq!(builtins.len(), 3, "expected exactly 3 built-in data-models");
    }

    #[test]
    fn all_builtins_have_is_builtin_true() {
        for dm in builtin_data_models() {
            assert!(dm.is_builtin, "built-in data-model '{}' must have is_builtin == true", dm.name);
        }
    }

    #[test]
    fn all_builtins_have_no_label_or_last_modified() {
        for dm in builtin_data_models() {
            assert!(dm.label.is_none(), "built-in data-model '{}' must have label == None", dm.name);
            assert!(
                dm.last_modified.is_none(),
                "built-in data-model '{}' must have last_modified == None",
                dm.name
            );
        }
    }

    /// No-drift guard: `data_model_name_from_iri(&dm.iri)` must equal `dm.name`
    /// for every built-in, ensuring the literal name and the IRI stay in sync.
    #[test]
    fn builtin_name_matches_iri_derived_name() {
        for dm in builtin_data_models() {
            let derived = data_model_name_from_iri(&dm.iri);
            assert_eq!(
                derived, dm.name,
                "built-in data-model IRI '{}' derives name '{}', but stored name is '{}' — IRI and name have drifted",
                dm.iri, derived, dm.name
            );
        }
    }

    #[test]
    fn builtin_names_are_expected_set() {
        let builtins = builtin_data_models();
        let names: Vec<&str> = builtins.iter().map(|dm| dm.name.as_str()).collect();
        assert!(names.contains(&"knora-api"), "knora-api must be a built-in");
        assert!(names.contains(&"standoff"), "standoff must be a built-in");
        assert!(names.contains(&"salsah-gui"), "salsah-gui must be a built-in");
    }

    // ── builtin_resource_types tests ──────────────────────────────────────────

    #[test]
    fn builtin_resource_types_returns_four_entries() {
        let builtins = builtin_resource_types();
        assert_eq!(builtins.len(), 4, "expected exactly 4 built-in resource-types");
    }

    #[test]
    fn all_builtin_resource_types_have_is_builtin_true() {
        for rt in builtin_resource_types() {
            assert!(
                rt.is_builtin,
                "built-in resource-type '{}' must have is_builtin == true",
                rt.name
            );
        }
    }

    #[test]
    fn builtin_resource_type_names_are_expected_set() {
        let builtins = builtin_resource_types();
        let names: Vec<&str> = builtins.iter().map(|rt| rt.name.as_str()).collect();
        assert!(names.contains(&"Region"), "Region must be a built-in");
        assert!(names.contains(&"AudioSegment"), "AudioSegment must be a built-in");
        assert!(names.contains(&"VideoSegment"), "VideoSegment must be a built-in");
        assert!(names.contains(&"LinkObj"), "LinkObj must be a built-in");
    }

    /// Explicit negative assertion: the deprecated Annotation class must NOT be
    /// included (see Locked decision #2 in implementation-plan.md).
    #[test]
    fn builtin_resource_types_excludes_annotation() {
        let builtins = builtin_resource_types();
        let names: Vec<&str> = builtins.iter().map(|rt| rt.name.as_str()).collect();
        assert!(
            !names.contains(&"Annotation"),
            "Annotation must NOT be included — it is deprecated in modern DaSCH tooling"
        );
    }

    // ── builtin_field_value_type tests ───────────────────────────────────────

    /// `hasTextFileValue` must map to `Other("text-file")`, matching the
    /// `map_object_type_to_value_type("TextFileValue")` path in http.rs.
    #[test]
    fn has_text_file_value_maps_to_other_text_file() {
        assert_eq!(
            builtin_field_value_type("hasTextFileValue"),
            Some(ValueType::Other("text-file".to_string())),
            "hasTextFileValue must map to Other(\"text-file\") to align with the resolved-node path"
        );
    }

    /// Spot-check the other file-value props to guard against regressions.
    #[test]
    fn builtin_field_value_type_spot_checks() {
        assert_eq!(builtin_field_value_type("hasStillImageFileValue"), Some(ValueType::StillImage));
        assert_eq!(builtin_field_value_type("hasAudioFileValue"), Some(ValueType::Audio));
        assert_eq!(builtin_field_value_type("unknown"), None);
    }

    /// No-drift guard: the hardcoded `name` must match the IRI fragment
    /// (`rsplit(['#', '/', ':'])`) for every built-in resource-type.
    ///
    /// NOTE: do NOT use `data_model_name_from_iri` here — that helper is
    /// path-based (strips trailing `/v2` and returns the last `/`-segment), which
    /// returns `v2` for a `#fragment` IRI like `…/knora-api/v2#Region`. Use the
    /// fragment-aware rsplit shown below (same extraction the client uses in
    /// `describe_data_model`).
    #[test]
    fn builtin_resource_type_name_matches_iri_fragment() {
        for rt in builtin_resource_types() {
            let derived = rt.iri.rsplit(['#', '/', ':']).next().unwrap_or("");
            assert_eq!(
                derived, rt.name,
                "built-in resource-type IRI '{}' derives fragment '{}', but stored name is '{}' — IRI and name have drifted",
                rt.iri, derived, rt.name
            );
        }
    }
}
