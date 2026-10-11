//! Readers for a raw KDL node's attributes and positional arguments.
//!
//! The reporting readers (`require_*`, `get_typed_*`, `get_bool_attr`,
//! `optional_string_attr`) push diagnostics, so only lowering calls them; a
//! `*_into_raw` sibling or a pre-scan reads through the silent ones
//! (`has_attr`, `get_attr`, `get_string_attr`, `first_positional_string`,
//! `positional_string_args`, `first_integer_arg`).

use crate::diagnostic::{Diagnostic, DiagnosticKind, ValidationReport};
use crate::ir::Value;

/// Whether a raw component carries the attribute `key`, whatever its value.
/// Diagnostic-free.
pub(super) fn has_attr(raw: &crate::config::Component, key: &str) -> bool {
    raw.attrs.iter().any(|(k, _)| k == key)
}

/// Get a named attribute value from a raw component (owned clone).
pub(super) fn get_attr(raw: &crate::config::Component, key: &str) -> Option<Value> {
    raw.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Get a named string attribute value from a raw component.
///
/// Diagnostic-free by design: absent and present-but-wrong-typed both yield
/// `None`, leaving the caller to decide whether that matters. This is now
/// **only** for the `*_into_raw` raw-fallback siblings (which must never
/// double-report a value lowering already reported) and for the handful of
/// config-internal pre-scans that run before lowering proper (e.g.
/// `lower_page_body`'s `page_list_ids` pre-scan). Lowering itself no longer
/// reads its own optional string attributes through this fn — it uses the
/// REPORTING `get_typed_string_attr` (absent ⇒ `None`, present-but-wrong-typed
/// ⇒ `InvalidAttributeType`) or, for required attributes, `require_string_attr`.
///
/// Only `require_string_attr` (required attributes), `get_bool_attr` and
/// `get_typed_string_attr` push `InvalidAttributeType`; all three must NEVER
/// be called from a raw sibling, since there they really would double-report
/// what the config-internal lowering pass already found. Before suppressing
/// anything in a raw sibling as a "double-report", check which of the three
/// the lowering path actually used for that attribute — don't assume from the
/// attribute's name.
pub(super) fn get_string_attr(raw: &crate::config::Component, key: &str) -> Option<String> {
    raw.attrs.iter().find(|(k, _)| k == key).and_then(|(_, v)| {
        if let Value::String(s) = v {
            Some(s.clone())
        } else {
            None
        }
    })
}

/// Get a named string attribute value from a raw component, reporting a
/// present-but-wrong-typed value as `InvalidAttributeType`.
///
/// Sits between `get_string_attr` (silently `None` on ANY non-string value,
/// including a wrong-typed one — never reports) and `require_string_attr`
/// (also reports when the attribute is entirely ABSENT): an absent attribute
/// here stays diagnostic-free `None`, mirroring `get_bool_attr`'s "absent ⇒
/// default, no diagnostic" rule, but a PRESENT attribute holding a
/// non-string KDL value pushes `InvalidAttributeType` — matching
/// `get_bool_attr`'s "present but wrong type" behaviour for booleans. The
/// caller's own value-level match (e.g. `"term"`/`"keyword"`) still runs
/// after this and can push its own `InvalidAttributeType` for a wrong
/// *value* of a correctly-typed string; the two never collide because a
/// non-string value never reaches that match (this fn already consumed it).
///
/// Like `get_bool_attr`, this DOES report — never call it from a
/// `*_into_raw` raw-fallback sibling (see `get_string_attr`'s doc comment
/// for the double-report rule this exists to avoid); use `get_string_attr`
/// there instead. Its lowering sites are every optional string attribute
/// read inside a `lower_*` fn: `chip kind=`, `order-by direction=`,
/// `resource-list`'s `id=`/`media=`, `resource-detail`'s `id=`/
/// `title-property=`/`class=`/`media=`, `compound-viewer`'s `media=`/
/// `picker=`, `search-bar`'s `placeholder=`/`scope=`, and `sort`'s
/// `direction=` (Step 7). None of these is reachable from any `*_into_raw`
/// sibling — every raw sibling reads the corresponding attribute with the
/// silent `get_string_attr` instead.
pub(super) fn get_typed_string_attr(
    raw: &crate::config::Component,
    key: &str,
    report: &mut ValidationReport,
) -> Option<String> {
    match raw.attrs.iter().find(|(k, _)| k == key) {
        None => None,
        Some((_, Value::String(s))) => Some(s.clone()),
        Some((_, other)) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "component '{}' attribute '{}' must be a string, got {}",
                    raw.name,
                    key,
                    value_type_name(other)
                ),
                raw.span,
            ));
            None
        }
    }
}

/// [`get_typed_string_attr`] with the absent/wrong-typed ambiguity resolved:
/// `Ok(None)` when absent, `Ok(Some)` for a string, `Err(())` when present but
/// not a string (already reported). Reports, so never call it from a
/// `*_into_raw` sibling.
pub(super) fn optional_string_attr(
    raw: &crate::config::Component,
    key: &str,
    report: &mut ValidationReport,
) -> Result<Option<String>, ()> {
    if !has_attr(raw, key) {
        return Ok(None);
    }
    get_typed_string_attr(raw, key, report).map(Some).ok_or(())
}

/// Get a named boolean attribute value from a raw component.
///
/// Unlike `get_string_attr` (which silently returns `None` on a wrong-typed
/// value, leaving the caller to decide whether that matters), `get_bool_attr`
/// itself pushes `InvalidAttributeType` when the attribute is present but not
/// a KDL boolean (e.g. `grid="true"` as a string, or `grid=1` as an integer)
/// — a caller that just did `.unwrap_or(false)` on a silent `None` would
/// otherwise mistake a config typo for a deliberate "not set", with no boot
/// error at all. Absent ⇒ `false`, no diagnostic.
pub(super) fn get_bool_attr(raw: &crate::config::Component, key: &str, report: &mut ValidationReport) -> bool {
    match raw.attrs.iter().find(|(k, _)| k == key) {
        None => false,
        Some((_, Value::Bool(b))) => *b,
        Some((_, other)) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "component '{}' attribute '{}' must be a boolean, got {}",
                    raw.name,
                    key,
                    value_type_name(other)
                ),
                raw.span,
            ));
            false
        }
    }
}

/// Require a named string attribute, pushing `MissingRequiredAttribute` if
/// absent or `InvalidAttributeType` if present but not a string.
/// Returns `None` in either error case (caller should bail).
pub(super) fn require_string_attr(
    raw: &crate::config::Component,
    key: &str,
    report: &mut ValidationReport,
) -> Option<String> {
    match raw.attrs.iter().find(|(k, _)| k == key) {
        None => {
            report.push(Diagnostic::error(
                DiagnosticKind::MissingRequiredAttribute,
                format!("component '{}' is missing the required '{}' attribute", raw.name, key),
                raw.span,
            ));
            None
        }
        Some((_, Value::String(s))) => Some(s.clone()),
        Some((_, other)) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "component '{}' attribute '{}' must be a string, got {}",
                    raw.name,
                    key,
                    value_type_name(other)
                ),
                raw.span,
            ));
            None
        }
    }
}

/// Require a **named** integer attribute (`axis-start=1470`), pushing
/// `MissingRequiredAttribute` if absent or `InvalidAttributeType` if present
/// but not a KDL integer. The integer counterpart of `require_string_attr`
/// (`first_integer_arg` reads *positional* args, not named attrs). Returns the
/// raw `i128`: the caller range-checks it before any narrowing — never `as`.
///
/// Reports, so never call it from a `*_into_raw` raw-fallback sibling.
pub(super) fn require_integer_attr(
    raw: &crate::config::Component,
    key: &str,
    report: &mut ValidationReport,
) -> Option<i128> {
    match raw.attrs.iter().find(|(k, _)| k == key) {
        None => {
            report.push(Diagnostic::error(
                DiagnosticKind::MissingRequiredAttribute,
                format!("component '{}' is missing the required '{}' attribute", raw.name, key),
                raw.span,
            ));
            None
        }
        Some((_, Value::Integer(n))) => Some(*n),
        Some((_, other)) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "component '{}' attribute '{}' must be an integer, got {}",
                    raw.name,
                    key,
                    value_type_name(other)
                ),
                raw.span,
            ));
            None
        }
    }
}

/// Get a named integer attribute value from a raw component, reporting a
/// present-but-wrong-typed value as `InvalidAttributeType`.
///
/// The integer counterpart of `get_typed_string_attr`: absent ⇒ `None`, no
/// diagnostic; present but not a KDL integer ⇒ `InvalidAttributeType` and
/// `None`. Returns the raw `i128` — the caller range-checks it into the
/// narrower type it needs (e.g. `u32::try_from(n).ok()`, never an `as`
/// cast — see `timeline::timeline_axis_year`).
///
/// Reports, so never call it from a `*_into_raw` raw-fallback sibling.
pub(super) fn get_typed_integer_attr(
    raw: &crate::config::Component,
    key: &str,
    report: &mut ValidationReport,
) -> Option<i128> {
    match raw.attrs.iter().find(|(k, _)| k == key) {
        None => None,
        Some((_, Value::Integer(n))) => Some(*n),
        Some((_, other)) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "component '{}' attribute '{}' must be an integer, got {}",
                    raw.name,
                    key,
                    value_type_name(other)
                ),
                raw.span,
            ));
            None
        }
    }
}

/// Get the first positional integer arg of a node (for `per-page`/`limit`).
pub(super) fn first_integer_arg(raw: &crate::config::Component) -> Option<i128> {
    raw.args
        .iter()
        .find_map(|v| if let Value::Integer(n) = v { Some(*n) } else { None })
}

/// Require a node's FIRST positional arg to be a string (the
/// `narrative-for "<id>"` shape), pushing `MissingRequiredAttribute` when the
/// node has no positional arg or `InvalidAttributeType` when the first one is
/// not a string — the `require_string_attr` pair, for a positional value.
/// Reports, so never call it from a `*_into_raw` sibling (use
/// `first_positional_string` there).
pub(super) fn require_first_positional_string(
    raw: &crate::config::Component,
    report: &mut ValidationReport,
) -> Option<String> {
    match raw.args.first() {
        None => {
            report.push(Diagnostic::error(
                DiagnosticKind::MissingRequiredAttribute,
                format!(
                    "'{}' is missing its required first positional argument (a resource id)",
                    raw.name
                ),
                raw.span,
            ));
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => {
            report.push(Diagnostic::error(
                DiagnosticKind::InvalidAttributeType,
                format!(
                    "'{}' first positional argument must be a string (a resource id), got {}",
                    raw.name,
                    value_type_name(other)
                ),
                raw.span,
            ));
            None
        }
    }
}

/// The diagnostic-free sibling of `require_first_positional_string`, for the
/// `*_into_raw` raw-fallback checks: `None` on an absent or non-string first
/// positional arg, never reporting.
pub(super) fn first_positional_string(raw: &crate::config::Component) -> Option<String> {
    match raw.args.first() {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// The positional string args of a raw child node (`show-properties`,
/// `group`), in order; non-string args are silently dropped. Diagnostic-free,
/// so it is shared by lowering and the `*_into_raw` raw siblings alike.
pub(super) fn positional_string_args(node: &crate::config::Component) -> Vec<String> {
    node.args
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

/// Human-readable type name for a `Value`, used in `InvalidFieldType` messages.
pub(super) fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}
