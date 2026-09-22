//! Deconflicting proposed entity ids against what a target checkout already
//! holds, and rewriting every reference to a renumbered id.
//!
//! Entity ids appear as whole string values at `attributions[].contributor`,
//! `funding[].funders[]` and `contactPoint[]` in projects and
//! `affiliations[]` in persons. [`rewrite`] walks the whole JSON value rather
//! than targeting those four paths by name, so it cannot miss a site that a
//! new field introduces.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use editor_core::collection::ProposedEntityView;
use editor_core::proposals::{next_entity_id, ProposalKind, ProposalOperation};
use serde_json::Value;

/// Every id collision a batch of proposed entities would create against
/// `taken`, and the id each collision is renumbered to.
///
/// Only a [`ProposalOperation::New`] entity is eligible: a
/// [`ProposalOperation::Change`] proposal names an id `taken` is expected to
/// already hold, and renumbering it would point the change at the wrong
/// entity. Every id — allocated or left alone — is folded into a running
/// working copy of `taken` before the next entity is considered, so two
/// entities in the same batch that both collide never receive the same new
/// id.
#[must_use]
pub fn plan(entities: &[ProposedEntityView], taken: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut running: BTreeSet<String> = taken.clone();
    let mut renumbered = BTreeMap::new();

    for entity in entities {
        if matches!(entity.operation.parse(), Ok(ProposalOperation::New)) {
            if let Ok(kind) = ProposalKind::from_str(&entity.kind) {
                if running.contains(&entity.id) {
                    let new_id = next_entity_id(kind, running.iter().map(String::as_str));
                    running.insert(new_id.clone());
                    renumbered.insert(entity.id.clone(), new_id);
                    continue;
                }
            }
        }
        running.insert(entity.id.clone());
    }

    renumbered
}

/// Rewrites every JSON string value that is exactly a key of `mapping` into
/// its value, walking objects, arrays and `value` itself.
///
/// The match is exact, never a substring, and object *keys* are never
/// candidates — only the string values a walk reaches.
pub fn rewrite(value: &mut Value, mapping: &BTreeMap<String, String>) {
    match value {
        Value::String(s) => {
            if let Some(replacement) = mapping.get(s.as_str()) {
                *s = replacement.clone();
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrite(item, mapping);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                rewrite(item, mapping);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(kind: &str, operation: &str, id: &str) -> ProposedEntityView {
        ProposedEntityView {
            kind: kind.to_string(),
            operation: operation.to_string(),
            id: id.to_string(),
            body: Value::Null,
        }
    }

    #[test]
    fn a_new_entity_whose_id_is_free_keeps_it() {
        let entities = vec![entity("person", "new", "person-010")];
        let mapping = plan(&entities, &BTreeSet::new());
        assert!(mapping.is_empty(), "{mapping:?}");
    }

    #[test]
    fn a_new_entity_whose_id_is_taken_is_renumbered_to_a_free_id() {
        let entities = vec![entity("person", "new", "person-001")];
        let taken: BTreeSet<String> = ["person-001", "person-002"].into_iter().map(String::from).collect();
        let mapping = plan(&entities, &taken);
        let new_id = mapping.get("person-001").expect("renumbered");
        assert!(!taken.contains(new_id), "{new_id}");
    }

    #[test]
    fn two_colliding_new_entities_of_the_same_kind_get_two_different_new_ids() {
        // If `plan` allocated both against the original `taken` instead of a
        // running copy, both would land on `person-003`.
        let entities = vec![
            entity("person", "new", "person-001"),
            entity("person", "new", "person-002"),
        ];
        let taken: BTreeSet<String> = ["person-001", "person-002"].into_iter().map(String::from).collect();
        let mapping = plan(&entities, &taken);
        let first = mapping.get("person-001").expect("first renumbered");
        let second = mapping.get("person-002").expect("second renumbered");
        assert_ne!(first, second, "{mapping:?}");
    }

    #[test]
    fn a_change_entity_is_never_renumbered_even_when_its_id_is_taken() {
        let entities = vec![entity("person", "change", "person-001")];
        let taken: BTreeSet<String> = ["person-001".to_string()].into_iter().collect();
        let mapping = plan(&entities, &taken);
        assert!(mapping.is_empty(), "{mapping:?}");
    }

    #[test]
    fn rewrite_replaces_a_matching_id_nested_in_arrays_and_objects() {
        let mut value = serde_json::json!({
            "attributions": [{ "contributor": "person-001" }],
            "funding": [{ "funders": ["person-001", "organization-002"] }],
        });
        let mapping: BTreeMap<String, String> =
            [("person-001".to_string(), "person-050".to_string())].into_iter().collect();
        rewrite(&mut value, &mapping);
        assert_eq!(value["attributions"][0]["contributor"], "person-050");
        assert_eq!(value["funding"][0]["funders"][0], "person-050");
        assert_eq!(value["funding"][0]["funders"][1], "organization-002");
    }

    #[test]
    fn rewrite_never_substring_matches_or_touches_object_keys() {
        let mut value = serde_json::json!({
            "note": "see person-417 for details",
            "person-417": "person-417",
        });
        let mapping: BTreeMap<String, String> =
            [("person-417".to_string(), "person-500".to_string())].into_iter().collect();
        rewrite(&mut value, &mapping);
        assert_eq!(value["note"], "see person-417 for details");
        assert!(value.get("person-417").is_some(), "the key must survive untouched");
        assert_eq!(value["person-417"], "person-500");
    }
}
