//! The `part` grammar a composed gloss, a record header's imprint and an
//! edition card's imprint share: lowering a `part`, and the value types each
//! part role may render.

use super::attrs::{get_typed_string_attr, require_string_attr};
use super::facts::{push_internal_query_error, ProjectFacts};
use crate::diagnostic::{Diagnostic, DiagnosticKind, ValidationReport};
use crate::ir::GlossPart;

/// The value types a composed-gloss or imprint `part` renders.
const PART_VALUE_TYPES: &[&str] = &["text", "richtext", "date", "integer", "decimal"];
/// The value types a `unit=` template can wrap.
const UNIT_VALUE_TYPES: &[&str] = &["integer", "decimal"];

/// `(property, role, the types the role accepts)` for each of a composed
/// gloss's or an imprint's `(part property, has unit=)` pairs, the role names
/// prefixed with `prefix` (`""` for a gloss, `"imprint "` for an imprint).
pub(super) fn part_roles<'a>(
    parts: &[(&'a str, bool)],
    prefix: &str,
) -> Vec<(&'a str, String, &'static [&'static str])> {
    let mut roles = Vec::new();
    for &(property, has_unit) in parts {
        roles.push((property, format!("{prefix}part"), PART_VALUE_TYPES));
        if has_unit {
            roles.push((property, format!("{prefix}unit"), UNIT_VALUE_TYPES));
        }
    }
    roles
}

/// The shared body of the gloss and header-slot type checks: one
/// `IncompatibleGlossPart` per role whose property is stored with a type the
/// role cannot render. `surface` names the component in the message. A
/// property absent from the class is skipped: its existence is the
/// component's own sparse/unknown check's business.
pub(super) fn check_role_value_types(
    class: &str,
    roles: &[(&str, String, &[&str])],
    surface: &str,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    for (property, role, accepted) in roles {
        let (property, role) = (*property, role.as_str());
        let types = match data.property_value_types_on_class(class, property) {
            Ok(t) => t,
            Err(e) => {
                push_internal_query_error(
                    report,
                    "gloss property value type",
                    format!(
                        "page '{route}': {surface}: could not check the value type \
                         of property '{property}' on class '{class}': {e}"
                    ),
                );
                continue;
            }
        };
        // The lexicographically smallest offending type, for a stable message.
        if let Some(bad) = types.iter().filter(|t| !accepted.contains(&t.as_str())).min() {
            report.push(Diagnostic::error_spanless(
                DiagnosticKind::IncompatibleGlossPart {
                    property: property.to_owned(),
                    role: role.to_owned(),
                    value_type: bad.clone(),
                    class: class.to_owned(),
                    page: route.to_owned(),
                },
                format!(
                    "page '{route}': {surface} {role} '{property}' on class \
                     '{class}' has value type '{bad}', which the {role} cannot render"
                ),
            ));
        }
    }
}

/// Lower one `part property="…" unit="…"` child of a composed gloss or an
/// imprint (`owner` names which, for the messages). `property` is required,
/// `unit` optional, both through the reporting readers; a property already
/// in `earlier` is `InvalidAttributeType`. `None` when the part is dropped
/// (the boot fails anyway).
pub(super) fn lower_gloss_part(
    child: &crate::config::Component,
    earlier: &[GlossPart],
    owner: &str,
    report: &mut ValidationReport,
) -> Option<GlossPart> {
    let property = require_string_attr(child, "property", report);
    let unit = get_typed_string_attr(child, "unit", report);
    let property = property?;
    if earlier.iter().any(|p| p.property == property) {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidAttributeType,
            format!("{owner} names the part property '{property}' more than once"),
            child.span,
        ));
        return None;
    }
    Some(GlossPart { property, unit })
}

/// Lower one `imprint { part … }` block (a record header's, or an edition
/// card block's): each `part` follows the composed-gloss grammar
/// ([`lower_gloss_part`]); any other child is `UnknownChildNode { component:
/// "imprint" }`; a block with no `part` is `InvalidAttributeType`.
///
/// Reports, so never call it from a `*_into_raw` sibling.
pub(super) fn lower_imprint(block: &crate::config::Component, report: &mut ValidationReport) -> Vec<GlossPart> {
    let mut imprint: Vec<GlossPart> = Vec::new();
    for child in &block.children {
        if child.name == "part" {
            if let Some(part) = lower_gloss_part(child, &imprint, "imprint", report) {
                imprint.push(part);
            }
            continue;
        }
        report.push(Diagnostic::error(
            DiagnosticKind::UnknownChildNode { component: "imprint".to_owned(), child: child.name.clone() },
            format!("imprint has an unknown child node '{}'; expected part", child.name),
            child.span,
        ));
    }
    if !block.children.iter().any(|c| c.name == "part") {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidAttributeType,
            "an imprint needs at least one 'part'",
            block.span,
        ));
    }
    imprint
}
