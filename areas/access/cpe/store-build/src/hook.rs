//! The seam between the mapping and a project's hook: the rules of a project that its
//! `remodel` block cannot say.
//!
//! A hook is called once, after the mapping, with the [`Draft`] and a sink for its refusals. It
//! does not get the snapshot: it reads a resource through a [`View`], which gives what the
//! mapping made of the resource, the resource's IRI, label and parents, and the archive's
//! values only of a property the block declares.
//! Every change goes through a method of the [`Draft`] that holds it against the block's `hook`
//! node. A change the node does not declare is not made, and the build refuses it
//! (`hook-exceeds-declaration`).
//!
//! The fields of [`Draft`] are private to this module, so a hook cannot reach the builder's own
//! draft, whose writers check nothing.

use std::cell::RefCell;
use std::collections::BTreeSet;

use cpe_core::remodel::{ClassIndex, PropertySource, Remodel, RemodelClass, RemodelHook, RemodelProperty};

use crate::draft::{self, Entry, EntryClass};
pub use crate::draft::{Cause, Value};
use crate::error::{BuildError, Rule};
use crate::generic::curation::Curation;
use crate::generic::{lists, text};
use crate::mapping::{label, values};

/// A project's rules that its mapping cannot say. It sees the draft and nothing else.
pub type HookFn = fn(&mut Draft<'_>, &mut Refusals);

/// One row of the builder's table of hooks.
pub struct Hook {
    /// The project's folder name.
    pub project: &'static str,
    pub remodel: HookFn,
}

/// Where a hook reports what it refuses. A refusal can be added and never taken back.
pub struct Refusals {
    refused: Vec<BuildError>,
}

impl Refusals {
    /// Refuses `subject`, a resource IRI or an id, by `rule`.
    pub fn refuse(&mut self, rule: Rule, subject: &str, detail: &str) {
        self.refused.push(BuildError::refused(rule, subject, detail));
    }
}

/// What the mapping made of one property of one resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapped<'d, 'a> {
    /// The property's values, in the order they are written in; none for a property that has
    /// none.
    Values(&'d [Value<'a>]),
    /// A step of the mapping refused the property, which is reported.
    Refused(Cause),
}

/// The mapped draft of a build, as a hook sees it.
pub struct Draft<'a> {
    entries: draft::Draft<'a>,
    curation: Curation<'a>,
    mapping: &'a Remodel,
    declared: &'a RemodelHook,
    /// What the hook's calls made the builder refuse: a read or a change beyond the
    /// declaration, a list value that names no node, and a curated text that holds what none
    /// may. A set, so that a call made twice is reported once.
    reported: RefCell<BTreeSet<BuildError>>,
}

/// Calls `hook` over `entries`, whose mapping declares it as `declared`, and collects in
/// `errors` what the hook refused and what its calls were refused for. Gives back the draft as
/// the hook left it.
pub(crate) fn call<'a>(
    hook: &Hook,
    entries: draft::Draft<'a>,
    curation: Curation<'a>,
    mapping: &'a Remodel,
    declared: &'a RemodelHook,
    errors: &mut Vec<BuildError>,
) -> (draft::Draft<'a>, Curation<'a>) {
    let mut draft = Draft {
        entries,
        curation,
        mapping,
        declared,
        reported: RefCell::new(BTreeSet::new()),
    };
    // Built here alone: a hook that could make an empty sink could put it in place of this one.
    let mut refusals = Refusals { refused: Vec::new() };
    (hook.remodel)(&mut draft, &mut refusals);
    errors.extend(refusals.refused);
    errors.extend(draft.reported.into_inner());
    (draft.entries, draft.curation)
}

impl<'a> Draft<'a> {
    /// The text that stands for "missing" in the archive's data, when the block declares one.
    pub fn sentinel(&self) -> Option<&'a str> {
        self.mapping.sentinel.as_deref()
    }

    /// The resource `iri`; `None` for one the snapshot does not have, and for one that was
    /// refused for its class.
    pub fn entry(&self, iri: &str) -> Option<View<'_, 'a>> {
        let entry = self.entries.get(iri)?;
        Some(View { draft: self, entry })
    }

    /// The resources of the class with the CPE name `class` that get a row, in the order of
    /// their IRIs: one that was refused or dropped is not among them. A name that is no class
    /// of the block is beyond the declaration, and gives none.
    pub fn entries_of(&self, class: &str) -> Vec<View<'_, 'a>> {
        let mut entries = self.all_entries_of(class);
        entries.retain(View::is_kept);
        entries
    }

    /// [`entries_of`](Self::entries_of) with the resources that get no row: those the mapping
    /// refused and those a hook dropped. For a rule that must see what is gone.
    pub fn all_entries_of(&self, class: &str) -> Vec<View<'_, 'a>> {
        let mut classes = self.mapping.indexed_classes();
        let Some((index, _)) = classes.find(|(_, declared)| declared.name.as_str() == class) else {
            self.exceed(class, "read the resources of a class the block does not declare".into());
            return Vec::new();
        };
        let entries = self.entries.entries();
        let entries = entries.filter(|entry| entry.is_of(index));
        entries.map(|entry| View { draft: self, entry }).collect()
    }

    /// Drops the resource `iri`: it gets no row in any table and keeps its id for the checks
    /// on ids and files, and the refusals of its property mapping are withdrawn. A resource
    /// that was refused stays refused. The block must declare `drops` for its class.
    pub fn drop(&mut self, iri: &str) {
        match self.class_of(iri) {
            Some(class) if self.declared.drops.contains(&class) => self.entries.drop_entry(iri),
            _ => self.exceed(iri, "drop".to_string()),
        }
    }

    /// Adds a text to the values of the property `name` of the resource `iri`. The block must
    /// declare `provides` for the name on its class. The text is written as given: the mapping
    /// has no node for the property and so no check, and what the text may hold is the hook's
    /// rule.
    pub fn provide_text(&mut self, iri: &str, name: &str, text: &'a str, lang: Option<&'a str>) {
        let Some(name) = self.provided_on(iri, name) else {
            return;
        };
        let value = Value::Text(text, lang);
        if self.holds_no_nul(iri, name, &value) {
            self.entries.append(iri, name, value);
        }
    }

    /// Whether `value`, which a hook gives for the property `name` of the resource `iri`, is
    /// free of U+0000, which no text of the store may hold (`interior-nul`). The snapshot's
    /// texts are checked before the mapping; a hook's own are checked here.
    fn holds_no_nul(&self, iri: &str, name: &str, value: &Value<'a>) -> bool {
        let nul = match value {
            Value::Text(text, lang) => text.contains('\0') || lang.is_some_and(|l| l.contains('\0')),
            Value::Date { .. } | Value::Link(_) | Value::ListNode(_) | Value::Image(_) | Value::Integer(_) => false,
        };
        if nul {
            let refusal = BuildError::refused(Rule::InteriorNul, iri, name);
            self.reported.borrow_mut().insert(refusal);
        }
        !nul
    }

    /// Adds the node `node` of the list `list` to the values of the property `name` of the
    /// resource `iri`. The block must declare `provides` for the name on its class, and the
    /// list and its node: a node the block does not have is refused (`unknown-list-node`).
    pub fn provide_list_value(&mut self, iri: &str, name: &str, list: &str, node: &str) {
        let Some(name) = self.provided_on(iri, name) else {
            return;
        };
        let lists = self.mapping.lists.iter();
        let found = lists.filter(|declared| declared.name == list).find_map(|declared| {
            let mut nodes = declared.nodes.iter();
            let node = nodes.find(|candidate| candidate.slug == node)?;
            Some(lists::node_id(declared, node))
        });
        match found {
            Some(id) => self.entries.append(iri, name, Value::ListNode(id)),
            None => {
                let refusal = BuildError::refused(Rule::UnknownListNode, iri, node);
                self.reported.borrow_mut().insert(refusal);
            }
        }
    }

    /// Sets the values of the mapped property `name` of the resource `iri`, under every name
    /// the property is written under. The block must declare `overrides` for it on its class.
    ///
    /// A value is of the kind the property writes: a text for a text, an address and a label;
    /// a link to a resource of the property's target class; a date the mapping gave. The
    /// checks the mapping makes of a text run on these values once the hook has returned.
    pub fn override_values(&mut self, iri: &str, name: &str, values: Vec<Value<'a>>) {
        let class = self.class_of(iri);
        let Some(node) = class.and_then(|class| self.overridden(class, name)) else {
            self.exceed(iri, format!("override {name}"));
            return;
        };
        if !values.iter().all(|value| self.fits(iri, node, value)) {
            let what = format!("override {name} with a value the property does not take");
            self.exceed(iri, what);
            return;
        }
        if !values.iter().all(|value| self.holds_no_nul(iri, name, value)) {
            return;
        }
        let names: Vec<&'a str> = draft::names(node).collect();
        self.entries.set(iri, &names, &values);
    }

    /// Whether `value` is one the property `node` of the resource `iri` writes.
    fn fits(&self, iri: &str, node: &RemodelProperty, value: &Value<'a>) -> bool {
        // Every source is named: a new one must say here what a hook may set for it.
        match &node.source {
            PropertySource::Label { .. } | PropertySource::Text { .. } | PropertySource::Uri { .. } => {
                matches!(value, Value::Text(..))
            }
            PropertySource::Link { target, .. } => {
                let Value::Link(to) = value else {
                    return false;
                };
                let linked = self.entries.get(to);
                linked.is_some_and(|linked| linked.is_of(*target))
            }
            // A hook writes no date of its own: it can leave out one the mapping gave.
            PropertySource::Date { .. } => {
                let entry = self.entries.get(iri);
                let given = entry.and_then(|entry| {
                    let name = draft::names(node).next()?;
                    entry.values.get(name)
                });
                matches!(value, Value::Date { .. }) && given.is_some_and(|given| given.contains(value))
            }
        }
    }

    /// The declared class of the resource `iri`; `None` for an annotation and for a resource
    /// the draft does not have.
    fn class_of(&self, iri: &str) -> Option<ClassIndex> {
        let (index, _) = mapped(self.entries.get(iri)?)?;
        Some(index)
    }

    /// The name as the block declares it under `provides` on `class`.
    fn provided(&self, class: ClassIndex, name: &str) -> Option<&'a str> {
        let declared: &'a RemodelHook = self.declared;
        let mut provides = declared.provides.iter();
        let found = provides.find(|provided| provided.class == class && provided.name.as_str() == name);
        found.map(|provided| provided.name.as_str())
    }

    /// [`provided`](Self::provided) for the class of the resource `iri`; an undeclared name is
    /// recorded.
    fn provided_on(&self, iri: &str, name: &str) -> Option<&'a str> {
        let class = self.class_of(iri);
        let found = class.and_then(|class| self.provided(class, name));
        if found.is_none() {
            self.exceed(iri, format!("provide {name}"));
        }
        found
    }

    /// The `property` node of `class` that is written under `name`, when the block declares
    /// `overrides` for it.
    fn overridden(&self, class: ClassIndex, name: &str) -> Option<&'a RemodelProperty> {
        let mapping: &'a Remodel = self.mapping;
        let on_class = self.declared.overrides.iter();
        let mut nodes = on_class
            .filter(|overridden| overridden.class == class)
            .map(|overridden| mapping.class(overridden.class).property(overridden.property));
        nodes.find(|node| draft::names(node).any(|own| own == name))
    }

    fn reads_key(&self, class: ClassIndex, key: &str) -> bool {
        let mut keys = self.declared.keys.iter();
        keys.any(|declared| declared.class == class && declared.key.as_str() == key)
    }

    /// Records that the hook went beyond the block on `subject`: a resource, by its IRI, or a
    /// class the block does not declare, by the name the hook gave.
    fn exceed(&self, subject: &str, what: String) {
        let refusal = BuildError::refused(Rule::HookExceedsDeclaration, subject, what);
        self.reported.borrow_mut().insert(refusal);
    }
}

/// The declared class of `entry` and its index; `None` for an annotation.
fn mapped<'a>(entry: &Entry<'a>) -> Option<(ClassIndex, &'a RemodelClass)> {
    match entry.class {
        EntryClass::Mapped(index, class) => Some((index, class)),
        EntryClass::Annotation | EntryClass::AnnotationLink => None,
    }
}

/// One resource of the draft, to read.
#[derive(Clone, Copy)]
pub struct View<'d, 'a> {
    draft: &'d Draft<'a>,
    entry: &'d Entry<'a>,
}

impl<'d, 'a> View<'d, 'a> {
    pub fn iri(&self) -> &'a str {
        self.entry.resource.iri.as_str()
    }

    pub fn label(&self) -> &'a str {
        self.entry.resource.label.as_str()
    }

    /// Whether the resource's label may be written as a text: it is not empty once stripped,
    /// holds no address where its class takes none, and does not hold the sentinel. This is
    /// the mapping's rule for a label, applied whether or not the mapping read the label: it
    /// reads none while a curated text stands in for it, and none of a class no node of which
    /// tells a property from it. Nothing is reported here.
    pub fn label_is_text(&self) -> bool {
        let no_addresses = mapped(self.entry).is_some_and(|(_, class)| class.no_addresses);
        let limits = label::limits(no_addresses, self.draft.sentinel(), true);
        text::offence(self.label(), limits).is_none()
    }

    /// The IRIs of the resource's parents, as served.
    pub fn part_of(&self) -> Vec<&'a str> {
        let parents = self.entry.resource.part_of.iter();
        parents.map(|parent| parent.as_str()).collect()
    }

    /// The CPE name of the resource's class; `None` for an annotation.
    pub fn class(&self) -> Option<&'a str> {
        mapped(self.entry).map(|(_, class)| class.name.as_str())
    }

    /// The id the mapping gave the resource; `None` for one that was refused before it got
    /// one, and for an annotation. A dropped resource keeps its id.
    pub fn id(&self) -> Option<&'d str> {
        self.entry.id.as_deref()
    }

    /// Whether the resource gets a row: it has an id, and was neither refused nor dropped.
    pub fn is_kept(&self) -> bool {
        self.entry.kept_id().is_some()
    }

    /// What the mapping made of the property `name`: of a `property` or a `curated` node of
    /// the resource's class, as far as the mapping goes before a hook is called, or of a name
    /// the block declares under `provides`, which holds what the hook provided so far. Any
    /// other name is beyond the declaration.
    pub fn values(&self, name: &str) -> Mapped<'d, 'a> {
        let declared = mapped(self.entry).is_some_and(|(index, class)| {
            let mut nodes = class.properties.iter();
            let mut curated = class.curated.iter();
            nodes.any(|node| draft::names(node).any(|own| own == name))
                || curated.any(|node| node.name.as_str() == name)
                || self.draft.provided(index, name).is_some()
        });
        if !declared {
            self.draft.exceed(self.iri(), format!("read {name}"));
            return Mapped::Values(&[]);
        }
        if let Some(cause) = self.entry.refused_properties.get(name) {
            return Mapped::Refused(*cause);
        }
        Mapped::Values(self.entry.values.get(name).map_or(&[], Vec::as_slice))
    }

    /// The values the resource has in the archive under the source of the `property` node
    /// `name`, as served: not counted, not checked, none left out. A value of another kind
    /// than the node reads is not among them. A name that is no `property` node of the
    /// resource's class is beyond the declaration.
    pub fn served(&self, name: &str) -> Vec<Value<'a>> {
        let node = mapped(self.entry).and_then(|(_, class)| {
            let mut nodes = class.properties.iter();
            nodes.find(|node| draft::names(node).any(|own| own == name))
        });
        match node {
            Some(node) => values::served(self.entry.resource, node),
            None => {
                self.draft.exceed(self.iri(), format!("read {name} as served"));
                Vec::new()
            }
        }
    }

    /// The texts the editors curated for the resource under `key`, each with its language and
    /// without the whitespace around it; one that is empty then is left out. `None` when a text
    /// holds an address or the sentinel: the build refuses the key for that, once, and only
    /// where a hook reads it so. The block must declare `key` for the key on the resource's
    /// class.
    pub fn curated(&self, key: &str) -> Option<Vec<(&'a str, Option<&'a str>)>> {
        if !self.reads(key) {
            return None;
        }
        let mut refused = Vec::new();
        let said = self.draft.curation.said(self.iri(), key, &mut refused);
        self.draft.reported.borrow_mut().extend(refused);
        said
    }

    /// The texts of [`curated`](Self::curated) as served: whitespace and all, none left out.
    pub fn curated_served(&self, key: &str) -> Vec<(&'a str, Option<&'a str>)> {
        if !self.reads(key) {
            return Vec::new();
        }
        let served = self.draft.curation.served(self.iri(), key);
        let texts = served.into_iter().map(|value| (value.text.as_str(), value.lang.as_deref()));
        texts.collect()
    }

    /// Whether the block declares `key` for the hook on the resource's class; an undeclared
    /// key is recorded.
    fn reads(&self, key: &str) -> bool {
        let declared = mapped(self.entry).is_some_and(|(class, _)| self.draft.reads_key(class, key));
        if !declared {
            self.draft.exceed(self.iri(), format!("read the curated key {key}"));
        }
        declared
    }
}
