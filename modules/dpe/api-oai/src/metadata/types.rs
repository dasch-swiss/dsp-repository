//! OAI-PMH and metadata record types.

/// Dublin Core metadata record containing the 15 DC elements.
#[derive(Debug, Default)]
pub struct DublinCoreRecord {
    pub titles: Vec<String>,
    pub creators: Vec<String>,
    pub subjects: Vec<String>,
    pub descriptions: Vec<String>,
    pub publisher: String,
    pub contributors: Vec<String>,
    pub dates: Vec<String>,
    pub resource_type: String,
    pub identifiers: Vec<String>,
    pub languages: Vec<String>,
    pub relations: Vec<String>,
    pub coverages: Vec<String>,
    pub rights: Vec<String>,
}

/// DataCite 4.6 metadata record containing mandatory and recommended properties.
#[derive(Debug, Default)]
pub struct DataCiteRecord {
    pub identifier: String,
    pub identifier_type: String,
    pub creators: Vec<DataCiteCreator>,
    pub titles: Vec<DataCiteTitle>,
    pub publisher: String,
    pub publication_year: String,
    pub resource_type: String,
    pub resource_type_general: String,
    pub subjects: Vec<DataCiteSubject>,
    pub contributors: Vec<DataCiteContributor>,
    pub descriptions: Vec<DataCiteDescription>,
    pub dates: Vec<DataCiteDate>,
    pub language: Option<String>,
    pub related_identifiers: Vec<DataCiteRelatedIdentifier>,
    pub rights_list: Vec<DataCiteRights>,
    pub geo_locations: Vec<DataCiteGeoLocation>,
    pub funding_references: Vec<DataCiteFundingReference>,
}

#[derive(Debug, Default)]
pub struct DataCiteCreator {
    pub name: String,
    pub name_type: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteContributor {
    pub name: String,
    pub name_type: Option<String>,
    pub contributor_type: String,
}

#[derive(Debug, Default)]
pub struct DataCiteTitle {
    pub title: String,
    pub title_type: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteSubject {
    pub subject: String,
    pub subject_scheme: Option<String>,
    pub scheme_uri: Option<String>,
    pub lang: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteDescription {
    pub description: String,
    pub description_type: String,
    pub lang: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteDate {
    pub date: String,
    pub date_type: String,
}

#[derive(Debug, Default)]
pub struct DataCiteRelatedIdentifier {
    pub identifier: String,
    pub related_identifier_type: String,
    pub relation_type: String,
}

#[derive(Debug, Default)]
pub struct DataCiteRights {
    pub rights: String,
    pub rights_uri: Option<String>,
    pub rights_identifier: Option<String>,
    pub rights_identifier_scheme: Option<String>,
}

#[derive(Debug, Default)]
pub struct DataCiteGeoLocation {
    pub geo_location_place: String,
}

#[derive(Debug, Default)]
pub struct DataCiteFundingReference {
    pub funder_name: String,
    pub award_number: Option<String>,
    pub award_title: Option<String>,
    pub award_uri: Option<String>,
}

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
