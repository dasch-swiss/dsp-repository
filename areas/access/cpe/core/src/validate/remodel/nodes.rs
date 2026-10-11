//! The node-level checks and readers every part of the remodel lowering shares,
//! and the record of what one class has declared so far.
//!
//! It imports nothing from `validate::remodel`, so the parent and each of its
//! other child modules can import from here without forming a cycle.

use std::collections::HashSet;
use std::hash::Hash;

use super::super::attrs::{get_attr, optional_string_attr, value_type_name};
use crate::config::Component as RawNode;
use crate::diagnostic::{Diagnostic, DiagnosticKind, Span, ValidationReport};
use crate::ir::Value;
use crate::remodel::{CuratedKey, IMAGE_LINK};

pub(super) fn push(report: &mut ValidationReport, kind: DiagnosticKind, span: Span, message: String) {
    report.push(Diagnostic::error(kind, message, span));
}

fn expected_list(allowed: &[&str]) -> String {
    if allowed.is_empty() {
        "it takes none".to_owned()
    } else {
        format!("expected one of {}", allowed.join(", "))
    }
}

/// Record `item` in `seen`. A repeat is `DuplicateRemodelEntry` at `span`
/// with the message `message` builds; returns whether `item` was new.
pub(super) fn first_occurrence<T: Eq + Hash>(
    seen: &mut HashSet<T>,
    item: T,
    span: Span,
    message: impl FnOnce() -> String,
    report: &mut ValidationReport,
) -> bool {
    let first = seen.insert(item);
    if !first {
        push(report, DiagnosticKind::DuplicateRemodelEntry, span, message());
    }
    first
}

/// The children of `parent` called `name`, in declared order.
pub(super) fn children_named<'a>(parent: &'a RawNode, name: &'a str) -> impl Iterator<Item = &'a RawNode> + 'a {
    parent.children.iter().filter(move |c| c.name == name)
}

/// Every attribute of `node` outside `allowed` is `UnknownRemodelAttribute`;
/// one written twice is `DuplicateRemodelEntry`.
pub(super) fn check_attrs(node: &RawNode, allowed: &[&str], report: &mut ValidationReport) {
    let mut seen = HashSet::new();
    for (key, _) in &node.attrs {
        let message = || format!("'{}' has the attribute '{key}' more than once", node.name);
        if !first_occurrence(&mut seen, key.as_str(), node.span, message, report) {
            continue;
        }
        if !allowed.contains(&key.as_str()) {
            push(
                report,
                DiagnosticKind::UnknownRemodelAttribute,
                node.span,
                format!("'{}' has an unknown attribute '{key}'; {}", node.name, expected_list(allowed)),
            );
        }
    }
}

/// Every child of `node` outside `allowed` is `UnknownChildNode`.
pub(super) fn check_children(node: &RawNode, allowed: &[&str], report: &mut ValidationReport) {
    for child in &node.children {
        if !allowed.contains(&child.name.as_str()) {
            push(
                report,
                DiagnosticKind::UnknownChildNode { component: node.name.clone(), child: child.name.clone() },
                child.span,
                format!(
                    "'{}' has an unknown child node '{}'; {}",
                    node.name,
                    child.name,
                    expected_list(allowed)
                ),
            );
        }
    }
}

/// The first child of `parent` called `name`; a further one is
/// `DuplicateRemodelEntry` at its own span.
pub(super) fn at_most_one<'a>(
    parent: &'a RawNode,
    name: &'a str,
    report: &mut ValidationReport,
) -> Option<&'a RawNode> {
    let mut found = children_named(parent, name);
    let first = found.next();
    for extra in found {
        push(
            report,
            DiagnosticKind::DuplicateRemodelEntry,
            extra.span,
            format!("'{}' has more than one '{name}' node", parent.name),
        );
    }
    first
}

/// [`at_most_one`], and a missing child is `MissingRequiredAttribute` at the
/// parent's span.
pub(super) fn exactly_one<'a>(
    parent: &'a RawNode,
    name: &'a str,
    report: &mut ValidationReport,
) -> Option<&'a RawNode> {
    let first = at_most_one(parent, name, report);
    if first.is_none() {
        push(
            report,
            DiagnosticKind::MissingRequiredAttribute,
            parent.span,
            format!("'{}' is missing its required '{name}' node", parent.name),
        );
    }
    first
}

/// A leaf: it takes no attribute and no child node.
pub(super) fn check_leaf(node: &RawNode, report: &mut ValidationReport) {
    check_attrs(node, &[], report);
    check_children(node, &[], report);
}

/// A node that takes no positional argument: any is `InvalidRemodelValue`.
pub(super) fn check_no_args(node: &RawNode, report: &mut ValidationReport) {
    if !node.args.is_empty() {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            format!("'{}' takes no argument", node.name),
        );
    }
}

/// `value` unless it is empty; an empty one is `InvalidRemodelValue` at
/// `span`, naming `what`.
pub(super) fn non_empty(value: String, what: &str, span: Span, report: &mut ValidationReport) -> Option<String> {
    if value.is_empty() {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            span,
            format!("{what} must not be empty"),
        );
        return None;
    }
    Some(value)
}

/// The node's one positional argument as a string, naming `what` in the
/// diagnostic when it is absent or not a string. A further argument is
/// `InvalidRemodelValue`.
pub(super) fn single_string(node: &RawNode, what: &str, report: &mut ValidationReport) -> Option<String> {
    if node.args.len() > 1 {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            format!("'{}' takes one argument, {what}, got {}", node.name, node.args.len()),
        );
    }
    match node.args.first() {
        None => {
            push(
                report,
                DiagnosticKind::MissingRequiredAttribute,
                node.span,
                format!("'{}' is missing its first argument, {what}", node.name),
            );
            None
        }
        Some(Value::String(s)) => Some(s.clone()),
        Some(other) => {
            push(
                report,
                DiagnosticKind::InvalidAttributeType,
                node.span,
                format!(
                    "'{}' first argument ({what}) must be a string, got {}",
                    node.name,
                    value_type_name(other)
                ),
            );
            None
        }
    }
}

/// Every positional argument as a string, in order; a non-string one is
/// `InvalidAttributeType` and left out.
pub(super) fn string_args(node: &RawNode, report: &mut ValidationReport) -> Vec<String> {
    let mut out = Vec::new();
    for (index, arg) in node.args.iter().enumerate() {
        match arg {
            Value::String(s) => out.push(s.clone()),
            other => push(
                report,
                DiagnosticKind::InvalidAttributeType,
                node.span,
                format!(
                    "'{}' argument {} must be a string, got {}",
                    node.name,
                    index + 1,
                    value_type_name(other)
                ),
            ),
        }
    }
    out
}

// ── The forms of a property ───────────────────────────────────────────────────

/// The forms a `property` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Form {
    Text,
    Uri,
    Date,
    Link,
    Label,
}

/// The `type` words a `property` supports, each with the form it gives. This
/// is the one place they are written: the lowering and the outline's silent
/// reading of a property both come here.
const TYPE_WORDS: [(&str, Form); 4] = [
    ("text", Form::Text),
    ("uri", Form::Uri),
    ("date", Form::Date),
    ("link", Form::Link),
];

/// The supported `type` words, for a diagnostic.
pub(super) fn type_words() -> Vec<&'static str> {
    TYPE_WORDS.iter().map(|(word, _)| *word).collect()
}

/// The form the `type` word `word` gives; `None` for a word that is none of
/// the supported ones.
pub(super) fn form_of_type(word: &str) -> Option<Form> {
    let mut words = TYPE_WORDS.iter();
    words.find(|(known, _)| *known == word).map(|(_, form)| *form)
}

/// The form the source attributes of `property` give, read silently. `None`
/// for a source the lowering refuses: an `iri` beside a `from` or neither, an
/// `iri` that is no string or empty, an `iri` without a supported `type` word,
/// a `from` that is not `label`, and a `type` beside `from`.
///
/// A reference to a property (an `id`'s `property`, a `via`) asks this, so
/// that one whose source is refused resolves without a second diagnostic. A
/// property the lowering accepts has the form this gives.
pub(super) fn form_of_source(property: &RawNode) -> Option<Form> {
    let attr = |name: &str| get_attr(property, name);
    match (attr("iri"), attr("from"), attr("type")) {
        (Some(Value::String(iri)), None, Some(Value::String(word))) if !iri.is_empty() => form_of_type(&word),
        (None, Some(Value::String(word)), None) if word == "label" => Some(Form::Label),
        _ => None,
    }
}

// ── What a class has declared ─────────────────────────────────────────────────

/// What one class has declared so far: its `property` iris, its CPE property
/// names (of every node that yields a property) and its curated keys.
///
/// Its fields are private: [`declare_iri`], [`declare_name`] and
/// [`declare_key`] are the only writers, so no node can take a name or a key
/// past the checks they hold.
pub(super) struct ClassSeen {
    iris: HashSet<String>,
    names: HashSet<String>,
    keys: HashSet<String>,
    /// Whether the block has an `images` node, which reserves the property
    /// name [`IMAGE_LINK`].
    reserves_image: bool,
}

impl ClassSeen {
    /// The record of a class that has declared nothing yet, in a block that
    /// has an `images` node or has none.
    pub(super) fn new(block_has_images: bool) -> Self {
        Self {
            iris: HashSet::new(),
            names: HashSet::new(),
            keys: HashSet::new(),
            reserves_image: block_has_images,
        }
    }
}

/// Records `iri` as the source of a `property` of the class `seen` belongs
/// to; an iri that class already reads is `DuplicateRemodelEntry` at `span`.
pub(super) fn declare_iri(seen: &mut ClassSeen, iri: &str, span: Span, report: &mut ValidationReport) {
    let message = || format!("the class already has a property with iri '{iri}'");
    first_occurrence(&mut seen.iris, iri.to_owned(), span, message, report);
}

/// Records `name` as a CPE property name of the class `seen` belongs to; a
/// name that class already has is `DuplicateRemodelEntry` at `span`, and the
/// reserved name is `ReservedRemodelName`.
pub(super) fn declare_name(seen: &mut ClassSeen, name: &str, span: Span, report: &mut ValidationReport) {
    if seen.reserves_image && name == IMAGE_LINK {
        push(
            report,
            DiagnosticKind::ReservedRemodelName,
            span,
            format!("the property name '{IMAGE_LINK}' is reserved in a block with an 'images' node"),
        );
    }
    let message = || format!("the class already has a property named '{name}'");
    first_occurrence(&mut seen.names, name.to_owned(), span, message, report);
}

/// Records `key` as declared on the class `seen` belongs to; a key that class
/// already declared is `DuplicateRemodelEntry` at `span`. Returns the typed key.
pub(super) fn declare_key(seen: &mut ClassSeen, key: String, span: Span, report: &mut ValidationReport) -> CuratedKey {
    let message = || format!("the class already declares the curated key '{key}'");
    first_occurrence(&mut seen.keys, key.clone(), span, message, report);
    CuratedKey::new(key)
}

// ── Attribute readers ─────────────────────────────────────────────────────────

/// An optional attribute holding a curated key: `Ok(None)` when absent,
/// `Err(())` when unreadable (already reported).
pub(super) fn key_attr(
    node: &RawNode,
    attr: &str,
    seen: &mut ClassSeen,
    report: &mut ValidationReport,
) -> Result<Option<CuratedKey>, ()> {
    match optional_string_attr(node, attr, report)? {
        None => Ok(None),
        Some(key) => {
            let what = format!("a {}'s {attr}", node.name);
            let key = non_empty(key, &what, node.span, report).ok_or(())?;
            Ok(Some(declare_key(seen, key, node.span, report)))
        }
    }
}

/// `word` as one of `words`; any other word is `InvalidRemodelValue`.
pub(super) fn closed_word<T: Copy>(
    word: &str,
    attr: &str,
    words: &[(&str, T)],
    node: &RawNode,
    report: &mut ValidationReport,
) -> Option<T> {
    let found = words.iter().find(|(w, _)| *w == word).map(|(_, t)| *t);
    if found.is_none() {
        let expected: Vec<&str> = words.iter().map(|(w, _)| *w).collect();
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            format!(
                "'{}' attribute '{attr}' is '{word}'; expected one of {}",
                node.name,
                expected.join(", ")
            ),
        );
    }
    found
}

/// An optional attribute holding one word of `words`: `Ok(None)` when absent,
/// `Err(())` when unreadable or not one of them (already reported).
pub(super) fn word_attr<T: Copy>(
    node: &RawNode,
    attr: &str,
    words: &[(&str, T)],
    report: &mut ValidationReport,
) -> Result<Option<T>, ()> {
    match optional_string_attr(node, attr, report)? {
        None => Ok(None),
        Some(word) => closed_word(&word, attr, words, node, report).map(Some).ok_or(()),
    }
}

/// `name`'s position among `declared`.
pub(super) fn position_of(declared: &[Option<String>], name: &str) -> Option<usize> {
    declared.iter().position(|d| d.as_deref() == Some(name))
}

/// An optional attribute naming one of `declared`: `Ok(None)` when absent,
/// `Err(())` when unreadable, empty or naming none (already reported).
pub(super) fn reference_attr(
    node: &RawNode,
    attr: &str,
    what: &str,
    declared: &[Option<String>],
    report: &mut ValidationReport,
) -> Result<Option<usize>, ()> {
    let Some(name) = optional_string_attr(node, attr, report)? else {
        return Ok(None);
    };
    let name = non_empty(name, &format!("a {}'s {attr}", node.name), node.span, report).ok_or(())?;
    match position_of(declared, &name) {
        Some(index) => Ok(Some(index)),
        None => {
            push(
                report,
                DiagnosticKind::UnknownRemodelReference,
                node.span,
                format!("'{}' names '{name}' as its {attr}, which is no declared {what}", node.name),
            );
            Err(())
        }
    }
}
