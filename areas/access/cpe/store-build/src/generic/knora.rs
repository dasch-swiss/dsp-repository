//! knora-base's classes and properties: what an annotation is and carries in every project of
//! the archive, so no mapping declares them.

pub(crate) const REGION_CLASS: &str = "http://www.knora.org/ontology/knora-base#Region";
pub(crate) const LINK_OBJ_CLASS: &str = "http://www.knora.org/ontology/knora-base#LinkObj";

pub(crate) const IS_REGION_OF: &str = "http://www.knora.org/ontology/knora-base#isRegionOf";
pub(crate) const HAS_LINK_TO: &str = "http://www.knora.org/ontology/knora-base#hasLinkTo";
pub(crate) const HAS_GEOMETRY: &str = "http://www.knora.org/ontology/knora-base#hasGeometry";
pub(crate) const HAS_COLOR: &str = "http://www.knora.org/ontology/knora-base#hasColor";
/// A Region's remark and a LinkObj's comment. A project's own comment property is another.
pub(crate) const HAS_COMMENT: &str = "http://www.knora.org/ontology/knora-base#hasComment";
