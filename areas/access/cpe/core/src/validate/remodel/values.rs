//! The lowering of a class's `property` and `curated` nodes: the value rules.
//!
//! It imports nothing from `validate::remodel`; whatever of the block it needs
//! (the sentinel, the class and list names) arrives as a [`Scope`], and the
//! names and keys a class has seen so far as a [`ClassSeen`].
//!
//! `validate/remodel/structure.rs` reads a `property`'s names, `type`, `from`,
//! `max` and `target` a second time, silently, to resolve a reference to the
//! property. What this module refuses of those must stay faulty there.

use super::super::attrs::{get_bool_attr, get_typed_integer_attr, has_attr, optional_string_attr, require_string_attr};
use super::nodes::{
    check_attrs, check_children, closed_word, declare_iri, declare_key, declare_name, form_of_source, form_of_type,
    key_attr, non_empty, push, reference_attr, single_string, string_args, type_words, word_attr, ClassSeen, Form,
};
use crate::config::Component as RawNode;
use crate::diagnostic::{DiagnosticKind, Span, ValidationReport};
use crate::remodel::{
    ArchivePropertyIri, ClassIndex, CpeProperty, CuratedForm, EmptyText, ListIndex, PropertySource, RemodelCurated,
    RemodelProperty, SentinelWhole, SentinelWithin, TextRule, ValueCount,
};

/// The `type` words of the port's value kinds that `property` does not support
/// yet.
pub(super) const UNSUPPORTED_TYPES: [&str; 6] = ["integer", "decimal", "boolean", "list-node", "geometry", "color"];

/// What the nodes of a class need to know of the block around them.
pub(super) struct Scope<'a> {
    /// Whether the block has a `sentinel` node.
    pub(super) has_sentinel: bool,
    /// Each `class` node's name in declared order (`None` where unreadable).
    pub(super) class_names: &'a [Option<String>],
    /// Each `list` node's name in declared order (`None` where unreadable).
    pub(super) list_names: &'a [Option<String>],
}

/// How a diagnostic names `form`.
fn describe(form: Form) -> &'static str {
    match form {
        Form::Text => "type=\"text\"",
        Form::Uri => "type=\"uri\"",
        Form::Date => "type=\"date\"",
        Form::Link => "type=\"link\"",
        Form::Label => "from=\"label\"",
    }
}

const ARCHIVE_FORMS: &[Form] = &[Form::Text, Form::Uri, Form::Date, Form::Link];

/// Which forms take each attribute of a `property` beyond its source
/// (`iri`, `from`). An attribute present on a node whose form is not listed
/// is `ConflictingRemodelAttributes`; nothing else decides it.
const FORM_TABLE: &[(&str, &[Form])] = &[
    ("type", ARCHIVE_FORMS),
    ("curated-key", &[Form::Text, Form::Label]),
    ("empty", &[Form::Text]),
    ("min", ARCHIVE_FORMS),
    ("max", ARCHIVE_FORMS),
    ("sentinel", &[Form::Text, Form::Uri]),
    ("sentinel-within", &[Form::Text, Form::Uri]),
    ("exclude-label", &[Form::Text]),
    ("display-key", &[Form::Date]),
    ("target", &[Form::Link]),
];

/// An optional attribute holding a count: `Ok(None)` when absent, `Err(())`
/// when unreadable, negative or above `u32::MAX` (already reported).
fn count_attr(node: &RawNode, attr: &str, report: &mut ValidationReport) -> Result<Option<u32>, ()> {
    if !has_attr(node, attr) {
        return Ok(None);
    }
    let n = get_typed_integer_attr(node, attr, report).ok_or(())?;
    match u32::try_from(n) {
        Ok(n) => Ok(Some(n)),
        Err(_) => {
            let bound = if n < 0 {
                "must not be negative".to_owned()
            } else {
                format!("must be at most {}", u32::MAX)
            };
            push(
                report,
                DiagnosticKind::InvalidRemodelValue,
                node.span,
                format!("'{}' attribute '{attr}' {bound}, got {n}", node.name),
            );
            Err(())
        }
    }
}

// ── property ──────────────────────────────────────────────────────────────────

pub(super) fn lower_property(
    node: &RawNode,
    scope: &Scope<'_>,
    seen: &mut ClassSeen,
    report: &mut ValidationReport,
) -> Option<RemodelProperty> {
    let allowed: Vec<&str> = ["iri", "from"]
        .into_iter()
        .chain(FORM_TABLE.iter().map(|(attr, _)| *attr))
        .collect();
    check_attrs(node, &allowed, report);
    check_children(node, &[], report);

    let property_names = string_args(node, report);
    if node.args.is_empty() {
        push(
            report,
            DiagnosticKind::MissingRequiredAttribute,
            node.span,
            "'property' needs at least one CPE name".to_owned(),
        );
    }
    for name in &property_names {
        if name.is_empty() {
            push(
                report,
                DiagnosticKind::InvalidRemodelValue,
                node.span,
                "a property's CPE name must not be empty".to_owned(),
            );
            continue;
        }
        declare_name(seen, name, node.span, report);
    }

    let Source { iri, form, candidates } = read_source(node, seen, report);
    // An attribute is read only where a form the node may have takes it: on
    // any other form it is reported once, as an attribute the form does not
    // take, and what it holds is not judged. A node that names no form at all
    // is held to no form, and every attribute it has is read.
    let taken = |attr: &str| {
        let forms = FORM_TABLE.iter().find(|(known, _)| *known == attr);
        let takes = |(_, forms): &(&str, &[Form])| candidates.iter().any(|f| forms.contains(f));
        candidates.is_empty() || forms.is_some_and(takes)
    };
    let curated_key = match taken("curated-key") {
        true => key_attr(node, "curated-key", seen, report),
        false => Ok(None),
    };
    let display_key = match taken("display-key") {
        true => key_attr(node, "display-key", seen, report),
        false => Ok(None),
    };
    let count = match taken("min") || taken("max") {
        true => read_count(node, report),
        false => ValueCount::new(0, None),
    };
    let empty = match taken("empty") {
        true => word_attr(
            node,
            "empty",
            &[("drop", EmptyText::Drop), ("refuse", EmptyText::Refuse)],
            report,
        ),
        false => Ok(None),
    };
    let sentinels = match taken("sentinel") || taken("sentinel-within") {
        true => read_sentinels(node, scope, report),
        false => Some((SentinelWhole::Refuse, SentinelWithin::Refuse)),
    };
    let exclude_label = taken("exclude-label") && get_bool_attr(node, "exclude-label", report);
    if taken("exclude-label")
        && taken("curated-key")
        && has_attr(node, "exclude-label")
        && has_attr(node, "curated-key")
    {
        push(
            report,
            DiagnosticKind::ConflictingRemodelAttributes,
            node.span,
            "'property' has both 'exclude-label' and 'curated-key'".to_owned(),
        );
    }
    let target = match taken("target") {
        true => reference_attr(node, "target", "class", scope.class_names, report).map(|index| index.map(ClassIndex)),
        false => Ok(None),
    };

    let every_name_read = !property_names.is_empty()
        && property_names.len() == node.args.len()
        && property_names.iter().all(|n| !n.is_empty());
    if !every_name_read {
        return None;
    }
    let form = form?;
    // The outline reads the same source silently: where it gives a form, it
    // is this one.
    debug_assert!(!matches!(form_of_source(node), Some(silent) if silent != form));
    let source = match form {
        Form::Label => PropertySource::Label { curated_key: curated_key.ok()? },
        Form::Text => {
            let (sentinel, sentinel_within) = sentinels?;
            PropertySource::Text {
                iri: ArchivePropertyIri::new(iri?),
                count: count?,
                rule: match (curated_key.ok()?, exclude_label) {
                    // Both together: reported above.
                    (Some(_), true) => return None,
                    (Some(key), false) => TextRule::CuratedOverride(key),
                    (None, true) => TextRule::ExcludeLabel,
                    (None, false) => TextRule::Plain,
                },
                empty: empty.ok()?.unwrap_or(EmptyText::Drop),
                sentinel,
                sentinel_within,
            }
        }
        Form::Uri => {
            let (sentinel, sentinel_within) = sentinels?;
            PropertySource::Uri {
                iri: ArchivePropertyIri::new(iri?),
                count: count?,
                sentinel,
                sentinel_within,
            }
        }
        Form::Date => PropertySource::Date {
            iri: ArchivePropertyIri::new(iri?),
            count: count?,
            display_key: display_key.ok()?,
        },
        Form::Link => PropertySource::Link {
            iri: ArchivePropertyIri::new(iri?),
            count: count?,
            target: target.ok()??,
        },
    };
    Some(RemodelProperty {
        names: property_names.into_iter().map(CpeProperty::new).collect(),
        source,
    })
}

/// What a `property` node's source attributes (`iri`, `type`, `from`) give.
struct Source {
    /// The archive IRI, when one is written and not empty.
    iri: Option<String>,
    /// The node's form; `None` when the source is faulty (already reported).
    form: Option<Form>,
    /// The forms the node's other attributes are held to: its form, or, when
    /// its source is faulty, every form its source attributes name. Empty for
    /// a node that names no form at all.
    candidates: Vec<Form>,
}

/// Reads the source of a `property`, an `iri` with a `type` or `from`, and
/// holds the node's other attributes to the form table.
fn read_source(node: &RawNode, seen: &mut ClassSeen, report: &mut ValidationReport) -> Source {
    let iri_attr = optional_string_attr(node, "iri", report);
    // A `type` beside `from` alone is an attribute the label does not take:
    // it is reported as that, below, and its word is not judged.
    let type_attr = match has_attr(node, "from") && !has_attr(node, "iri") {
        true => Ok(None),
        false => optional_string_attr(node, "type", report),
    };
    let from_attr = word_attr(node, "from", &[("label", ())], report);

    let iri_readable = matches!(iri_attr, Ok(Some(_)));
    let iri = match iri_attr {
        Ok(Some(iri)) => non_empty(iri, "a property's iri", node.span, report)
            .inspect(|iri| declare_iri(seen, iri, node.span, report)),
        Ok(None) | Err(()) => None,
    };
    let kind = match &type_attr {
        Ok(Some(word)) => lower_property_type(word, node.span, report),
        Ok(None) | Err(()) => None,
    };
    let form = match (has_attr(node, "iri"), has_attr(node, "from")) {
        (true, true) => {
            push(
                report,
                DiagnosticKind::ConflictingRemodelAttributes,
                node.span,
                "'property' has both an 'iri' and a 'from'; it takes one source".to_owned(),
            );
            None
        }
        (false, false) => {
            push(
                report,
                DiagnosticKind::ConflictingRemodelAttributes,
                node.span,
                "'property' has no source; give it an 'iri' with a 'type', or from=\"label\"".to_owned(),
            );
            None
        }
        (true, false) => match type_attr {
            Ok(Some(_)) => kind,
            Ok(None) => {
                if iri_readable {
                    push(
                        report,
                        DiagnosticKind::ConflictingRemodelAttributes,
                        node.span,
                        "'property' has an 'iri' but no 'type'".to_owned(),
                    );
                }
                None
            }
            Err(()) => None,
        },
        (false, true) => from_attr.ok().flatten().map(|()| Form::Label),
    };

    // The forms the attributes are held to: the node's form, or, when its
    // source is faulty, every form its source attributes name. An attribute
    // none of them takes is reported beside the source's own error.
    let candidates: Vec<Form> = match form {
        Some(form) => vec![form],
        None => kind.into_iter().chain(has_attr(node, "from").then_some(Form::Label)).collect(),
    };
    if !candidates.is_empty() {
        let described: Vec<&str> = candidates.iter().map(|form| describe(*form)).collect();
        for (attr, forms) in FORM_TABLE {
            if has_attr(node, attr) && !candidates.iter().any(|form| forms.contains(form)) {
                push(
                    report,
                    DiagnosticKind::ConflictingRemodelAttributes,
                    node.span,
                    format!("a property with {} does not take '{attr}'", described.join(" or ")),
                );
            }
        }
    }
    if form == Some(Form::Link) && !has_attr(node, "target") {
        push(
            report,
            DiagnosticKind::ConflictingRemodelAttributes,
            node.span,
            "a property with type=\"link\" needs a 'target'".to_owned(),
        );
    }
    Source { iri, form, candidates }
}

/// A `property`'s `min` and `max` as its count; `None` when either is
/// unreadable or the `max` is below the `min` (already reported).
fn read_count(node: &RawNode, report: &mut ValidationReport) -> Option<ValueCount> {
    let min = count_attr(node, "min", report);
    let max = count_attr(node, "max", report);
    let (Ok(min), Ok(max)) = (min, max) else {
        return None;
    };
    let min = min.unwrap_or(0);
    let count = ValueCount::new(min, max);
    if count.is_none() {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            format!("'property' has a 'max' below its 'min' of {min}"),
        );
    }
    count
}

/// A `property`'s `sentinel` and `sentinel-within`, each with its default;
/// `None` when either is unreadable (already reported). Either one in a
/// block without a `sentinel` node is `ConflictingRemodelAttributes`.
fn read_sentinels(
    node: &RawNode,
    scope: &Scope<'_>,
    report: &mut ValidationReport,
) -> Option<(SentinelWhole, SentinelWithin)> {
    let sentinel = word_attr(
        node,
        "sentinel",
        &[
            ("refuse", SentinelWhole::Refuse),
            ("absent", SentinelWhole::Absent),
            ("keep", SentinelWhole::Keep),
        ],
        report,
    );
    let sentinel_within = word_attr(
        node,
        "sentinel-within",
        &[("refuse", SentinelWithin::Refuse), ("keep", SentinelWithin::Keep)],
        report,
    );
    for attr in ["sentinel", "sentinel-within"] {
        if has_attr(node, attr) && !scope.has_sentinel {
            push(
                report,
                DiagnosticKind::ConflictingRemodelAttributes,
                node.span,
                format!("'property' has '{attr}' but the block has no 'sentinel' node"),
            );
        }
    }
    Some((
        sentinel.ok()?.unwrap_or(SentinelWhole::Refuse),
        sentinel_within.ok()?.unwrap_or(SentinelWithin::Refuse),
    ))
}

/// The form the `type` word `word` gives; a word that gives none is reported
/// as not supported yet or as unknown.
fn lower_property_type(word: &str, span: Span, report: &mut ValidationReport) -> Option<Form> {
    let form = form_of_type(word);
    if form.is_none() {
        let (kind, message) = if UNSUPPORTED_TYPES.contains(&word) {
            let message = format!("property type '{word}' is not supported yet");
            (DiagnosticKind::UnsupportedRemodelType, message)
        } else {
            let expected = type_words().join(", ");
            let message = format!("unknown property type '{word}'; expected one of {expected}");
            (DiagnosticKind::UnknownRemodelType, message)
        };
        push(report, kind, span, message);
    }
    form
}

// ── curated ───────────────────────────────────────────────────────────────────

/// The forms of a `curated` node's `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CuratedKind {
    Text,
    List,
    PerLanguage,
}

pub(super) fn lower_curated(
    node: &RawNode,
    scope: &Scope<'_>,
    seen: &mut ClassSeen,
    report: &mut ValidationReport,
) -> Option<RemodelCurated> {
    check_attrs(node, &["key", "type", "list", "min"], report);
    check_children(node, &[], report);

    let name = single_string(node, "the property's CPE name", report)
        .and_then(|n| non_empty(n, "a curated property's CPE name", node.span, report));
    if let Some(name) = &name {
        declare_name(seen, name, node.span, report);
    }

    let key = require_string_attr(node, "key", report)
        .and_then(|k| non_empty(k, "a curated property's key", node.span, report))
        .map(|k| declare_key(seen, k, node.span, report));

    let kind = require_string_attr(node, "type", report).and_then(|word| {
        closed_word(
            &word,
            "type",
            &[
                ("text", CuratedKind::Text),
                ("list", CuratedKind::List),
                ("per-language", CuratedKind::PerLanguage),
            ],
            node,
            report,
        )
    });

    // A `list` beside another type is reported as that, below, and what it
    // names is not judged.
    let list = match kind {
        Some(CuratedKind::Text | CuratedKind::PerLanguage) => Ok(None),
        Some(CuratedKind::List) | None => {
            reference_attr(node, "list", "list", scope.list_names, report).map(|index| index.map(ListIndex))
        }
    };
    match (kind, has_attr(node, "list")) {
        (Some(CuratedKind::List), false) => push(
            report,
            DiagnosticKind::ConflictingRemodelAttributes,
            node.span,
            "a curated property with type=\"list\" needs a 'list'".to_owned(),
        ),
        (Some(CuratedKind::Text | CuratedKind::PerLanguage), true) => push(
            report,
            DiagnosticKind::ConflictingRemodelAttributes,
            node.span,
            "a curated property takes 'list' only with type=\"list\"".to_owned(),
        ),
        _ => {}
    }

    let required = match count_attr(node, "min", report) {
        Ok(None | Some(0)) => Ok(false),
        Ok(Some(1)) => Ok(true),
        Ok(Some(other)) => {
            push(
                report,
                DiagnosticKind::InvalidRemodelValue,
                node.span,
                format!("a curated property's 'min' is 0 or 1, got {other}"),
            );
            Err(())
        }
        Err(()) => Err(()),
    };

    let form = match kind? {
        CuratedKind::Text => CuratedForm::Text,
        CuratedKind::PerLanguage => CuratedForm::PerLanguage,
        CuratedKind::List => CuratedForm::List(list.ok()??),
    };
    Some(RemodelCurated {
        name: CpeProperty::new(name?),
        key: key?,
        form,
        required: required.ok()?,
    })
}
