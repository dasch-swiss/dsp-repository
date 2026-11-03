//! Format rules: whether a stored value has the shape its kind of value needs.
//!
//! A finding never stops a value being stored. The appliers keep what was typed, a
//! finding is computed from the draft whenever the form renders, and only submit
//! refuses one ([`crate::submission::check`]). Nothing here is carried by a write,
//! so every rendering of one draft shows the same findings.

use std::collections::HashSet;

use serde_json::Value;
use shared_metadata::is_placeholder;

use super::registry::Field;
use super::{stored_row_key, FormBody, Shape};
use crate::draft::ProjectDraft;

/// A kind of value with a format to check. What a failing value is told is the
/// caller's wording, keyed by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// A web address.
    Link,
    /// A DOI, bare or as a resolver link.
    Doi,
}

impl ValueKind {
    /// Whether `value` has this kind's format.
    #[must_use]
    pub fn accepts(self, _value: &str) -> bool {
        match self {
            Self::Link | Self::Doi => true,
        }
    }
}

/// Where a value with a format sits in one field.
#[derive(Debug, Clone, Copy)]
pub struct Member {
    /// The name the row's control posts under after `{field}.{key}.`, or `""`
    /// where the row is the value. Unused for a field without rows, whose control
    /// posts under the field id.
    pub posted: &'static str,
    /// The path to the value inside a stored row, or inside the field's own value
    /// for a field without rows; empty where that is the value.
    pub stored: &'static [&'static str],
    pub kind: ValueKind,
}

/// The members of `field` that carry a format: the registry hook the walk reads.
/// Each member must name a control its field's widget posts and marks.
#[must_use]
pub fn members(_field: &Field) -> &'static [Member] {
    &[]
}

/// One value without the format its kind needs.
#[derive(Debug, Clone)]
pub struct Finding {
    pub field: &'static Field,
    /// The name of the control showing the value, which is where the finding is
    /// rendered.
    pub control: String,
    pub kind: ValueKind,
}

/// Every finding in `fields`, from the values `draft` holds.
///
/// A row's control is named by its row key, so the key decides where a finding
/// renders. A render from `posted` names its rows by the posted keys, and a blank
/// row there is never stored, so a stored position is no key: each posted row is
/// matched to the stored values by its own value. Without a posted body the rows
/// are named by [`stored_row_key`], as the form names them.
#[must_use]
pub fn findings(
    fields: impl Iterator<Item = &'static Field>,
    draft: &ProjectDraft,
    posted: Option<&FormBody>,
) -> Vec<Finding> {
    findings_by(fields, draft, posted, members, ValueKind::accepts)
}

/// The name of the control a row's `member` posts under, or, for a field without
/// rows, the field id.
#[must_use]
pub fn control_name(field: &Field, key: &str, member: &Member) -> String {
    match (field.shape.is_some_and(Shape::has_rows), member.posted) {
        (false, _) => field.id.to_string(),
        (true, "") => format!("{}.{key}", field.id),
        (true, posted) => format!("{}.{key}.{posted}", field.id),
    }
}

fn findings_by(
    fields: impl Iterator<Item = &'static Field>,
    draft: &ProjectDraft,
    posted: Option<&FormBody>,
    members: impl Fn(&Field) -> &'static [Member],
    accepts: impl Fn(ValueKind, &str) -> bool,
) -> Vec<Finding> {
    let mut found = Vec::new();
    for field in fields {
        let Some(shape) = field.shape else { continue };
        for member in members(field) {
            let fails = |value: &str| checkable(value).is_some_and(|value| !accepts(member.kind, value));
            let controls = if shape.has_rows() {
                row_controls(field, member, draft, posted, fails)
            } else {
                let value = match shape {
                    Shape::Url(slot) => draft.url_slot(slot),
                    _ => draft.get(field.id).and_then(|value| at(value, member.stored)),
                };
                value
                    .filter(|value| fails(value))
                    .map(|_| control_name(field, "", member))
                    .into_iter()
                    .collect()
            };
            found.extend(
                controls
                    .into_iter()
                    .map(|control| Finding { field, control, kind: member.kind }),
            );
        }
    }
    found
}

/// The controls of `field`'s rows whose `member` fails.
fn row_controls(
    field: &Field,
    member: &Member,
    draft: &ProjectDraft,
    posted: Option<&FormBody>,
    fails: impl Fn(&str) -> bool,
) -> Vec<String> {
    let rows = draft
        .get(field.id)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let Some(body) = posted.filter(|body| body.has(&format!("{}.row", field.id))) else {
        return rows
            .iter()
            .enumerate()
            .filter(|(_, row)| at(row, member.stored).is_some_and(&fails))
            .map(|(position, _)| control_name(field, &stored_row_key(position), member))
            .collect();
    };
    let failing: HashSet<&str> = rows
        .iter()
        .filter_map(|row| at(row, member.stored))
        .filter(|value| fails(value))
        .map(str::trim)
        .collect();
    if failing.is_empty() {
        return Vec::new();
    }
    let typed = body.first_values(&format!("{}.", field.id));
    body.rows(field.id)
        .into_iter()
        .map(|key| control_name(field, key, member))
        .filter(|control| typed.get(control.as_str()).is_some_and(|value| failing.contains(value.trim())))
        .collect()
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(value, |value, member| value.get(member))?.as_str()
}

/// A value worth checking: an empty one has nothing to check, and a placeholder
/// sentinel is the platform's own "no value yet", refused at submit by a gate of its
/// own when typed.
fn checkable(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty() && !is_placeholder(value)).then_some(value)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::form::registry::field;

    const REFERENCE_URL: &[Member] = &[Member { posted: "ref.url", stored: &["url"], kind: ValueKind::Link }];
    const WHOLE_VALUE: &[Member] = &[Member { posted: "", stored: &[], kind: ValueKind::Link }];

    /// A stand-in rule, since the real ones are not this module's subject: a value
    /// is a link when it contains `://`.
    fn rule(_kind: ValueKind, value: &str) -> bool {
        value.contains("://")
    }

    fn walk(
        id: &'static str,
        members: &'static [Member],
        draft: &ProjectDraft,
        posted: Option<&FormBody>,
    ) -> Vec<String> {
        let field = field(id).expect("a known field");
        findings_by(std::iter::once(field), draft, posted, |_| members, rule)
            .into_iter()
            .map(|finding| {
                assert_eq!(finding.field.id, id);
                assert_eq!(finding.kind, members[0].kind);
                finding.control
            })
            .collect()
    }

    fn spatial(rows: Value) -> ProjectDraft {
        let mut draft = ProjectDraft::default();
        draft.set("spatialCoverage", rows);
        draft
    }

    fn body(pairs: &[(&str, &str)]) -> FormBody {
        FormBody::from_pairs(pairs.iter().map(|(n, v)| ((*n).to_string(), (*v).to_string())).collect())
    }

    #[test]
    fn test_findings_without_a_body_are_keyed_by_stored_position() {
        let draft = spatial(json!([
            { "type": "Geonames", "url": "https://www.geonames.org/1" },
            { "type": "Geonames", "url": "Basel" },
        ]));
        assert_eq!(
            walk("spatialCoverage", REFERENCE_URL, &draft, None),
            ["spatialCoverage.r1.ref.url"]
        );
    }

    #[test]
    fn test_findings_from_a_body_land_on_the_posted_row_behind_a_blank_one() {
        // The blank row is not stored, so the stored value sits at position 0 while
        // its control posts under `r1`.
        let draft = spatial(json!([{ "type": "Geonames", "url": "Basel" }]));
        let posted = body(&[
            ("spatialCoverage.row", "r0"),
            ("spatialCoverage.r0.ref.url", ""),
            ("spatialCoverage.row", "r1"),
            ("spatialCoverage.r1.ref.url", " Basel "),
        ]);
        assert_eq!(
            walk("spatialCoverage", REFERENCE_URL, &draft, Some(&posted)),
            ["spatialCoverage.r1.ref.url"]
        );
    }

    #[test]
    fn test_a_posted_value_the_draft_does_not_hold_has_no_finding() {
        let draft = spatial(json!([{ "type": "Geonames", "url": "https://www.geonames.org/1" }]));
        let posted = body(&[("spatialCoverage.row", "r0"), ("spatialCoverage.r0.ref.url", "Basel")]);
        assert!(walk("spatialCoverage", REFERENCE_URL, &draft, Some(&posted)).is_empty());
    }

    #[test]
    fn test_an_empty_value_and_a_placeholder_have_no_finding() {
        let draft = spatial(json!([{ "url": "MISSING" }, { "url": "  " }, { "text": "no url" }]));
        assert!(walk("spatialCoverage", REFERENCE_URL, &draft, None).is_empty());
    }

    #[test]
    fn test_a_field_without_rows_is_keyed_by_its_own_id() {
        // The secondary URL is read through its slot, here the array's second element.
        let mut draft = ProjectDraft::default();
        draft.set("url", json!(["https://example.test", "Basel"]));
        draft.set("dataManagementPlan", json!("not accessible"));
        assert_eq!(walk("secondaryUrl", WHOLE_VALUE, &draft, None), ["secondaryUrl"]);
        assert!(walk("url", WHOLE_VALUE, &draft, None).is_empty());
        assert_eq!(walk("dataManagementPlan", WHOLE_VALUE, &draft, None), ["dataManagementPlan"]);
    }

    #[test]
    fn test_a_row_that_is_the_value_is_named_by_its_key_alone() {
        let mut draft = ProjectDraft::default();
        draft.set("additionalMaterial", json!(["https://example.test", "Basel", "Basel"]));
        assert_eq!(
            walk("additionalMaterial", WHOLE_VALUE, &draft, None),
            ["additionalMaterial.r1", "additionalMaterial.r2"]
        );
    }

    #[test]
    fn test_a_body_that_does_not_carry_the_field_falls_back_to_stored_positions() {
        // A section posts only its own fields, so a body without `{field}.row` says
        // nothing about this field's rows.
        let draft = spatial(json!([{ "type": "Geonames", "url": "Basel" }]));
        let posted = body(&[("name", "A project")]);
        assert_eq!(
            walk("spatialCoverage", REFERENCE_URL, &draft, Some(&posted)),
            ["spatialCoverage.r0.ref.url"]
        );
    }
}
