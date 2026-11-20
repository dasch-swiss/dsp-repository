//! Shared IRI constants and small term helpers.
//!
//! Namespaces are split into the *fixed* vocabularies (knora-base, rdf, rdfs,
//! xsd — stable across projects) and the *project* ontology, whose IRI is
//! detected from the dump at load time rather than hard-coded (see `dump.rs`).

use oxigraph::model::NamedNode;

// --- fixed vocabularies -----------------------------------------------------

#[cfg(test)]
pub const KB: &str = "http://www.knora.org/ontology/knora-base#";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";
pub const RDF_SUBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";
pub const RDF_PREDICATE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";
pub const RDF_OBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";
pub const RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
pub const RDFS_COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
pub const RDFS_SUBPROPERTYOF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
pub const XSD_ANYURI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";

/// knora-base local names this spike reaches for explicitly.
pub mod kb {
    pub const IS_DELETED: &str = "http://www.knora.org/ontology/knora-base#isDeleted";
    pub const IS_PART_OF: &str = "http://www.knora.org/ontology/knora-base#isPartOf";
    pub const SEQNUM: &str = "http://www.knora.org/ontology/knora-base#seqnum";
    pub const HAS_LINK_TO: &str = "http://www.knora.org/ontology/knora-base#hasLinkTo";
    /// Region → the image resource it marks up (kb built-in, used directly).
    pub const IS_REGION_OF: &str = "http://www.knora.org/ontology/knora-base#isRegionOf";
    /// Link minted from a standoff `<a>` in a richtext value (kb built-in).
    pub const HAS_STANDOFF_LINK_TO: &str = "http://www.knora.org/ontology/knora-base#hasStandoffLinkTo";
    pub const ATTACHED_TO_USER: &str = "http://www.knora.org/ontology/knora-base#attachedToUser";
    pub const VALUE_HAS_GEOMETRY: &str = "http://www.knora.org/ontology/knora-base#valueHasGeometry";
    pub const VALUE_HAS_COLOR: &str = "http://www.knora.org/ontology/knora-base#valueHasColor";
    pub const VALUE_HAS_URI: &str = "http://www.knora.org/ontology/knora-base#valueHasUri";
    pub const ATTACHED_TO_PROJECT: &str = "http://www.knora.org/ontology/knora-base#attachedToProject";
    pub const LINK_VALUE: &str = "http://www.knora.org/ontology/knora-base#LinkValue";
    pub const VALUE_HAS_STRING: &str = "http://www.knora.org/ontology/knora-base#valueHasString";
    pub const VALUE_HAS_INTEGER: &str = "http://www.knora.org/ontology/knora-base#valueHasInteger";
    pub const VALUE_HAS_DECIMAL: &str = "http://www.knora.org/ontology/knora-base#valueHasDecimal";
    pub const VALUE_HAS_BOOLEAN: &str = "http://www.knora.org/ontology/knora-base#valueHasBoolean";
    pub const VALUE_HAS_LANGUAGE: &str = "http://www.knora.org/ontology/knora-base#valueHasLanguage";
    pub const VALUE_HAS_ORDER: &str = "http://www.knora.org/ontology/knora-base#valueHasOrder";
    pub const VALUE_HAS_COMMENT: &str = "http://www.knora.org/ontology/knora-base#valueHasComment";
    pub const VALUE_CREATION_DATE: &str = "http://www.knora.org/ontology/knora-base#valueCreationDate";
    pub const VALUE_HAS_MAX_STANDOFF_START_INDEX: &str =
        "http://www.knora.org/ontology/knora-base#valueHasMaxStandoffStartIndex";
    pub const VALUE_HAS_CALENDAR: &str = "http://www.knora.org/ontology/knora-base#valueHasCalendar";
    pub const VALUE_HAS_START_JDN: &str = "http://www.knora.org/ontology/knora-base#valueHasStartJDN";
    pub const VALUE_HAS_END_JDN: &str = "http://www.knora.org/ontology/knora-base#valueHasEndJDN";
    pub const VALUE_HAS_START_PRECISION: &str = "http://www.knora.org/ontology/knora-base#valueHasStartPrecision";
    pub const VALUE_HAS_END_PRECISION: &str = "http://www.knora.org/ontology/knora-base#valueHasEndPrecision";
    pub const VALUE_HAS_UUID: &str = "http://www.knora.org/ontology/knora-base#valueHasUUID";
    /// How many links a LinkValue stands for; never carried.
    pub const VALUE_HAS_REF_COUNT: &str = "http://www.knora.org/ontology/knora-base#valueHasRefCount";
    /// Newer version → the version it replaced.
    pub const PREVIOUS_VALUE: &str = "http://www.knora.org/ontology/knora-base#previousValue";
    pub const VALUE_HAS_MAPPING: &str = "http://www.knora.org/ontology/knora-base#valueHasMapping";
    pub const VALUE_HAS_STANDOFF: &str = "http://www.knora.org/ontology/knora-base#valueHasStandoff";
    /// The XML DSP-API renders from a text value's standoff and stores beside it.
    pub const VALUE_HAS_XML: &str = "http://www.knora.org/ontology/knora-base#valueHasXml";
    /// Whether a text value is formatted; DSP-API writes it on new text values.
    pub const HAS_TEXT_VALUE_TYPE: &str = "http://www.knora.org/ontology/knora-base#hasTextValueType";
    pub const UNFORMATTED_TEXT: &str = "http://www.knora.org/ontology/knora-base#UnformattedText";
    pub const FORMATTED_TEXT: &str = "http://www.knora.org/ontology/knora-base#FormattedText";
    pub const CUSTOM_FORMATTED_TEXT: &str = "http://www.knora.org/ontology/knora-base#CustomFormattedText";
    pub const UNDEFINED_TEXT_TYPE: &str = "http://www.knora.org/ontology/knora-base#UndefinedTextType";
    pub const HAS_STILL_IMAGE_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#hasStillImageFileValue";
    pub const STILL_IMAGE_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#StillImageFileValue";
    pub const INTERNAL_FILENAME: &str = "http://www.knora.org/ontology/knora-base#internalFilename";
    pub const INTERNAL_MIME_TYPE: &str = "http://www.knora.org/ontology/knora-base#internalMimeType";
    pub const ORIGINAL_FILENAME: &str = "http://www.knora.org/ontology/knora-base#originalFilename";
    pub const ORIGINAL_MIME_TYPE: &str = "http://www.knora.org/ontology/knora-base#originalMimeType";
    pub const DIM_X: &str = "http://www.knora.org/ontology/knora-base#dimX";
    pub const DIM_Y: &str = "http://www.knora.org/ontology/knora-base#dimY";
    pub const HAS_AUTHORSHIP: &str = "http://www.knora.org/ontology/knora-base#hasAuthorship";
    pub const HAS_COPYRIGHT_HOLDER: &str = "http://www.knora.org/ontology/knora-base#hasCopyrightHolder";
    pub const CREATION_DATE: &str = "http://www.knora.org/ontology/knora-base#creationDate";
    pub const LAST_MODIFICATION_DATE: &str = "http://www.knora.org/ontology/knora-base#lastModificationDate";
    pub const HAS_PERMISSIONS: &str = "http://www.knora.org/ontology/knora-base#hasPermissions";
    pub const IS_PART_OF_VALUE: &str = "http://www.knora.org/ontology/knora-base#isPartOfValue";
    pub const HAS_LINK_TO_VALUE: &str = "http://www.knora.org/ontology/knora-base#hasLinkToValue";
    pub const IS_REGION_OF_VALUE: &str = "http://www.knora.org/ontology/knora-base#isRegionOfValue";
    pub const HAS_STANDOFF_LINK_TO_VALUE: &str = "http://www.knora.org/ontology/knora-base#hasStandoffLinkToValue";
    pub const HAS_COMMENT: &str = "http://www.knora.org/ontology/knora-base#hasComment";
    pub const HAS_COLOR: &str = "http://www.knora.org/ontology/knora-base#hasColor";
    pub const HAS_GEOMETRY: &str = "http://www.knora.org/ontology/knora-base#hasGeometry";
    pub const VALUE_HAS_LIST_NODE: &str = "http://www.knora.org/ontology/knora-base#valueHasListNode";
    pub const HAS_SUB_LIST_NODE: &str = "http://www.knora.org/ontology/knora-base#hasSubListNode";
    pub const LIST_NODE_POSITION: &str = "http://www.knora.org/ontology/knora-base#listNodePosition";
    pub const IS_ROOT_NODE: &str = "http://www.knora.org/ontology/knora-base#isRootNode";
    pub const HAS_ROOT_NODE: &str = "http://www.knora.org/ontology/knora-base#hasRootNode";
    pub const LIST_NODE_NAME: &str = "http://www.knora.org/ontology/knora-base#listNodeName";

    // Classes the snapshot knows a subject kind for (FORMAT.md §11).
    pub const REGION: &str = "http://www.knora.org/ontology/knora-base#Region";
    pub const LINK_OBJ: &str = "http://www.knora.org/ontology/knora-base#LinkObj";
    pub const LIST_NODE: &str = "http://www.knora.org/ontology/knora-base#ListNode";
    pub const TEXT_VALUE: &str = "http://www.knora.org/ontology/knora-base#TextValue";
    pub const INT_VALUE: &str = "http://www.knora.org/ontology/knora-base#IntValue";
    pub const DECIMAL_VALUE: &str = "http://www.knora.org/ontology/knora-base#DecimalValue";
    pub const BOOLEAN_VALUE: &str = "http://www.knora.org/ontology/knora-base#BooleanValue";
    pub const URI_VALUE: &str = "http://www.knora.org/ontology/knora-base#UriValue";
    pub const DATE_VALUE: &str = "http://www.knora.org/ontology/knora-base#DateValue";
    pub const LIST_VALUE: &str = "http://www.knora.org/ontology/knora-base#ListValue";
    pub const COLOR_VALUE: &str = "http://www.knora.org/ontology/knora-base#ColorValue";
    pub const GEOM_VALUE: &str = "http://www.knora.org/ontology/knora-base#GeomValue";
    pub const INTERVAL_VALUE: &str = "http://www.knora.org/ontology/knora-base#IntervalValue";
    pub const TIME_VALUE: &str = "http://www.knora.org/ontology/knora-base#TimeValue";
    pub const GEONAME_VALUE: &str = "http://www.knora.org/ontology/knora-base#GeonameValue";
    pub const AUDIO_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#AudioFileValue";
    pub const MOVING_IMAGE_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#MovingImageFileValue";
    pub const DOCUMENT_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#DocumentFileValue";
    pub const ARCHIVE_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#ArchiveFileValue";
    pub const TEXT_FILE_VALUE: &str = "http://www.knora.org/ontology/knora-base#TextFileValue";
    pub const STILL_IMAGE_EXTERNAL_FILE_VALUE: &str =
        "http://www.knora.org/ontology/knora-base#StillImageExternalFileValue";
}

// --- helpers ----------------------------------------------------------------

/// Build a `NamedNode` from a known-valid IRI string.
pub fn nn(iri: &str) -> NamedNode {
    NamedNode::new_unchecked(iri)
}
