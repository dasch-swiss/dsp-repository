//! Lowering of a project's `remodel { … }` block into a [`Remodel`].
//!
//! Config-internal: it needs no store, so `validate_inner` calls it before the
//! `validate_with_data` fork and it reaches both arms. Every check appends to
//! the report and nothing returns early, so one pass reports every defect of
//! the block; the typed value is returned only when no error was pushed.
//!
//! Every diagnostic is located at the offending node's span: a `Component`
//! holds one span per node and none per attribute.
//!
//! To add a node to the block, change all of these together:
//! - the typed model in `core/src/remodel.rs`;
//! - a `lower_*` fn, which appends every defect and never returns early. It lives in this file or,
//!   for a node of a feature group that has one, in that group's child module
//!   (`validate/remodel/values.rs` holds a class's `property` and `curated` nodes,
//!   `validate/remodel/structure.rs` its `id`, `member-of`, `member-count`, `incoming-count` and
//!   `cover` nodes, `validate/remodel/hook.rs` the `hook` node, which is lowered after the classes
//!   and enters its keys and names into their `ClassSeen`);
//! - for a node that yields a property or reads a curated key, its entry in the class's
//!   `ClassSeen`, made through `declare_name` or `declare_key` (the `declare_*` fns are its only
//!   writers), so the name or key cannot be taken twice;
//! - for a node that adds to `RemodelClass::properties`, the lookup `Outline::property` in
//!   `validate/remodel/structure.rs`, which takes a `property` node's position among its class's
//!   `property` nodes for its index in that list;
//! - for a new child module, its row in `MODULES` in `tools/module-graph.py`, in the commit that
//!   adds the file;
//! - the parent's child allow-list (its `check_children` call), or the node is refused as an
//!   unknown child, and the arm that hands the node to its `lower_*` fn (for a child of `class`: in
//!   `lower_class`, or in `ClassStructure::lower_child`), or the node is allowed and never read;
//! - a `DiagnosticKind` in `core/src/diagnostic.rs` for each new defect, with its severity arm in
//!   the same file;
//! - the grammar table and the validation list in `docs/CONFIG_LANGUAGE.md`;
//! - `CONTEXT.md`, when the node introduces a new noun.
//!
//! The node checks and readers every `lower_*` fn shares live in
//! `validate/remodel/nodes.rs`; a child module of `validate::remodel` imports
//! them from there and nothing from its parent.

mod hook;
mod nodes;
mod structure;
mod values;

use std::collections::HashSet;

use hook::lower_hook;
use nodes::{
    at_most_one, check_attrs, check_children, check_leaf, check_no_args, children_named, exactly_one, first_occurrence,
    non_empty, push, single_string, string_args, ClassSeen,
};
use structure::{ClassStructure, Outline, Structure};
use values::{lower_curated, lower_property, Scope};

use super::attrs::{get_bool_attr, require_string_attr};
use crate::config::Component as RawNode;
use crate::diagnostic::{DiagnosticKind, Severity, ValidationReport};
use crate::ir::Value;
use crate::remodel::{
    is_strip_space, ArchiveClassIri, ClassIndex, CpeClass, Remodel, RemodelAnnotations, RemodelClass, RemodelCurated,
    RemodelImages, RemodelLabel, RemodelList, RemodelListNode, RemodelProperty, Shortcode,
};

/// Lower the top-level `remodel` node `raw`.
///
/// `languages` are the site's declared languages, read silently from the raw
/// config. When the slice is empty no list reports a per-language diagnostic
/// and the labels are lowered as written. The mapping keeps them, in this
/// order, as `Remodel::languages`.
///
/// Returns `Some` only when this call pushed no error.
pub(crate) fn lower(raw: &RawNode, languages: &[&str], report: &mut ValidationReport) -> Option<Remodel> {
    let errors_before = error_count(report);

    check_attrs(raw, &[], report);
    check_no_args(raw, report);
    check_children(
        raw,
        &[
            "shortcode",
            "sentinel",
            "hook",
            "list",
            "class",
            "images",
            "annotations",
        ],
        report,
    );

    let shortcode = exactly_one(raw, "shortcode", report).and_then(|n| lower_shortcode(n, report));
    let sentinel = at_most_one(raw, "sentinel", report).and_then(|n| lower_sentinel(n, report));
    let lists = lower_lists(raw, languages, report);
    let (images_class, images) =
        at_most_one(raw, "images", report).map_or((None, None), |n| lower_images(n, classes_before(raw, n), report));
    let class_names = declared_names(raw, "class");
    let list_names = declared_names(raw, "list");
    let scope = Scope {
        has_sentinel: children_named(raw, "sentinel").next().is_some(),
        class_names: &class_names,
        list_names: &list_names,
    };
    let outline = Outline::read(raw, &class_names);
    let (classes, mut seen) = lower_classes(raw, images_class.as_deref(), &scope, &outline, report);
    let annotations = at_most_one(raw, "annotations", report).and_then(|n| lower_annotations(n, &class_names, report));
    // After the classes: a key or a name the hook declares is held against
    // what its class has declared.
    let hook = at_most_one(raw, "hook", report).and_then(|n| lower_hook(n, &outline, &mut seen, report));

    if error_count(report) > errors_before {
        return None;
    }
    Some(Remodel {
        shortcode: shortcode?,
        sentinel,
        languages: languages.iter().map(|&language| language.to_owned()).collect(),
        lists: lists?,
        classes: classes?,
        images,
        annotations,
        hook,
    })
}

fn error_count(report: &ValidationReport) -> usize {
    report.iter().filter(|d| d.severity == Severity::Error).count()
}

/// A leaf node holding one string: no attributes, no children.
fn lower_string_leaf(node: &RawNode, report: &mut ValidationReport) -> Option<String> {
    check_leaf(node, report);
    single_string(node, "its text", report)
}

/// [`lower_string_leaf`] for a text that must not be empty.
fn lower_nonempty_leaf(node: &RawNode, report: &mut ValidationReport) -> Option<String> {
    let value = lower_string_leaf(node, report)?;
    non_empty(value, &format!("'{}'", node.name), node.span, report)
}

fn lower_shortcode(node: &RawNode, report: &mut ValidationReport) -> Option<Shortcode> {
    let value = lower_string_leaf(node, report)?;
    let valid = value.chars().count() == 4 && value.chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c));
    if !valid {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            format!("shortcode '{value}' must be four characters of 0-9 and A-F"),
        );
        return None;
    }
    Some(Shortcode::new(value))
}

fn lower_sentinel(node: &RawNode, report: &mut ValidationReport) -> Option<String> {
    let value = lower_string_leaf(node, report)?;
    if value.is_empty() {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            "sentinel must not be empty".to_owned(),
        );
        return None;
    }
    // A text is compared with the sentinel once the whitespace around the text
    // is cut: a sentinel that starts or ends with any of it would match no text.
    if value.starts_with(is_strip_space) || value.ends_with(is_strip_space) {
        push(
            report,
            DiagnosticKind::InvalidRemodelValue,
            node.span,
            "sentinel must not start or end with whitespace".to_owned(),
        );
        return None;
    }
    Some(value)
}

// ── Lists ─────────────────────────────────────────────────────────────────────

fn lower_lists(raw: &RawNode, languages: &[&str], report: &mut ValidationReport) -> Option<Vec<RemodelList>> {
    let mut seen = HashSet::new();
    let lists: Vec<Option<RemodelList>> = children_named(raw, "list")
        .map(|node| lower_list(node, languages, &mut seen, report))
        .collect();
    lists.into_iter().collect()
}

fn lower_list(
    node: &RawNode,
    languages: &[&str],
    seen: &mut HashSet<String>,
    report: &mut ValidationReport,
) -> Option<RemodelList> {
    check_attrs(node, &[], report);
    check_children(node, &["list-node"], report);

    let name =
        single_string(node, "the list's name", report).and_then(|n| non_empty(n, "the list's name", node.span, report));
    if let Some(name) = &name {
        let message = || format!("list '{name}' is declared more than once");
        first_occurrence(seen, name.clone(), node.span, message, report);
    }

    let list_nodes: Vec<&RawNode> = children_named(node, "list-node").collect();
    if list_nodes.is_empty() {
        push(
            report,
            DiagnosticKind::MissingRequiredAttribute,
            node.span,
            "'list' needs at least one 'list-node'".to_owned(),
        );
    }
    let mut slugs = HashSet::new();
    let nodes: Vec<Option<RemodelListNode>> = list_nodes
        .into_iter()
        .map(|n| lower_list_node(n, languages, &mut slugs, report))
        .collect();
    let nodes: Option<Vec<RemodelListNode>> = nodes.into_iter().collect();

    Some(RemodelList { name: name?, nodes: nodes? })
}

fn lower_list_node(
    node: &RawNode,
    languages: &[&str],
    slugs: &mut HashSet<String>,
    report: &mut ValidationReport,
) -> Option<RemodelListNode> {
    check_attrs(node, &[], report);

    let slug =
        single_string(node, "the node's slug", report).and_then(|s| non_empty(s, "the node's slug", node.span, report));
    if let Some(slug) = &slug {
        let message = || format!("list-node '{slug}' is declared more than once in its list");
        first_occurrence(slugs, slug.clone(), node.span, message, report);
    }

    let mut seen_languages: HashSet<&str> = HashSet::new();
    let mut labels = Vec::new();
    let mut complete = true;
    for child in &node.children {
        check_leaf(child, report);
        let message = || format!("list-node has more than one label in language '{}'", child.name);
        if !first_occurrence(&mut seen_languages, child.name.as_str(), child.span, message, report) {
            continue;
        }
        if !languages.is_empty() && !languages.contains(&child.name.as_str()) {
            push(
                report,
                DiagnosticKind::UnknownRemodelReference,
                child.span,
                format!("list-node label language '{}' is not a language of the site", child.name),
            );
        }
        let text = single_string(child, "the label", report).and_then(|t| non_empty(t, "a label", child.span, report));
        match text {
            Some(text) => labels.push(RemodelLabel { language: child.name.clone(), text }),
            None => complete = false,
        }
    }
    for language in languages {
        if !seen_languages.contains(language) {
            push(
                report,
                DiagnosticKind::IncompleteListTranslation,
                node.span,
                format!(
                    "list-node {} has no label in language '{language}'",
                    slug.as_deref().unwrap_or("(unnamed)")
                ),
            );
        }
    }

    if !complete {
        return None;
    }
    Some(RemodelListNode { slug: slug?, labels })
}

// ── Classes and properties ────────────────────────────────────────────────────

/// The name of each `node_name` child of `raw` in declared order (`None` where it
/// is unreadable or empty), read silently. A reference to a `class` or `list`
/// resolves against it, so one that failed to lower elsewhere still resolves
/// and causes no second diagnostic.
fn declared_names(raw: &RawNode, node_name: &str) -> Vec<Option<String>> {
    children_named(raw, node_name)
        .map(|node| match node.args.first() {
            Some(Value::String(name)) if !name.is_empty() => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// Lowers every `class`. Also returns what each class has declared, by its
/// position among the `class` nodes, whether or not it lowered.
fn lower_classes(
    raw: &RawNode,
    images_class: Option<&str>,
    scope: &Scope<'_>,
    outline: &Outline<'_>,
    report: &mut ValidationReport,
) -> (Option<Vec<RemodelClass>>, Vec<ClassSeen>) {
    let class_nodes: Vec<&RawNode> = children_named(raw, "class").collect();
    if class_nodes.is_empty() {
        push(
            report,
            DiagnosticKind::MissingRequiredAttribute,
            raw.span,
            "'remodel' needs at least one 'class' node".to_owned(),
        );
    }

    let mut names = HashSet::new();
    let mut iris = HashSet::new();
    let (classes, seen): (Vec<Option<RemodelClass>>, Vec<ClassSeen>) = class_nodes
        .into_iter()
        .enumerate()
        .map(|(index, node)| {
            let structure = ClassStructure::new(node, index, outline, report);
            lower_class(node, images_class, scope, structure, &mut names, &mut iris, report)
        })
        .unzip();
    (classes.into_iter().collect(), seen)
}

/// Lowers one `class`; `None` when it is faulty (already reported). Also
/// returns what the class has declared.
fn lower_class(
    node: &RawNode,
    images_class: Option<&str>,
    scope: &Scope<'_>,
    mut structure: ClassStructure<'_>,
    names: &mut HashSet<String>,
    iris: &mut HashSet<String>,
    report: &mut ValidationReport,
) -> (Option<RemodelClass>, ClassSeen) {
    check_attrs(node, &["iri", "no-addresses", "image"], report);
    check_children(
        node,
        &[
            "property",
            "curated",
            "id",
            "member-of",
            "member-count",
            "incoming-count",
            "cover",
        ],
        report,
    );
    let no_addresses = get_bool_attr(node, "no-addresses", report);

    let name = single_string(node, "the class's CPE name", report)
        .and_then(|n| non_empty(n, "the class's CPE name", node.span, report));
    let iri = require_string_attr(node, "iri", report).and_then(|i| non_empty(i, "the class's iri", node.span, report));

    if let Some(name) = &name {
        let message = || format!("class '{name}' is declared more than once");
        first_occurrence(names, name.clone(), node.span, message, report);
        if images_class == Some(name.as_str()) {
            push(
                report,
                DiagnosticKind::ReservedRemodelName,
                node.span,
                format!("class '{name}' is named like the 'images' block's class"),
            );
        }
    }
    if let Some(iri) = &iri {
        let message = || format!("class iri '{iri}' is declared more than once");
        first_occurrence(iris, iri.clone(), node.span, message, report);
    }

    // Every node that yields a property or reads a curated key shares the
    // class's one namespace of names and of keys, so the children are lowered
    // in declared order.
    let mut seen = ClassSeen::new(structure.block_has_images());
    let mut properties: Vec<Option<RemodelProperty>> = Vec::new();
    let mut curated: Vec<Option<RemodelCurated>> = Vec::new();
    for child in &node.children {
        match child.name.as_str() {
            "property" => properties.push(lower_property(child, scope, &mut seen, report)),
            "curated" => curated.push(lower_curated(child, scope, &mut seen, report)),
            _ => structure.lower_child(child, &mut seen, report),
        }
    }
    let properties: Option<Vec<RemodelProperty>> = properties.into_iter().collect();
    let curated: Option<Vec<RemodelCurated>> = curated.into_iter().collect();
    let class = match (name, iri, properties, curated, structure.finish(report)) {
        (Some(name), Some(iri), Some(properties), Some(curated), Some(structure)) => {
            let Structure { image, id, member_of, member_counts, incoming_counts } = structure;
            Some(RemodelClass {
                name: CpeClass::new(name),
                iri: ArchiveClassIri::new(iri),
                no_addresses,
                image,
                id,
                member_of,
                properties,
                curated,
                member_counts,
                incoming_counts,
            })
        }
        _ => None,
    };
    (class, seen)
}

// ── images and annotations ────────────────────────────────────────────────────

/// How many `class` children of `raw` are written before its child `node`.
fn classes_before(raw: &RawNode, node: &RawNode) -> usize {
    raw.children
        .iter()
        .take_while(|child| !std::ptr::eq(*child, node))
        .filter(|child| child.name == "class")
        .count()
}

/// Returns the `class=` name (so a `class` node can be checked against it even
/// when the rest of the block is faulty) and the lowered block, which stands
/// after `classes_before` classes.
fn lower_images(
    node: &RawNode,
    classes_before: usize,
    report: &mut ValidationReport,
) -> (Option<String>, Option<RemodelImages>) {
    check_attrs(node, &["class", "iiif-base"], report);
    check_children(node, &["authorship", "license"], report);

    check_no_args(node, report);
    let class =
        require_string_attr(node, "class", report).and_then(|c| non_empty(c, "the images class", node.span, report));
    let base = require_string_attr(node, "iiif-base", report);
    if let Some(base) = &base {
        let host_present = base
            .strip_prefix("https://")
            .is_some_and(|rest| !rest.is_empty() && !rest.starts_with('/'));
        if !(host_present && base.ends_with('/')) {
            push(
                report,
                DiagnosticKind::InvalidRemodelValue,
                node.span,
                format!("iiif-base '{base}' must be 'https://', a host and '/'"),
            );
        }
    }
    let authorship = exactly_one(node, "authorship", report).and_then(|n| lower_nonempty_leaf(n, report));
    let license = exactly_one(node, "license", report).and_then(|n| lower_nonempty_leaf(n, report));

    let images = match (&class, base, authorship, license) {
        (Some(class), Some(iiif_base), Some(authorship), Some(license)) => Some(RemodelImages {
            class: CpeClass::new(class.clone()),
            iiif_base,
            authorship,
            license,
            classes_before,
        }),
        _ => None,
    };
    (class, images)
}

fn lower_annotations(
    node: &RawNode,
    class_names: &[Option<String>],
    report: &mut ValidationReport,
) -> Option<RemodelAnnotations> {
    check_attrs(node, &[], report);
    check_no_args(node, report);
    check_children(node, &["on", "link-targets"], report);

    let on = at_most_one(node, "on", report).map_or(Some(Vec::new()), |n| lower_class_refs(n, class_names, report));
    let link_targets = at_most_one(node, "link-targets", report)
        .map_or(Some(Vec::new()), |n| lower_class_refs(n, class_names, report));
    Some(RemodelAnnotations { on: on?, link_targets: link_targets? })
}

/// The class names `node` lists, resolved to their position among the
/// `class` nodes. A name no class declares is `UnknownRemodelReference`.
fn lower_class_refs(
    node: &RawNode,
    class_names: &[Option<String>],
    report: &mut ValidationReport,
) -> Option<Vec<ClassIndex>> {
    check_leaf(node, report);

    if node.args.is_empty() {
        push(
            report,
            DiagnosticKind::MissingRequiredAttribute,
            node.span,
            format!("'{}' needs at least one class name", node.name),
        );
    }
    let mut indexes = Vec::new();
    let mut named = HashSet::new();
    for name in string_args(node, report) {
        let what = format!("a class name of '{}'", node.name);
        let Some(name) = non_empty(name, &what, node.span, report) else {
            continue;
        };
        let message = || format!("'{}' names '{name}' more than once", node.name);
        if !first_occurrence(&mut named, name.clone(), node.span, message, report) {
            continue;
        }
        match class_names.iter().position(|c| c.as_deref() == Some(name.as_str())) {
            Some(index) => indexes.push(ClassIndex(index)),
            None => {
                push(
                    report,
                    DiagnosticKind::UnknownRemodelReference,
                    node.span,
                    format!("'{}' names '{name}', which is no declared class", node.name),
                );
            }
        }
    }
    (indexes.len() == node.args.len()).then_some(indexes)
}

#[cfg(test)]
mod tests {
    use super::values::UNSUPPORTED_TYPES;
    use super::*;
    use crate::diagnostic::Span;
    use crate::remodel::{
        ArchivePropertyIri, CpeProperty, CuratedForm, CuratedKey, EmptyText, HookKey, HookOverrides, HookProvides,
        IdPolicy, IdSource, ImageRule, ListIndex, Membership, PropertyIndex, PropertySource, RemodelHook,
        RemodelIncomingCount, RemodelMemberCount, SentinelWhole, SentinelWithin, TextRule, ValueCount,
    };

    /// A block holding every node of the grammar, with two languages.
    const FULL_BLOCK: &str = r#"
remodel {
    shortcode "0803"
    sentinel "[missing]"
    hook {
        key "sort_key" class="Book"
        provides "side" class="Page"
        overrides "catalogue-uri" class="Book"
        drops class="Page"
    }
    list "office" {
        list-node "amerbach" { de "Johann Amerbach"; en "Johann Amerbach" }
        list-node "other" { en "Unnamed printer"; de "Ohne genannten Drucker" }
    }
    list "side" {
        list-node "left" { de "links"; en "left" }
    }
    class "Book" iri="http://example.org/onto#Book" {
        id curated-key="slug"
        property "alt-title" iri="http://example.org/onto#hasTitle" type="text"
        property "catalogue-uri" iri="http://example.org/onto#hasUri" type="uri"
        member-count "extent" of="Page"
        incoming-count "page-uses" from="Page" {
            via "strip"
        }
        cover curated-key="cover_page"
    }
    images class="Image" iiif-base="https://iiif.example.org/0803/" {
        authorship "Universitaetsbibliothek Basel"
        license "CC BY 4.0"
    }
    class "Page" iri="http://example.org/onto#Page" image=#true {
        member-of "Book" seq="seq-number"
        id from="property" property="page-id" under-parent=#true
        property "name" "page-id" iri="http://example.org/onto#hasPagenum" type="text" max=1
        property "strip" iri="http://example.org/onto#hasBand" type="link" target="Book"
        property "pub-date" iri="http://example.org/onto#hasPubdate" type="date"
    }
    annotations {
        on "Page"
        link-targets "Book" "Page"
    }
}
"#;

    fn label(language: &str, text: &str) -> RemodelLabel {
        RemodelLabel { language: language.to_owned(), text: text.to_owned() }
    }

    fn property(names: &[&str], source: PropertySource) -> RemodelProperty {
        RemodelProperty {
            names: names.iter().map(|n| CpeProperty::new(*n)).collect(),
            source,
        }
    }

    fn count(min: u32, max: Option<u32>) -> ValueCount {
        ValueCount::new(min, max).expect("the fixture's max is not below its min")
    }

    /// The count an absent `min` and `max` give.
    fn unbounded() -> ValueCount {
        count(0, None)
    }

    /// The policy of `id from="iri"`, which the fixtures give a class whose
    /// id is not what they test.
    fn from_iri() -> IdPolicy {
        IdPolicy::Minted { source: IdSource::Iri, under_parent: false }
    }

    /// A `text` source with every default of the grammar.
    fn text(iri: &str) -> PropertySource {
        PropertySource::Text {
            iri: ArchivePropertyIri::new(iri),
            count: unbounded(),
            rule: TextRule::Plain,
            empty: EmptyText::Drop,
            sentinel: SentinelWhole::Refuse,
            sentinel_within: SentinelWithin::Refuse,
        }
    }

    /// A `uri` source with every default of the grammar.
    fn uri(iri: &str) -> PropertySource {
        PropertySource::Uri {
            iri: ArchivePropertyIri::new(iri),
            count: unbounded(),
            sentinel: SentinelWhole::Refuse,
            sentinel_within: SentinelWithin::Refuse,
        }
    }

    /// A `date` source with every default of the grammar.
    fn date(iri: &str) -> PropertySource {
        PropertySource::Date {
            iri: ArchivePropertyIri::new(iri),
            count: unbounded(),
            display_key: None,
        }
    }

    /// A `link` source with every default of the grammar.
    fn link(iri: &str, target: usize) -> PropertySource {
        PropertySource::Link {
            iri: ArchivePropertyIri::new(iri),
            count: unbounded(),
            target: ClassIndex(target),
        }
    }

    #[test]
    fn a_block_of_every_node_lowers_to_the_expected_remodel() {
        let raw = crate::config::parse(FULL_BLOCK).expect("fixture must parse");
        let block = raw.remodel.as_ref().expect("fixture has a remodel block");
        let mut report = ValidationReport::new();

        let remodel = lower(block, &["de", "en"], &mut report);

        let expected = Remodel {
            shortcode: Shortcode::new("0803"),
            sentinel: Some("[missing]".to_owned()),
            languages: vec!["de".to_owned(), "en".to_owned()],
            lists: vec![
                RemodelList {
                    name: "office".to_owned(),
                    nodes: vec![
                        RemodelListNode {
                            slug: "amerbach".to_owned(),
                            labels: vec![label("de", "Johann Amerbach"), label("en", "Johann Amerbach")],
                        },
                        RemodelListNode {
                            slug: "other".to_owned(),
                            labels: vec![label("en", "Unnamed printer"), label("de", "Ohne genannten Drucker")],
                        },
                    ],
                },
                RemodelList {
                    name: "side".to_owned(),
                    nodes: vec![RemodelListNode {
                        slug: "left".to_owned(),
                        labels: vec![label("de", "links"), label("en", "left")],
                    }],
                },
            ],
            classes: vec![
                RemodelClass {
                    name: CpeClass::new("Book"),
                    iri: ArchiveClassIri::new("http://example.org/onto#Book"),
                    no_addresses: false,
                    image: ImageRule::Cover(CuratedKey::new("cover_page")),
                    id: IdPolicy::Curated { key: CuratedKey::new("slug"), fallback: None },
                    member_of: None,
                    properties: vec![
                        property(&["alt-title"], text("http://example.org/onto#hasTitle")),
                        property(&["catalogue-uri"], uri("http://example.org/onto#hasUri")),
                    ],
                    curated: vec![],
                    member_counts: vec![RemodelMemberCount { name: CpeProperty::new("extent"), of: ClassIndex(1) }],
                    incoming_counts: vec![RemodelIncomingCount {
                        name: CpeProperty::new("page-uses"),
                        from: ClassIndex(1),
                        via: vec![PropertyIndex(1)],
                    }],
                },
                RemodelClass {
                    name: CpeClass::new("Page"),
                    iri: ArchiveClassIri::new("http://example.org/onto#Page"),
                    no_addresses: false,
                    image: ImageRule::Required,
                    id: IdPolicy::Minted {
                        source: IdSource::Property(PropertyIndex(0)),
                        under_parent: true,
                    },
                    member_of: Some(Membership {
                        parent: ClassIndex(0),
                        seq: Some(CpeProperty::new("seq-number")),
                    }),
                    properties: vec![
                        property(
                            &["name", "page-id"],
                            PropertySource::Text {
                                iri: ArchivePropertyIri::new("http://example.org/onto#hasPagenum"),
                                count: count(0, Some(1)),
                                rule: TextRule::Plain,
                                empty: EmptyText::Drop,
                                sentinel: SentinelWhole::Refuse,
                                sentinel_within: SentinelWithin::Refuse,
                            },
                        ),
                        property(&["strip"], link("http://example.org/onto#hasBand", 0)),
                        property(&["pub-date"], date("http://example.org/onto#hasPubdate")),
                    ],
                    curated: vec![],
                    member_counts: vec![],
                    incoming_counts: vec![],
                },
            ],
            images: Some(RemodelImages {
                class: CpeClass::new("Image"),
                iiif_base: "https://iiif.example.org/0803/".to_owned(),
                authorship: "Universitaetsbibliothek Basel".to_owned(),
                license: "CC BY 4.0".to_owned(),
                classes_before: 1,
            }),
            annotations: Some(RemodelAnnotations {
                on: vec![ClassIndex(1)],
                link_targets: vec![ClassIndex(0), ClassIndex(1)],
            }),
            hook: Some(RemodelHook {
                keys: vec![HookKey { class: ClassIndex(0), key: CuratedKey::new("sort_key") }],
                provides: vec![HookProvides { class: ClassIndex(1), name: CpeProperty::new("side") }],
                overrides: vec![HookOverrides { class: ClassIndex(0), property: PropertyIndex(1) }],
                drops: vec![ClassIndex(1)],
            }),
        };
        assert_eq!(remodel, Some(expected));
        assert!(
            report.is_empty(),
            "a faultless block reports nothing: {:?}",
            report.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn each_value_attribute_lowers_to_its_typed_value_and_an_absent_one_to_its_default() {
        let src = r#"
remodel {
    shortcode "0803"
    sentinel "[missing]"
    list "office" { list-node "a" { en "A" } }
    list "side" { list-node "l" { en "L" } }
    class "Book" iri="http://e/o#Book" no-addresses=#true {
        id from="iri"
        property "label" from="label" curated-key="label-key"
        property "title" "alt" iri="http://e/o#title" type="text" min=1 max=3 empty="refuse" sentinel="absent" sentinel-within="keep" exclude-label=#true
        property "subtitle" iri="http://e/o#subtitle" type="text" curated-key="subtitle-key"
        property "catalogue" iri="http://e/o#catalogue" type="uri" min=2 sentinel="keep" sentinel-within="keep"
        property "printed" iri="http://e/o#printed" type="date" max=1 display-key="printed-display"
        property "strip" iri="http://e/o#strip" type="link" min=1 target="Strip"
        curated "office" key="office-key" type="list" list="office" min=1
        curated "blurb" key="blurb-key" type="per-language"
        curated "note" key="note-key" type="text" min=0
    }
    class "Strip" iri="http://e/o#Strip" {
        id from="iri"
        property "name" from="label"
        property "plain-text" iri="http://e/o#plainText" type="text"
        property "plain-uri" iri="http://e/o#plainUri" type="uri"
        property "plain-date" iri="http://e/o#plainDate" type="date"
        property "book" iri="http://e/o#book" type="link" target="Book"
        curated "side" key="side-key" type="list" list="side"
        curated "plain" key="label-key" type="text"
    }
}
"#;
        let out = lower_src(src, &["en"]);

        let book_properties = vec![
            property(
                &["label"],
                PropertySource::Label { curated_key: Some(CuratedKey::new("label-key")) },
            ),
            property(
                &["title", "alt"],
                PropertySource::Text {
                    iri: ArchivePropertyIri::new("http://e/o#title"),
                    count: count(1, Some(3)),
                    rule: TextRule::ExcludeLabel,
                    empty: EmptyText::Refuse,
                    sentinel: SentinelWhole::Absent,
                    sentinel_within: SentinelWithin::Keep,
                },
            ),
            property(
                &["subtitle"],
                PropertySource::Text {
                    iri: ArchivePropertyIri::new("http://e/o#subtitle"),
                    count: unbounded(),
                    rule: TextRule::CuratedOverride(CuratedKey::new("subtitle-key")),
                    empty: EmptyText::Drop,
                    sentinel: SentinelWhole::Refuse,
                    sentinel_within: SentinelWithin::Refuse,
                },
            ),
            property(
                &["catalogue"],
                PropertySource::Uri {
                    iri: ArchivePropertyIri::new("http://e/o#catalogue"),
                    count: count(2, None),
                    sentinel: SentinelWhole::Keep,
                    sentinel_within: SentinelWithin::Keep,
                },
            ),
            property(
                &["printed"],
                PropertySource::Date {
                    iri: ArchivePropertyIri::new("http://e/o#printed"),
                    count: count(0, Some(1)),
                    display_key: Some(CuratedKey::new("printed-display")),
                },
            ),
            property(
                &["strip"],
                PropertySource::Link {
                    iri: ArchivePropertyIri::new("http://e/o#strip"),
                    count: count(1, None),
                    target: ClassIndex(1),
                },
            ),
        ];
        let book_curated = vec![
            RemodelCurated {
                name: CpeProperty::new("office"),
                key: CuratedKey::new("office-key"),
                form: CuratedForm::List(ListIndex(0)),
                required: true,
            },
            RemodelCurated {
                name: CpeProperty::new("blurb"),
                key: CuratedKey::new("blurb-key"),
                form: CuratedForm::PerLanguage,
                required: false,
            },
            RemodelCurated {
                name: CpeProperty::new("note"),
                key: CuratedKey::new("note-key"),
                form: CuratedForm::Text,
                required: false,
            },
        ];
        let strip_properties = vec![
            property(&["name"], PropertySource::Label { curated_key: None }),
            property(&["plain-text"], text("http://e/o#plainText")),
            property(&["plain-uri"], uri("http://e/o#plainUri")),
            property(&["plain-date"], date("http://e/o#plainDate")),
            property(&["book"], link("http://e/o#book", 0)),
        ];
        let strip_curated = vec![
            RemodelCurated {
                name: CpeProperty::new("side"),
                key: CuratedKey::new("side-key"),
                form: CuratedForm::List(ListIndex(1)),
                required: false,
            },
            RemodelCurated {
                name: CpeProperty::new("plain"),
                key: CuratedKey::new("label-key"),
                form: CuratedForm::Text,
                required: false,
            },
        ];
        let remodel = out.remodel.as_ref().unwrap_or_else(|| panic!("{}", out.dump()));
        assert_eq!(out.count(), 0, "{}", out.dump());
        assert_eq!(
            remodel.classes,
            vec![
                RemodelClass {
                    name: CpeClass::new("Book"),
                    iri: ArchiveClassIri::new("http://e/o#Book"),
                    no_addresses: true,
                    image: ImageRule::Optional,
                    id: from_iri(),
                    member_of: None,
                    properties: book_properties,
                    curated: book_curated,
                    member_counts: vec![],
                    incoming_counts: vec![],
                },
                RemodelClass {
                    name: CpeClass::new("Strip"),
                    iri: ArchiveClassIri::new("http://e/o#Strip"),
                    no_addresses: false,
                    image: ImageRule::Optional,
                    id: from_iri(),
                    member_of: None,
                    properties: strip_properties,
                    curated: strip_curated,
                    member_counts: vec![],
                    incoming_counts: vec![],
                },
            ]
        );
    }

    #[test]
    fn each_id_policy_lowers_to_its_typed_policy() {
        let src = r#"
remodel {
    shortcode "0803"
    class "Curated" iri="http://e/o#A" { id curated-key="slug" }
    class "CuratedOrLabel" iri="http://e/o#B" { id curated-key="slug" from="label" }
    class "CuratedOrIri" iri="http://e/o#C" { id curated-key="slug" from="iri" }
    class "CuratedOrProperty" iri="http://e/o#D" {
        id curated-key="slug" from="property" property="code"
        property "code" iri="http://e/o#code" type="text" max=1
    }
    class "Label" iri="http://e/o#E" { id from="label" }
    class "Iri" iri="http://e/o#F" { id from="iri" }
    class "Property" iri="http://e/o#G" {
        id from="property" property="code"
        property "title" iri="http://e/o#title" type="text"
        property "code" iri="http://e/o#code" type="text" min=1 max=1
    }
    class "Member" iri="http://e/o#H" {
        member-of "Label"
        id from="label" under-parent=#true
    }
    class "MemberOnItsOwn" iri="http://e/o#I" {
        member-of "Label"
        id from="iri" under-parent=#false
    }
}
"#;
        let out = lower_src(src, &[]);

        let curated = |fallback: Option<IdSource>| IdPolicy::Curated { key: CuratedKey::new("slug"), fallback };
        let minted = |source: IdSource, under_parent: bool| IdPolicy::Minted { source, under_parent };
        let expected = vec![
            ("Curated", curated(None)),
            ("CuratedOrLabel", curated(Some(IdSource::Label))),
            ("CuratedOrIri", curated(Some(IdSource::Iri))),
            ("CuratedOrProperty", curated(Some(IdSource::Property(PropertyIndex(0))))),
            ("Label", minted(IdSource::Label, false)),
            ("Iri", minted(IdSource::Iri, false)),
            ("Property", minted(IdSource::Property(PropertyIndex(1)), false)),
            ("Member", minted(IdSource::Label, true)),
            ("MemberOnItsOwn", minted(IdSource::Iri, false)),
        ];
        assert_eq!(out.count(), 0, "{}", out.dump());
        let remodel = out.remodel.as_ref().expect("the block lowers");
        let lowered: Vec<(&str, IdPolicy)> = remodel
            .classes
            .iter()
            .map(|class| (class.name.as_str(), class.id.clone()))
            .collect();
        assert_eq!(lowered, expected);
    }

    #[test]
    fn a_class_lists_every_curated_key_it_reads_wherever_it_is_declared() {
        let src = r#"
remodel {
    shortcode "0803"
    images class="Image" iiif-base="https://h/" { authorship "A"; license "L" }
    class "Book" iri="http://e/o#Book" {
        curated "office" key="in-curated" type="text"
        property "printed" iri="http://e/o#printed" type="date" display-key="in-display"
        property "title" iri="http://e/o#title" type="text" curated-key="in-text"
        property "name" from="label" curated-key="in-label"
        property "plain" iri="http://e/o#plain" type="text"
        cover curated-key="in-cover"
        id curated-key="in-id"
    }
    class "Page" iri="http://e/o#Page" {
        member-of "Book"
        id from="iri"
    }
}
"#;
        let out = lower_src(src, &[]);

        assert_eq!(out.count(), 0, "{}", out.dump());
        let remodel = out.remodel.as_ref().expect("the block lowers");
        let keys = |class: usize| -> Vec<&str> {
            remodel.classes[class]
                .curated_keys()
                .into_iter()
                .map(|key| key.as_str())
                .collect()
        };
        assert_eq!(
            keys(0),
            ["in-id", "in-cover", "in-display", "in-text", "in-label", "in-curated"]
        );
        assert_eq!(keys(1), [] as [&str; 0]);
    }

    /// The outcome of lowering one source, with what the assertions need.
    struct Lowered {
        /// Names the fixture in a failure message; empty when the test has one.
        name: String,
        src: String,
        block: Span,
        remodel: Option<Remodel>,
        report: ValidationReport,
    }

    fn lower_src(src: &str, languages: &[&str]) -> Lowered {
        let raw = crate::config::parse(src).expect("fixture must parse");
        let block = raw.remodel.as_ref().expect("fixture has a remodel block");
        let mut report = ValidationReport::new();
        let remodel = lower(block, languages, &mut report);
        Lowered {
            name: String::new(),
            src: src.to_owned(),
            block: block.span,
            remodel,
            report,
        }
    }

    fn unknown_child(component: &str, child: &str) -> DiagnosticKind {
        DiagnosticKind::UnknownChildNode { component: component.to_owned(), child: child.to_owned() }
    }

    impl Lowered {
        fn named(mut self, name: &str) -> Self {
            self.name = format!("fixture `{name}`: ");
            self
        }

        fn count(&self) -> usize {
            self.report.iter().count()
        }

        fn dump(&self) -> String {
            format!("{}{:#?}", self.name, self.report.iter().collect::<Vec<_>>())
        }

        /// Exactly one diagnostic of `kind` is located at the node whose
        /// source text starts with `snippet`.
        fn assert_error(&self, kind: &DiagnosticKind, snippet: &str) {
            assert!(
                self.src.contains(snippet),
                "{}snippet `{snippet}` is not in the fixture",
                self.name
            );
            let hits = self
                .report
                .iter()
                .filter(|d| {
                    d.kind == *kind && d.span.is_some_and(|s| self.src[s.offset..].trim_start().starts_with(snippet))
                })
                .count();
            assert_eq!(
                hits,
                1,
                "expected exactly one {kind:?} at `{snippet}`; reported: {}",
                self.dump()
            );
        }

        /// Every diagnostic has a span, and it lies inside the `remodel` node.
        fn assert_spans_in_block(&self) {
            let (start, end) = (self.block.offset, self.block.offset + self.block.len);
            for d in self.report.iter() {
                let span = d.span.expect("a remodel diagnostic carries a span");
                assert!(
                    span.offset >= start && span.offset + span.len <= end,
                    "{}span {span:?} of {:?} lies outside the block {start}..{end}",
                    self.name,
                    d.kind
                );
            }
        }

        /// The block holds exactly `count` diagnostics, all inside the block,
        /// and does not lower.
        fn assert_refused(&self, count: usize) {
            assert_eq!(self.count(), count, "nothing beyond the planted errors: {}", self.dump());
            self.assert_spans_in_block();
            assert_eq!(self.remodel, None, "{}a block with errors does not lower", self.name);
        }
    }

    #[test]
    fn every_error_the_grammar_gives_these_nodes_is_reported_in_one_pass() {
        let src = r#"
remodel {
    shortcode "08G3"
    sentinel ""
    list "dup-list" { list-node "a" { de "a"; en "a" } }
    list "dup-list" { list-node "b" { de "b"; en "b" } }
    list "slugs" {
        list-node "same" { de "one"; en "one" }
        list-node "same" { de "two"; en "two" }
    }
    list "partial" { list-node "no-en" { de "x" } }
    list "langs" { list-node "with-fr" { de "x"; en "x"; fr "x" } }
    class "Twin" iri="http://e/o#T1" {
        id from="iri"
        property "notype" iri="http://e/o#p1"
    }
    class "Twin" iri="http://e/o#T2" { id from="iri" }
    class "First" iri="http://e/o#Shared" { id from="iri" }
    class "Second" iri="http://e/o#Shared" { id from="iri" }
    class "Image" iri="http://e/o#Img" { id from="iri" }
    class "NoIri" { id from="iri" }
    class "BadIri" iri=42 { id from="iri" }
    images class="Image" iiif-base="ftp://iiif.example.org/0803" {
        authorship "A"
        license "L"
    }
    annotations {
        on "Ghost"
        link-targets "Phantom"
    }
}
"#;
        let out = lower_src(src, &["de", "en"]);

        use DiagnosticKind as K;
        out.assert_error(&K::InvalidRemodelValue, "shortcode \"08G3\"");
        out.assert_error(&K::InvalidRemodelValue, "sentinel \"\"");
        out.assert_error(&K::DuplicateRemodelEntry, "list \"dup-list\" { list-node \"b\"");
        out.assert_error(&K::DuplicateRemodelEntry, "list-node \"same\" { de \"two\"");
        out.assert_error(&K::IncompleteListTranslation, "list-node \"no-en\"");
        out.assert_error(&K::UnknownRemodelReference, "fr \"x\"");
        out.assert_error(&K::DuplicateRemodelEntry, "class \"Twin\" iri=\"http://e/o#T2\"");
        out.assert_error(&K::DuplicateRemodelEntry, "class \"Second\"");
        out.assert_error(&K::ReservedRemodelName, "class \"Image\"");
        out.assert_error(&K::ConflictingRemodelAttributes, "property \"notype\"");
        out.assert_error(&K::InvalidRemodelValue, "images class=");
        out.assert_error(&K::UnknownRemodelReference, "on \"Ghost\"");
        out.assert_error(&K::UnknownRemodelReference, "link-targets \"Phantom\"");
        out.assert_error(&K::MissingRequiredAttribute, "class \"NoIri\"");
        out.assert_error(&K::InvalidAttributeType, "class \"BadIri\"");

        out.assert_refused(15);
    }

    #[test]
    fn an_unknown_child_or_attribute_is_refused_on_every_node() {
        // A child of `list-node` is always read as a language, so no unknown
        // child can be told from one there; that depth is covered by the
        // `UnknownRemodelReference` for `fr` in the every-error test.
        let src = r#"
remodel bogus-remodel=1 {
    extra-in-remodel
    shortcode "0803" bogus-shortcode=1
    sentinel "[m]" bogus-sentinel=1
    list "l" bogus-list=1 {
        extra-in-list
        list-node "n" bogus-list-node=1 { de "x"; en "x" }
    }
    class "C" iri="http://e/o#C" bogus-class=1 {
        id from="iri"
        extra-in-class
        property "p" iri="http://e/o#p" type="text" bogus-property=1 { cover }
    }
    images class="Image" iiif-base="https://iiif.example.org/0803/" no-addresses=#true {
        member-count
        authorship "A" bogus-authorship=1
        license "L" bogus-license=1
    }
    annotations bogus-annotations=1 {
        incoming-count
        on "C" bogus-on=1
        link-targets "C" bogus-link-targets=1
    }
}
"#;
        let out = lower_src(src, &["de", "en"]);

        use DiagnosticKind as K;
        out.assert_error(&unknown_child("remodel", "extra-in-remodel"), "extra-in-remodel");
        out.assert_error(&unknown_child("list", "extra-in-list"), "extra-in-list");
        out.assert_error(&unknown_child("class", "extra-in-class"), "extra-in-class");
        out.assert_error(&unknown_child("property", "cover"), "cover");
        out.assert_error(&unknown_child("images", "member-count"), "member-count");
        out.assert_error(&unknown_child("annotations", "incoming-count"), "incoming-count");

        let attr = &K::UnknownRemodelAttribute;
        out.assert_error(attr, "remodel bogus-remodel=1");
        out.assert_error(attr, "shortcode \"0803\" bogus-shortcode=1");
        out.assert_error(attr, "sentinel \"[m]\" bogus-sentinel=1");
        out.assert_error(attr, "list \"l\" bogus-list=1");
        out.assert_error(attr, "list-node \"n\" bogus-list-node=1");
        out.assert_error(attr, "class \"C\" iri=\"http://e/o#C\" bogus-class=1");
        out.assert_error(attr, "property \"p\" iri=\"http://e/o#p\" type=\"text\" bogus-property=");
        out.assert_error(attr, "images class=\"Image\"");
        out.assert_error(attr, "authorship \"A\" bogus-authorship=1");
        out.assert_error(attr, "license \"L\" bogus-license=1");
        out.assert_error(attr, "annotations bogus-annotations=1");
        out.assert_error(attr, "on \"C\" bogus-on=1");
        out.assert_error(attr, "link-targets \"C\" bogus-link-targets=1");

        out.assert_refused(19);
    }

    /// Blocks whose defects cannot share one block (a missing node and a
    /// repeated one, two errors of one kind at one node), each with the
    /// errors it holds as (kind, source text the located node starts with).
    #[test]
    fn each_remaining_branch_of_the_grammar_is_reported_by_kind_and_node() {
        use DiagnosticKind as K;
        type Fixture<'a> = (&'a str, &'a str, Vec<(DiagnosticKind, &'a str)>);
        let fixtures: Vec<Fixture> = vec![
            (
                "a second node of each kind that takes one",
                r#"
remodel {
    shortcode "0803"
    shortcode "0804"
    sentinel "a"
    sentinel "b"
    class "C" iri="http://e/o#C" { id from="iri" }
    images class="I" iiif-base="https://h/" { authorship "A"; license "L" }
    images class="J" iiif-base="https://h/" { authorship "A"; license "L" }
    annotations {
        on "C"
        on "Again"
        link-targets "C"
        link-targets "Again"
    }
    annotations { on "C" }
}
"#,
                vec![
                    (K::DuplicateRemodelEntry, "shortcode \"0804\""),
                    (K::DuplicateRemodelEntry, "sentinel \"b\""),
                    (K::DuplicateRemodelEntry, "images class=\"J\""),
                    (K::DuplicateRemodelEntry, "on \"Again\""),
                    (K::DuplicateRemodelEntry, "link-targets \"Again\""),
                    (K::DuplicateRemodelEntry, "annotations { on \"C\" }"),
                ],
            ),
            (
                "no shortcode",
                "remodel {\n    class \"C\" iri=\"http://e/o#C\" { id from=\"iri\" }\n}\n",
                vec![(K::MissingRequiredAttribute, "remodel {")],
            ),
            (
                "no class",
                "remodel {\n    shortcode \"0803\"\n}\n",
                vec![(K::MissingRequiredAttribute, "remodel {")],
            ),
            (
                "a list without a node and a label language twice",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" { id from="iri" }
    list "bare"
    list "twice" { list-node "n" { de "a"; de "b"; en "c" } }
}
"#,
                vec![
                    (K::MissingRequiredAttribute, "list \"bare\""),
                    (K::DuplicateRemodelEntry, "de \"b\""),
                ],
            ),
            (
                "an empty on, a class repeated in link-targets and a property name that is no string",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" {
        id from="iri"
        property 42 iri="http://e/o#p" type="text"
    }
    annotations {
        on
        link-targets "C" "C"
    }
}
"#,
                vec![
                    (K::MissingRequiredAttribute, "on\n"),
                    (K::DuplicateRemodelEntry, "link-targets \"C\" \"C\""),
                    (K::InvalidAttributeType, "property 42"),
                ],
            ),
            (
                "a shortcode and an on that are no string",
                r#"
remodel {
    shortcode 42
    class "C" iri="http://e/o#C" { id from="iri" }
    annotations { on 42 }
}
"#,
                vec![
                    (K::InvalidAttributeType, "shortcode 42"),
                    (K::InvalidAttributeType, "on 42"),
                ],
            ),
            (
                "a type that is no string and an images block without authorship",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" {
        id from="iri"
        property "p" iri="http://e/o#p" type=42
    }
    images class="I" iiif-base="https://h/" { license "L" }
}
"#,
                vec![
                    (K::InvalidAttributeType, "property \"p\""),
                    (K::MissingRequiredAttribute, "images class=\"I\""),
                ],
            ),
            (
                "an images block without license",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" { id from="iri" }
    images class="I" iiif-base="https://h/" { authorship "A" }
}
"#,
                vec![(K::MissingRequiredAttribute, "images class=\"I\"")],
            ),
            (
                "an argument a node does not take",
                r#"
remodel "extra" {
    shortcode "0803" "x"
    list "l" "m" { list-node "n" "o" { de "x" "y" } }
    class "A" "B" iri="http://e/o#A" { id from="iri" }
    images "x" class="I" iiif-base="https://h/" { authorship "A" "B"; license "L" }
    annotations "z" { on "A" }
}
"#,
                vec![
                    (K::InvalidRemodelValue, "remodel \"extra\""),
                    (K::InvalidRemodelValue, "shortcode \"0803\" \"x\""),
                    (K::InvalidRemodelValue, "list \"l\" \"m\""),
                    (K::InvalidRemodelValue, "list-node \"n\" \"o\""),
                    (K::InvalidRemodelValue, "de \"x\" \"y\""),
                    (K::InvalidRemodelValue, "class \"A\" \"B\""),
                    (K::InvalidRemodelValue, "images \"x\""),
                    (K::InvalidRemodelValue, "authorship \"A\" \"B\""),
                    (K::InvalidRemodelValue, "annotations \"z\""),
                ],
            ),
            (
                "an attribute twice, a class named twice and an iiif-base without a host",
                r#"
remodel {
    shortcode "0803"
    class "A" iri="http://e/o#A" iri="http://e/o#A2" { id from="iri" }
    images class="I" iiif-base="https:///" { authorship "A"; license "L" }
    annotations { on "A" "A"; link-targets "A" }
}
"#,
                vec![
                    (K::DuplicateRemodelEntry, "class \"A\" iri=\"http://e/o#A\" iri="),
                    (K::InvalidRemodelValue, "images class=\"I\""),
                    (K::DuplicateRemodelEntry, "on \"A\" \"A\""),
                ],
            ),
            (
                "an empty text wherever one is read",
                r#"
remodel {
    shortcode "0803"
    list "" { list-node "" { de "" } }
    class "" iri="http://e/o#E" { id from="iri" }
    class "Z" iri="" {
        id from="iri"
        property "" iri="http://e/o#q" type="text"
        property "q2" iri="" type="text"
    }
    images class="" iiif-base="https://h/" { authorship ""; license "" }
}
"#,
                vec![
                    (K::InvalidRemodelValue, "list \"\""),
                    (K::InvalidRemodelValue, "list-node \"\""),
                    (K::InvalidRemodelValue, "de \"\""),
                    (K::InvalidRemodelValue, "class \"\" iri="),
                    (K::InvalidRemodelValue, "class \"Z\" iri=\"\""),
                    (K::InvalidRemodelValue, "property \"\" iri="),
                    (K::InvalidRemodelValue, "property \"q2\" iri=\"\""),
                    (K::InvalidRemodelValue, "images class=\"\""),
                    (K::InvalidRemodelValue, "authorship \"\""),
                    (K::InvalidRemodelValue, "license \"\""),
                ],
            ),
            (
                "an empty class name in on and in link-targets",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" { id from="iri" }
    annotations {
        on ""
        link-targets "C" ""
    }
}
"#,
                vec![
                    (K::InvalidRemodelValue, "on \"\""),
                    (K::InvalidRemodelValue, "link-targets \"C\" \"\""),
                ],
            ),
            (
                "a label with an attribute, one with a child and one that is no string",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" { id from="iri" }
    list "l" { list-node "n" { de "x" width=1; en "y" { gloss } } }
    list "m" { list-node "o" { de 42 } }
}
"#,
                vec![
                    (K::UnknownRemodelAttribute, "de \"x\" width=1"),
                    (unknown_child("en", "gloss"), "gloss"),
                    (K::InvalidAttributeType, "de 42"),
                ],
            ),
            (
                "an images block without its class",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" { id from="iri" }
    images iiif-base="https://h/" { authorship "A"; license "L" }
}
"#,
                vec![(K::MissingRequiredAttribute, "images iiif-base=")],
            ),
            (
                "an images block without its iiif-base",
                r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" { id from="iri" }
    images class="I" { authorship "A"; license "L" }
}
"#,
                vec![(K::MissingRequiredAttribute, "images class=\"I\" {")],
            ),
            (
                "a sentinel of whitespace alone",
                "remodel {\n    shortcode \"0803\"\n    sentinel \" \"\n    class \"C\" iri=\"http://e/o#C\" { id from=\"iri\" }\n}\n",
                vec![(K::InvalidRemodelValue, "sentinel \" \"")],
            ),
            (
                "a sentinel with whitespace after it",
                "remodel {\n    shortcode \"0803\"\n    sentinel \"[m]\\n\"\n    class \"C\" iri=\"http://e/o#C\" { id from=\"iri\" }\n}\n",
                vec![(K::InvalidRemodelValue, "sentinel \"[m]")],
            ),
            (
                "a sentinel with a separator Python strips before it",
                "remodel {\n    shortcode \"0803\"\n    sentinel \"\\u{1f}[m]\"\n    class \"C\" iri=\"http://e/o#C\" { id from=\"iri\" }\n}\n",
                vec![(K::InvalidRemodelValue, "sentinel \"\\u{1f}[m]")],
            ),
        ];

        for (name, src, errors) in fixtures {
            let out = lower_src(src, &[]).named(name);
            for (kind, snippet) in &errors {
                out.assert_error(kind, snippet);
            }
            out.assert_refused(errors.len());
        }
    }

    fn type_block(word: &str) -> String {
        let target = if word == "link" { r#" target="C""# } else { "" };
        format!(
            r#"
remodel {{
    shortcode "0803"
    class "C" iri="http://e/o#C" {{
        id from="iri"
        property "p" iri="http://e/o#p" type="{word}"{target}
    }}
}}
"#
        )
    }

    #[test]
    fn a_property_type_is_supported_unsupported_or_unknown() {
        for word in ["text", "uri", "date", "link"] {
            let out = lower_src(&type_block(word), &[]);
            assert_eq!(out.count(), 0, "`{word}` is supported: {}", out.dump());
            assert!(out.remodel.is_some(), "`{word}` lowers");
        }
        for word in UNSUPPORTED_TYPES {
            let out = lower_src(&type_block(word), &[]);
            out.assert_error(&DiagnosticKind::UnsupportedRemodelType, "property \"p\"");
            assert_eq!(out.count(), 1, "`{word}`: {}", out.dump());
            assert_eq!(out.remodel, None, "`{word}` does not lower");
        }
        let out = lower_src(&type_block("textual"), &[]);
        out.assert_error(&DiagnosticKind::UnknownRemodelType, "property \"p\"");
        assert_eq!(out.count(), 1, "{}", out.dump());
        assert_eq!(out.remodel, None);
    }

    #[test]
    fn a_class_repeats_neither_a_property_iri_nor_a_property_name() {
        let src = r#"
remodel {
    shortcode "0803"
    class "A" iri="http://e/o#A" {
        id from="iri"
        property "first" iri="http://e/o#i1" type="text"
        property "second" iri="http://e/o#i1" type="text"
        property "named" iri="http://e/o#i2" type="text"
        property "named" iri="http://e/o#i3" type="text"
    }
    class "B" iri="http://e/o#B" {
        id from="iri"
        property "first" iri="http://e/o#i1" type="text"
        property "named" iri="http://e/o#i2" type="text"
    }
}
"#;
        let out = lower_src(src, &[]);

        out.assert_error(
            &DiagnosticKind::DuplicateRemodelEntry,
            "property \"second\" iri=\"http://e/o#i1\"",
        );
        out.assert_error(
            &DiagnosticKind::DuplicateRemodelEntry,
            "property \"named\" iri=\"http://e/o#i3\"",
        );
        // Class `B` reuses `A`'s iri and name: the namespace is per class.
        out.assert_refused(2);
    }

    #[test]
    fn with_no_site_language_a_list_reports_no_per_language_diagnostic() {
        let src = r#"
remodel {
    shortcode "0803"
    list "office" {
        list-node "both" { de "Zwei"; en "Two" }
        list-node "only-fr" { fr "Un" }
        list-node "bare"
    }
    class "C" iri="http://e/o#C" { id from="iri" }
}
"#;
        let out = lower_src(src, &[]);

        assert_eq!(out.count(), 0, "{}", out.dump());
        let remodel = out.remodel.expect("the block lowers");
        let nodes = &remodel.lists[0].nodes;
        assert_eq!(nodes[0].labels, vec![label("de", "Zwei"), label("en", "Two")]);
        assert_eq!(nodes[1].labels, vec![label("fr", "Un")]);
        assert_eq!(nodes[2].labels, vec![]);
    }

    /// A block with a `sentinel` node (when `sentinel` is not empty), a list
    /// `office`, a class `C` holding `body` and an empty class `T`.
    fn block_with(sentinel: &str, body: &str) -> String {
        format!(
            r#"
remodel {{
    shortcode "0803"
    {sentinel}
    list "office" {{ list-node "a" {{ en "A" }} }}
    class "C" iri="http://e/o#C" {{
        id from="iri"
{body}
    }}
    class "T" iri="http://e/o#T" {{ id from="iri" }}
}}
"#
        )
    }

    fn block(body: &str) -> String {
        block_with(r#"sentinel "[m]""#, body)
    }

    type Fixture<'a> = (&'a str, String, Vec<(DiagnosticKind, &'a str)>);

    /// Each fixture holds exactly the errors it lists, each at its node.
    fn assert_fixtures(fixtures: Vec<Fixture>) {
        for (name, src, errors) in fixtures {
            let out = lower_src(&src, &[]).named(name);
            for (kind, snippet) in &errors {
                out.assert_error(kind, snippet);
            }
            out.assert_refused(errors.len());
        }
    }

    /// A `property` named `p` of `form`, carrying `attr` with `value`.
    fn form_property(form: &str, attr: &str, value: &str) -> String {
        let source = if form == "label" {
            r#"from="label""#.to_owned()
        } else {
            format!(r#"iri="http://e/o#p" type="{form}""#)
        };
        let target = if form == "link" && attr != "target" {
            r#" target="T""#
        } else {
            ""
        };
        format!("        property \"p\" {source}{target} {attr}={value}")
    }

    #[test]
    fn a_property_attribute_a_form_does_not_take_is_refused_and_one_it_takes_lowers() {
        use DiagnosticKind as K;
        let forms = ["text", "uri", "date", "link", "label"];
        let attrs = [
            ("curated-key", r#""k""#),
            ("empty", r#""drop""#),
            ("min", "1"),
            ("max", "2"),
            ("sentinel", r#""keep""#),
            ("sentinel-within", r#""keep""#),
            ("exclude-label", "#true"),
            ("display-key", r#""dk""#),
            ("target", r#""T""#),
        ];
        // The "no" cells of the grammar's table, written out.
        let no: Vec<(&str, &str)> = vec![
            ("curated-key", "uri"),
            ("curated-key", "date"),
            ("curated-key", "link"),
            ("empty", "uri"),
            ("empty", "date"),
            ("empty", "link"),
            ("empty", "label"),
            ("min", "label"),
            ("max", "label"),
            ("sentinel", "date"),
            ("sentinel", "link"),
            ("sentinel", "label"),
            ("sentinel-within", "date"),
            ("sentinel-within", "link"),
            ("sentinel-within", "label"),
            ("exclude-label", "uri"),
            ("exclude-label", "date"),
            ("exclude-label", "link"),
            ("exclude-label", "label"),
            ("display-key", "text"),
            ("display-key", "uri"),
            ("display-key", "link"),
            ("display-key", "label"),
            ("target", "text"),
            ("target", "uri"),
            ("target", "date"),
            ("target", "label"),
        ];
        assert_eq!(no.len(), 27, "a cell of the table was dropped");

        let mut yes = 0;
        for (attr, value) in attrs {
            for form in forms {
                let src = block(&form_property(form, attr, value));
                let name = format!("`{attr}` on {form}");
                let out = lower_src(&src, &[]).named(&name);
                if no.contains(&(attr, form)) {
                    out.assert_error(&K::ConflictingRemodelAttributes, "property \"p\"");
                    out.assert_refused(1);
                } else {
                    yes += 1;
                    assert_eq!(out.count(), 0, "{}", out.dump());
                    assert!(out.remodel.is_some(), "{name} lowers");
                }
            }
        }
        assert_eq!(yes, 18, "the other cells of the table are taken");
    }

    #[test]
    fn a_property_whose_attributes_contradict_each_other_or_the_block_is_refused() {
        use DiagnosticKind as K;
        let text = r#"iri="http://e/o#p" type="text""#;
        let prop = |attrs: &str| format!("        property \"p\" {text} {attrs}");
        assert_fixtures(vec![
            (
                "exclude-label with curated-key",
                block(&prop(r#"exclude-label=#true curated-key="k""#)),
                vec![(K::ConflictingRemodelAttributes, "property \"p\"")],
            ),
            (
                "a link without a target",
                block(r#"        property "p" iri="http://e/o#p" type="link""#),
                vec![(K::ConflictingRemodelAttributes, "property \"p\"")],
            ),
            (
                "max below min",
                block(&prop("min=3 max=1")),
                vec![(K::InvalidRemodelValue, "property \"p\"")],
            ),
            (
                "sentinel without a sentinel node",
                block_with("", &prop(r#"sentinel="keep""#)),
                vec![(K::ConflictingRemodelAttributes, "property \"p\"")],
            ),
            (
                "sentinel-within without a sentinel node",
                block_with("", &prop(r#"sentinel-within="keep""#)),
                vec![(K::ConflictingRemodelAttributes, "property \"p\"")],
            ),
        ]);
    }

    #[test]
    fn a_count_out_of_range_is_refused_with_the_bound_it_broke() {
        let with_max = |value: &str| {
            let node = format!("        property \"p\" iri=\"http://e/o#p\" type=\"text\" max={value}");
            lower_src(&block(&node), &[]).named(value)
        };
        for (value, says) in [
            ("-1", "must not be negative"),
            ("4294967296", "must be at most 4294967295"),
        ] {
            let out = with_max(value);
            out.assert_error(&DiagnosticKind::InvalidRemodelValue, "property \"p\"");
            out.assert_refused(1);
            let message = &out.report.iter().next().expect("one diagnostic").message;
            assert!(message.contains(says), "`{message}` lacks `{says}`");
        }

        let out = with_max("4294967295");
        assert_eq!(out.count(), 0, "{}", out.dump());
        assert!(out.remodel.is_some(), "the largest count lowers");
    }

    #[test]
    fn a_conflict_is_decided_by_an_attribute_being_written_whatever_its_value() {
        use DiagnosticKind as K;
        let at_p = || vec![(K::ConflictingRemodelAttributes, "property \"p\"")];
        assert_fixtures(vec![
            (
                "exclude-label=#false on a uri",
                block(r#"        property "p" iri="http://e/o#p" type="uri" exclude-label=#false"#),
                at_p(),
            ),
            (
                "exclude-label=#false beside curated-key",
                block(r#"        property "p" iri="http://e/o#p" type="text" exclude-label=#false curated-key="k""#),
                at_p(),
            ),
        ]);
    }

    #[test]
    fn a_property_with_a_faulty_source_still_reports_an_attribute_none_of_its_forms_takes() {
        use DiagnosticKind as K;
        let prop = |rest: &str| block(&format!("        property \"p\" {rest}"));
        // (name, source, the attributes refused beside the source's own error,
        // the kind of the source's error)
        let fixtures = [
            (
                "both sources: empty goes with neither a uri nor the label",
                prop(r#"iri="http://e/o#p" type="uri" from="label" empty="drop""#),
                vec!["empty"],
                K::ConflictingRemodelAttributes,
            ),
            (
                "both sources: curated-key goes with the label, display-key with neither",
                prop(r#"iri="http://e/o#p" type="uri" from="label" curated-key="k" display-key="d""#),
                vec!["display-key"],
                K::ConflictingRemodelAttributes,
            ),
            (
                "no source: the type names the form",
                prop(r#"type="uri" empty="drop" exclude-label=#true"#),
                vec!["empty", "exclude-label"],
                K::ConflictingRemodelAttributes,
            ),
            (
                "a from word outside its vocabulary still names the label form",
                prop(r#"from="iri" min=1"#),
                vec!["min"],
                K::InvalidRemodelValue,
            ),
        ];
        for (name, src, refused, source_error) in fixtures {
            let out = lower_src(&src, &[]).named(name);
            let count = |kind: &DiagnosticKind, takes: Option<&str>| {
                out.report
                    .iter()
                    .filter(|d| d.kind == *kind)
                    .filter(|d| match takes {
                        Some(attr) => d.message.contains(&format!("does not take '{attr}'")),
                        None => !d.message.contains("does not take"),
                    })
                    .count()
            };
            for attr in &refused {
                let hits = count(&K::ConflictingRemodelAttributes, Some(attr));
                assert_eq!(hits, 1, "`{attr}` is refused once: {}", out.dump());
            }
            assert_eq!(
                count(&source_error, None),
                1,
                "the source's own error is reported once: {}",
                out.dump()
            );
            out.assert_refused(refused.len() + 1);
        }
    }

    #[test]
    fn a_property_with_no_readable_form_at_all_reports_its_source_only() {
        use DiagnosticKind as K;
        let fixtures = vec![
            (
                "neither source nor type",
                block(r#"        property "p" empty="drop" display-key="d""#),
                vec![(K::ConflictingRemodelAttributes, "property \"p\"")],
            ),
            (
                "an iri with a type word that is no kind",
                block(r#"        property "p" iri="http://e/o#p" type="textual" display-key="d""#),
                vec![(K::UnknownRemodelType, "property \"p\"")],
            ),
        ];
        assert_fixtures(fixtures);
    }

    #[test]
    fn a_curated_key_declared_twice_on_one_class_is_refused_wherever_it_is_declared() {
        use DiagnosticKind as K;
        let by_property = r#"        property "a" iri="http://e/o#a" type="text" curated-key="k""#;
        let second_property = r#"        property "b" iri="http://e/o#b" type="text" curated-key="k""#;
        let by_display = r#"        property "d" iri="http://e/o#d" type="date" display-key="k""#;
        let second_display = r#"        property "e" iri="http://e/o#e" type="date" display-key="k""#;
        let by_label = r#"        property "l" from="label" curated-key="k""#;
        let curated = r#"        curated "c" key="k" type="text""#;
        let second_curated = r#"        curated "c2" key="k" type="text""#;
        let body = |first: &str, second: &str| block(&format!("{first}\n{second}"));
        assert_fixtures(vec![
            (
                "curated-key twice",
                body(by_property, second_property),
                vec![(K::DuplicateRemodelEntry, "property \"b\"")],
            ),
            (
                "curated-key, then display-key",
                body(by_property, by_display),
                vec![(K::DuplicateRemodelEntry, "property \"d\"")],
            ),
            (
                "display-key, then curated-key",
                body(by_display, by_property),
                vec![(K::DuplicateRemodelEntry, "property \"a\"")],
            ),
            (
                "display-key twice",
                body(by_display, second_display),
                vec![(K::DuplicateRemodelEntry, "property \"e\"")],
            ),
            (
                "curated-key, then curated key",
                body(by_property, curated),
                vec![(K::DuplicateRemodelEntry, "curated \"c\"")],
            ),
            (
                "a label's curated-key, then curated key",
                body(by_label, curated),
                vec![(K::DuplicateRemodelEntry, "curated \"c\"")],
            ),
            (
                "display-key, then curated key",
                body(by_display, curated),
                vec![(K::DuplicateRemodelEntry, "curated \"c\"")],
            ),
            (
                "curated key, then curated-key",
                body(curated, by_property),
                vec![(K::DuplicateRemodelEntry, "property \"a\"")],
            ),
            (
                "two curated nodes",
                body(curated, second_curated),
                vec![(K::DuplicateRemodelEntry, "curated \"c2\"")],
            ),
        ]);
    }

    #[test]
    fn the_same_curated_key_on_two_classes_is_accepted() {
        let src = r#"
remodel {
    shortcode "0803"
    class "A" iri="http://e/o#A" {
        id from="iri"
        property "a" iri="http://e/o#a" type="text" curated-key="k"
    }
    class "B" iri="http://e/o#B" {
        id from="iri"
        curated "c" key="k" type="text"
    }
    class "D" iri="http://e/o#D" {
        id from="iri"
        property "d" iri="http://e/o#d" type="date" display-key="k"
    }
}
"#;
        let out = lower_src(src, &[]);

        assert_eq!(out.count(), 0, "{}", out.dump());
        assert!(out.remodel.is_some(), "the block lowers");
    }

    #[test]
    fn a_curated_node_and_a_property_of_one_class_cannot_share_a_name() {
        use DiagnosticKind as K;
        let property = r#"        property "x" iri="http://e/o#x" type="text""#;
        let curated = r#"        curated "x" key="k" type="text""#;
        let second_name = r#"        property "y" "x" iri="http://e/o#y" type="text""#;
        let body = |first: &str, second: &str| block(&format!("{first}\n{second}"));
        assert_fixtures(vec![
            (
                "a property, then a curated node",
                body(property, curated),
                vec![(K::DuplicateRemodelEntry, "curated \"x\"")],
            ),
            (
                "a curated node, then a property",
                body(curated, property),
                vec![(K::DuplicateRemodelEntry, "property \"x\"")],
            ),
            (
                "a curated node, then a property's second name",
                body(curated, second_name),
                vec![(K::DuplicateRemodelEntry, "property \"y\" \"x\"")],
            ),
        ]);
    }

    #[test]
    fn the_same_name_on_two_classes_is_accepted_for_both_kinds_of_node() {
        let src = r#"
remodel {
    shortcode "0803"
    class "A" iri="http://e/o#A" {
        id from="iri"
        property "x" iri="http://e/o#x" type="text"
    }
    class "B" iri="http://e/o#B" {
        id from="iri"
        curated "x" key="k" type="text"
    }
}
"#;
        let out = lower_src(src, &[]);

        assert_eq!(out.count(), 0, "{}", out.dump());
        assert!(out.remodel.is_some(), "the block lowers");
    }

    #[test]
    fn a_curated_node_reports_each_of_its_own_errors() {
        use DiagnosticKind as K;
        let curated = |node: &str| block(&format!("        {node}"));
        let at_c = |kind: DiagnosticKind| vec![(kind, "curated \"c\"")];
        assert_fixtures(vec![
            (
                "list names no declared list",
                curated(r#"curated "c" key="k" type="list" list="ghost""#),
                at_c(K::UnknownRemodelReference),
            ),
            (
                "list beside type text",
                curated(r#"curated "c" key="k" type="text" list="office""#),
                at_c(K::ConflictingRemodelAttributes),
            ),
            (
                "list beside type per-language",
                curated(r#"curated "c" key="k" type="per-language" list="office""#),
                at_c(K::ConflictingRemodelAttributes),
            ),
            (
                "type list without a list",
                curated(r#"curated "c" key="k" type="list""#),
                at_c(K::ConflictingRemodelAttributes),
            ),
            (
                "an unknown type word",
                curated(r#"curated "c" key="k" type="fancy""#),
                at_c(K::InvalidRemodelValue),
            ),
            (
                "no key",
                curated(r#"curated "c" type="text""#),
                at_c(K::MissingRequiredAttribute),
            ),
            ("no type", curated(r#"curated "c" key="k""#), at_c(K::MissingRequiredAttribute)),
            (
                "min above 1",
                curated(r#"curated "c" key="k" type="text" min=2"#),
                at_c(K::InvalidRemodelValue),
            ),
            (
                "a negative min",
                curated(r#"curated "c" key="k" type="text" min=-1"#),
                at_c(K::InvalidRemodelValue),
            ),
            (
                "a second positional argument",
                curated(r#"curated "c" "d" key="k" type="text""#),
                vec![(K::InvalidRemodelValue, "curated \"c\" \"d\"")],
            ),
            (
                "an empty name",
                curated(r#"curated "" key="k" type="text""#),
                vec![(K::InvalidRemodelValue, "curated \"\"")],
            ),
            (
                "an empty key",
                curated(r#"curated "c" key="" type="text""#),
                at_c(K::InvalidRemodelValue),
            ),
            (
                "an empty list",
                curated(r#"curated "c" key="k" type="list" list="""#),
                at_c(K::InvalidRemodelValue),
            ),
            (
                "an unknown attribute",
                curated(r#"curated "c" key="k" type="text" bogus=1"#),
                at_c(K::UnknownRemodelAttribute),
            ),
            (
                "a child node",
                curated(r#"curated "c" key="k" type="text" { extra }"#),
                vec![(unknown_child("curated", "extra"), "extra")],
            ),
            (
                "an attribute twice",
                curated(r#"curated "c" key="k" type="text" type="text""#),
                at_c(K::DuplicateRemodelEntry),
            ),
        ]);
    }

    #[test]
    fn a_property_reports_each_of_its_remaining_errors() {
        use DiagnosticKind as K;
        let text = r#"iri="http://e/o#p" type="text""#;
        let date = r#"iri="http://e/o#p" type="date""#;
        let link = r#"iri="http://e/o#p" type="link""#;
        let prop = |rest: &str| block(&format!("        property \"p\" {rest}"));
        let at_p = |kind: DiagnosticKind| vec![(kind, "property \"p\"")];
        let class_flag = r#"
remodel {
    shortcode "0803"
    class "C" iri="http://e/o#C" no-addresses="yes" { id from="iri" }
}
"#;
        assert_fixtures(vec![
            (
                "both iri and from",
                prop(&format!(r#"{text} from="label""#)),
                at_p(K::ConflictingRemodelAttributes),
            ),
            (
                "neither iri nor from",
                block(r#"        property "p""#),
                at_p(K::ConflictingRemodelAttributes),
            ),
            (
                "from is a word other than label",
                prop(r#"from="iri""#),
                at_p(K::InvalidRemodelValue),
            ),
            (
                "type beside from label",
                prop(r#"from="label" type="text""#),
                at_p(K::ConflictingRemodelAttributes),
            ),
            (
                "an unknown word for empty",
                prop(&format!(r#"{text} empty="x""#)),
                at_p(K::InvalidRemodelValue),
            ),
            (
                "an unknown word for sentinel",
                prop(&format!(r#"{text} sentinel="x""#)),
                at_p(K::InvalidRemodelValue),
            ),
            (
                "absent for sentinel-within",
                prop(&format!(r#"{text} sentinel-within="absent""#)),
                at_p(K::InvalidRemodelValue),
            ),
            ("a negative min", prop(&format!("{text} min=-1")), at_p(K::InvalidRemodelValue)),
            (
                "an empty curated-key",
                prop(&format!(r#"{text} curated-key="""#)),
                at_p(K::InvalidRemodelValue),
            ),
            (
                "an empty display-key",
                prop(&format!(r#"{date} display-key="""#)),
                at_p(K::InvalidRemodelValue),
            ),
            (
                "an empty target",
                prop(&format!(r#"{link} target="""#)),
                at_p(K::InvalidRemodelValue),
            ),
            (
                "a target naming no class",
                prop(&format!(r#"{link} target="Ghost""#)),
                at_p(K::UnknownRemodelReference),
            ),
            (
                "a min that is a string",
                prop(&format!(r#"{text} min="1""#)),
                at_p(K::InvalidAttributeType),
            ),
            (
                "an exclude-label that is a string",
                prop(&format!(r#"{text} exclude-label="yes""#)),
                at_p(K::InvalidAttributeType),
            ),
            (
                "a curated-key that is a number",
                prop(&format!("{text} curated-key=1")),
                at_p(K::InvalidAttributeType),
            ),
            (
                "min written twice",
                prop(&format!("{text} min=1 min=2")),
                at_p(K::DuplicateRemodelEntry),
            ),
            (
                "iri written twice",
                prop(r#"iri="http://e/o#p" iri="http://e/o#p" type="text""#),
                at_p(K::DuplicateRemodelEntry),
            ),
            (
                "no-addresses that is a string",
                class_flag.to_owned(),
                vec![(K::InvalidAttributeType, "class \"C\"")],
            ),
        ]);
    }

    // ── ids, membership, counts, images ───────────────────────────────────────

    /// A `class` node named `name` with the attributes `attrs` and one child
    /// per line of `body`.
    fn class_node(name: &str, attrs: &str, body: &[&str]) -> String {
        let body: String = body.iter().map(|line| format!("        {line}\n")).collect();
        format!("    class \"{name}\" iri=\"http://e/o#{name}\"{attrs} {{\n{body}    }}\n")
    }

    /// A class named `name` whose only child is an `id`.
    fn plain(name: &str) -> String {
        class_node(name, "", &[r#"id from="iri""#])
    }

    /// A class `M` that is a member of the class `C`.
    fn member() -> String {
        class_node("M", "", &[r#"member-of "C""#, r#"id from="iri""#])
    }

    /// A class `S` with a link to `C`, a link to itself and a text.
    fn source() -> String {
        class_node(
            "S",
            "",
            &[
                r#"id from="iri""#,
                r#"property "to-c" iri="http://e/o#toC" type="link" target="C""#,
                r#"property "to-s" iri="http://e/o#toS" type="link" target="S""#,
                r#"property "text" iri="http://e/o#text" type="text""#,
            ],
        )
    }

    /// A block of `classes`, with an `images` node when `images` is set.
    fn classes_block(images: bool, classes: &[String]) -> String {
        let images = if images {
            r#"images class="Image" iiif-base="https://h/" { authorship "A"; license "L" }"#
        } else {
            ""
        };
        format!("\nremodel {{\n    shortcode \"0803\"\n    {images}\n{}}}\n", classes.concat())
    }

    /// The class `C` with the children `body`, in a block without `images`.
    fn c_with(body: &[&str]) -> String {
        classes_block(false, &[class_node("C", "", body)])
    }

    /// The fixture lowers without a diagnostic.
    fn assert_lowers(name: &str, src: &str) {
        let out = lower_src(src, &[]).named(name);
        assert_eq!(out.count(), 0, "{}", out.dump());
        assert!(out.remodel.is_some(), "{name} lowers");
    }

    #[test]
    fn an_id_reports_every_error_the_grammar_gives_it() {
        use DiagnosticKind as K;
        let one_text = r#"property "p" iri="http://e/o#p" type="text" max=1"#;
        // The class `M`, a member of `C`, with `id` as its id.
        let on_member = |id: &str| classes_block(false, &[plain("C"), class_node("M", "", &[r#"member-of "C""#, id])]);
        let conflict = |snippet: &'static str| vec![(K::ConflictingRemodelAttributes, snippet)];
        let unknown = |snippet: &'static str| vec![(K::UnknownRemodelReference, snippet)];
        assert_fixtures(vec![
            ("neither curated-key nor from", c_with(&["id"]), conflict("id\n")),
            (
                "property beside another from",
                c_with(&[r#"id from="label" property="p""#, one_text]),
                conflict("id from="),
            ),
            (
                "property without a from",
                c_with(&[r#"id curated-key="k" property="p""#, one_text]),
                conflict("id curated-key="),
            ),
            (
                "from property without a property",
                c_with(&[r#"id from="property""#]),
                conflict("id from="),
            ),
            (
                "under-parent on a class without member-of",
                c_with(&[r#"id from="iri" under-parent=#true"#]),
                conflict("id from="),
            ),
            (
                "under-parent written as false on a class without member-of",
                c_with(&[r#"id from="iri" under-parent=#false"#]),
                conflict("id from="),
            ),
            (
                "under-parent beside curated-key",
                on_member(r#"id curated-key="k" from="iri" under-parent=#true"#),
                conflict("id curated-key="),
            ),
            (
                "property names no property of the class",
                c_with(&[r#"id from="property" property="ghost""#, one_text]),
                unknown("id from="),
            ),
            (
                "property names a text that may hold several values",
                c_with(&[
                    r#"id from="property" property="p""#,
                    r#"property "p" iri="http://e/o#p" type="text""#,
                ]),
                unknown("id from="),
            ),
            (
                "property names a uri",
                c_with(&[
                    r#"id from="property" property="p""#,
                    r#"property "p" iri="http://e/o#p" type="uri" max=1"#,
                ]),
                unknown("id from="),
            ),
            (
                "property names the label",
                c_with(&[r#"id from="property" property="p""#, r#"property "p" from="label""#]),
                unknown("id from="),
            ),
            (
                "a from word outside its vocabulary",
                c_with(&[r#"id from="slug""#]),
                vec![(K::InvalidRemodelValue, "id from=")],
            ),
            (
                "a class without an id",
                classes_block(false, &[class_node("C", "", &[])]),
                vec![(K::MissingRequiredAttribute, "class \"C\"")],
            ),
            (
                "a second id",
                c_with(&[r#"id from="iri""#, r#"id from="label""#]),
                vec![(K::DuplicateRemodelEntry, "id from=\"label\"")],
            ),
            (
                "an argument",
                c_with(&[r#"id "x" from="iri""#]),
                vec![(K::InvalidRemodelValue, "id \"x\"")],
            ),
            (
                "an unknown attribute",
                c_with(&[r#"id from="iri" bogus=1"#]),
                vec![(K::UnknownRemodelAttribute, "id from=")],
            ),
            (
                "a child node",
                c_with(&[r#"id from="iri" { extra }"#]),
                vec![(unknown_child("id", "extra"), "extra")],
            ),
            (
                "an empty curated-key",
                c_with(&[r#"id curated-key="""#]),
                vec![(K::InvalidRemodelValue, "id curated-key=")],
            ),
            (
                "an empty property",
                c_with(&[r#"id from="property" property="""#]),
                vec![(K::InvalidRemodelValue, "id from=")],
            ),
            (
                "a curated-key that is a number",
                c_with(&["id curated-key=1"]),
                vec![(K::InvalidAttributeType, "id curated-key=")],
            ),
            (
                "an under-parent that is a string",
                on_member(r#"id from="iri" under-parent="yes""#),
                vec![(K::InvalidAttributeType, "id from=")],
            ),
            (
                "from written twice",
                c_with(&[r#"id from="iri" from="label""#]),
                vec![(K::DuplicateRemodelEntry, "id from=")],
            ),
            (
                "the id's key declared again by a curated node",
                c_with(&[r#"id curated-key="k""#, r#"curated "c" key="k" type="text""#]),
                vec![(K::DuplicateRemodelEntry, "curated \"c\"")],
            ),
        ]);
    }

    #[test]
    fn a_membership_that_leads_back_to_its_class_is_a_cycle_at_each_member_of_on_it() {
        use DiagnosticKind as K;
        let member_of =
            |name: &str, parent: &str| class_node(name, "", &[&format!("member-of \"{parent}\""), r#"id from="iri""#]);
        let cycle = |snippet: &'static str| (K::RemodelMembershipCycle, snippet);
        assert_fixtures(vec![
            (
                "a class that is a member of itself",
                classes_block(false, &[member_of("A", "A")]),
                vec![cycle("member-of \"A\"")],
            ),
            (
                "two classes that are members of each other",
                classes_block(false, &[member_of("A", "B"), member_of("B", "A")]),
                vec![cycle("member-of \"B\""), cycle("member-of \"A\"")],
            ),
            (
                "a cycle over three classes",
                classes_block(false, &[member_of("A", "B"), member_of("B", "C"), member_of("C", "A")]),
                vec![
                    cycle("member-of \"B\""),
                    cycle("member-of \"C\""),
                    cycle("member-of \"A\""),
                ],
            ),
            (
                "a class whose parent is on a cycle is not on it",
                classes_block(
                    false,
                    &[
                        class_node("D", "", &[r#"member-of "A" seq="s""#, r#"id from="iri""#]),
                        member_of("A", "B"),
                        member_of("B", "A"),
                    ],
                ),
                vec![cycle("member-of \"B\""), cycle("member-of \"A\"\n")],
            ),
        ]);

        assert_lowers(
            "a chain of members that ends",
            &classes_block(false, &[member_of("A", "B"), member_of("B", "C"), plain("C")]),
        );
    }

    #[test]
    fn a_member_of_reports_each_of_its_remaining_errors() {
        use DiagnosticKind as K;
        let on_member = |lines: &[&str]| {
            let mut body = lines.to_vec();
            body.push(r#"id from="iri""#);
            classes_block(false, &[plain("C"), class_node("M", "", &body)])
        };
        assert_fixtures(vec![
            (
                "a parent no class declares",
                on_member(&[r#"member-of "Ghost""#]),
                vec![(K::UnknownRemodelReference, "member-of \"Ghost\"")],
            ),
            (
                "a second member-of",
                on_member(&[r#"member-of "C" seq="s""#, r#"member-of "C""#]),
                vec![(K::DuplicateRemodelEntry, "member-of \"C\"\n")],
            ),
            (
                "no parent",
                on_member(&["member-of"]),
                vec![(K::MissingRequiredAttribute, "member-of\n")],
            ),
            (
                "a second argument",
                on_member(&[r#"member-of "C" "D""#]),
                vec![(K::InvalidRemodelValue, "member-of \"C\" \"D\"")],
            ),
            (
                "an empty parent",
                on_member(&[r#"member-of """#]),
                vec![(K::InvalidRemodelValue, "member-of \"\"")],
            ),
            (
                "an empty seq",
                on_member(&[r#"member-of "C" seq="""#]),
                vec![(K::InvalidRemodelValue, "member-of \"C\"")],
            ),
            (
                "a seq that is a number",
                on_member(&[r#"member-of "C" seq=1"#]),
                vec![(K::InvalidAttributeType, "member-of \"C\"")],
            ),
            (
                "an unknown attribute",
                on_member(&[r#"member-of "C" bogus=1"#]),
                vec![(K::UnknownRemodelAttribute, "member-of \"C\"")],
            ),
            (
                "a child node",
                on_member(&[r#"member-of "C" { extra }"#]),
                vec![(unknown_child("member-of", "extra"), "extra")],
            ),
        ]);
    }

    #[test]
    fn an_image_or_a_cover_the_block_cannot_serve_is_refused() {
        use DiagnosticKind as K;
        let cover = r#"cover curated-key="k""#;
        let id = r#"id from="iri""#;
        // The class `C` with `attrs` and `body`, and its member `M`.
        let with_member =
            |images: bool, attrs: &str, body: &[&str]| classes_block(images, &[class_node("C", attrs, body), member()]);
        let at_cover = |kind: DiagnosticKind| vec![(kind, "cover")];
        assert_fixtures(vec![
            (
                "image without an images block",
                with_member(false, " image=#true", &[id]),
                vec![(K::ConflictingRemodelAttributes, "class \"C\"")],
            ),
            (
                "image written as false without an images block",
                with_member(false, " image=#false", &[id]),
                vec![(K::ConflictingRemodelAttributes, "class \"C\"")],
            ),
            (
                "cover without an images block",
                with_member(false, "", &[id, cover]),
                at_cover(K::ConflictingRemodelAttributes),
            ),
            (
                "cover on a class without members",
                classes_block(true, &[class_node("C", "", &[id, cover])]),
                at_cover(K::ConflictingRemodelAttributes),
            ),
            (
                "cover beside image",
                with_member(true, " image=#true", &[id, cover]),
                at_cover(K::ConflictingRemodelAttributes),
            ),
            (
                "cover beside image written as false",
                with_member(true, " image=#false", &[id, cover]),
                at_cover(K::ConflictingRemodelAttributes),
            ),
            (
                "cover without a curated-key",
                with_member(true, "", &[id, "cover"]),
                at_cover(K::MissingRequiredAttribute),
            ),
            (
                "cover with an empty curated-key",
                with_member(true, "", &[id, r#"cover curated-key="""#]),
                at_cover(K::InvalidRemodelValue),
            ),
            (
                "a second cover",
                with_member(true, "", &[id, cover, r#"cover curated-key="j""#]),
                vec![(K::DuplicateRemodelEntry, "cover curated-key=\"j\"")],
            ),
            (
                "cover with an argument",
                with_member(true, "", &[id, r#"cover "x" curated-key="k""#]),
                at_cover(K::InvalidRemodelValue),
            ),
            (
                "cover with an unknown attribute",
                with_member(true, "", &[id, r#"cover curated-key="k" bogus=1"#]),
                at_cover(K::UnknownRemodelAttribute),
            ),
            (
                "cover with a child node",
                with_member(true, "", &[id, r#"cover curated-key="k" { extra }"#]),
                vec![(unknown_child("cover", "extra"), "extra")],
            ),
            (
                "cover declares the key the id declared",
                with_member(true, "", &[r#"id curated-key="k""#, cover]),
                at_cover(K::DuplicateRemodelEntry),
            ),
            (
                "an image that is a string",
                with_member(true, r#" image="yes""#, &[id]),
                vec![(K::InvalidAttributeType, "class \"C\"")],
            ),
        ]);

        let src = classes_block(
            true,
            &[
                class_node("C", "", &[id, cover]),
                class_node("M", " image=#true", &[r#"member-of "C""#, id]),
                plain("T"),
            ],
        );
        let out = lower_src(&src, &[]);
        assert_eq!(out.count(), 0, "{}", out.dump());
        let classes = &out.remodel.as_ref().expect("the block lowers").classes;
        assert_eq!(classes[0].image, ImageRule::Cover(CuratedKey::new("k")));
        assert_eq!(classes[1].image, ImageRule::Required);
        assert_eq!(classes[2].image, ImageRule::Optional);
        assert_eq!(classes[1].member_of, Some(Membership { parent: ClassIndex(0), seq: None }));
    }

    /// An attribute that stands where its node does not take it is one
    /// mistake: it is reported as that, and what it holds is not judged, so a
    /// value that would be refused where the attribute belongs adds nothing.
    #[test]
    fn an_attribute_its_node_does_not_take_is_reported_once_whatever_it_holds() {
        use DiagnosticKind as K;
        let conflict = |snippet: &'static str| vec![(K::ConflictingRemodelAttributes, snippet)];
        let no_sentinel = |body: &str| block_with("", body);
        assert_fixtures(vec![
            (
                "a sentinel on a date in a block without a sentinel node",
                no_sentinel(&form_property("date", "sentinel", "\"keep\"")),
                conflict("property"),
            ),
            (
                "an unknown word of empty on a uri",
                block(&form_property("uri", "empty", "\"x\"")),
                conflict("property"),
            ),
            (
                "an empty curated key on a link",
                block(&form_property("link", "curated-key", "\"\"")),
                conflict("property"),
            ),
            (
                "an empty display key on a text",
                block(&form_property("text", "display-key", "\"\"")),
                conflict("property"),
            ),
            (
                "a target no class declares on a text",
                block(&form_property("text", "target", "\"Ghost\"")),
                conflict("property"),
            ),
            (
                "a negative max on the label",
                block(&form_property("label", "max", "-1")),
                conflict("property"),
            ),
            (
                "an exclude-label that is no boolean on a uri",
                block(&form_property("uri", "exclude-label", "\"yes\"")),
                conflict("property"),
            ),
            (
                "an unknown type word on the label",
                block(&form_property("label", "type", "\"textual\"")),
                conflict("property"),
            ),
            (
                "a list no list declares on a curated text",
                block(r#"        curated "c" key="k" type="text" list="ghost""#),
                conflict("curated"),
            ),
            (
                "an id's property no property has, beside from=\"label\"",
                c_with(&[r#"id from="label" property="ghost""#]),
                conflict("id"),
            ),
            (
                "an id's property no property has, beside a curated key alone",
                c_with(&[r#"id curated-key="slug" property="ghost""#]),
                conflict("id"),
            ),
        ]);
    }

    #[test]
    fn a_reference_to_a_faulty_node_is_reported_at_that_node_only() {
        use DiagnosticKind as K;
        let id = r#"id from="iri""#;
        // The class `C`, whose id is made from its property `p`.
        let id_from = |property: &str| c_with(&[r#"id from="property" property="p""#, property]);
        // The class `C` counting the links `x` of the class `S`.
        let via = |property: &str| {
            classes_block(
                false,
                &[
                    class_node("C", "", &[id, r#"incoming-count "n" from="S" { via "x" }"#]),
                    class_node("S", "", &[id, property]),
                ],
            )
        };
        // The class `C` with a member count and a cover, and the class `M`
        // with `member_of`.
        let members = |member_of: &str| {
            classes_block(
                true,
                &[
                    class_node("C", "", &[id, r#"member-count "n" of="M""#, r#"cover curated-key="k""#]),
                    class_node("M", "", &[member_of, id]),
                ],
            )
        };
        let at_property = |kind: DiagnosticKind| vec![(kind, "property")];
        assert_fixtures(vec![
            (
                "an id's property whose max is no integer",
                id_from(r#"property "p" iri="http://e/o#p" type="text" max=1.5"#),
                at_property(K::InvalidAttributeType),
            ),
            (
                "an id's property whose max is negative",
                id_from(r#"property "p" iri="http://e/o#p" type="text" max=-1"#),
                at_property(K::InvalidRemodelValue),
            ),
            (
                "an id's property whose type is no kind",
                id_from(r#"property "p" iri="http://e/o#p" type="textual" max=1"#),
                at_property(K::UnknownRemodelType),
            ),
            (
                "an id's property whose from is no word of its vocabulary",
                id_from(r#"property "p" from="bogus""#),
                at_property(K::InvalidRemodelValue),
            ),
            (
                "an id's property with a name that is no string",
                id_from(r#"property "p" 42 iri="http://e/o#p" type="text""#),
                at_property(K::InvalidAttributeType),
            ),
            (
                "a via's property with a type and no source",
                via(r#"property "x" type="text""#),
                at_property(K::ConflictingRemodelAttributes),
            ),
            (
                "a via's link without an iri",
                via(r#"property "x" type="link" target="C""#),
                at_property(K::ConflictingRemodelAttributes),
            ),
            (
                "an id's property with a type and no source",
                id_from(r#"property "p" type="text" max=1"#),
                at_property(K::ConflictingRemodelAttributes),
            ),
            (
                "an id's property whose iri is empty",
                id_from(r#"property "p" iri="" type="text" max=1"#),
                at_property(K::InvalidRemodelValue),
            ),
            (
                "a via's property with both sources",
                via(r#"property "x" iri="http://e/o#x" from="label""#),
                at_property(K::ConflictingRemodelAttributes),
            ),
            (
                "a via's property whose target is no class",
                via(r#"property "x" iri="http://e/o#x" type="link" target="Ghost""#),
                at_property(K::UnknownRemodelReference),
            ),
            (
                "a via's property that is a link without a target",
                via(r#"property "x" iri="http://e/o#x" type="link""#),
                at_property(K::ConflictingRemodelAttributes),
            ),
            (
                "a via's property whose type is no string",
                via(r#"property "x" iri="http://e/o#x" type=42"#),
                at_property(K::InvalidAttributeType),
            ),
            (
                "a member whose parent is no string",
                members("member-of 42"),
                vec![(K::InvalidAttributeType, "member-of 42")],
            ),
            (
                "a member whose parent is no class",
                members(r#"member-of "Ghost""#),
                vec![(K::UnknownRemodelReference, "member-of \"Ghost\"")],
            ),
            (
                "a member of the images class",
                members(r#"member-of "Image""#),
                vec![(K::UnknownRemodelReference, "member-of \"Image\"")],
            ),
        ]);
    }

    #[test]
    fn an_id_with_a_property_and_nothing_to_read_it_from_reports_both_conflicts() {
        let out = lower_src(
            &c_with(&[
                r#"id property="p""#,
                r#"property "p" iri="http://e/o#p" type="text" max=1"#,
            ]),
            &[],
        );

        let at_id = out
            .report
            .iter()
            .filter(|d| d.kind == DiagnosticKind::ConflictingRemodelAttributes)
            .filter(|d| d.span.is_some_and(|s| out.src[s.offset..].starts_with("id ")))
            .count();
        assert_eq!(at_id, 2, "{}", out.dump());
        out.assert_refused(2);
    }

    #[test]
    fn an_incoming_count_takes_only_links_of_its_from_class_that_point_at_this_class() {
        use DiagnosticKind as K;
        // The class `C` with the incoming count `count`, and the class `S`.
        let counted = |count: &[&str]| {
            let mut body = vec![r#"id from="iri""#];
            body.extend_from_slice(count);
            classes_block(false, &[class_node("C", "", &body), source()])
        };
        let from_s = |via: &str| format!("incoming-count \"n\" from=\"S\" {{ {via} }}");
        let at_count = |kind: DiagnosticKind| vec![(kind, "incoming-count")];
        assert_fixtures(vec![
            (
                "a via that is no link",
                counted(&[&from_s(r#"via "text""#)]),
                vec![(K::ConflictingRemodelAttributes, "via \"text\"")],
            ),
            (
                "a via whose target is another class",
                counted(&[&from_s(r#"via "to-s""#)]),
                vec![(K::ConflictingRemodelAttributes, "via \"to-s\"")],
            ),
            (
                "a via that names no property of the from class",
                counted(&[&from_s(r#"via "ghost""#)]),
                vec![(K::UnknownRemodelReference, "via \"ghost\"")],
            ),
            (
                "a from no class declares",
                counted(&[r#"incoming-count "n" from="Ghost" { via "to-c" }"#]),
                at_count(K::UnknownRemodelReference),
            ),
            (
                "no via",
                counted(&[r#"incoming-count "n" from="S""#]),
                at_count(K::MissingRequiredAttribute),
            ),
            (
                "one property in two via nodes",
                counted(&[&from_s(r#"via "to-c"; via "to-c""#)]),
                vec![(K::DuplicateRemodelEntry, "via \"to-c\"")],
            ),
            (
                "no from",
                counted(&[r#"incoming-count "n" { via "to-c" }"#]),
                at_count(K::MissingRequiredAttribute),
            ),
            (
                "an empty from",
                counted(&[r#"incoming-count "n" from="" { via "to-c" }"#]),
                at_count(K::InvalidRemodelValue),
            ),
            (
                "no name",
                counted(&[r#"incoming-count from="S" { via "to-c" }"#]),
                at_count(K::MissingRequiredAttribute),
            ),
            (
                "an empty name",
                counted(&[r#"incoming-count "" from="S" { via "to-c" }"#]),
                at_count(K::InvalidRemodelValue),
            ),
            (
                "a second name",
                counted(&[r#"incoming-count "n" "m" from="S" { via "to-c" }"#]),
                at_count(K::InvalidRemodelValue),
            ),
            (
                "an unknown attribute",
                counted(&[r#"incoming-count "n" from="S" bogus=1 { via "to-c" }"#]),
                at_count(K::UnknownRemodelAttribute),
            ),
            (
                "an unknown child",
                counted(&[&from_s(r#"via "to-c"; extra"#)]),
                vec![(unknown_child("incoming-count", "extra"), "extra")],
            ),
            (
                "a via without a property",
                counted(&[&from_s("via")]),
                vec![(K::MissingRequiredAttribute, "via }")],
            ),
            (
                "a via with an empty property",
                counted(&[&from_s(r#"via """#)]),
                vec![(K::InvalidRemodelValue, "via \"\"")],
            ),
            (
                "a via with a second argument",
                counted(&[&from_s(r#"via "to-c" "to-s""#)]),
                vec![(K::InvalidRemodelValue, "via \"to-c\" \"to-s\"")],
            ),
            (
                "a via with an attribute",
                counted(&[&from_s(r#"via "to-c" bogus=1"#)]),
                vec![(K::UnknownRemodelAttribute, "via \"to-c\"")],
            ),
            (
                "one property under two of its names",
                classes_block(
                    false,
                    &[
                        class_node(
                            "C",
                            "",
                            &[
                                r#"id from="iri""#,
                                r#"incoming-count "n" from="S" { via "a"; via "b" }"#,
                            ],
                        ),
                        class_node(
                            "S",
                            "",
                            &[
                                r#"id from="iri""#,
                                r#"property "a" "b" iri="http://e/o#a" type="link" target="C""#,
                            ],
                        ),
                    ],
                ),
                vec![(K::DuplicateRemodelEntry, "via \"b\"")],
            ),
        ]);

        assert_lowers(
            "a class counting the links between its own resources",
            &classes_block(
                false,
                &[class_node(
                    "S",
                    "",
                    &[
                        r#"id from="iri""#,
                        r#"property "to-s" iri="http://e/o#toS" type="link" target="S""#,
                        r#"incoming-count "n" from="S" { via "to-s" }"#,
                    ],
                )],
            ),
        );
    }

    #[test]
    fn a_member_count_counts_only_a_class_that_is_a_member_of_this_one() {
        use DiagnosticKind as K;
        // The class `C` with the member count `count`, its member `M` and
        // the unrelated class `T`.
        let counted = |count: &str| {
            classes_block(
                false,
                &[class_node("C", "", &[r#"id from="iri""#, count]), member(), plain("T")],
            )
        };
        let at_count = |kind: DiagnosticKind| vec![(kind, "member-count")];
        assert_fixtures(vec![
            (
                "of names a class that is no member",
                counted(r#"member-count "n" of="T""#),
                at_count(K::UnknownRemodelReference),
            ),
            (
                "of names the class itself",
                counted(r#"member-count "n" of="C""#),
                at_count(K::UnknownRemodelReference),
            ),
            (
                "of names no class",
                counted(r#"member-count "n" of="Ghost""#),
                at_count(K::UnknownRemodelReference),
            ),
            ("no of", counted(r#"member-count "n""#), at_count(K::MissingRequiredAttribute)),
            (
                "an empty of",
                counted(r#"member-count "n" of="""#),
                at_count(K::InvalidRemodelValue),
            ),
            (
                "an of that is a number",
                counted(r#"member-count "n" of=1"#),
                at_count(K::InvalidAttributeType),
            ),
            (
                "no name",
                counted(r#"member-count of="M""#),
                at_count(K::MissingRequiredAttribute),
            ),
            (
                "an empty name",
                counted(r#"member-count "" of="M""#),
                at_count(K::InvalidRemodelValue),
            ),
            (
                "an unknown attribute",
                counted(r#"member-count "n" of="M" bogus=1"#),
                at_count(K::UnknownRemodelAttribute),
            ),
            (
                "a child node",
                counted(r#"member-count "n" of="M" { extra }"#),
                vec![(unknown_child("member-count", "extra"), "extra")],
            ),
        ]);
    }

    /// The images block keeps its place among the classes: before the first, between two, and
    /// after the last. Nodes of other kinds are not counted.
    #[test]
    fn the_images_block_keeps_the_number_of_classes_written_before_it() {
        let images = r#"images class="Image" iiif-base="https://h/" { authorship "A"; license "L" }"#;
        let class = |name: &str| class_node(name, "", &[r#"id from="iri""#]);
        let block = |nodes: &[&str]| {
            let nodes = nodes.join("\n");
            format!("\nremodel {{\n    shortcode \"0803\"\n    list \"l\" {{ list-node \"n\" }}\n{nodes}\n}}\n")
        };
        let (a, b) = (class("A"), class("B"));

        for (classes_before, nodes) in [
            (0, [images, a.as_str(), b.as_str()]),
            (1, [a.as_str(), images, b.as_str()]),
            (2, [a.as_str(), b.as_str(), images]),
        ] {
            let out = lower_src(&block(&nodes), &[]);
            let lowered = out.remodel.as_ref().and_then(|r| r.images.as_ref());
            assert_eq!(
                lowered.map(|images| images.classes_before),
                Some(classes_before),
                "{}",
                out.dump()
            );
        }
    }

    #[test]
    fn the_property_name_image_is_reserved_beside_an_images_block() {
        use DiagnosticKind as K;
        let id = r#"id from="iri""#;
        // The class `C` with `node`, its member `M` and the class `S`.
        let on_c =
            |images: bool, node: &str| classes_block(images, &[class_node("C", "", &[id, node]), member(), source()]);
        let reserved = |snippet: &'static str| vec![(K::ReservedRemodelName, snippet)];
        let property = r#"property "image" iri="http://e/o#p" type="text""#;
        assert_fixtures(vec![
            ("a property", on_c(true, property), reserved("property")),
            (
                "a property's second name",
                on_c(true, r#"property "x" "image" iri="http://e/o#p" type="text""#),
                reserved("property"),
            ),
            (
                "a curated property",
                on_c(true, r#"curated "image" key="k" type="text""#),
                reserved("curated"),
            ),
            (
                "a member count",
                on_c(true, r#"member-count "image" of="M""#),
                reserved("member-count"),
            ),
            (
                "an incoming count",
                on_c(true, r#"incoming-count "image" from="S" { via "to-c" }"#),
                reserved("incoming-count"),
            ),
            (
                "a member's seq",
                classes_block(true, &[plain("C"), class_node("M", "", &[r#"member-of "C" seq="image""#, id])]),
                reserved("member-of"),
            ),
        ]);

        assert_lowers("a block without images", &on_c(false, property));
    }

    #[test]
    fn a_count_or_a_seq_cannot_take_a_name_another_node_of_the_class_has() {
        use DiagnosticKind as K;
        let id = r#"id from="iri""#;
        let property = r#"property "n" iri="http://e/o#p" type="text""#;
        let curated = r#"curated "n" key="k" type="text""#;
        let member_count = r#"member-count "n" of="M""#;
        let incoming_count = r#"incoming-count "n" from="S" { via "to-c" }"#;
        let seq = r#"member-of "C" seq="n""#;
        // The class `C` with the two nodes, its member `M` and the class `S`.
        let on_c = |first: &str, second: &str| {
            classes_block(false, &[class_node("C", "", &[id, first, second]), member(), source()])
        };
        // The class `M`, a member of `C`, with the two nodes.
        let on_m =
            |first: &str, second: &str| classes_block(false, &[plain("C"), class_node("M", "", &[first, second, id])]);
        let twice = |snippet: &'static str| vec![(K::DuplicateRemodelEntry, snippet)];
        assert_fixtures(vec![
            (
                "a property, then a member count",
                on_c(property, member_count),
                twice("member-count"),
            ),
            (
                "a member count, then a property",
                on_c(member_count, property),
                twice("property"),
            ),
            ("a member count twice", on_c(member_count, member_count), twice("member-count")),
            (
                "a curated property, then an incoming count",
                on_c(curated, incoming_count),
                twice("incoming-count"),
            ),
            (
                "an incoming count, then a member count",
                on_c(incoming_count, member_count),
                twice("member-count"),
            ),
            ("a seq, then a property", on_m(seq, property), twice("property")),
            ("a property, then a seq", on_m(property, seq), twice("member-of")),
        ]);
    }

    // ── the hook node ─────────────────────────────────────────────────────────

    /// A `hook` node with one child per line of `body`.
    fn hook_node(body: &[&str]) -> String {
        let body: String = body.iter().map(|line| format!("        {line}\n")).collect();
        format!("    hook {{\n{body}    }}\n")
    }

    /// The class `C` with a curated id, a text `p` under two names and a
    /// curated `q`.
    fn c_for_a_hook() -> String {
        class_node(
            "C",
            "",
            &[
                r#"id curated-key="slug""#,
                r#"property "p" "p-too" iri="http://e/o#p" type="text""#,
                r#"curated "q" key="q-key" type="text""#,
            ],
        )
    }

    /// A `hook` node with the children `body`, then [`c_for_a_hook`] and the
    /// class `T`: the hook stands before the classes it names.
    fn hooked(body: &[&str]) -> String {
        classes_block(false, &[hook_node(body), c_for_a_hook(), plain("T")])
    }

    #[test]
    fn a_hook_child_reports_every_error_the_grammar_gives_it() {
        use DiagnosticKind as K;
        let one = |kind: K, snippet: &'static str| vec![(kind, snippet)];
        assert_fixtures(vec![
            (
                "a key on an undeclared class",
                hooked(&[r#"key "k" class="X""#]),
                one(K::UnknownRemodelReference, "key"),
            ),
            (
                "a key the class's id reads",
                hooked(&[r#"key "slug" class="C""#]),
                one(K::DuplicateRemodelEntry, "key"),
            ),
            (
                "a key a curated node of the class reads",
                hooked(&[r#"key "q-key" class="C""#]),
                one(K::DuplicateRemodelEntry, "key"),
            ),
            (
                "a key twice on one class",
                hooked(&[r#"key "k" class="C""#, r#"key  "k" class="C""#]),
                one(K::DuplicateRemodelEntry, r#"key  "k""#),
            ),
            (
                "a property overridden twice",
                hooked(&[r#"overrides "p" class="C""#, r#"overrides  "p" class="C""#]),
                one(K::DuplicateRemodelEntry, r#"overrides  "p""#),
            ),
            (
                "a property overridden under each of its two names",
                hooked(&[r#"overrides "p" class="C""#, r#"overrides "p-too" class="C""#]),
                one(K::DuplicateRemodelEntry, r#"overrides "p-too""#),
            ),
            (
                "a provided name on an undeclared class",
                hooked(&[r#"provides "n" class="X""#]),
                one(K::UnknownRemodelReference, "provides"),
            ),
            (
                "a provided name twice on one class",
                hooked(&[r#"provides "n" class="C""#, r#"provides  "n" class="C""#]),
                one(K::DuplicateRemodelEntry, r#"provides  "n""#),
            ),
            (
                "a provided name a curated node of the class has",
                hooked(&[r#"provides "q" class="C""#]),
                one(K::DuplicateRemodelEntry, "provides"),
            ),
            (
                "an override on an undeclared class",
                hooked(&[r#"overrides "p" class="X""#]),
                one(K::UnknownRemodelReference, "overrides"),
            ),
            (
                "an override of a name that is a curated node's",
                hooked(&[r#"overrides "q" class="C""#]),
                one(K::UnknownRemodelReference, "overrides"),
            ),
            (
                "an override of a property of another class",
                hooked(&[r#"overrides "p" class="T""#]),
                one(K::UnknownRemodelReference, "overrides"),
            ),
            (
                "a drop of an undeclared class",
                hooked(&[r#"drops class="X""#]),
                one(K::UnknownRemodelReference, "drops"),
            ),
            (
                "a drop of one class twice",
                hooked(&[r#"drops class="C""#, r#"drops  class="C""#]),
                one(K::DuplicateRemodelEntry, "drops  class"),
            ),
            (
                "a drop of a class that has members",
                classes_block(false, &[hook_node(&[r#"drops class="C""#]), plain("C"), member()]),
                one(K::ConflictingRemodelAttributes, "drops"),
            ),
            (
                "a child without its class",
                hooked(&[r#"key "k""#]),
                one(K::MissingRequiredAttribute, "key"),
            ),
            (
                "a child without its argument",
                hooked(&[r#"provides class="C""#]),
                one(K::MissingRequiredAttribute, "provides"),
            ),
            (
                "an empty key",
                hooked(&[r#"key "" class="C""#]),
                one(K::InvalidRemodelValue, "key"),
            ),
            (
                "a drop with an argument",
                hooked(&[r#"drops "C" class="C""#]),
                one(K::InvalidRemodelValue, "drops"),
            ),
            (
                "an unknown attribute on a child",
                hooked(&[r#"overrides "p" class="C" bogus=1"#]),
                one(K::UnknownRemodelAttribute, "overrides"),
            ),
            (
                "an unknown child of the hook",
                hooked(&[r#"bogus class="C""#]),
                one(unknown_child("hook", "bogus"), "bogus"),
            ),
            (
                "a child of a child",
                hooked(&[r#"drops class="C" { nested }"#]),
                one(unknown_child("drops", "nested"), "nested"),
            ),
            (
                "an argument and an attribute on the hook",
                classes_block(false, &["    hook \"x\" bogus=1 {\n    }\n".to_owned(), plain("C")]),
                vec![(K::InvalidRemodelValue, "hook"), (K::UnknownRemodelAttribute, "hook")],
            ),
            (
                "a second hook",
                classes_block(
                    false,
                    &[
                        hook_node(&[r#"drops class="C""#]),
                        "    hook  {\n    }\n".to_owned(),
                        plain("C"),
                    ],
                ),
                one(K::DuplicateRemodelEntry, "hook  {"),
            ),
        ]);
    }

    /// A provided property is one of the class's properties: it takes no
    /// name a `property` node of the class is stored under, and not the
    /// name the block reserves for the link to an image.
    #[test]
    fn a_hook_cannot_provide_a_name_a_property_of_the_class_has() {
        use DiagnosticKind as K;
        assert_fixtures(vec![
            (
                "the property's first name",
                hooked(&[r#"provides "p" class="C""#]),
                vec![(K::DuplicateRemodelEntry, "provides")],
            ),
            (
                "the property's second name",
                hooked(&[r#"provides "p-too" class="C""#]),
                vec![(K::DuplicateRemodelEntry, "provides")],
            ),
            (
                "the reserved name",
                classes_block(true, &[hook_node(&[r#"provides "image" class="C""#]), plain("C")]),
                vec![(K::ReservedRemodelName, "provides")],
            ),
        ]);
        assert_lowers(
            "the name of a property of another class",
            &hooked(&[r#"provides "p" class="T""#]),
        );
    }

    /// A hook's key and name belong to the class they are declared on, and
    /// the node may stand anywhere in the block.
    #[test]
    fn a_hook_takes_one_key_and_one_name_on_two_classes_and_stands_before_or_after_them() {
        let children = [
            r#"key "k" class="C""#,
            r#"key "k" class="T""#,
            r#"provides "n" class="C""#,
            r#"provides "n" class="T""#,
            r#"overrides "p-too" class="C""#,
            r#"drops class="C""#,
            r#"drops class="T""#,
        ];
        let before = lower_src(&hooked(&children), &[]);
        let after = lower_src(&classes_block(false, &[c_for_a_hook(), plain("T"), hook_node(&children)]), &[]);

        assert_eq!(before.count(), 0, "{}", before.dump());
        let hook = before.remodel.and_then(|remodel| remodel.hook);
        assert_eq!(
            hook.as_ref().map(|hook| hook.overrides.clone()),
            Some(vec![HookOverrides { class: ClassIndex(0), property: PropertyIndex(0) }])
        );
        assert_eq!(after.remodel.and_then(|remodel| remodel.hook), hook);
    }

    #[test]
    fn a_block_without_a_hook_node_lowers_to_no_hook() {
        let out = lower_src(&classes_block(false, &[plain("C")]), &[]);

        assert_eq!(out.count(), 0, "{}", out.dump());
        assert_eq!(out.remodel.map(|remodel| remodel.hook), Some(None));
    }
}
