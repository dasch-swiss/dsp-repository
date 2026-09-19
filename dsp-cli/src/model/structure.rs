//! Data-model structure domain shape — shared across the client → action boundary.
//!
//! Types here are the dsp-cli vocabulary for data-model structure data,
//! surfaced by `dsp vre data-model structure`. DSP-API wire types live inside
//! `src/client/http.rs` and are never exposed above the client layer. See
//! ADR-0001 and ADR-0008.
//!
//! Key CONTEXT.md vocabulary: structure, relation. Wire deserialization
//! (DSP-API `rdfs:subClassOf`, `owl:Restriction`, `knora-api:objectType`,
//! `knora-api:isLinkProperty`, CURIE prefix lookups, etc.) stays inside
//! `src/client/http.rs`. No serde derive: wire deserialization stays in
//! `src/client/http.rs`.

use std::fmt;

/// The structure of a data-model — the set of relations between its resource-types.
///
/// A describe-shaped DETAIL struct (like `DataModelDetail`), passed directly to
/// the renderer. Carries the baseline data-model name (for prose headers and
/// cross-data-model tagging) and the full sorted relation list. No `serde` derive:
/// wire deserialization stays in `src/client/http.rs`. See ADR-0001 / ADR-0008
/// and the CONTEXT.md "Structure" / "Relation" entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataModelStructure {
    /// Baseline data-model name (e.g. `beol`). Used as the prose header and as
    /// the baseline when the renderer tags cross-data-model targets (`[to <dm>]`):
    /// `target_data_model == Some(x) && x != data_model` → emit `[to x]`.
    pub data_model: String,
    /// All relations of the data-model, sorted per D6 by `(source, kind, field, target)`.
    /// The client sorts; the action filters by `is_builtin` unless `--include-builtins`.
    pub relations: Vec<Relation>,
}

/// A directed edge between two resource-types in (or out of) a data-model.
///
/// Two kinds: a **link relation** (a link field on the source resource-type
/// points to the target; the field name labels the relation — `field` is `Some`)
/// and an **inheritance relation** (the source extends the target as a superclass
/// — `field` is `None`). Carries a `target_data_model` tag for cross-data-model
/// targets (mirroring `Field.data_model` in the target direction) and an
/// `is_builtin` flag whose meaning is asymmetric by kind (see below).
///
/// **Invariants** (asserted in unit tests; not enforced structurally — same
/// precedent as `Field.link_target` in `resource_type.rs`):
/// - (a) `field.is_some()` iff `kind == RelationKind::Link`.
/// - (b) `target_data_model == None` whenever the target is in a system namespace
///   (`knora-api`, `knora-base`, etc.).
///
/// No `serde` derive: wire deserialization stays in `src/client/http.rs`. See
/// ADR-0001 / ADR-0008 and the CONTEXT.md "Relation" entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    /// Source resource-type local name (IRI fragment, e.g. `letter`).
    pub source: String,
    /// Target resource-type or superclass local name (IRI fragment, e.g. `person`
    /// or `writtenSource`).
    pub target: String,
    /// Whether this is a link relation or an inheritance relation.
    pub kind: RelationKind,
    /// The link-field local name that labels this relation (e.g. `hasSender`).
    /// `Some` iff `kind == RelationKind::Link`; `None` for inheritance relations.
    /// This invariant is asserted in unit tests — the type does not enforce it
    /// structurally.
    pub field: Option<String>,
    /// The target resource-type's data-model name (the CURIE prefix of the
    /// target's `@id`). `None` when the target's prefix is a system namespace
    /// (`knora-api`, `knora-base`, etc.); `Some(prefix)` otherwise. The renderer
    /// emits `[to <dm>]` iff `Some(x)` and `x != DataModelStructure.data_model`.
    /// Mirrors `Field.data_model` (which tags the source direction for
    /// cross-data-model fields). This invariant is asserted in unit tests.
    pub target_data_model: Option<String>,
    /// Builtin flag — semantics are **asymmetric** by kind:
    /// - `Link` → `true` iff the link **field's** CURIE prefix is a system namespace.
    ///   A project-defined link field pointing to a built-in target is `false`
    ///   (shown by default), because the *field* is project-defined.
    /// - `Inherits` → `true` iff the **superclass** (target) CURIE prefix is a
    ///   system namespace (e.g. `Resource`, `StillImageRepresentation`).
    ///
    /// The action filters `!r.is_builtin` by default; `--include-builtins` shows all.
    pub is_builtin: bool,
}

/// The kind of a relation — link or inheritance.
///
/// `Display` writes `"link"` / `"inherits"`. Derives `Ord` + `PartialOrd` with
/// **`Link` declared before `Inherits`**, so derived `Ord` puts link relations
/// before inherits relations for the same source under D6's tuple sort
/// `(source, kind, field, target)`. `Copy` / `Eq` cover invariant assertions.
/// No `serde` derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationKind {
    /// A link field on the source resource-type points to the target resource-type.
    /// The field name labels the relation. Display: `"link"`.
    ///
    /// **Variant order is load-bearing**: `Link` is declared before `Inherits` so
    /// that derived `Ord` gives `Link < Inherits`, making link relations sort before
    /// inherits relations for the same source (D6).
    Link,
    /// The source resource-type extends the target as a superclass. Display: `"inherits"`.
    Inherits,
}

impl fmt::Display for RelationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationKind::Link => f.write_str("link"),
            RelationKind::Inherits => f.write_str("inherits"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Construction, equality, clone round-trip ---

    #[test]
    fn data_model_structure_full_construction_and_equality() {
        let structure = DataModelStructure {
            data_model: "beol".into(),
            relations: vec![
                Relation {
                    source: "letter".into(),
                    target: "Book".into(),
                    kind: RelationKind::Link,
                    field: Some("cites".into()),
                    target_data_model: Some("biblio".into()),
                    is_builtin: false,
                },
                Relation {
                    source: "letter".into(),
                    target: "writtenSource".into(),
                    kind: RelationKind::Inherits,
                    field: None,
                    target_data_model: None,
                    is_builtin: false,
                },
            ],
        };
        let cloned = structure.clone();
        assert_eq!(structure, cloned);
        assert_eq!(structure.data_model, "beol");
        assert_eq!(structure.relations.len(), 2);
    }

    #[test]
    fn data_model_structure_empty_relations() {
        let structure = DataModelStructure {
            data_model: "minimal".into(),
            relations: vec![],
        };
        let cloned = structure.clone();
        assert_eq!(structure, cloned);
        assert_eq!(structure.data_model, "minimal");
        assert!(structure.relations.is_empty());
    }

    #[test]
    fn relation_link_full_construction_and_equality() {
        // A link relation with a field label and a cross-data-model target_data_model.
        let rel = Relation {
            source: "letter".into(),
            target: "Book".into(),
            kind: RelationKind::Link,
            field: Some("cites".into()),
            target_data_model: Some("biblio".into()),
            is_builtin: false,
        };
        let cloned = rel.clone();
        assert_eq!(rel, cloned);
        assert_eq!(rel.source, "letter");
        assert_eq!(rel.target, "Book");
        assert_eq!(rel.kind, RelationKind::Link);
        assert_eq!(rel.field.as_deref(), Some("cites"));
        assert_eq!(rel.target_data_model.as_deref(), Some("biblio"));
        assert!(!rel.is_builtin);
    }

    #[test]
    fn relation_inherits_construction_and_equality() {
        // An inherits relation: field is None, target_data_model is None (in-model).
        let rel = Relation {
            source: "letter".into(),
            target: "writtenSource".into(),
            kind: RelationKind::Inherits,
            field: None,
            target_data_model: None,
            is_builtin: false,
        };
        let cloned = rel.clone();
        assert_eq!(rel, cloned);
        assert_eq!(rel.source, "letter");
        assert_eq!(rel.target, "writtenSource");
        assert_eq!(rel.kind, RelationKind::Inherits);
        assert!(rel.field.is_none());
        assert!(rel.target_data_model.is_none());
        assert!(!rel.is_builtin);
    }

    // --- RelationKind Display ---

    #[test]
    fn relation_kind_display_link() {
        assert_eq!(RelationKind::Link.to_string(), "link");
    }

    #[test]
    fn relation_kind_display_inherits() {
        assert_eq!(RelationKind::Inherits.to_string(), "inherits");
    }

    // --- RelationKind ordering: Link < Inherits (D6 load-bearing) ---

    #[test]
    fn relation_kind_ordering_link_before_inherits() {
        assert!(
            RelationKind::Link < RelationKind::Inherits,
            "Link must sort before Inherits (D6: link before inherits for the same source)"
        );
    }

    #[test]
    fn relation_kind_ordering_inherits_not_less_than_link() {
        assert!(RelationKind::Inherits >= RelationKind::Link);
    }

    // --- Invariant (a): field.is_some() iff kind == Link, both directions ---

    #[test]
    fn link_relation_has_field_some() {
        // A Link relation MUST carry field == Some(...).
        let rel = Relation {
            source: "letter".into(),
            target: "person".into(),
            kind: RelationKind::Link,
            field: Some("hasSender".into()),
            target_data_model: None,
            is_builtin: false,
        };
        assert_eq!(rel.kind, RelationKind::Link);
        assert!(
            rel.field.is_some(),
            "a Link relation must have field == Some(...)"
        );
    }

    #[test]
    fn non_link_relation_has_field_none() {
        // An Inherits relation MUST carry field == None.
        let rel = Relation {
            source: "letter".into(),
            target: "writtenSource".into(),
            kind: RelationKind::Inherits,
            field: None,
            target_data_model: None,
            is_builtin: false,
        };
        assert_ne!(rel.kind, RelationKind::Link);
        assert!(
            rel.field.is_none(),
            "a non-Link relation must have field == None"
        );
    }

    // --- Asymmetric is_builtin cases ---

    #[test]
    fn project_link_field_to_builtin_target_is_not_builtin() {
        // A project-defined link field pointing to a built-in target is is_builtin=false
        // (shown by default), because is_builtin is keyed off the FIELD's prefix for
        // link relations, not the target's prefix. target_data_model is None because the
        // target's prefix is system (invariant b).
        let rel = Relation {
            source: "letter".into(),
            target: "Resource".into(),
            kind: RelationKind::Link,
            field: Some("hasRelation".into()),
            target_data_model: None, // system target → None (invariant b)
            is_builtin: false,       // project field → not builtin
        };
        assert_eq!(rel.kind, RelationKind::Link);
        assert!(
            !rel.is_builtin,
            "project link field to built-in target is is_builtin=false"
        );
        assert!(
            rel.target_data_model.is_none(),
            "system target → target_data_model must be None"
        );
    }

    #[test]
    fn inherits_relation_to_system_super_is_builtin() {
        // An inherits relation to a system superclass (e.g. Resource) is is_builtin=true
        // because is_builtin is keyed off the TARGET's prefix for inheritance relations.
        // target_data_model is None because the target's prefix is system (invariant b).
        let rel = Relation {
            source: "letter".into(),
            target: "Resource".into(),
            kind: RelationKind::Inherits,
            field: None,
            target_data_model: None, // system target → None (invariant b)
            is_builtin: true,        // system superclass → builtin
        };
        assert_eq!(rel.kind, RelationKind::Inherits);
        assert!(
            rel.is_builtin,
            "inherits relation to system superclass is is_builtin=true"
        );
        assert!(
            rel.target_data_model.is_none(),
            "system target → target_data_model must be None"
        );
    }

    // --- Invariant (b): Some-direction — project/sibling target → target_data_model == Some ---

    #[test]
    fn non_system_target_has_target_data_model_some() {
        // Invariant (b) Some-direction: a relation whose target is in a project or sibling
        // namespace (non-system) MUST carry target_data_model == Some(prefix).
        // This complements the existing None-direction tests (system targets above) and
        // documents both directions of the invariant at the model level.

        // In-model link: target is in the same DM ("beol") → Some("beol").
        let in_model_link = Relation {
            source: "letter".into(),
            target: "person".into(),
            kind: RelationKind::Link,
            field: Some("hasSender".into()),
            target_data_model: Some("beol".into()), // project target → Some (invariant b)
            is_builtin: false,
        };
        assert!(
            in_model_link.target_data_model.is_some(),
            "in-model link target must have target_data_model == Some(...)"
        );
        assert_eq!(
            in_model_link.target_data_model.as_deref(),
            Some("beol"),
            "in-model target_data_model must equal the baseline DM name"
        );

        // Cross-model link: target is in a sibling DM ("biblio") → Some("biblio").
        let cross_model_link = Relation {
            source: "letter".into(),
            target: "Book".into(),
            kind: RelationKind::Link,
            field: Some("cites".into()),
            target_data_model: Some("biblio".into()), // sibling DM target → Some (invariant b)
            is_builtin: false,
        };
        assert!(
            cross_model_link.target_data_model.is_some(),
            "cross-model link target must have target_data_model == Some(...)"
        );
        assert_eq!(
            cross_model_link.target_data_model.as_deref(),
            Some("biblio"),
            "cross-model target_data_model must equal the sibling DM name"
        );
        // Crucially: it is NOT None.
        assert_ne!(
            cross_model_link.target_data_model, None,
            "non-system target must NOT have target_data_model == None"
        );
    }
}
