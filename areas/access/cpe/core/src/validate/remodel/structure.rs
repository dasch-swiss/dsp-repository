//! The lowering of what places a class among the others: its `id`,
//! `member-of`, `members`, `member-count`, `incoming-count` and `cover` nodes
//! and its `image` attribute.
//!
//! It imports nothing from `validate::remodel`. These nodes refer to other
//! classes and to their properties, wherever in the block those stand, so the
//! parent hands in an [`Outline`] of the whole block.
//!
//! The outline reads a `property`'s names, `type`, `from`, `max` and `target`
//! and a `member-of`'s parent from the raw nodes, silently.
//! `validate/remodel/values.rs` and `ClassStructure::lower_member_of` lower
//! the same and report their faults, so a reading here must agree with them:
//! what they refuse of these is *faulty* here, and a reference to a faulty
//! node resolves without a diagnostic of its own.

use std::collections::HashSet;

use super::super::attrs::{
    first_positional_string, get_attr, get_bool_attr, get_string_attr, has_attr, optional_string_attr,
    positional_string_args,
};
use super::nodes::{
    at_most_one, check_attrs, check_children, check_leaf, check_no_args, children_named, declare_name, exactly_one,
    first_occurrence, form_of_source, key_attr, non_empty, position_of, push, reference_attr, single_string, word_attr,
    ClassSeen, Form,
};
use crate::config::Component as RawNode;
use crate::diagnostic::{DiagnosticKind, ValidationReport};
use crate::ir::Value;
use crate::remodel::{
    ClassIndex, CpeProperty, CuratedKey, IdPolicy, IdSource, ImageRule, Membership, PropertyIndex,
    RemodelIncomingCount, RemodelMemberCount, RemodelMembers, IMAGE_LINK, IMAGE_RIGHTS,
};

// ── The outline ───────────────────────────────────────────────────────────────

/// What these nodes need to know of the whole block.
pub(super) struct Outline<'a> {
    /// Whether the block has an `images` node.
    has_images: bool,
    /// The position of the class the `images` node names, when it names a declared one.
    declared_images: Option<usize>,
    /// Each `class` node's name in declared order (`None` where unreadable).
    class_names: &'a [Option<String>],
    /// Each `class` node in declared order.
    classes: Vec<&'a RawNode>,
}

/// What a class's first `member-of` says of its parent.
enum Parent {
    /// The class has no `member-of`.
    None,
    /// The class at this position.
    Class(usize),
    /// Faulty: its argument is no string, or names no declared class.
    Faulty,
}

/// What a `property` node reads, as far as these nodes ask.
enum Kind {
    Text,
    Link,
    /// Another kind of value, or the label.
    Other,
    /// Faulty: a name that is no string or empty, or a source the lowering
    /// refuses (`nodes::form_of_source`).
    Faulty,
}

/// Whether a property can be what an `id` is made from.
enum Fit {
    Yes,
    No,
    /// The property is faulty in what is asked of it.
    Faulty,
}

impl<'a> Outline<'a> {
    /// The outline of the `remodel` node `raw`, whose `class` nodes have the
    /// names `class_names`.
    pub(super) fn read(raw: &'a RawNode, class_names: &'a [Option<String>]) -> Self {
        let images = children_named(raw, "images").next();
        Self {
            has_images: images.is_some(),
            declared_images: images
                .and_then(|images| get_string_attr(images, "class"))
                .and_then(|class| position_of(class_names, &class)),
            class_names,
            classes: children_named(raw, "class").collect(),
        }
    }

    fn parent(&self, index: usize) -> Parent {
        let Some(member_of) = children_named(self.classes[index], "member-of").next() else {
            return Parent::None;
        };
        first_positional_string(member_of)
            .and_then(|parent| position_of(self.class_names, &parent))
            .map_or(Parent::Faulty, Parent::Class)
    }

    /// Whether following parents from the class at `index` leads back to it.
    fn is_on_a_cycle(&self, index: usize) -> bool {
        let mut current = index;
        // A chain that does not return within as many steps as there are
        // classes has entered a cycle this class is not part of.
        for _ in 0..self.classes.len() {
            match self.parent(current) {
                Parent::Class(parent) if parent == index => return true,
                Parent::Class(parent) => current = parent,
                Parent::None | Parent::Faulty => return false,
            }
        }
        false
    }

    /// Whether the class at `member` is a member of the class at `index`. A
    /// class whose `member-of` is faulty may be a member of any.
    fn may_be_member_of(&self, member: usize, index: usize) -> bool {
        match self.parent(member) {
            Parent::Class(parent) => parent == index,
            Parent::Faulty => true,
            Parent::None => false,
        }
    }

    /// Whether some class names the class at `index` as its `member-of`
    /// parent. A faulty `member-of` names none.
    pub(super) fn has_members(&self, index: usize) -> bool {
        (0..self.classes.len()).any(|member| matches!(self.parent(member), Parent::Class(parent) if parent == index))
    }

    /// The `property` node of the class at `class` that has the name `name`,
    /// with its position among that class's `property` nodes: the position
    /// lowering gives it in the class's property list.
    pub(super) fn property(&self, class: usize, name: &str) -> Option<(usize, &'a RawNode)> {
        children_named(self.classes[class], "property")
            .enumerate()
            .find(|(_, property)| positional_string_args(property).iter().any(|n| n == name))
    }
}

fn kind_of(property: &RawNode) -> Kind {
    let named = |arg: &Value| matches!(arg, Value::String(name) if !name.is_empty());
    if !property.args.iter().all(named) {
        return Kind::Faulty;
    }
    // Every form is named: a new one must say here what a reference makes of it.
    match form_of_source(property) {
        Some(Form::Text) => Kind::Text,
        Some(Form::Link) => Kind::Link,
        Some(Form::Uri | Form::Date | Form::Integer | Form::ListNode | Form::Label | Form::Class) => Kind::Other,
        None => Kind::Faulty,
    }
}

/// Whether `property` is a `text` property that holds at most one value.
fn fit_for_an_id(property: &RawNode) -> Fit {
    match kind_of(property) {
        Kind::Faulty => Fit::Faulty,
        Kind::Link | Kind::Other => Fit::No,
        Kind::Text => match get_attr(property, "max") {
            None => Fit::No,
            Some(Value::Integer(1)) => Fit::Yes,
            Some(Value::Integer(max)) if u32::try_from(max).is_ok() => Fit::No,
            Some(_) => Fit::Faulty,
        },
    }
}

// ── A class's structure ───────────────────────────────────────────────────────

/// What these nodes and the `image` attribute give one class.
pub(super) struct Structure {
    pub(super) image: ImageRule,
    pub(super) id: IdPolicy,
    pub(super) member_of: Option<Membership>,
    pub(super) members: Vec<RemodelMembers>,
    pub(super) member_counts: Vec<RemodelMemberCount>,
    pub(super) incoming_counts: Vec<RemodelIncomingCount>,
}

/// Collects the structure of one class while the parent walks the class's
/// children in declared order.
pub(super) struct ClassStructure<'a> {
    class: &'a RawNode,
    index: usize,
    outline: &'a Outline<'a>,
    /// The class's first `id`, `member-of` and `cover` node: the one of each
    /// that is lowered.
    id_node: Option<&'a RawNode>,
    member_of_node: Option<&'a RawNode>,
    cover_node: Option<&'a RawNode>,
    /// What each lowered to; `None` until it was walked, and when it is
    /// faulty.
    id: Option<IdPolicy>,
    member_of: Option<Membership>,
    cover: Option<CuratedKey>,
    members: Vec<Option<RemodelMembers>>,
    member_counts: Vec<Option<RemodelMemberCount>>,
    incoming_counts: Vec<Option<RemodelIncomingCount>>,
}

impl<'a> ClassStructure<'a> {
    /// For the `class` node at position `index` among the block's classes.
    /// Reports a missing `id` and a second `id`, `member-of` or `cover`.
    pub(super) fn new(
        class: &'a RawNode,
        index: usize,
        outline: &'a Outline<'a>,
        report: &mut ValidationReport,
    ) -> Self {
        Self {
            class,
            index,
            outline,
            id_node: exactly_one(class, "id", report),
            member_of_node: at_most_one(class, "member-of", report),
            cover_node: at_most_one(class, "cover", report),
            id: None,
            member_of: None,
            cover: None,
            members: Vec::new(),
            member_counts: Vec::new(),
            incoming_counts: Vec::new(),
        }
    }

    /// The property names no node of the class may take, because the builder writes them: the
    /// image link in a block whose `images` node names no declared class, and an image's rights
    /// on the class it names.
    pub(super) fn reserved_names(&self) -> &'static [&'static str] {
        match self.outline.declared_images {
            Some(images) if images == self.index => &IMAGE_RIGHTS,
            Some(_) => &[],
            None if self.outline.has_images => &[IMAGE_LINK],
            None => &[],
        }
    }

    /// Lowers `child` when it is one of this module's nodes; any other node
    /// is passed over, and so is a second `id`, `member-of` or `cover`.
    pub(super) fn lower_child(&mut self, child: &RawNode, seen: &mut ClassSeen, report: &mut ValidationReport) {
        let is = |first: Option<&RawNode>| first.is_some_and(|first| std::ptr::eq(first, child));
        match child.name.as_str() {
            "id" if is(self.id_node) => self.id = self.lower_id(child, seen, report),
            "member-of" if is(self.member_of_node) => {
                self.member_of = self.lower_member_of(child, seen, report);
            }
            "cover" if is(self.cover_node) => self.cover = self.lower_cover(child, seen, report),
            "members" => {
                let members = self.lower_members(child, seen, report);
                self.members.push(members);
            }
            "member-count" => {
                let count = self.lower_member_count(child, seen, report);
                self.member_counts.push(count);
            }
            "incoming-count" => {
                let count = self.lower_incoming_count(child, seen, report);
                self.incoming_counts.push(count);
            }
            _ => {}
        }
    }

    /// The class's structure once every child was walked; reads the `image`
    /// attribute. `None` when any part is faulty (already reported).
    pub(super) fn finish(self, report: &mut ValidationReport) -> Option<Structure> {
        let required = get_bool_attr(self.class, "image", report);
        if has_attr(self.class, "image") && !self.outline.has_images {
            conflict(self.class, "has 'image' but the block has no 'images' node", report);
        }
        if has_attr(self.class, "image") && self.outline.declared_images.is_some() {
            conflict(self.class, "has 'image' but the 'images' node names a declared class", report);
        }
        let image = match self.cover_node {
            Some(_) => ImageRule::Cover(self.cover?),
            None if required => ImageRule::Required,
            None => ImageRule::Optional,
        };
        let member_of = match self.member_of_node {
            Some(_) => Some(self.member_of?),
            None => None,
        };
        Some(Structure {
            image,
            id: self.id?,
            member_of,
            members: self.members.into_iter().collect::<Option<_>>()?,
            member_counts: self.member_counts.into_iter().collect::<Option<_>>()?,
            incoming_counts: self.incoming_counts.into_iter().collect::<Option<_>>()?,
        })
    }
}

/// Reports each of `messages` as a conflict at `node`; returns whether there
/// was none.
fn conflicts(node: &RawNode, messages: &[&str], report: &mut ValidationReport) -> bool {
    for message in messages {
        conflict(node, message, report);
    }
    messages.is_empty()
}

fn conflict(node: &RawNode, message: &str, report: &mut ValidationReport) {
    push(
        report,
        DiagnosticKind::ConflictingRemodelAttributes,
        node.span,
        format!("'{}' {message}", node.name),
    );
}

/// The node's one positional argument as a CPE property name, recorded as a
/// name of the class `seen` belongs to.
fn property_name(node: &RawNode, seen: &mut ClassSeen, report: &mut ValidationReport) -> Option<CpeProperty> {
    let what = format!("a {}'s CPE name", node.name);
    let name = single_string(node, "the property's CPE name", report)
        .and_then(|name| non_empty(name, &what, node.span, report))?;
    declare_name(seen, &name, node.span, report);
    Some(CpeProperty::new(name))
}

/// The required attribute `attr` as the position of the class it names.
pub(super) fn required_class(
    node: &RawNode,
    attr: &str,
    outline: &Outline<'_>,
    report: &mut ValidationReport,
) -> Option<usize> {
    let class = reference_attr(node, attr, "class", outline.class_names, report).ok()?;
    if class.is_none() {
        push(
            report,
            DiagnosticKind::MissingRequiredAttribute,
            node.span,
            format!("'{}' is missing its required '{attr}'", node.name),
        );
    }
    class
}

/// The words of an `id`'s `from`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdFrom {
    Label,
    Property,
    Iri,
}

impl ClassStructure<'_> {
    // ── id ────────────────────────────────────────────────────────────────────

    fn lower_id(&self, node: &RawNode, seen: &mut ClassSeen, report: &mut ValidationReport) -> Option<IdPolicy> {
        check_attrs(node, &["curated-key", "from", "property", "under-parent", "verbatim"], report);
        check_children(node, &[], report);
        check_no_args(node, report);

        let key = key_attr(node, "curated-key", seen, report);
        let from = word_attr(
            node,
            "from",
            &[
                ("label", IdFrom::Label),
                ("property", IdFrom::Property),
                ("iri", IdFrom::Iri),
            ],
            report,
        );
        // A `property` beside another `from`, or beside none, is reported as
        // that, below, and what it names is not judged.
        let property = match from {
            Ok(Some(IdFrom::Label | IdFrom::Iri) | None) => Ok(None),
            Ok(Some(IdFrom::Property)) | Err(()) => self.id_property(node, report),
        };
        let under_parent = get_bool_attr(node, "under-parent", report);
        let verbatim = get_bool_attr(node, "verbatim", report);

        let has = |attr: &str| has_attr(node, attr);
        let mut found = Vec::new();
        if !has("curated-key") && !has("from") {
            found.push("has neither a 'curated-key' nor a 'from'");
        }
        match (from, has("property")) {
            (Ok(Some(IdFrom::Property)), false) => {
                found.push("has from=\"property\" but no 'property'");
            }
            (Ok(Some(IdFrom::Label | IdFrom::Iri) | None), true) => {
                found.push("takes 'property' only with from=\"property\"");
            }
            _ => {}
        }
        if has("verbatim") && matches!(from, Ok(Some(IdFrom::Label | IdFrom::Iri) | None)) {
            found.push("takes 'verbatim' only with from=\"property\"");
        }
        if has("under-parent") && self.member_of_node.is_none() {
            found.push("has 'under-parent' but its class has no 'member-of'");
        }
        if has("under-parent") && has("curated-key") {
            found.push("has both 'under-parent' and 'curated-key'");
        }
        let consistent = conflicts(node, &found, report);

        let source = match from.ok()? {
            None => None,
            Some(IdFrom::Label) => Some(IdSource::Label),
            Some(IdFrom::Iri) => Some(IdSource::Iri),
            Some(IdFrom::Property) if verbatim => Some(IdSource::Verbatim(property.ok()??)),
            Some(IdFrom::Property) => Some(IdSource::Property(property.ok()??)),
        };
        if !consistent {
            return None;
        }
        match (key.ok()?, source) {
            (Some(key), fallback) => Some(IdPolicy::Curated { key, fallback }),
            (None, Some(source)) => Some(IdPolicy::Minted { source, under_parent }),
            // Neither: reported above.
            (None, None) => None,
        }
    }

    /// An `id`'s `property` as the position of the property it names:
    /// `Ok(None)` when absent, `Err(())` when unreadable, empty, or naming no
    /// `text` property of the class that holds at most one value (already
    /// reported). A faulty property resolves: its fault is reported there.
    fn id_property(&self, node: &RawNode, report: &mut ValidationReport) -> Result<Option<PropertyIndex>, ()> {
        let Some(name) = optional_string_attr(node, "property", report)? else {
            return Ok(None);
        };
        let name = non_empty(name, "an id's property", node.span, report).ok_or(())?;
        let found = self.outline.property(self.index, &name);
        let fit = found.map(|(_, property)| fit_for_an_id(property));
        if matches!(fit, None | Some(Fit::No)) {
            push(
                report,
                DiagnosticKind::UnknownRemodelReference,
                node.span,
                format!("'id' names '{name}', which is no text property of its class with max=1"),
            );
            return Err(());
        }
        Ok(found.map(|(position, _)| PropertyIndex(position)))
    }

    // ── member-of ─────────────────────────────────────────────────────────────

    fn lower_member_of(
        &self,
        node: &RawNode,
        seen: &mut ClassSeen,
        report: &mut ValidationReport,
    ) -> Option<Membership> {
        check_attrs(node, &["seq", "via"], report);
        check_children(node, &[], report);

        let parent = single_string(node, "the parent class", report)
            .and_then(|name| non_empty(name, "a member-of's parent class", node.span, report));
        let parent = match parent {
            None => None,
            Some(name) => {
                let position = position_of(self.outline.class_names, &name);
                if position.is_none() {
                    push(
                        report,
                        DiagnosticKind::UnknownRemodelReference,
                        node.span,
                        format!("'member-of' names '{name}', which is no declared class"),
                    );
                }
                position
            }
        };
        let on_a_cycle = self.outline.is_on_a_cycle(self.index);
        if on_a_cycle {
            push(
                report,
                DiagnosticKind::RemodelMembershipCycle,
                node.span,
                "'member-of' makes its class a member of itself, directly or through its parents".to_owned(),
            );
        }

        let seq = match optional_string_attr(node, "seq", report) {
            Ok(None) => Ok(None),
            Ok(Some(name)) => non_empty(name, "a member-of's seq", node.span, report)
                .map(|name| {
                    declare_name(seen, &name, node.span, report);
                    Some(CpeProperty::new(name))
                })
                .ok_or(()),
            Err(()) => Err(()),
        };
        let via = self.member_via(node, parent, report);

        if on_a_cycle {
            return None;
        }
        Some(Membership { parent: ClassIndex(parent?), seq: seq.ok()?, via: via.ok()? })
    }

    /// A `member-of`'s `via`: a link property of the parent class, at `parent`, whose target is
    /// this class. `Ok(None)` when absent; `Err(())` when unreadable, empty, naming no such
    /// property, or when the parent is unknown (each reported where it stands).
    fn member_via(
        &self,
        node: &RawNode,
        parent: Option<usize>,
        report: &mut ValidationReport,
    ) -> Result<Option<PropertyIndex>, ()> {
        let Some(name) = optional_string_attr(node, "via", report)? else {
            return Ok(None);
        };
        let name = non_empty(name, "a member-of's via", node.span, report).ok_or(())?;
        let Some((position, property)) = self.outline.property(parent.ok_or(())?, &name) else {
            push(
                report,
                DiagnosticKind::UnknownRemodelReference,
                node.span,
                format!("'member-of' names '{name}' as its via, which is no property of the parent class"),
            );
            return Err(());
        };
        match self.links_here(node, &name, property, report) {
            true => Ok(Some(PropertyIndex(position))),
            false => Err(()),
        }
    }

    // ── member-count and incoming-count ───────────────────────────────────────

    fn lower_member_count(
        &self,
        node: &RawNode,
        seen: &mut ClassSeen,
        report: &mut ValidationReport,
    ) -> Option<RemodelMemberCount> {
        check_attrs(node, &["of"], report);
        check_children(node, &[], report);

        let name = property_name(node, seen, report);
        let of = required_class(node, "of", self.outline, report);
        let of = match of {
            Some(of) if !self.outline.may_be_member_of(of, self.index) => {
                push(
                    report,
                    DiagnosticKind::UnknownRemodelReference,
                    node.span,
                    "'member-count' names as its 'of' a class that is no member-of this one".to_owned(),
                );
                None
            }
            of => of,
        };
        Some(RemodelMemberCount { name: name?, of: ClassIndex(of?) })
    }

    fn lower_incoming_count(
        &self,
        node: &RawNode,
        seen: &mut ClassSeen,
        report: &mut ValidationReport,
    ) -> Option<RemodelIncomingCount> {
        check_attrs(node, &["from"], report);
        check_children(node, &["via"], report);

        let name = property_name(node, seen, report);
        let from = required_class(node, "from", self.outline, report);

        let via_nodes: Vec<&RawNode> = children_named(node, "via").collect();
        if via_nodes.is_empty() {
            push(
                report,
                DiagnosticKind::MissingRequiredAttribute,
                node.span,
                "'incoming-count' needs at least one 'via'".to_owned(),
            );
        }
        let mut counted = HashSet::new();
        let via: Vec<Option<PropertyIndex>> = via_nodes
            .into_iter()
            .map(|via| self.lower_via(via, from, &mut counted, report))
            .collect();
        let via: Option<Vec<PropertyIndex>> = via.into_iter().collect();

        Some(RemodelIncomingCount {
            name: name?,
            from: ClassIndex(from?),
            via: via.filter(|via| !via.is_empty())?,
        })
    }

    /// One `via` of an `incoming-count` on this class: a link property of the
    /// class at `from` that points at this class and that no earlier `via`
    /// (whose properties are `counted`) names, under any of its names.
    fn lower_via(
        &self,
        node: &RawNode,
        from: Option<usize>,
        counted: &mut HashSet<usize>,
        report: &mut ValidationReport,
    ) -> Option<PropertyIndex> {
        check_leaf(node, report);
        let name = single_string(node, "a property of the 'from' class", report)
            .and_then(|name| non_empty(name, "a via's property", node.span, report))?;
        // Without a readable `from` there is no class to look the property up in.
        let Some((position, property)) = self.outline.property(from?, &name) else {
            push(
                report,
                DiagnosticKind::UnknownRemodelReference,
                node.span,
                format!("'via' names '{name}', which is no property of the 'from' class"),
            );
            return None;
        };
        let message = || format!("'incoming-count' names the property '{name}' in two 'via' nodes");
        if !first_occurrence(counted, position, node.span, message, report) {
            return None;
        }
        self.links_here(node, &name, property, report)
            .then_some(PropertyIndex(position))
    }

    /// Whether `property`, which `node` names `name`, is a link whose target is this class. One
    /// that is no link, or links elsewhere, is a conflict at `node`; a faulty one is reported
    /// where it stands.
    fn links_here(&self, node: &RawNode, name: &str, property: &RawNode, report: &mut ValidationReport) -> bool {
        match kind_of(property) {
            Kind::Faulty => return false,
            Kind::Text | Kind::Other => {
                conflict(node, &format!("names '{name}', which is no link"), report);
                return false;
            }
            Kind::Link => {}
        }
        // A link whose `target` names no class is faulty: reported there.
        let Some(Value::String(target)) = get_attr(property, "target") else {
            return false;
        };
        let Some(target) = position_of(self.outline.class_names, &target) else {
            return false;
        };
        if target != self.index {
            conflict(node, &format!("names '{name}', whose target is not this class"), report);
            return false;
        }
        true
    }

    // ── members ───────────────────────────────────────────────────────────────

    fn lower_members(
        &self,
        node: &RawNode,
        seen: &mut ClassSeen,
        report: &mut ValidationReport,
    ) -> Option<RemodelMembers> {
        check_attrs(node, &["from"], report);
        check_children(node, &[], report);

        let name = property_name(node, seen, report);
        let from = required_class(node, "from", self.outline, report);
        Some(RemodelMembers { name: name?, from: ClassIndex(from?) })
    }

    // ── cover ─────────────────────────────────────────────────────────────────

    fn lower_cover(&self, node: &RawNode, seen: &mut ClassSeen, report: &mut ValidationReport) -> Option<CuratedKey> {
        check_attrs(node, &["curated-key"], report);
        check_children(node, &[], report);
        check_no_args(node, report);

        let key = key_attr(node, "curated-key", seen, report);
        if !has_attr(node, "curated-key") {
            push(
                report,
                DiagnosticKind::MissingRequiredAttribute,
                node.span,
                "'cover' is missing its required 'curated-key'".to_owned(),
            );
        }

        let outline = self.outline;
        let has_members = (0..outline.classes.len()).any(|member| outline.may_be_member_of(member, self.index));
        let mut found = Vec::new();
        if !outline.has_images {
            found.push("needs an 'images' node in the block");
        }
        if outline.declared_images.is_some() {
            found.push("stands in a block whose 'images' node names a declared class");
        }
        if !has_members {
            found.push("stands on a class no class is a member-of");
        }
        if has_attr(self.class, "image") {
            found.push("stands beside its class's 'image'");
        }
        let consistent = conflicts(node, &found, report);
        key.ok().flatten().filter(|_| consistent)
    }
}
