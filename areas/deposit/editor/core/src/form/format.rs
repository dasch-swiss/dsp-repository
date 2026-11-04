//! Format and length rules: whether a stored value has the shape its kind of value
//! needs, and whether a capped field's value stays within its cap.
//!
//! A finding never stops a value being stored. The appliers keep what was typed, a
//! finding is computed from the draft whenever the form renders, and only submit
//! refuses one ([`crate::submission::check`]). Nothing here is carried by a write,
//! so every rendering of one draft shows the same findings.

use std::collections::HashSet;

use serde_json::Value;
use shared_metadata::{breaks_an_address, is_bare_doi, is_placeholder, DATA_MANAGEMENT_PLAN_NOT_ACCESSIBLE};
use url::{Host, Url};

use super::registry::Field;
use super::{stored_row_key, FormBody, Shape};
use crate::draft::ProjectDraft;

/// A kind of value with a format to check. What a failing value is told is the
/// caller's wording, keyed by this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// A full web address: `http://` or `https://`, then a host.
    Link,
    /// A DOI written bare (`10.1234/abcd`), or any [`Self::Link`], which is how a
    /// resolver link such as `https://doi.org/10.1234/abcd` passes.
    DoiOrLink,
    /// A [`Self::Link`], or [`DATA_MANAGEMENT_PLAN_NOT_ACCESSIBLE`] exactly: the
    /// public page links any other value.
    LinkOrNotAccessible,
}

impl ValueKind {
    /// Whether `value`, already trimmed, has this kind's format.
    #[must_use]
    pub fn accepts(self, value: &str) -> bool {
        match self {
            Self::Link => is_link(value),
            Self::DoiOrLink => is_bare_doi(value) || is_link(value),
            Self::LinkOrNotAccessible => value == DATA_MANAGEMENT_PLAN_NOT_ACCESSIBLE || is_link(value),
        }
    }

    /// Whether `value` is one this kind's rule finds fault with: an empty value
    /// and a placeholder are never checked.
    #[must_use]
    pub fn refuses(self, value: &str) -> bool {
        checkable(value).is_some_and(|value| !self.accepts(value))
    }
}

/// An absolute `http` or `https` address, written as a reader would copy it: a
/// dotted domain or an IP address as host, no credentials, and no whitespace,
/// control or bidirectional-formatting character. The URL standard the parser
/// follows is more lenient than that (`https:example.org`, `https:///example.org`,
/// `https://12345`), so each is checked on the text as typed.
fn is_link(value: &str) -> bool {
    let Some(rest) = ["http://", "https://"].iter().find_map(|scheme| {
        let head = value.get(..scheme.len())?;
        head.eq_ignore_ascii_case(scheme).then(|| &value[scheme.len()..])
    }) else {
        return false;
    };
    if rest.starts_with(['/', '\\']) || value.chars().any(breaks_an_address) {
        return false;
    }
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    let host = match url.host() {
        Some(Host::Domain(domain)) => {
            let domain = domain.strip_suffix('.').unwrap_or(domain);
            domain.contains('.') && !domain.split('.').any(str::is_empty)
        }
        // Written out, not a number the parser reads as an address.
        Some(Host::Ipv4(address)) => rest.starts_with(&address.to_string()),
        Some(Host::Ipv6(_)) => true,
        None => false,
    };
    host && url.username().is_empty() && url.password().is_none()
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

/// A field, or each row of one, whose whole value is a web address.
pub const LINK: &[Member] = &[Member { posted: "", stored: &[], kind: ValueKind::Link }];

/// A data management plan's link, or its "not accessible".
pub const PLAN_LINK: &[Member] = &[Member {
    posted: "",
    stored: &[],
    kind: ValueKind::LinkOrNotAccessible,
}];

/// The address of each authority reference in a field's rows.
pub const REFERENCE_LINK: &[Member] = &[Member { posted: "ref.url", stored: &["url"], kind: ValueKind::Link }];

/// What a [`Finding`] found fault with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingKind {
    Format(ValueKind),
    /// A value longer than its field's [`Field::max_chars`], which `max` carries.
    TooLong {
        max: usize,
    },
}

/// One value without the format its kind needs, or longer than its field allows.
#[derive(Debug, Clone)]
pub struct Finding {
    pub field: &'static Field,
    /// The name of the control showing the value, which is where the finding is
    /// rendered.
    pub control: String,
    pub kind: FindingKind,
}

/// Every finding in `fields`, from the values `draft` holds, in form order (field,
/// then stored or row position).
///
/// A row's control is named by its row key, so the key decides where a finding
/// renders. A render from `posted` names its rows by the posted keys, and a blank
/// row there is never stored, so a stored position is no key: each posted row is
/// matched to the stored values by its own value. Without a posted body the rows
/// are named by [`stored_row_key`], as the form names them.
///
/// A value equal to the one `published` holds for the same field is never too long,
/// so a project published over the cap keeps submitting its other changes; `None`
/// means nothing is published. Characters are Unicode scalar values, never UTF-16
/// units.
#[must_use]
pub fn findings(
    fields: impl Iterator<Item = &'static Field>,
    draft: &ProjectDraft,
    posted: Option<&FormBody>,
    published: Option<&ProjectDraft>,
) -> Vec<Finding> {
    fields
        .flat_map(|field| {
            let mut found =
                findings_by(std::iter::once(field), draft, posted, |field| field.format, ValueKind::accepts);
            found.extend(too_long(field, draft, published));
            found
        })
        .collect()
}

fn too_long(field: &'static Field, draft: &ProjectDraft, published: Option<&ProjectDraft>) -> Option<Finding> {
    let max = field.max_chars?;
    let value = draft.get(field.id).and_then(Value::as_str)?;
    let held = published.and_then(|published| published.get(field.id)).and_then(Value::as_str);
    (value.chars().count() > max && held != Some(value)).then(|| Finding {
        field,
        control: field.id.to_string(),
        kind: FindingKind::TooLong { max },
    })
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
            found.extend(controls.into_iter().map(|control| Finding {
                field,
                control,
                kind: FindingKind::Format(member.kind),
            }));
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
                assert_eq!(finding.kind, FindingKind::Format(members[0].kind));
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
            walk("spatialCoverage", REFERENCE_LINK, &draft, None),
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
            walk("spatialCoverage", REFERENCE_LINK, &draft, Some(&posted)),
            ["spatialCoverage.r1.ref.url"]
        );
    }

    #[test]
    fn test_a_posted_value_the_draft_does_not_hold_has_no_finding() {
        let draft = spatial(json!([{ "type": "Geonames", "url": "https://www.geonames.org/1" }]));
        let posted = body(&[("spatialCoverage.row", "r0"), ("spatialCoverage.r0.ref.url", "Basel")]);
        assert!(walk("spatialCoverage", REFERENCE_LINK, &draft, Some(&posted)).is_empty());
    }

    #[test]
    fn test_an_empty_value_and_a_placeholder_have_no_finding() {
        let draft = spatial(json!([{ "url": "MISSING" }, { "url": "  " }, { "text": "no url" }]));
        assert!(walk("spatialCoverage", REFERENCE_LINK, &draft, None).is_empty());
    }

    #[test]
    fn test_a_field_without_rows_is_keyed_by_its_own_id() {
        // The secondary URL is read through its slot, here the array's second element.
        let mut draft = ProjectDraft::default();
        draft.set("url", json!(["https://example.test", "Basel"]));
        draft.set("dataManagementPlan", json!("on request"));
        assert_eq!(walk("secondaryUrl", LINK, &draft, None), ["secondaryUrl"]);
        assert!(walk("url", LINK, &draft, None).is_empty());
        assert_eq!(walk("dataManagementPlan", LINK, &draft, None), ["dataManagementPlan"]);
    }

    #[test]
    fn test_a_row_that_is_the_value_is_named_by_its_key_alone() {
        let mut draft = ProjectDraft::default();
        draft.set("additionalMaterial", json!(["https://example.test", "Basel", "Basel"]));
        assert_eq!(
            walk("additionalMaterial", LINK, &draft, None),
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
            walk("spatialCoverage", REFERENCE_LINK, &draft, Some(&posted)),
            ["spatialCoverage.r0.ref.url"]
        );
    }

    #[test]
    fn test_link_accepts_a_full_web_address() {
        for value in [
            "https://www.dasch.swiss",
            "http://www.dasch.swiss",
            "HTTPS://www.dasch.swiss/path?query=1#part",
            "https://192.0.2.1/",
            "https://[2001:db8::1]/",
            "https://münchen.de/",
            "https://www.dasch.swiss./",
            "https://dasch.swiss/a%20b",
        ] {
            assert!(ValueKind::Link.accepts(value), "{value}");
        }
    }

    #[test]
    fn test_link_refuses_a_value_that_is_not_a_full_web_address() {
        for value in [
            "www.dasch.swiss",
            "Basel",
            "https:www.dasch.swiss",
            "ftp://www.dasch.swiss",
            "https://dasch",
            "https://",
            "https://www.dasch .swiss",
            "https://dasch.swiss/a b",
            "https://dasch.swiss/\ta",
            "https://dasch..swiss",
            "https://.dasch.swiss",
            "https:///www.dasch.swiss",
            "https://12345",
            "https://10.1234/abcd",
            "https://www.dasch.swiss@example.org/",
            "https://admin:secret@www.dasch.swiss/",
            "https://www.dasch.swiss/a\u{202E}fdp.exe",
            "https://www.dasch.swiss/\u{0}",
            "doi.asdasd..asdasd",
        ] {
            assert!(!ValueKind::Link.accepts(value), "{value}");
        }
    }

    #[test]
    fn test_a_plan_link_is_a_full_web_address_or_not_accessible() {
        assert!(ValueKind::LinkOrNotAccessible.accepts("not accessible"));
        assert!(ValueKind::LinkOrNotAccessible.accepts("https://doi.org/10.5281/zenodo.7038186"));
        for value in ["Not accessible", "javascript:alert(1)", "on request"] {
            assert!(!ValueKind::LinkOrNotAccessible.accepts(value), "{value}");
        }
    }

    #[test]
    fn test_doi_or_link_accepts_a_bare_doi_a_resolver_link_and_a_web_address() {
        for value in [
            "10.1234/abcd",
            "10.1000.10/123(456)",
            "https://doi.org/10.1234/abcd",
            "https://dx.doi.org/10.1234/abcd",
            "https://zenodo.org/record/7248296",
        ] {
            assert!(ValueKind::DoiOrLink.accepts(value), "{value}");
        }
    }

    #[test]
    fn test_doi_or_link_refuses_a_value_that_is_neither() {
        for value in [
            "doi.asdasd..asdasd",
            "10.1234",
            "10./abcd",
            "10.12a4/abcd",
            "10.1234/",
            "10.1234/ab cd",
            "10.1234/a\u{202E}fdp.exe",
            "10.1234/ab\u{0007}cd",
            "10.1234/a\u{00AD}b",
            "doi:10.1234/abcd",
            "www.dasch.swiss",
        ] {
            assert!(!ValueKind::DoiOrLink.accepts(value), "{value}");
        }
    }
}
