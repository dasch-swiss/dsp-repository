//! Lowering and boot checks of Incunabula's `timeline`, reached
//! through its `validate::project_components` row.

use super::attrs::{get_string_attr, get_typed_string_attr, require_integer_attr, require_string_attr};
use super::context::ValidationContext;
use super::facts::{push_internal_query_error, ProjectFacts};
use crate::diagnostic::{Diagnostic, DiagnosticKind, Severity, ValidationReport};
use crate::ir::Component;

/// The shortest legal timeline axis, in years. The lanes variant draws one
/// grid line every three years, so a shorter axis has no whole grid cell.
const TIMELINE_MIN_AXIS_SPAN: i64 = 3;

/// Range-check a KDL axis integer (`i128`) into an `i32` year. `None` ⇒ out
/// of range (the caller reports `TimelineAxisOutOfRange`). Never an `as` cast.
fn timeline_axis_year(n: i128) -> Option<i32> {
    i32::try_from(n).ok()
}

/// Whether `axis_start..axis_end` is a legal timeline axis: `axis_end`
/// exceeds `axis_start` by at least [`TIMELINE_MIN_AXIS_SPAN`] years. Widened
/// to `i64` so the subtraction can never overflow at the `i32` extremes.
fn timeline_axis_is_valid(axis_start: i32, axis_end: i32) -> bool {
    i64::from(axis_end) - i64::from(axis_start) >= TIMELINE_MIN_AXIS_SPAN
}

/// Lower a `timeline` node.
///
/// `class`/`date-property`/`lane-property` are required strings
/// (`require_string_attr`); `extent-property` is an optional string read with
/// the **reporting** `get_typed_string_attr`, so `extent-property=1` is
/// `InvalidAttributeType` rather than a silently-dropped extent segment.
/// `axis-start`/`axis-end` are required named integers
/// (`require_integer_attr`), range-checked into `i32`
/// (`TimelineAxisOutOfRange`) and then checked as a pair
/// (`InvalidTimelineAxis`). Every attribute is read before any early return,
/// so one node reports all of its defects at once. No unknown-attribute
/// check (the engine has none).
///
/// Reached only through its `validate::project_components` row (`lower`), whose
/// signature fixes the unused `_ctx` parameter.
pub(crate) fn lower_timeline(
    raw: &crate::config::Component,
    _ctx: &ValidationContext<'_>,
    report: &mut ValidationReport,
) -> Option<Component> {
    let class = require_string_attr(raw, "class", report);
    let date_property = require_string_attr(raw, "date-property", report);
    let lane_property = require_string_attr(raw, "lane-property", report);
    let extent_property = get_typed_string_attr(raw, "extent-property", report);

    let axis_year = |key: &str, report: &mut ValidationReport| -> Option<i32> {
        let n = require_integer_attr(raw, key, report)?;
        let year = timeline_axis_year(n);
        if year.is_none() {
            report.push(Diagnostic::error(
                DiagnosticKind::TimelineAxisOutOfRange { attribute: key.to_owned() },
                format!(
                    "component 'timeline' attribute '{key}' = {n} is out of range \
                     (must fit in a 32-bit signed integer)"
                ),
                raw.span,
            ));
        }
        year
    };
    let axis_start = axis_year("axis-start", report);
    let axis_end = axis_year("axis-end", report);

    let (axis_start, axis_end) = (axis_start?, axis_end?);
    if !timeline_axis_is_valid(axis_start, axis_end) {
        report.push(Diagnostic::error(
            DiagnosticKind::InvalidTimelineAxis { axis_start, axis_end },
            format!(
                "component 'timeline' axis {axis_start}–{axis_end} is invalid: axis-end must \
                 exceed axis-start by at least {TIMELINE_MIN_AXIS_SPAN} years"
            ),
            raw.span,
        ));
        return None;
    }

    Some(Component::Timeline {
        class: class?,
        date_property: date_property?,
        lane_property: lane_property?,
        extent_property,
        axis_start,
        axis_end,
    })
}

/// Data-dependent checks for one lowered `timeline` on
/// page `route` — see `check_single_timeline`. The class and properties are
/// always literals. Reached through the timeline's `validate::project_components`
/// row (`check_data`); a non-`Timeline` variant is ignored.
pub(crate) fn check_timeline_data(
    component: &Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    if let Component::Timeline { class, date_property, lane_property, extent_property, .. } = component {
        check_single_timeline(
            class,
            date_property,
            lane_property,
            extent_property.as_deref(),
            route,
            data,
            report,
        );
    }
}

/// Raw sibling of `check_timeline_data`, for the `validate_with_data` failure
/// arm (reached through the timeline's registry row, `check_data_into_raw`,
/// which already filtered on `(project, name)`). Reads every attribute with
/// the diagnostic-free `get_string_attr` and silently skips an
/// absent/wrong-typed one — `lower_timeline` already reported those
/// (`require_string_attr` for the three required strings,
/// `get_typed_string_attr` for `extent-property`). A `timeline` missing
/// `class` is skipped entirely; one missing only some properties is still
/// checked on the ones it has.
pub(crate) fn check_timeline_data_into_raw(
    raw_comp: &crate::config::Component,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let Some(class) = get_string_attr(raw_comp, "class") else {
        return;
    };
    let date_property = get_string_attr(raw_comp, "date-property");
    let lane_property = get_string_attr(raw_comp, "lane-property");
    let extent_property = get_string_attr(raw_comp, "extent-property");
    check_timeline_properties(
        &class,
        &[
            date_property.as_deref().map(|p| (p, TimelineRole::Date)),
            lane_property.as_deref().map(|p| (p, TimelineRole::Lane)),
            extent_property.as_deref().map(|p| (p, TimelineRole::Extent)),
        ],
        route,
        data,
        report,
    );
}

/// The role a `timeline` property plays, which fixes its required stored
/// `value_type` and its absent-from-class tier.
#[derive(Debug, Clone, Copy)]
enum TimelineRole {
    /// `date-property` — `date`; absent from the class is an Error.
    Date,
    /// `lane-property` — `list_value`; absent from the class is an Error.
    Lane,
    /// `extent-property` — `integer`; absent from the class is a Warning.
    Extent,
}

impl TimelineRole {
    /// The `properties.value_type` this role requires.
    fn expected_value_type(self) -> &'static str {
        match self {
            TimelineRole::Date => "date",
            TimelineRole::Lane => "list_value",
            TimelineRole::Extent => "integer",
        }
    }

    /// Whether a property absent from the whole class is a hard-fail. The
    /// component is useless without a date or a lane (every marker would be
    /// skipped, or dropped into the `—` lane) — a stated deviation from
    /// `validate::check_single_resource_list`'s Warning tier. The extent segment is
    /// optional, so it keeps the Warning.
    fn absent_from_class_is_error(self) -> bool {
        match self {
            TimelineRole::Date | TimelineRole::Lane => true,
            TimelineRole::Extent => false,
        }
    }
}

/// The shared body of the `timeline` data checks, for a fully-lowered node.
fn check_single_timeline(
    class: &str,
    date_property: &str,
    lane_property: &str,
    extent_property: Option<&str>,
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    check_timeline_properties(
        class,
        &[
            Some((date_property, TimelineRole::Date)),
            Some((lane_property, TimelineRole::Lane)),
            extent_property.map(|p| (p, TimelineRole::Extent)),
        ],
        route,
        data,
        report,
    );
}

/// The `timeline` data checks proper, shared by both `validate_with_data`
/// arms (the raw sibling may lack some properties, hence the `Option`s):
///
/// - `class` not in the data → `UnknownTimelineClass` (Error); the per-property checks then run the
///   absent-everywhere tier only;
/// - a property absent from the data entirely → `UnknownTimelineProperty` (Error);
/// - present in the data but carried by no resource of `class` (the union over the class — no
///   per-resource coverage check, see `dsp-incubator cpe/CLAUDE.md`'s `SparseProperty` note) →
///   `MissingTimelineProperty` (Error) for the date and lane, `SparseTimelineProperty` (Warning)
///   for the extent;
/// - carried by the class with any stored `value_type` other than the role's →
///   `IncompatibleTimelineProperty` (Error).
///
/// Every query failure is `InternalQueryError`. Never short-circuits across
/// properties.
fn check_timeline_properties(
    class: &str,
    properties: &[Option<(&str, TimelineRole)>],
    route: &str,
    data: &dyn ProjectFacts,
    report: &mut ValidationReport,
) {
    let class_known = match data.class_exists(class) {
        Ok(b) => b,
        Err(e) => {
            push_internal_query_error(
                report,
                "timeline class existence",
                format!(
                    "page '{route}': timeline: could not check class existence for \
                     '{class}': {e}"
                ),
            );
            return;
        }
    };
    if !class_known {
        report.push(Diagnostic {
            kind: DiagnosticKind::UnknownTimelineClass { class: class.to_owned(), page: route.to_owned() },
            message: format!(
                "page '{route}': timeline references class '{class}' which does not exist \
                 in the data"
            ),
            span: None,
            severity: Severity::Error,
        });
    }

    for &(prop, role) in properties.iter().flatten() {
        let exists_anywhere = match data.property_exists_anywhere(prop) {
            Ok(b) => b,
            Err(e) => {
                push_internal_query_error(
                    report,
                    "timeline property existence",
                    format!(
                        "page '{route}': timeline: could not check property existence for \
                         '{prop}': {e}"
                    ),
                );
                continue;
            }
        };
        if !exists_anywhere {
            report.push(Diagnostic {
                kind: DiagnosticKind::UnknownTimelineProperty {
                    property: prop.to_owned(),
                    class: class.to_owned(),
                    page: route.to_owned(),
                },
                message: format!(
                    "page '{route}': timeline property '{prop}' does not exist anywhere in \
                     the data (class '{class}')"
                ),
                span: None,
                severity: Severity::Error,
            });
            continue;
        }
        if !class_known {
            // Already reported the class; skip the class-scoped tiers.
            continue;
        }
        let types = match data.property_value_types_on_class(class, prop) {
            Ok(t) => t,
            Err(e) => {
                push_internal_query_error(
                    report,
                    "timeline property value type",
                    format!(
                        "page '{route}': timeline: could not check the value type of \
                         property '{prop}' on class '{class}': {e}"
                    ),
                );
                continue;
            }
        };
        if types.is_empty() {
            let (kind, severity, consequence) = if role.absent_from_class_is_error() {
                (
                    DiagnosticKind::MissingTimelineProperty {
                        property: prop.to_owned(),
                        class: class.to_owned(),
                        page: route.to_owned(),
                    },
                    Severity::Error,
                    "the timeline cannot place a single marker",
                )
            } else {
                (
                    DiagnosticKind::SparseTimelineProperty {
                        property: prop.to_owned(),
                        class: class.to_owned(),
                        page: route.to_owned(),
                    },
                    Severity::Warning,
                    "every marker will omit its extent",
                )
            };
            report.push(Diagnostic {
                kind,
                message: format!(
                    "page '{route}': timeline property '{prop}' is not present on any \
                     resource of class '{class}'; {consequence}"
                ),
                span: None,
                severity,
            });
            continue;
        }
        let expected = role.expected_value_type();
        // The lexicographically smallest offending type, for a stable message
        // (not arbitrary HashSet iteration order).
        if let Some(bad) = types.iter().filter(|t| t.as_str() != expected).min() {
            report.push(Diagnostic {
                kind: DiagnosticKind::IncompatibleTimelineProperty {
                    property: prop.to_owned(),
                    value_type: bad.clone(),
                    expected: expected.to_owned(),
                    class: class.to_owned(),
                    page: route.to_owned(),
                },
                message: format!(
                    "page '{route}': timeline property '{prop}' on class '{class}' has value \
                     type '{bad}', but its role requires '{expected}'"
                ),
                span: None,
                severity: Severity::Error,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_axis_year_accepts_exactly_the_i32_range() {
        assert_eq!(timeline_axis_year(1470), Some(1470));
        assert_eq!(timeline_axis_year(-500), Some(-500));
        assert_eq!(timeline_axis_year(i128::from(i32::MAX)), Some(i32::MAX));
        assert_eq!(timeline_axis_year(i128::from(i32::MIN)), Some(i32::MIN));
        assert_eq!(timeline_axis_year(i128::from(i32::MAX) + 1), None);
        assert_eq!(timeline_axis_year(i128::from(i32::MIN) - 1), None);
        assert_eq!(timeline_axis_year(i128::MAX), None);
    }

    #[test]
    fn timeline_axis_is_valid_requires_a_span_of_at_least_three() {
        // The Incunabula axis (span 36).
        assert!(timeline_axis_is_valid(1470, 1506));
        // Exactly one three-year grid cell.
        assert!(timeline_axis_is_valid(1470, 1473));
        assert!(!timeline_axis_is_valid(1470, 1472));
        assert!(!timeline_axis_is_valid(1470, 1470));
        assert!(!timeline_axis_is_valid(1506, 1470));
        // No overflow at the i32 extremes.
        assert!(timeline_axis_is_valid(i32::MIN, i32::MAX));
        assert!(!timeline_axis_is_valid(i32::MAX, i32::MIN));
    }
}
