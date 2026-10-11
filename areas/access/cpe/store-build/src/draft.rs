//! The draft of a build: every resource of the snapshot that has a class under the mapping, with
//! what the build has found out about it so far.

use std::collections::{BTreeMap, BTreeSet};

use cpe_core::remodel::{ClassIndex, RemodelClass, RemodelProperty};
use cpe_ports::{Calendar, DatePrecision, Resource};

use crate::error::BuildError;
use crate::generic::images::Scan;

/// What a resource is under the mapping.
#[derive(Clone, Copy)]
pub(crate) enum EntryClass<'a> {
    /// A resource of a class the mapping declares: the class and its index in the mapping.
    Mapped(ClassIndex, &'a RemodelClass),
    /// An annotation on a resource.
    Annotation,
    /// An annotation that links annotations and resources.
    AnnotationLink,
}

/// Whether the build still maps a resource.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Kept,
    /// A rule refused the resource: it is mapped no further, and what points at it is not
    /// refused for it.
    Refused,
    /// A hook dropped the resource: it gets no row in any table. It keeps its id, so that it
    /// stays in every check on ids and files, and what points at it is refused for it.
    Dropped,
}

/// Why the mapping gave a property of a resource no values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// A value under the property is of another kind than the property reads.
    Kind,
    /// The resource has not as many values under the property as the property takes.
    Count,
    /// A value was refused, or the text the property reads in its place was.
    Value,
}

/// A member's place: its one parent, by IRI, and its position among that parent's members.
#[derive(Clone, Copy)]
pub(crate) struct Member<'a> {
    pub(crate) parent: &'a str,
    pub(crate) seqnum: i64,
}

/// One value of a property of a resource, as the mapping gave it or a hook set it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    /// A text and its language, where it has one. An address is a text.
    Text(&'a str, Option<&'a str>),
    /// A date from `start` to `end`, both written at `precision`, with the text a reader is
    /// shown for it and that text's language, where the mapping asks for one.
    Date {
        start: String,
        end: String,
        precision: DatePrecision,
        calendar: Calendar,
        display: Option<(&'a str, &'a str)>,
    },
    /// A link to the resource of this IRI. Its id is read when the rows are written: a target
    /// that has none by then gives no row.
    Link(&'a str),
    /// A node of a list the mapping declares, by the node's id in `list_nodes`.
    ListNode(String),
    /// A link to the Image of this id.
    Image(&'a str),
    /// A number the mapping derived: a position or a count.
    Integer(i64),
}

/// The names the values of `node` are held under, and written under. It stands here, below
/// both the mapping and the generic code, because both name a property by them.
pub(crate) fn names(node: &RemodelProperty) -> impl Iterator<Item = &str> {
    node.names.iter().map(|name| name.as_str())
}

/// One resource of the draft.
pub(crate) struct Entry<'a> {
    pub(crate) resource: &'a Resource,
    pub(crate) class: EntryClass<'a>,
    /// `None` for a resource of a class without `member-of`, and for a member that was refused
    /// for its parent or its position.
    pub(crate) member: Option<Member<'a>>,
    pub(crate) state: State,
    /// The file of a resource whose class requires an image; `None` until it is read, and for
    /// a resource of any other class.
    pub(crate) scan: Option<Scan<'a>>,
    /// `None` until the id is minted, for a resource that was refused before or for it, and
    /// for an annotation, whose id is not held here.
    pub(crate) id: Option<String>,
    /// Whether the label was read and refused: what reads it then gives nothing and reports
    /// nothing of its own.
    pub(crate) label_refused: bool,
    /// The values of the properties mapped so far, by the property's name in the store, each
    /// property's in the order they are written in.
    pub(crate) values: BTreeMap<&'a str, Vec<Value<'a>>>,
    /// The properties a step of the mapping refused, by name, each with why. A property is
    /// here or in `values`, never in both.
    pub(crate) refused_properties: BTreeMap<&'a str, Cause>,
    /// The names whose values a hook set in place of the mapping's. Their number is the
    /// hook's, so it is held to the node's count once the hook has run.
    pub(crate) set_by_hook: BTreeSet<&'a str>,
    /// The refusals of the resource's property mapping, a value of another kind excepted. They
    /// are kept with the resource and reported when the build ends, also for a resource that
    /// was refused. A hook's drop withdraws them.
    pub(crate) held: Vec<BuildError>,
}

impl<'a> Entry<'a> {
    /// A kept entry of `class` for `resource`, which is not yet placed in a parent and has no id
    /// and no value.
    pub(crate) fn new(resource: &'a Resource, class: EntryClass<'a>) -> Self {
        Self {
            resource,
            class,
            member: None,
            state: State::Kept,
            scan: None,
            id: None,
            label_refused: false,
            values: BTreeMap::new(),
            refused_properties: BTreeMap::new(),
            set_by_hook: BTreeSet::new(),
            held: Vec::new(),
        }
    }

    /// Whether the resource is of the declared class `class`.
    pub(crate) fn is_of(&self, class: ClassIndex) -> bool {
        matches!(self.class, EntryClass::Mapped(own, _) if own == class)
    }

    /// Whether the build still maps the resource: no rule refused it and no hook dropped it.
    pub(crate) fn is_kept(&self) -> bool {
        self.state == State::Kept
    }

    /// The id of a resource the build still maps; `None` for one that gets no row.
    pub(crate) fn kept_id(&self) -> Option<&str> {
        match self.state {
            State::Kept => self.id.as_deref(),
            State::Refused | State::Dropped => None,
        }
    }

    /// The id of a resource a hook dropped: it has one and gets no row.
    pub(crate) fn dropped_id(&self) -> Option<&str> {
        match self.state {
            State::Dropped => self.id.as_deref(),
            State::Kept | State::Refused => None,
        }
    }
}

/// The entries by IRI, so that no walk over them follows the snapshot's order, which the port
/// leaves open, or a hash's.
#[derive(Default)]
pub(crate) struct Draft<'a> {
    entries: BTreeMap<&'a str, Entry<'a>>,
}

impl<'a> Draft<'a> {
    /// Adds `entry` under its resource's IRI. The port's contract holds that no two resources
    /// share one.
    pub(crate) fn insert(&mut self, entry: Entry<'a>) {
        self.entries.insert(entry.resource.iri.as_str(), entry);
    }

    /// The entry of the resource `iri`; `None` for a resource that has no class, which was
    /// refused for that.
    pub(crate) fn get(&self, iri: &str) -> Option<&Entry<'a>> {
        self.entries.get(iri)
    }

    /// The entries in the order of their IRIs.
    pub(crate) fn entries(&self) -> impl Iterator<Item = &Entry<'a>> {
        self.entries.values()
    }

    /// The resources of declared classes that the build still maps, each with its class and
    /// that class's index, in the order of their IRIs. An annotation is not among them. A
    /// resource has its id only once the ids are minted: the steps before that (membership,
    /// the files, the minting itself) walk resources without one.
    pub(crate) fn kept(&self) -> impl Iterator<Item = (&Entry<'a>, ClassIndex, &'a RemodelClass)> {
        self.entries().filter_map(|entry| match entry.class {
            EntryClass::Mapped(index, class) => entry.is_kept().then_some((entry, index, class)),
            EntryClass::Annotation | EntryClass::AnnotationLink => None,
        })
    }

    /// Changes the entry of the resource `iri`. A resource without an entry was refused for
    /// its class, and nothing is recorded about it.
    fn update(&mut self, iri: &str, change: impl FnOnce(&mut Entry<'a>)) {
        if let Some(entry) = self.entries.get_mut(iri) {
            change(entry);
        }
    }

    /// Places the resource `iri` in its parent, or refuses it when it has no place.
    pub(crate) fn place(&mut self, iri: &str, member: Option<Member<'a>>) {
        self.update(iri, |entry| {
            entry.member = member;
            if member.is_none() {
                entry.state = State::Refused;
            }
        });
    }

    /// Refuses the resource `iri`: it is mapped no further and gets no row. The refusal that
    /// says why is the caller's to report.
    pub(crate) fn refuse(&mut self, iri: &str) {
        self.update(iri, |entry| {
            entry.state = State::Refused;
            entry.id = None;
        });
    }

    /// Records the file of the resource `iri`, whose class requires an image.
    pub(crate) fn scan(&mut self, iri: &str, scan: Scan<'a>) {
        self.update(iri, |entry| entry.scan = Some(scan));
    }

    /// Gives the resource `iri` its id. It keeps the id when a hook drops it, and loses it
    /// when a rule refuses it.
    pub(crate) fn name(&mut self, iri: &str, id: String) {
        self.update(iri, |entry| entry.id = Some(id));
    }

    /// Records that the label of the resource `iri` was read and refused: a property that
    /// reads the label then gives nothing and reports nothing of its own.
    pub(crate) fn refuse_label(&mut self, iri: &str) {
        self.update(iri, |entry| entry.label_refused = true);
    }

    /// Drops the resource `iri` for a hook: it gets no row, keeps its id, and the refusals
    /// held on it are withdrawn. A resource that was refused stays refused.
    pub(crate) fn drop_entry(&mut self, iri: &str) {
        self.update(iri, |entry| {
            if entry.is_kept() {
                entry.state = State::Dropped;
                entry.held.clear();
            }
        });
    }

    /// Writes `values` under each of `names` on the resource `iri`, in place of what the
    /// names held.
    pub(crate) fn give(&mut self, iri: &str, names: impl IntoIterator<Item = &'a str>, values: &[Value<'a>]) {
        self.update(iri, |entry| {
            for name in names {
                entry.refused_properties.remove(name);
                entry.values.insert(name, values.to_vec());
            }
        });
    }

    /// [`give`](Self::give) for a hook that sets the values of a mapped property.
    pub(crate) fn set(&mut self, iri: &str, names: &[&'a str], values: &[Value<'a>]) {
        self.give(iri, names.iter().copied(), values);
        self.update(iri, |entry| entry.set_by_hook.extend(names));
    }

    /// Adds `value` after the values the resource `iri` has under `name`, for a property no
    /// node of the mapping yields: nothing is checked, and a refusal of `name` is not lifted.
    pub(crate) fn append(&mut self, iri: &str, name: &'a str, value: Value<'a>) {
        self.update(iri, |entry| entry.values.entry(name).or_default().push(value));
    }

    /// Records that a step refused the property of the resource `iri` that is written under
    /// `names`: it gives no values.
    pub(crate) fn refuse_property(&mut self, iri: &str, names: impl IntoIterator<Item = &'a str>, cause: Cause) {
        self.update(iri, |entry| {
            for name in names {
                entry.values.remove(name);
                entry.refused_properties.insert(name, cause);
            }
        });
    }

    /// Holds `refusals` on the resource `iri` until the build ends.
    pub(crate) fn hold(&mut self, iri: &str, refusals: Vec<BuildError>) {
        self.update(iri, |entry| entry.held.extend(refusals));
    }
}
