//! The data checks of one `media="…"` attribute, shared by the components
//! that name a media property (`resource-detail`, `compound-viewer`,
//! `resource-list`, `edition-cards`, `strip-row`) and by both
//! `validate_with_data` arms.

use super::facts::{push_internal_query_error, ProjectFacts};
use crate::diagnostic::{Diagnostic, DiagnosticKind, Severity, ValidationReport};

/// Perform the four-check `media="…"` data-dependent validation for
/// one component (either a `resource-detail` or a `resource-list`), mirroring
/// `validate::check_single_facet`'s priority-ordered structure:
///
/// 1. `UnknownMediaProperty` (Error) — property absent everywhere.
/// 2. `IncompatibleMediaProperty` (Error) — present on the class but not `link`-typed.
/// 3. `SparseMediaProperty` (Warning) — present somewhere, absent on this class.
/// 4. `MediaPropertyTargetNotImagelike` (Error) — `link`-typed and present, but its resolvable
///    targets belong to a class with no `representations` rows.
///
/// Appends to `report`. Checks 1–3 are mutually exclusive (stop after the
/// first fires); check 4 only runs when the property is confirmed present
/// and `link`-typed on `class`.
pub(super) fn check_single_media_property(
    class: &str,
    property: &str,
    page_route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    // Check 1: property must exist anywhere in the data (hard-fail).
    let exists_anywhere = match data.property_exists_anywhere(property) {
        Ok(b) => b,
        Err(e) => {
            push_internal_query_error(
                report,
                "media property existence",
                format!(
                    "page '{page_route}': could not check existence of media property \
                     '{property}': {e}"
                ),
            );
            return;
        }
    };

    if !exists_anywhere {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownMediaProperty {
                property: property.to_owned(),
                class: class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "page '{page_route}': media property '{property}' does not exist anywhere \
                 in the data (class '{class}')"
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Check 2: if present on the class, verify link-typedness.
    let types = match data.property_value_types_on_class(class, property) {
        Ok(t) => t,
        Err(e) => {
            push_internal_query_error(
                report,
                "media property value type",
                format!(
                    "page '{page_route}': could not check value type for media property \
                     '{property}' on class '{class}': {e}"
                ),
            );
            return;
        }
    };

    if !types.is_empty() && !types.contains("link") {
        let actual_type = types.iter().min().expect("types is non-empty — checked above").clone();
        report.push(Diagnostic {
            kind: DiagnosticKind::IncompatibleMediaProperty {
                property: property.to_owned(),
                class: class.to_owned(),
                value_type: actual_type.clone(),
                page: page_route.to_owned(),
            },
            message: format!(
                "page '{page_route}': media property '{property}' on class '{class}' has \
                 value type '{actual_type}', but 'media' requires a 'link' property"
            ),
            span: None,
            severity: Severity::Error,
        });
        return;
    }

    // Check 3: property must be present on this class (warn when sparse).
    let class_props = match data.properties_present_on_class(class) {
        Ok(set) => set,
        Err(e) => {
            report.push(Diagnostic {
                kind: DiagnosticKind::SparseMediaProperty {
                    property: property.to_owned(),
                    class: class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!("page '{page_route}': could not fetch properties for class '{class}': {e}"),
                span: None,
                severity: Severity::Warning,
            });
            return;
        }
    };

    if !class_props.contains(property) {
        report.push(Diagnostic {
            kind: DiagnosticKind::SparseMediaProperty {
                property: property.to_owned(),
                class: class.to_owned(),
                page: page_route.to_owned(),
            },
            message: format!(
                "page '{page_route}': media property '{property}' is not present on any \
                 resource of class '{class}'"
            ),
            span: None,
            severity: Severity::Warning,
        });
        return;
    }

    // Check 4: the property's resolvable link targets must belong to a class
    // whose resources carry `representations` rows (image-like). Only
    // meaningful once we know the property is actually present and
    // `link`-typed on this class (checks 1–3 passed).
    let target_classes = match data.link_target_classes_on_class(class, property) {
        Ok(set) => set,
        Err(e) => {
            push_internal_query_error(
                report,
                "media property target classes",
                format!(
                    "page '{page_route}': could not resolve link target classes for media \
                     property '{property}' on class '{class}': {e}"
                ),
            );
            return;
        }
    };

    // Deterministic iteration: sort so the diagnostic (and any accumulated
    // test assertions) don't depend on HashSet order.
    let mut sorted_targets: Vec<&String> = target_classes.iter().collect();
    sorted_targets.sort();

    for target_class in sorted_targets {
        let has_representations = match data.class_has_representations(target_class) {
            Ok(b) => b,
            Err(e) => {
                push_internal_query_error(
                    report,
                    "media property target representations",
                    format!(
                        "page '{page_route}': could not check representations for media \
                         target class '{target_class}': {e}"
                    ),
                );
                continue;
            }
        };

        if !has_representations {
            report.push(Diagnostic {
                kind: DiagnosticKind::MediaPropertyTargetNotImagelike {
                    property: property.to_owned(),
                    class: class.to_owned(),
                    target_class: target_class.to_owned(),
                    page: page_route.to_owned(),
                },
                message: format!(
                    "page '{page_route}': media property '{property}' on class '{class}' \
                     targets class '{target_class}', which has no 'representations' rows on \
                     any of its resources — 'media' requires an image-like target class"
                ),
                span: None,
                severity: Severity::Error,
            });
        }
    }
}
