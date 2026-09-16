//! JSON hygiene shared by the draft representation and the canonical writer.

use serde_json::Value;

/// Removes `null` object members, recursively.
///
/// Object members only, never array elements: `authorship`, `contributorType`
/// and `typeOfData` are lists whose length is data. `ProjectRaw` carries no
/// `skip_serializing_if` because `dpe-server` serializes it through
/// `axum::Json`, so the attribute would drop nulls from DPE's API too. `retain`
/// rather than `remove`: under `preserve_order`, `Map::remove` is swap-remove
/// and reorders the surviving keys.
pub fn strip_null_members(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, member| !member.is_null());
            for (_, member) in map.iter_mut() {
                strip_null_members(member);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(strip_null_members),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn strips_null_members_at_every_depth() {
        let mut value = json!({
            "a": null,
            "b": {"c": null, "d": 1},
            "e": [{"f": null, "g": 2}],
        });
        strip_null_members(&mut value);
        assert_eq!(value, json!({"b": {"d": 1}, "e": [{"g": 2}]}));
    }

    /// A null array element is data, not an absent field: removing one would
    /// renumber every element after it.
    #[test]
    fn keeps_null_array_elements() {
        let mut value = json!({"authorship": ["a", null, "b"]});
        strip_null_members(&mut value);
        assert_eq!(value, json!({"authorship": ["a", null, "b"]}));
    }

    #[test]
    fn preserves_the_order_of_surviving_members() {
        let mut value = json!({"id": 1, "gone": null, "pid": 2, "name": 3});
        strip_null_members(&mut value);
        let keys: Vec<&String> = value.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["id", "pid", "name"]);
    }
}
