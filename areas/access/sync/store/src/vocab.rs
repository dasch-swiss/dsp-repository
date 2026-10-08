//! Every IRI the mapping reads, transcribed from `FORMAT.md` §15 at
//! `dasch-swiss/dsp-incubator@b2226ff4a987bf5f4b5cd0f33a8bf923a3033c7f`
//! (`cpe/tools/dao-lift/FORMAT.md`), the commit `areas/access/sync/data/PROVENANCE` pins.
//!
//! No IRI is written anywhere else in the crate. When the pin moves, re-transcribe from the new
//! commit's `FORMAT.md` and update the commit above.

use oxrdf::NamedNodeRef;

/// The graph every quad sits in is this prefix followed by the shortcode (§1).
pub const PROJECT_GRAPH_PREFIX: &str = "urn:dsp:project:";

/// A value node's IRI is this prefix followed by its `dao:valueHasUUID` (§3).
pub const VALUE_IRI_PREFIX: &str = "urn:dsp:value:";

// A predicate in none of these namespaces, with a resource as object, is a link (§3.1, §6).
pub const DAO_NAMESPACE: &str = "https://ontology.dasch.swiss/dao#";
pub const OA_NAMESPACE: &str = "http://www.w3.org/ns/oa#";
pub const RDF_NAMESPACE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const RDFS_NAMESPACE: &str = "http://www.w3.org/2000/01/rdf-schema#";

/// §2.
pub const RDF_TYPE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
/// §3.
pub const RDF_VALUE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#value");
/// §2.
pub const RDFS_LABEL: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2000/01/rdf-schema#label");

/// §4.2.
pub const XSD_INTEGER: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2001/XMLSchema#integer");
/// §4.3.
pub const XSD_DECIMAL: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2001/XMLSchema#decimal");
/// §4.4.
pub const XSD_BOOLEAN: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2001/XMLSchema#boolean");
/// §4.5.
pub const XSD_ANY_URI: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2001/XMLSchema#anyURI");

/// §2.
pub const DAO_RESOURCE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#Resource");
/// §2.
pub const DAO_SOURCE_CLASS: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#sourceClass");
/// §3.
pub const DAO_VALUE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#Value");
/// §3.
pub const DAO_VALUE_HAS_UUID: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#valueHasUUID");
/// §3.
pub const DAO_SOURCE_PROPERTY: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#sourceProperty");
/// §5.
pub const DAO_VALUE_HAS_ORDER: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#valueHasOrder");

/// §4.6.
pub const DAO_DATE: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#date");
/// §4.6.
pub const DAO_DATE_CALENDAR: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dateCalendar");
/// §4.6.
pub const DAO_DATE_START_JDN: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dateStartJDN");
/// §4.6.
pub const DAO_DATE_END_JDN: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dateEndJDN");
/// §4.6.
pub const DAO_DATE_START_PRECISION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dateStartPrecision");
/// §4.6.
pub const DAO_DATE_END_PRECISION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dateEndPrecision");

/// §4.7.
pub const DAO_SOURCE_LIST_NODE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#sourceListNode");

/// §7.
pub const DAO_IS_PART_OF: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#isPartOf");
/// §7.
pub const DAO_SEQNUM: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#seqnum");

/// §8.
pub const DAO_HAS_REPRESENTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#hasRepresentation");
/// §8.
pub const DAO_REPRESENTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#Representation");
/// §8.
pub const DAO_REPRESENTATION_TYPE: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#representationType");
/// §8.
pub const DAO_STILL_IMAGE_REPRESENTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#StillImageRepresentation");
/// §8.
pub const DAO_AUDIO_REPRESENTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#AudioRepresentation");
/// §8.
pub const DAO_MOVING_IMAGE_REPRESENTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#MovingImageRepresentation");
/// §8.
pub const DAO_DOCUMENT_REPRESENTATION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#DocumentRepresentation");
/// §8.
pub const DAO_INTERNAL_FILENAME: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#internalFilename");
/// §8.
pub const DAO_DIM_X: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dimX");
/// §8.
pub const DAO_DIM_Y: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#dimY");

/// §9.
pub const SKOS_CONCEPT: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2004/02/skos/core#Concept");
/// §9.
pub const SKOS_PREF_LABEL: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("http://www.w3.org/2004/02/skos/core#prefLabel");
/// §9.
pub const SKOS_BROADER: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/2004/02/skos/core#broader");
/// §9.
pub const DAO_LIST_NODE_POSITION: NamedNodeRef<'_> =
    NamedNodeRef::new_unchecked("https://ontology.dasch.swiss/dao#listNodePosition");

/// The one marker of an annotation, a Region or LinkObj of any subclass (§10). Never identify
/// one by `dao:sourceClass`.
pub const OA_ANNOTATION: NamedNodeRef<'_> = NamedNodeRef::new_unchecked("http://www.w3.org/ns/oa#Annotation");
