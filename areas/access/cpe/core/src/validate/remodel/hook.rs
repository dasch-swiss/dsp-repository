//! The lowering of the `hook` node: what the project's hook reads from the
//! curation table, which properties it writes or sets, and which classes it
//! may drop resources of.
//!
//! It imports nothing from `validate::remodel`. Each child names a class, and
//! a `key` and a `provides` enter that class's one namespace of keys and of
//! property names, so the parent hands in what every class has declared: the
//! node is lowered after the classes, and a clash is reported here.

use std::collections::HashSet;

use super::nodes::{
    check_attrs, check_children, check_no_args, declare_key, declare_name, first_occurrence, non_empty, push,
    single_string, ClassSeen,
};
use super::structure::{required_class, Outline};
use crate::config::Component as RawNode;
use crate::diagnostic::{DiagnosticKind, ValidationReport};
use crate::remodel::{ClassIndex, CpeProperty, HookKey, HookOverrides, HookProvides, PropertyIndex, RemodelHook};

/// Lowers the `hook` node. `seen` holds what each class has declared, by the
/// class's position in the block; a `key` and a `provides` are recorded there.
///
/// Returns `None` when a child is faulty (already reported).
pub(super) fn lower_hook(
    node: &RawNode,
    outline: &Outline<'_>,
    seen: &mut [ClassSeen],
    report: &mut ValidationReport,
) -> Option<RemodelHook> {
    check_attrs(node, &[], report);
    check_no_args(node, report);
    check_children(node, &["key", "provides", "overrides", "drops"], report);

    let mut keys = Vec::new();
    let mut provides = Vec::new();
    let mut overrides = Vec::new();
    let mut drops = Vec::new();
    let mut overridden = HashSet::new();
    let mut dropped = HashSet::new();
    let mut whole = true;
    for child in &node.children {
        match child.name.as_str() {
            "key" => {
                let key = lower_key(child, outline, seen, report);
                whole &= key.is_some();
                keys.extend(key);
            }
            "provides" => {
                let provided = lower_provides(child, outline, seen, report);
                whole &= provided.is_some();
                provides.extend(provided);
            }
            "overrides" => {
                let overridden = lower_overrides(child, outline, &mut overridden, report);
                whole &= overridden.is_some();
                overrides.extend(overridden);
            }
            "drops" => {
                let class = lower_drops(child, outline, &mut dropped, report);
                whole &= class.is_some();
                drops.extend(class);
            }
            // Reported above as an unknown child.
            _ => {}
        }
    }
    whole.then_some(RemodelHook { keys, provides, overrides, drops })
}

/// The child's one argument, naming `what`, and the class its `class=` names.
/// Both are read before either is required, so that a child lacking both is
/// reported for both.
fn named_on_a_class(
    node: &RawNode,
    what: &str,
    outline: &Outline<'_>,
    report: &mut ValidationReport,
) -> (Option<String>, Option<usize>) {
    check_attrs(node, &["class"], report);
    check_children(node, &[], report);
    let name = single_string(node, what, report).and_then(|name| non_empty(name, what, node.span, report));
    let class = required_class(node, "class", outline, report);
    (name, class)
}

fn lower_key(
    node: &RawNode,
    outline: &Outline<'_>,
    seen: &mut [ClassSeen],
    report: &mut ValidationReport,
) -> Option<HookKey> {
    let (key, class) = named_on_a_class(node, "the curated key", outline, report);
    let (key, class) = (key?, class?);
    let key = declare_key(seen.get_mut(class)?, key, node.span, report);
    Some(HookKey { class: ClassIndex(class), key })
}

fn lower_provides(
    node: &RawNode,
    outline: &Outline<'_>,
    seen: &mut [ClassSeen],
    report: &mut ValidationReport,
) -> Option<HookProvides> {
    let (name, class) = named_on_a_class(node, "the property's CPE name", outline, report);
    let (name, class) = (name?, class?);
    declare_name(seen.get_mut(class)?, &name, node.span, report);
    Some(HookProvides { class: ClassIndex(class), name: CpeProperty::new(name) })
}

/// A property a hook overrides is a `property` node of the class, under any
/// of its names. A property is overridden once: `overridden` holds each as
/// (class, position), so two names of one property are one property.
fn lower_overrides(
    node: &RawNode,
    outline: &Outline<'_>,
    overridden: &mut HashSet<(usize, usize)>,
    report: &mut ValidationReport,
) -> Option<HookOverrides> {
    let (name, class) = named_on_a_class(node, "the property's CPE name", outline, report);
    let (name, class) = (name?, class?);
    let Some((position, _)) = outline.property(class, &name) else {
        push(
            report,
            DiagnosticKind::UnknownRemodelReference,
            node.span,
            format!("'overrides' names '{name}', which is no property of its class"),
        );
        return None;
    };
    let message = || format!("'hook' overrides the property '{name}' in two 'overrides' nodes");
    if !first_occurrence(overridden, (class, position), node.span, message, report) {
        return None;
    }
    Some(HookOverrides { class: ClassIndex(class), property: PropertyIndex(position) })
}

fn lower_drops(
    node: &RawNode,
    outline: &Outline<'_>,
    dropped: &mut HashSet<usize>,
    report: &mut ValidationReport,
) -> Option<ClassIndex> {
    check_attrs(node, &["class"], report);
    check_children(node, &[], report);
    check_no_args(node, report);
    let class = required_class(node, "class", outline, report)?;
    // A dropped resource has no row, and its members would be members of nothing.
    if outline.has_members(class) {
        push(
            report,
            DiagnosticKind::ConflictingRemodelAttributes,
            node.span,
            "'drops' names a class that another class is a member-of".to_owned(),
        );
        return None;
    }
    let message = || "'hook' drops the same class in two 'drops' nodes".to_owned();
    first_occurrence(dropped, class, node.span, message, report).then_some(ClassIndex(class))
}
