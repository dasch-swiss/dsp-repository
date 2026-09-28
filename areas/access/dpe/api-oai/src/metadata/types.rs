//! OAI-PMH record types.

use shared_fair::{DataCiteRecord, DublinCoreRecord};

/// OAI-PMH record header containing identifier and datestamp.
#[derive(Debug)]
pub struct OaiRecordHeader {
    pub identifier: String,
    pub datestamp: String,
    pub set_specs: Vec<String>,
}

/// Complete OAI-PMH record with header and metadata.
#[derive(Debug)]
pub struct OaiRecord {
    pub header: OaiRecordHeader,
    pub dublin_core: Option<DublinCoreRecord>,
    pub datacite: Option<DataCiteRecord>,
}
