//! The boundary DTOs: one project's facts in the archive's shape.
//!
//! The shape follows the archive's canonical DAO model wherever its decisions have settled
//! (`dsp-repository-design`, `spycherli/decisions-active.md`). A field that DAO drops or leaves
//! open says so and names the decision: it is provisional.
//!
//! An adapter serves current, live facts only: no deleted resources or values, and no superseded
//! value versions. Every fact not listed here is omitted, never an error and never substituted by a
//! string form; `areas/access/cpe/CONTEXT.md` lists what is in and what is out.

use std::borrow::Borrow;

macro_rules! iri {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }
    };
}

iri! {
    /// The archive's resource IRI, e.g. `http://rdfh.ch/0803/<id>`.
    ResourceIri
}

iri! {
    /// A resource's own class IRI as the archive records it (`dao:sourceClass`): a project class.
    ClassIri
}

iri! {
    /// A value's source property, e.g. `incunabula:hasTitle`: the project property the value was
    /// recorded under (`dao:sourceProperty`), never the canonical predicate DAO's data carries.
    /// Two project properties that crosswalk to one standard term stay distinct, and the project's
    /// KDL names properties this way.
    PropertyIri
}

iri! {
    /// The IRI of a node in one of the project's lists.
    ListNodeIri
}

/// One project's current facts, whole.
///
/// A snapshot carries no revision or hash: every rebuild starts from empty (ADR-0008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub shortcode: String,
    /// Order unspecified; IRIs unique.
    pub resources: Vec<Resource>,
    /// Every list of the project, flattened; a root has no parent.
    ///
    /// Provisional: DAO keeps lists in a project's application profile (d.70), which the Access
    /// Area does not receive, so the snapshot carries them itself.
    pub list_nodes: Vec<ListNode>,
}

/// A resource of the project, with its values, file and membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub iri: ResourceIri,
    pub class: ClassIri,
    /// `rdfs:label`: exactly one, a plain literal.
    pub label: String,
    /// Within one property, the archive's order: `valueHasOrder` ascending, a missing order
    /// counting as 0, ties broken by UUID; links, which have neither, order by target IRI. The
    /// relative order of different properties is unspecified. Never holds membership.
    ///
    /// Provisional: DAO drops `valueHasOrder` (d.48).
    pub values: Vec<Value>,
    /// A Representation has exactly one file.
    pub file: Option<File>,
    /// The parents of `dao:isPartOf`; order unspecified. Several parents are legitimate, and a
    /// parent that is an omitted resource is left out here while `seqnum` stays.
    pub part_of: Vec<ResourceIri>,
    /// `dao:seqnum`, verbatim. Gaps, ties and a `seqnum` without a parent are archive facts;
    /// positional order and its tie-break are CPE's derivation, not the port's.
    pub seqnum: Option<i64>,
}

/// One value of a resource, under its source property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub property: PropertyIri,
    /// `dao:valueHasUUID`, unique within the resource. `None` exactly for a link, which DAO keeps
    /// as a bare resource-to-resource triple (d.69).
    pub uuid: Option<String>,
    pub kind: ValueKind,
}

/// The value kinds the port serves. `Decimal`, `Uri` and `lang` are passed through as the archive
/// records them; the port does not check their syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueKind {
    /// The plain text, also for a text that carries markup: the markup is left out, never the
    /// value. `lang` is the text's language, `None` where the archive records none.
    ///
    /// `lang` is provisional: DAO does not yet say whether a text keeps its language (d.48).
    Text {
        text: String,
        lang: Option<String>,
    },
    Integer(i64),
    /// The `xsd:decimal` lexical form.
    Decimal(String),
    Boolean(bool),
    Date(DateValue),
    /// The `xsd:anyURI` lexical form.
    Uri(String),
    ListNode(ListNodeIri),
    /// Subject to object, as the archive records it. A reverse link is CPE's derivation, and a link
    /// to an omitted resource or one outside the project is itself omitted.
    Link(ResourceIri),
}

/// A date as the archive records it; converting to year, month, day and era is CPE's remodel. A
/// single-point date has equal bounds.
///
/// Provisional: DAO keeps only a lexical date and drops the Julian Day Numbers (d.48); its date
/// vocabulary is open (Q29).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateValue {
    pub calendar: Calendar,
    pub start: DateBound,
    pub end: DateBound,
}

/// One bound of a date: a Julian Day Number and the precision it was recorded with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DateBound {
    pub jdn: i64,
    pub precision: DatePrecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatePrecision {
    Year,
    Month,
    Day,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Calendar {
    Gregorian,
    Julian,
}

/// A node of one of the project's lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListNode {
    pub iri: ListNodeIri,
    /// `None` for a root.
    pub parent: Option<ListNodeIri>,
    /// The node's position among its siblings, verbatim; roots have none.
    pub position: Option<u32>,
    /// The node's labels, language-tagged.
    pub labels: Vec<LangString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangString {
    pub text: String,
    pub lang: Option<String>,
}

/// A resource's file.
///
/// `asset` is the archive's internal filename, the key today's IIIF server serves by. It is
/// provisional, as are a still image's dimensions: DAO's file properties are open (Q10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum File {
    StillImage { asset: String, width: u32, height: u32 },
    Audio { asset: String },
    MovingImage { asset: String },
    Document { asset: String },
}
