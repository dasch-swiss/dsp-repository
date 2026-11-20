//! Every IRI the snapshot writes, one `const` each. `FORMAT.md` §15 indexes the
//! same terms; [`TERMS`] mirrors its rows, and `format_doc_tests` holds the two
//! equal and fails on a term `const` (any but a `*_PREFIX`) with no row.

use crate::model;

/// §2: a resource's `dao:Resource` type.
pub(crate) const RDF_TYPE: &str = model::RDF_TYPE;
/// §2: a resource's one plain label.
pub(crate) const RDFS_LABEL: &str = model::RDFS_LABEL;
/// §2: the type of every resource.
pub(crate) const DAO_RESOURCE: &str = "https://ontology.dasch.swiss/dao#Resource";
/// §2: a resource's class, the full IRI the dump records.
pub(crate) const DAO_SOURCE_CLASS: &str = "https://ontology.dasch.swiss/dao#sourceClass";
/// §2, §8: `"public"` or `"restricted-view"` on every resource and representation node (D15).
pub(crate) const DAO_ACCESS_RIGHTS: &str = "https://ontology.dasch.swiss/dao#accessRights";
/// §2, §8: `"unclassified"` wherever `dao:accessRights` is written (D16).
pub(crate) const DAO_DATA_CLASSIFICATION: &str = "https://ontology.dasch.swiss/dao#dataClassification";

/// §3: a value node's one value.
pub(crate) const RDF_VALUE: &str = model::RDF_VALUE;
/// §3: the type of every value node.
pub(crate) const DAO_VALUE: &str = "https://ontology.dasch.swiss/dao#Value";
/// §3: the UUID the node IRI is minted from, repeated as a plain literal.
pub(crate) const DAO_VALUE_HAS_UUID: &str = "https://ontology.dasch.swiss/dao#valueHasUUID";
/// §3: the edge predicate, repeated on the node.
pub(crate) const DAO_SOURCE_PROPERTY: &str = "https://ontology.dasch.swiss/dao#sourceProperty";
/// §5: the dump's `kb:valueHasOrder`, only when it records one.
pub(crate) const DAO_VALUE_HAS_ORDER: &str = "https://ontology.dasch.swiss/dao#valueHasOrder";
/// §4.1: a formatted text's XML, from `kb:valueHasXml` (D12).
pub(crate) const DAO_VALUE_HAS_XML: &str = "https://ontology.dasch.swiss/dao#valueHasXml";
/// §4.6: the datatype of a date's DSP lexical.
pub(crate) const DAO_DATE: &str = "https://ontology.dasch.swiss/dao#date";
/// §4.6: `"GREGORIAN"` or `"JULIAN"`.
pub(crate) const DAO_DATE_CALENDAR: &str = "https://ontology.dasch.swiss/dao#dateCalendar";
/// §4.6: the first Julian Day Number.
pub(crate) const DAO_DATE_START_JDN: &str = "https://ontology.dasch.swiss/dao#dateStartJDN";
/// §4.6: the last Julian Day Number.
pub(crate) const DAO_DATE_END_JDN: &str = "https://ontology.dasch.swiss/dao#dateEndJDN";
/// §4.6: `"YEAR"`, `"MONTH"` or `"DAY"` of the start.
pub(crate) const DAO_DATE_START_PRECISION: &str = "https://ontology.dasch.swiss/dao#dateStartPrecision";
/// §4.6: `"YEAR"`, `"MONTH"` or `"DAY"` of the end.
pub(crate) const DAO_DATE_END_PRECISION: &str = "https://ontology.dasch.swiss/dao#dateEndPrecision";
/// §4.7: the list node a list value names; list values have no `rdf:value`.
pub(crate) const DAO_SOURCE_LIST_NODE: &str = "https://ontology.dasch.swiss/dao#sourceListNode";
/// §4.8: the datatype of a colour's lexical.
pub(crate) const DAO_COLOR: &str = "https://ontology.dasch.swiss/dao#color";
/// §4.9: the datatype of a geometry's JSON.
pub(crate) const DAO_GEOMETRY: &str = "https://ontology.dasch.swiss/dao#geometry";
/// §7: a resource's parent, for every live parent under a sub-property of `kb:isPartOf`.
pub(crate) const DAO_IS_PART_OF: &str = "https://ontology.dasch.swiss/dao#isPartOf";
/// §7: a resource's one position among its parent's parts.
pub(crate) const DAO_SEQNUM: &str = "https://ontology.dasch.swiss/dao#seqnum";
/// §8: a resource's one representation.
pub(crate) const DAO_HAS_REPRESENTATION: &str = "https://ontology.dasch.swiss/dao#hasRepresentation";
/// §8: the type of every representation node.
pub(crate) const DAO_REPRESENTATION: &str = "https://ontology.dasch.swiss/dao#Representation";
/// §8: which of the four representation types a node is.
pub(crate) const DAO_REPRESENTATION_TYPE: &str = "https://ontology.dasch.swiss/dao#representationType";
/// §8: from `kb:StillImageFileValue`.
pub(crate) const DAO_STILL_IMAGE_REPRESENTATION: &str = "https://ontology.dasch.swiss/dao#StillImageRepresentation";
/// §8: from `kb:AudioFileValue`.
pub(crate) const DAO_AUDIO_REPRESENTATION: &str = "https://ontology.dasch.swiss/dao#AudioRepresentation";
/// §8: from `kb:MovingImageFileValue`.
pub(crate) const DAO_MOVING_IMAGE_REPRESENTATION: &str = "https://ontology.dasch.swiss/dao#MovingImageRepresentation";
/// §8: from `kb:DocumentFileValue`.
pub(crate) const DAO_DOCUMENT_REPRESENTATION: &str = "https://ontology.dasch.swiss/dao#DocumentRepresentation";
/// §8: the file's name in the archive, from `kb:internalFilename`.
pub(crate) const DAO_INTERNAL_FILENAME: &str = "https://ontology.dasch.swiss/dao#internalFilename";
/// §8: a still image's width.
pub(crate) const DAO_DIM_X: &str = "https://ontology.dasch.swiss/dao#dimX";
/// §8: a still image's height.
pub(crate) const DAO_DIM_Y: &str = "https://ontology.dasch.swiss/dao#dimY";
/// §8: one of the file's authors, from `kb:hasAuthorship` (D14).
pub(crate) const DAO_AUTHORSHIP: &str = "https://ontology.dasch.swiss/dao#authorship";
/// §8: the file's copyright holder, from `kb:hasCopyrightHolder` (D14).
pub(crate) const DAO_COPYRIGHT_HOLDER: &str = "https://ontology.dasch.swiss/dao#copyrightHolder";
/// §9: the type of every list node.
pub(crate) const SKOS_CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#Concept";
/// §9: a list node's labels, one per language tag.
pub(crate) const SKOS_PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
/// §9: a non-root list node's one parent.
pub(crate) const SKOS_BROADER: &str = "http://www.w3.org/2004/02/skos/core#broader";
/// §9: a non-root list node's position among its siblings, when recorded.
pub(crate) const DAO_LIST_NODE_POSITION: &str = "https://ontology.dasch.swiss/dao#listNodePosition";
/// §10: the second type of every annotation.
pub(crate) const OA_ANNOTATION: &str = "http://www.w3.org/ns/oa#Annotation";
/// §10: an annotation's one motivation.
pub(crate) const OA_MOTIVATED_BY: &str = "http://www.w3.org/ns/oa#motivatedBy";
/// §10: one per carried link of an annotation's target property.
pub(crate) const OA_HAS_TARGET: &str = "http://www.w3.org/ns/oa#hasTarget";
/// §10: a Region with a live comment.
pub(crate) const OA_COMMENTING: &str = "http://www.w3.org/ns/oa#commenting";
/// §10: a Region without one.
pub(crate) const OA_HIGHLIGHTING: &str = "http://www.w3.org/ns/oa#highlighting";
/// §10: a LinkObj.
pub(crate) const OA_LINKING: &str = "http://www.w3.org/ns/oa#linking";
/// §4.2, §5: integer values and orders.
pub(crate) const XSD_INTEGER: &str = model::XSD_INTEGER;
/// §4.3: decimal values.
pub(crate) const XSD_DECIMAL: &str = model::XSD_DECIMAL;
/// §4.4: boolean values.
pub(crate) const XSD_BOOLEAN: &str = model::XSD_BOOLEAN;
/// §4.5: URI values.
pub(crate) const XSD_ANYURI: &str = model::XSD_ANYURI;

/// §1: the one named graph is this prefix plus the project shortcode.
pub(crate) const PROJECT_GRAPH_PREFIX: &str = "urn:dsp:project:";
/// §3: a value node is this prefix plus its `kb:valueHasUUID`.
pub(crate) const VALUE_PREFIX: &str = "urn:dsp:value:";
/// §8: a representation node is this prefix plus its file value's `kb:valueHasUUID`.
pub(crate) const REPRESENTATION_PREFIX: &str = "urn:dsp:representation:";

/// `(IRI, FORMAT.md section, "DAO" or the §12 deviation id)`, one row per term above.
#[cfg(test)]
pub(crate) const TERMS: &[(&str, &str, &str)] = &[
    (RDF_TYPE, "§2", "DAO"),
    (RDFS_LABEL, "§2", "DAO"),
    (DAO_RESOURCE, "§2", "DAO"),
    (DAO_SOURCE_CLASS, "§2", "DAO"),
    (DAO_ACCESS_RIGHTS, "§2", "D15"),
    (DAO_DATA_CLASSIFICATION, "§2", "D16"),
    (RDF_VALUE, "§3", "DAO"),
    (DAO_VALUE, "§3", "DAO"),
    (DAO_VALUE_HAS_UUID, "§3", "DAO"),
    (DAO_SOURCE_PROPERTY, "§3", "DAO"),
    (DAO_VALUE_HAS_ORDER, "§5", "D4"),
    (DAO_VALUE_HAS_XML, "§4.1", "D12"),
    (DAO_DATE, "§4.6", "DAO"),
    (DAO_DATE_CALENDAR, "§4.6", "D6"),
    (DAO_DATE_START_JDN, "§4.6", "D6"),
    (DAO_DATE_END_JDN, "§4.6", "D6"),
    (DAO_DATE_START_PRECISION, "§4.6", "D6"),
    (DAO_DATE_END_PRECISION, "§4.6", "D6"),
    (DAO_SOURCE_LIST_NODE, "§4.7", "DAO"),
    (DAO_COLOR, "§4.8", "D10"),
    (DAO_GEOMETRY, "§4.9", "D10"),
    (DAO_IS_PART_OF, "§7", "DAO"),
    (DAO_SEQNUM, "§7", "DAO"),
    (DAO_HAS_REPRESENTATION, "§8", "DAO"),
    (DAO_REPRESENTATION, "§8", "DAO"),
    (DAO_REPRESENTATION_TYPE, "§8", "D7"),
    (DAO_STILL_IMAGE_REPRESENTATION, "§8", "D7"),
    (DAO_AUDIO_REPRESENTATION, "§8", "D7"),
    (DAO_MOVING_IMAGE_REPRESENTATION, "§8", "D7"),
    (DAO_DOCUMENT_REPRESENTATION, "§8", "D7"),
    (DAO_INTERNAL_FILENAME, "§8", "D7"),
    (DAO_DIM_X, "§8", "D7"),
    (DAO_DIM_Y, "§8", "D7"),
    (DAO_AUTHORSHIP, "§8", "D14"),
    (DAO_COPYRIGHT_HOLDER, "§8", "D14"),
    (SKOS_CONCEPT, "§9", "DAO"),
    (SKOS_PREF_LABEL, "§9", "DAO"),
    (SKOS_BROADER, "§9", "DAO"),
    (DAO_LIST_NODE_POSITION, "§9", "D8"),
    (OA_ANNOTATION, "§10", "DAO"),
    (OA_MOTIVATED_BY, "§10", "DAO"),
    (OA_HAS_TARGET, "§10", "DAO"),
    (OA_COMMENTING, "§10", "DAO"),
    (OA_HIGHLIGHTING, "§10", "DAO"),
    (OA_LINKING, "§10", "DAO"),
    (XSD_INTEGER, "§4.2", "DAO"),
    (XSD_DECIMAL, "§4.3", "DAO"),
    (XSD_BOOLEAN, "§4.4", "DAO"),
    (XSD_ANYURI, "§4.5", "DAO"),
];
