//! The typed form of a project's `remodel {}` block: the mapping the store
//! builder reads to turn an archive snapshot into CPE's store.
//!
//! Lowering (`validate::remodel`) builds a [`Remodel`] from the parsed config and
//! resolves every reference to an index, so a consumer never looks a name up.
//! Declared order is kept throughout. The newtypes keep the strings apart: an
//! archive IRI is never a CPE name, and neither is a shortcode.

/// Generates a string newtype with `new` and `as_str`.
macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name(String);

        impl $name {
            /// Wraps `value` without checking it; only lowering may call this, because it
            /// has validated the value.
            pub(crate) fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The wrapped string.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_newtype!(
    /// A project's DSP shortcode: four characters of `0-9A-F`.
    Shortcode
);
string_newtype!(
    /// The full IRI of a class in the archive's ontology.
    ArchiveClassIri
);
string_newtype!(
    /// The full IRI of a property in the archive's ontology.
    ArchivePropertyIri
);
string_newtype!(
    /// A class's name in CPE's store.
    CpeClass
);
string_newtype!(
    /// A property's name in CPE's store.
    CpeProperty
);

string_newtype!(
    /// A key of the project's curation table.
    CuratedKey
);

/// Whether the store builder cuts `c` from the ends of a text before it compares the text:
/// the whitespace of Python's `str.strip()`, which is `char::is_whitespace` and the separators
/// U+001C to U+001F. Lowering holds a sentinel to it and the builder trims by it, so the two
/// cannot disagree on what a stripped text can equal.
pub fn is_strip_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Position of a list in [`Remodel::lists`].
///
/// Only lowering builds one, from a position in the list it is producing, so every index in a
/// lowered [`Remodel`] is in bounds. Read it through [`Remodel::list`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListIndex(pub(crate) usize);

/// Position of a class in [`Remodel::classes`].
///
/// Only lowering builds one, from a position in the class list it is producing, so every
/// index in a lowered [`Remodel`] is in bounds. Read it through [`Remodel::class`]. Two indexes
/// of one `Remodel` are equal for the same class, and ordered as the classes are declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClassIndex(pub(crate) usize);

/// Position of a property in [`RemodelClass::properties`].
///
/// Only lowering builds one, from a position in the property list of the class it belongs to.
/// Read it through [`RemodelClass::property`] of that class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyIndex(pub(crate) usize);

/// The name of the link from a resource to its image, in CPE's store. A block with an `images`
/// node has the builder write that link itself, so lowering lets no node of such a block take
/// the name, and the builder writes the link under it.
pub const IMAGE_LINK: &str = "image";

/// A project's whole mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remodel {
    /// The project's shortcode.
    pub shortcode: Shortcode,
    /// The text standing for "missing" in the archive's data, when declared.
    pub sentinel: Option<String>,
    /// The site's languages, in declared order. A text per language is written in this order,
    /// and a curated text that must be in a language of the site is held to these.
    pub languages: Vec<String>,
    /// The lists, in declared order.
    pub lists: Vec<RemodelList>,
    /// The classes, in declared order.
    pub classes: Vec<RemodelClass>,
    /// The `images` block, when the project models files.
    pub images: Option<RemodelImages>,
    /// The `annotations` block, when the project models annotations.
    pub annotations: Option<RemodelAnnotations>,
    /// The `hook` node, when the project has a hook: code that runs over what the mapping
    /// gave. `None` says that no code takes part.
    pub hook: Option<RemodelHook>,
}

impl Remodel {
    /// The class `index` names.
    ///
    /// A [`ClassIndex`] cannot be built outside this crate, and lowering only builds one from a
    /// position in the same `classes` list, so the lookup is in bounds for any index taken from
    /// this `Remodel`. An index taken from a different `Remodel` is a caller error.
    pub fn class(&self, index: ClassIndex) -> &RemodelClass {
        &self.classes[index.0]
    }

    /// The classes in declared order, each with its index.
    pub fn indexed_classes(&self) -> impl Iterator<Item = (ClassIndex, &RemodelClass)> {
        let classes = self.classes.iter().enumerate();
        classes.map(|(position, class)| (ClassIndex(position), class))
    }

    /// The list `index` names.
    ///
    /// Like [`Remodel::class`]: in bounds for any index taken from this `Remodel`.
    pub fn list(&self, index: ListIndex) -> &RemodelList {
        &self.lists[index.0]
    }
}

/// One `list`: a name and its nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelList {
    /// The list's name.
    pub name: String,
    /// The nodes, in declared order.
    pub nodes: Vec<RemodelListNode>,
}

/// One `list-node`: a slug and a label per site language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelListNode {
    /// The node's slug, unique within its list.
    pub slug: String,
    /// The labels, in declared order.
    pub labels: Vec<RemodelLabel>,
}

/// A node's label in one language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelLabel {
    /// The language code.
    pub language: String,
    /// The label text.
    pub text: String,
}

/// One `class`: its CPE name, its archive IRI, how its resources get their id and their image,
/// what it is a member of, and its properties of every kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelClass {
    /// The class's name in CPE's store.
    pub name: CpeClass,
    /// The class's IRI in the archive.
    pub iri: ArchiveClassIri,
    /// Whether its resources carry no data address.
    pub no_addresses: bool,
    /// How a resource of the class gets its image.
    pub image: ImageRule,
    /// How a resource of the class gets its id.
    pub id: IdPolicy,
    /// The class its resources are members of, if any. An [`IdPolicy::Minted`] with
    /// `under_parent` stands only beside one.
    pub member_of: Option<Membership>,
    /// The properties, in declared order.
    pub properties: Vec<RemodelProperty>,
    /// The properties read from the curation table, in declared order.
    pub curated: Vec<RemodelCurated>,
    /// The properties counting the class's members, in declared order.
    pub member_counts: Vec<RemodelMemberCount>,
    /// The properties counting the links that point at the class, in declared order.
    pub incoming_counts: Vec<RemodelIncomingCount>,
}

impl RemodelClass {
    /// The property `index` names.
    ///
    /// In bounds for an index lowering built for this class: [`IdSource::Property`] of its own
    /// `id`, and a `via` of an incoming count whose `from` is this class.
    pub fn property(&self, index: PropertyIndex) -> &RemodelProperty {
        &self.properties[index.0]
    }

    /// The properties in declared order, each with its index.
    pub fn indexed_properties(&self) -> impl Iterator<Item = (PropertyIndex, &RemodelProperty)> {
        let properties = self.properties.iter().enumerate();
        properties.map(|(position, property)| (PropertyIndex(position), property))
    }

    /// Every curation-table key the class reads, each once: the keys of its `id`, its cover,
    /// its properties and its curated properties, in that order.
    pub fn curated_keys(&self) -> Vec<&CuratedKey> {
        // Every field of the class is named, so a new one must say whether it
        // reads a key.
        let Self {
            name: _,
            iri: _,
            no_addresses: _,
            image,
            id,
            member_of: _,
            properties,
            curated,
            member_counts: _,
            incoming_counts: _,
        } = self;
        let mut keys = Vec::new();
        match id {
            IdPolicy::Curated { key, .. } => keys.push(key),
            IdPolicy::Minted { .. } => {}
        }
        match image {
            ImageRule::Cover(key) => keys.push(key),
            ImageRule::Optional | ImageRule::Required => {}
        }
        for property in properties {
            match &property.source {
                PropertySource::Label { curated_key } => keys.extend(curated_key),
                PropertySource::Text { rule, .. } => match rule {
                    TextRule::CuratedOverride(key) => keys.push(key),
                    TextRule::Plain | TextRule::ExcludeLabel => {}
                },
                PropertySource::Date { display_key, .. } => keys.extend(display_key),
                PropertySource::Uri { .. } | PropertySource::Link { .. } => {}
            }
        }
        keys.extend(curated.iter().map(|curated| &curated.key));
        keys
    }
}

/// How a resource gets its image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageRule {
    /// The class has no image: the files of its resources are not read.
    Optional,
    /// Every resource has a still-image file, and that is its image. Only in a mapping with
    /// an `images` block.
    Required,
    /// The image is that of the member the curation table names under this key. Only in a
    /// mapping with an `images` block, on a class that has members.
    Cover(CuratedKey),
}

/// The text an id is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdSource {
    /// The resource's label.
    Label,
    /// The one value of a text property of the class that holds at most one.
    Property(PropertyIndex),
    /// The last part of the resource's IRI.
    Iri,
}

/// How a resource of a class gets its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdPolicy {
    /// The id is the value the curation table holds under `key`.
    Curated {
        /// The curation-table key.
        key: CuratedKey,
        /// What the id is made from when the table holds no value; `None` requires one.
        fallback: Option<IdSource>,
    },
    /// The id is made from `source`.
    Minted {
        /// What the id is made from.
        source: IdSource,
        /// Whether the id starts with the id of the resource's parent.
        under_parent: bool,
    },
}

/// A `member-of`: the class's resources are members of a resource of `parent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    /// The parent class; following parents never leads back to the class.
    pub parent: ClassIndex,
    /// The property a member's position is stored under, if it is stored.
    pub seq: Option<CpeProperty>,
}

/// One `member-count`: a property holding how many members a resource has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelMemberCount {
    /// The property's name in CPE's store.
    pub name: CpeProperty,
    /// The member class counted; it is a member of the class the count stands on.
    pub of: ClassIndex,
}

/// One `incoming-count`: a property holding how many links point at a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelIncomingCount {
    /// The property's name in CPE's store.
    pub name: CpeProperty,
    /// The class whose links are counted.
    pub from: ClassIndex,
    /// The link properties of `from` that are counted, in declared order; never empty. Each
    /// points at the class the count stands on.
    pub via: Vec<PropertyIndex>,
}

/// What a text value does when it is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyText {
    /// The empty value is left out.
    Drop,
    /// The empty value is refused.
    Refuse,
}

/// What a property does with a value that is the project's sentinel in full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentinelWhole {
    /// The value is refused.
    Refuse,
    /// The value is treated as absent.
    Absent,
    /// The value is kept as written.
    Keep,
}

/// What a property does with a value that contains the project's sentinel as a part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentinelWithin {
    /// The value is refused.
    Refuse,
    /// The value is kept as written.
    Keep,
}

/// How many values a property may hold. The most is never below the fewest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueCount {
    min: u32,
    max: Option<u32>,
}

impl ValueCount {
    /// A count of `min` to `max` values; `None` when `max` is below `min`.
    pub(crate) fn new(min: u32, max: Option<u32>) -> Option<Self> {
        if max.is_some_and(|max| max < min) {
            return None;
        }
        Some(Self { min, max })
    }

    /// The fewest values; `0` makes the property optional.
    pub fn min(&self) -> u32 {
        self.min
    }

    /// The most values; `None` for no limit.
    pub fn max(&self) -> Option<u32> {
        self.max
    }
}

/// The rule a text property has beyond its count and its policies. It has at
/// most one: a curated override and the label's exclusion never go together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRule {
    /// Neither.
    Plain,
    /// The curation-table key that overrides the value.
    CuratedOverride(CuratedKey),
    /// A value equal to the resource's label is left out.
    ExcludeLabel,
}

/// Where a `property` gets its value, with exactly the rules its form takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertySource {
    /// The resource's label.
    Label {
        /// The curation-table key that overrides the label.
        curated_key: Option<CuratedKey>,
    },
    /// An archive text property.
    Text {
        /// The property's IRI in the archive.
        iri: ArchivePropertyIri,
        /// How many values it may hold.
        count: ValueCount,
        /// Its curated override or its exclusion of the label, if either.
        rule: TextRule,
        /// What an empty value does.
        empty: EmptyText,
        /// What a value that is the sentinel in full does.
        sentinel: SentinelWhole,
        /// What a value that contains the sentinel does.
        sentinel_within: SentinelWithin,
    },
    /// An archive URI property.
    Uri {
        /// The property's IRI in the archive.
        iri: ArchivePropertyIri,
        /// How many values it may hold.
        count: ValueCount,
        /// What a value that is the sentinel in full does.
        sentinel: SentinelWhole,
        /// What a value that contains the sentinel does.
        sentinel_within: SentinelWithin,
    },
    /// An archive date property.
    Date {
        /// The property's IRI in the archive.
        iri: ArchivePropertyIri,
        /// How many values it may hold.
        count: ValueCount,
        /// The curation-table key holding the date's display text.
        display_key: Option<CuratedKey>,
    },
    /// An archive link property.
    Link {
        /// The property's IRI in the archive.
        iri: ArchivePropertyIri,
        /// How many values it may hold.
        count: ValueCount,
        /// The class the link points at.
        target: ClassIndex,
    },
}

impl PropertySource {
    /// The archive property's IRI; `None` for the label.
    pub fn iri(&self) -> Option<&ArchivePropertyIri> {
        match self {
            Self::Label { .. } => None,
            Self::Text { iri, .. } | Self::Uri { iri, .. } | Self::Date { iri, .. } | Self::Link { iri, .. } => {
                Some(iri)
            }
        }
    }
}

/// One `property` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelProperty {
    /// The CPE names the property is stored under, in declared order;
    /// never empty.
    pub names: Vec<CpeProperty>,
    /// Where its value comes from.
    pub source: PropertySource,
}

/// The form of a curated property's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CuratedForm {
    /// One text.
    Text,
    /// A node of a list.
    List(ListIndex),
    /// One text per language.
    PerLanguage,
}

/// One `curated` node: a property read from the curation table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelCurated {
    /// The property's name in CPE's store.
    pub name: CpeProperty,
    /// The curation-table key it reads.
    pub key: CuratedKey,
    /// The form of its value.
    pub form: CuratedForm,
    /// Whether every resource of the class must have a value.
    pub required: bool,
}

/// The `images` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelImages {
    /// The name of the class the images are stored as.
    pub class: CpeClass,
    /// The IIIF base every image URI starts with.
    pub iiif_base: String,
    /// The authorship every image carries.
    pub authorship: String,
    /// The licence every image carries.
    pub license: String,
    /// How many `class` nodes are written before the block. The images are stored at this
    /// place among the classes: after the resources of that many classes.
    pub classes_before: usize,
}

/// The `annotations` block: which classes can be annotated and which can be
/// the target of an annotation link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelAnnotations {
    /// The classes annotations sit on, in declared order.
    pub on: Vec<ClassIndex>,
    /// The classes annotation links can point at, in declared order.
    pub link_targets: Vec<ClassIndex>,
}

/// The `hook` node: everything the project's hook may do beyond reading. A change the node
/// does not list is not the hook's to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemodelHook {
    /// The curation-table keys the hook reads, in declared order.
    pub keys: Vec<HookKey>,
    /// The properties the hook writes and the mapping does not have, in declared order.
    pub provides: Vec<HookProvides>,
    /// The mapped properties whose values the hook may set, in declared order.
    pub overrides: Vec<HookOverrides>,
    /// The classes whose resources the hook may drop, in declared order, each once.
    pub drops: Vec<ClassIndex>,
}

/// One `key` of a `hook`: a curation-table key the hook reads on resources of `class`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookKey {
    /// The class whose resources carry the key.
    pub class: ClassIndex,
    /// The key; no other node of the class reads it.
    pub key: CuratedKey,
}

/// One `provides` of a `hook`: a property the hook writes on resources of `class`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookProvides {
    /// The class whose resources get the property.
    pub class: ClassIndex,
    /// The property's name in CPE's store; no node of the class yields it.
    pub name: CpeProperty,
}

/// One `overrides` of a `hook`: a `property` of `class` whose values the hook may set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HookOverrides {
    /// The class the property belongs to.
    pub class: ClassIndex,
    /// The property, among that class's properties.
    pub property: PropertyIndex,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every code point `c` with `chr(c).isspace()` in `python3`.
    const PY_SPACE: [u32; 29] = [
        0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x001C, 0x001D, 0x001E, 0x001F, 0x0020, 0x0085, 0x00A0, 0x1680, 0x2000,
        0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A, 0x2028, 0x2029, 0x202F, 0x205F,
        0x3000,
    ];

    #[test]
    fn test_is_strip_space_matches_python_for_every_char() {
        let ours: Vec<u32> = (0..=u32::from(char::MAX))
            .filter(|&code| char::from_u32(code).is_some_and(is_strip_space))
            .collect();

        assert_eq!(ours, PY_SPACE);
    }

    #[test]
    fn a_value_count_whose_most_is_below_its_fewest_cannot_be_built() {
        assert_eq!(ValueCount::new(2, Some(1)), None);
        assert_eq!(ValueCount::new(1, Some(0)), None);
    }

    #[test]
    fn a_value_count_reads_back_the_bounds_it_was_built_from() {
        let exact = ValueCount::new(1, Some(1)).expect("one to one is a count");
        assert_eq!((exact.min(), exact.max()), (1, Some(1)));
        let open = ValueCount::new(0, None).expect("no limit is a count");
        assert_eq!((open.min(), open.max()), (0, None));
        let range = ValueCount::new(2, Some(5)).expect("two to five is a count");
        assert_eq!((range.min(), range.max()), (2, Some(5)));
    }
}
