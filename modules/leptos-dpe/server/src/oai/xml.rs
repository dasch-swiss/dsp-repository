use chrono::Utc;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

use super::error::OaiError;
use super::metadata::{DataCiteRecord, DublinCoreRecord, OaiRecord};

pub const BASE_URL: &str = "https://meta.dasch.swiss/oai";

/// Earliest datestamp (fallback)
pub const EARLIEST_DATESTAMP: &str = "2015-01-01";

const OAI_NS: &str = "http://www.openarchives.org/OAI/2.0/";
const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
const OAI_DC_NS: &str = "http://www.openarchives.org/OAI/2.0/oai_dc/";
const DATACITE_NS: &str = "http://datacite.org/schema/kernel-4";
const OAI_DATACITE_NS: &str = "http://schema.datacite.org/oai/oai-1.1/";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

const OAI_PMH_SCHEMA_LOC: &str =
    "http://www.openarchives.org/OAI/2.0/ http://www.openarchives.org/OAI/2.0/OAI-PMH.xsd";
const OAI_DC_SCHEMA_LOC: &str =
    "http://www.openarchives.org/OAI/2.0/oai_dc/ http://www.openarchives.org/OAI/2.0/oai_dc.xsd";
const OAI_DATACITE_SCHEMA_LOC: &str =
    "http://schema.datacite.org/oai/oai-1.1/ http://schema.datacite.org/oai/oai-1.1/oai.xsd";
const DATACITE_SCHEMA_LOC: &str =
    "http://datacite.org/schema/kernel-4 https://schema.datacite.org/meta/kernel-4/metadata.xsd";

pub struct OaiXmlBuilder {
    writer: Writer<Cursor<Vec<u8>>>,
}

impl OaiXmlBuilder {
    /// Creates a new XML builder with the OAI-PMH root element.
    pub fn new() -> Self {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .expect("Failed to write XML declaration");

        let mut root = BytesStart::new("OAI-PMH");
        root.push_attribute(("xmlns", OAI_NS));
        root.push_attribute(("xmlns:xsi", XSI_NS));
        root.push_attribute(("xsi:schemaLocation", OAI_PMH_SCHEMA_LOC));
        writer.write_event(Event::Start(root)).expect("Failed to write root element");

        let response_date = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        writer.write_event(Event::Start(BytesStart::new("responseDate"))).expect("write");
        writer.write_event(Event::Text(BytesText::new(&response_date))).expect("write");
        writer.write_event(Event::End(BytesEnd::new("responseDate"))).expect("write");

        Self { writer }
    }

    fn write(&mut self, event: Event) {
        self.writer.write_event(event).expect("Failed to write XML event");
    }

    pub fn start_element(&mut self, name: &str) {
        self.write(Event::Start(BytesStart::new(name)));
    }

    pub fn end_element(&mut self, name: &str) {
        self.write(Event::End(BytesEnd::new(name)));
    }

    pub fn write_element(&mut self, name: &str, text: &str) {
        self.start_element(name);
        self.write(Event::Text(BytesText::new(text)));
        self.end_element(name);
    }

    fn write_prefixed_element(&mut self, prefix: &str, name: &str, text: &str) {
        let full_name = format!("{}:{}", prefix, name);
        self.start_element(&full_name);
        self.write(Event::Text(BytesText::new(text)));
        self.end_element(&full_name);
    }

    fn write_element_with_attrs(&mut self, name: &str, attrs: &[(&str, &str)], text: &str) {
        let mut elem = BytesStart::new(name);
        for (key, value) in attrs {
            elem.push_attribute((*key, *value));
        }
        self.write(Event::Start(elem));
        self.write(Event::Text(BytesText::new(text)));
        self.end_element(name);
    }

    /// Writes the request element with verb and parameters.
    pub fn write_request(&mut self, verb: &str, params: &[(&str, &str)]) {
        let mut request = BytesStart::new("request");
        request.push_attribute(("verb", verb));
        for (key, value) in params {
            request.push_attribute((*key, *value));
        }
        self.write(Event::Start(request));
        self.write(Event::Text(BytesText::new(BASE_URL)));
        self.end_element("request");
    }

    /// Writes the request element for badVerb error responses (no verb attribute).
    pub fn write_error_request(&mut self) {
        self.start_element("request");
        self.write(Event::Text(BytesText::new(BASE_URL)));
        self.end_element("request");
    }

    /// Writes the request element for error responses where the verb was recognized.
    pub fn write_error_request_with_verb(&mut self, verb: &str) {
        let mut request = BytesStart::new("request");
        request.push_attribute(("verb", verb));
        self.write(Event::Start(request));
        self.write(Event::Text(BytesText::new(BASE_URL)));
        self.end_element("request");
    }

    /// Writes an error response.
    pub fn write_error(&mut self, error: &OaiError) {
        let message = error.message();
        self.write_element_with_attrs("error", &[("code", error.code())], &message);
    }

    /// Writes the Identify response content.
    pub fn write_identify(&mut self, earliest_datestamp: &str) {
        self.start_element("Identify");
        self.write_element("repositoryName", "DaSCH Service Platform Repository");
        self.write_element("baseURL", BASE_URL);
        self.write_element("protocolVersion", "2.0");
        self.write_element("adminEmail", "info@dasch.swiss");
        self.write_element("earliestDatestamp", earliest_datestamp);
        self.write_element("deletedRecord", "no");
        self.write_element("granularity", "YYYY-MM-DD");
        self.end_element("Identify");
    }

    /// Writes the ListMetadataFormats response content.
    pub fn write_list_metadata_formats(&mut self) {
        self.start_element("ListMetadataFormats");

        self.start_element("metadataFormat");
        self.write_element("metadataPrefix", "oai_dc");
        self.write_element("schema", "http://www.openarchives.org/OAI/2.0/oai_dc.xsd");
        self.write_element("metadataNamespace", OAI_DC_NS);
        self.end_element("metadataFormat");

        self.start_element("metadataFormat");
        self.write_element("metadataPrefix", "oai_datacite");
        self.write_element("schema", "http://schema.datacite.org/oai/oai-1.1/oai.xsd");
        self.write_element("metadataNamespace", OAI_DATACITE_NS);
        self.end_element("metadataFormat");

        self.end_element("ListMetadataFormats");
    }

    /// Writes the ListSets response content.
    pub fn write_list_sets(&mut self) {
        self.start_element("ListSets");

        self.start_element("set");
        self.write_element("setSpec", "entityType:ProjectCluster");
        self.write_element("setName", "Project Clusters");
        self.end_element("set");

        self.start_element("set");
        self.write_element("setSpec", "entityType:ResearchProject");
        self.write_element("setName", "Research Projects");
        self.end_element("set");

        self.end_element("ListSets");
    }

    /// Writes a record header.
    pub fn write_record_header(&mut self, identifier: &str, datestamp: &str, set_specs: &[String]) {
        self.start_element("header");
        self.write_element("identifier", identifier);
        self.write_element("datestamp", datestamp);
        for set_spec in set_specs {
            self.write_element("setSpec", set_spec);
        }
        self.end_element("header");
    }

    /// Writes Dublin Core metadata.
    pub fn write_dublin_core_metadata(&mut self, dc: &DublinCoreRecord) {
        self.start_element("metadata");

        let mut dc_root = BytesStart::new("oai_dc:dc");
        dc_root.push_attribute(("xmlns:oai_dc", OAI_DC_NS));
        dc_root.push_attribute(("xmlns:dc", DC_NS));
        dc_root.push_attribute(("xmlns:xsi", XSI_NS));
        dc_root.push_attribute(("xsi:schemaLocation", OAI_DC_SCHEMA_LOC));
        self.write(Event::Start(dc_root));

        for title in &dc.titles {
            self.write_prefixed_element("dc", "title", title);
        }
        for creator in &dc.creators {
            self.write_prefixed_element("dc", "creator", creator);
        }
        for subject in &dc.subjects {
            self.write_prefixed_element("dc", "subject", subject);
        }
        for description in &dc.descriptions {
            self.write_prefixed_element("dc", "description", description);
        }
        if !dc.publisher.is_empty() {
            self.write_prefixed_element("dc", "publisher", &dc.publisher);
        }
        for contributor in &dc.contributors {
            self.write_prefixed_element("dc", "contributor", contributor);
        }
        for date in &dc.dates {
            self.write_prefixed_element("dc", "date", date);
        }
        if !dc.resource_type.is_empty() {
            self.write_prefixed_element("dc", "type", &dc.resource_type);
        }
        for identifier in &dc.identifiers {
            self.write_prefixed_element("dc", "identifier", identifier);
        }
        for language in &dc.languages {
            self.write_prefixed_element("dc", "language", language);
        }
        for relation in &dc.relations {
            self.write_prefixed_element("dc", "relation", relation);
        }
        for coverage in &dc.coverages {
            self.write_prefixed_element("dc", "coverage", coverage);
        }
        for rights in &dc.rights {
            self.write_prefixed_element("dc", "rights", rights);
        }

        self.write(Event::End(BytesEnd::new("oai_dc:dc")));
        self.end_element("metadata");
    }

    /// Writes DataCite 4.6 metadata.
    pub fn write_datacite_metadata(&mut self, datacite: &DataCiteRecord) {
        self.start_element("metadata");

        let mut oai_datacite = BytesStart::new("oai_datacite");
        oai_datacite.push_attribute(("xmlns", OAI_DATACITE_NS));
        oai_datacite.push_attribute(("xmlns:xsi", XSI_NS));
        oai_datacite.push_attribute(("xsi:schemaLocation", OAI_DATACITE_SCHEMA_LOC));
        self.write(Event::Start(oai_datacite));

        self.write_element("schemaVersion", "4.6");
        self.write_element("datacentreSymbol", "DASCH.DSP");
        self.start_element("payload");

        let mut resource = BytesStart::new("resource");
        resource.push_attribute(("xmlns", DATACITE_NS));
        resource.push_attribute(("xmlns:xsi", XSI_NS));
        resource.push_attribute(("xsi:schemaLocation", DATACITE_SCHEMA_LOC));
        self.write(Event::Start(resource));

        // Identifier (mandatory)
        self.write_element_with_attrs(
            "identifier",
            &[("identifierType", &datacite.identifier_type)],
            &datacite.identifier,
        );

        // Creators (mandatory)
        self.start_element("creators");
        for creator in &datacite.creators {
            self.start_element("creator");
            let mut attrs = vec![];
            if let Some(ref name_type) = creator.name_type {
                attrs.push(("nameType", name_type.as_str()));
            }
            self.write_element_with_attrs("creatorName", &attrs, &creator.name);
            self.end_element("creator");
        }
        self.end_element("creators");

        // Titles (mandatory)
        self.start_element("titles");
        for title in &datacite.titles {
            let mut attrs = vec![];
            if let Some(ref lang) = title.lang {
                attrs.push(("xml:lang", lang.as_str()));
            }
            if let Some(ref title_type) = title.title_type {
                attrs.push(("titleType", title_type.as_str()));
            }
            self.write_element_with_attrs("title", &attrs, &title.title);
        }
        self.end_element("titles");

        // Publisher (mandatory)
        self.write_element("publisher", &datacite.publisher);

        // PublicationYear (mandatory)
        self.write_element("publicationYear", &datacite.publication_year);

        // ResourceType (mandatory)
        self.write_element_with_attrs(
            "resourceType",
            &[("resourceTypeGeneral", &datacite.resource_type_general)],
            &datacite.resource_type,
        );

        // Subjects (recommended)
        if !datacite.subjects.is_empty() {
            self.start_element("subjects");
            for subject in &datacite.subjects {
                let mut attrs = vec![];
                if let Some(ref scheme) = subject.subject_scheme {
                    attrs.push(("subjectScheme", scheme.as_str()));
                }
                if let Some(ref scheme_uri) = subject.scheme_uri {
                    attrs.push(("schemeURI", scheme_uri.as_str()));
                }
                if let Some(ref lang) = subject.lang {
                    attrs.push(("xml:lang", lang.as_str()));
                }
                self.write_element_with_attrs("subject", &attrs, &subject.subject);
            }
            self.end_element("subjects");
        }

        // Contributors
        if !datacite.contributors.is_empty() {
            self.start_element("contributors");
            for contributor in &datacite.contributors {
                let mut contrib_elem = BytesStart::new("contributor");
                contrib_elem.push_attribute(("contributorType", contributor.contributor_type.as_str()));
                self.write(Event::Start(contrib_elem));
                let mut attrs = vec![];
                if let Some(ref name_type) = contributor.name_type {
                    attrs.push(("nameType", name_type.as_str()));
                }
                self.write_element_with_attrs("contributorName", &attrs, &contributor.name);
                self.end_element("contributor");
            }
            self.end_element("contributors");
        }

        // Descriptions (recommended)
        if !datacite.descriptions.is_empty() {
            self.start_element("descriptions");
            for desc in &datacite.descriptions {
                let mut attrs = vec![("descriptionType", desc.description_type.as_str())];
                if let Some(ref lang) = desc.lang {
                    attrs.push(("xml:lang", lang.as_str()));
                }
                self.write_element_with_attrs("description", &attrs, &desc.description);
            }
            self.end_element("descriptions");
        }

        // Dates
        if !datacite.dates.is_empty() {
            self.start_element("dates");
            for date in &datacite.dates {
                self.write_element_with_attrs("date", &[("dateType", &date.date_type)], &date.date);
            }
            self.end_element("dates");
        }

        // Language
        if let Some(ref language) = datacite.language {
            self.write_element("language", language);
        }

        // RelatedIdentifiers
        if !datacite.related_identifiers.is_empty() {
            self.start_element("relatedIdentifiers");
            for ri in &datacite.related_identifiers {
                self.write_element_with_attrs(
                    "relatedIdentifier",
                    &[
                        ("relatedIdentifierType", &ri.related_identifier_type),
                        ("relationType", &ri.relation_type),
                    ],
                    &ri.identifier,
                );
            }
            self.end_element("relatedIdentifiers");
        }

        // Rights
        if !datacite.rights_list.is_empty() {
            self.start_element("rightsList");
            for rights in &datacite.rights_list {
                let mut attrs = vec![];
                if let Some(ref uri) = rights.rights_uri {
                    attrs.push(("rightsURI", uri.as_str()));
                }
                if let Some(ref identifier) = rights.rights_identifier {
                    attrs.push(("rightsIdentifier", identifier.as_str()));
                }
                if let Some(ref scheme) = rights.rights_identifier_scheme {
                    attrs.push(("rightsIdentifierScheme", scheme.as_str()));
                }
                self.write_element_with_attrs("rights", &attrs, &rights.rights);
            }
            self.end_element("rightsList");
        }

        // GeoLocations
        if !datacite.geo_locations.is_empty() {
            self.start_element("geoLocations");
            for geo in &datacite.geo_locations {
                self.start_element("geoLocation");
                self.write_element("geoLocationPlace", &geo.geo_location_place);
                self.end_element("geoLocation");
            }
            self.end_element("geoLocations");
        }

        // FundingReferences
        if !datacite.funding_references.is_empty() {
            self.start_element("fundingReferences");
            for fr in &datacite.funding_references {
                self.start_element("fundingReference");
                self.write_element("funderName", &fr.funder_name);
                if let Some(ref number) = fr.award_number {
                    let mut attrs = vec![];
                    if let Some(ref uri) = fr.award_uri {
                        attrs.push(("awardURI", uri.as_str()));
                    }
                    self.write_element_with_attrs("awardNumber", &attrs, number);
                }
                if let Some(ref title) = fr.award_title {
                    self.write_element("awardTitle", title);
                }
                self.end_element("fundingReference");
            }
            self.end_element("fundingReferences");
        }

        self.write(Event::End(BytesEnd::new("resource")));
        self.end_element("payload");
        self.write(Event::End(BytesEnd::new("oai_datacite")));
        self.end_element("metadata");
    }

    /// Writes a complete OAI record.
    pub fn write_record(&mut self, record: &OaiRecord) {
        self.start_element("record");
        self.write_record_header(&record.header.identifier, &record.header.datestamp, &record.header.set_specs);
        if let Some(ref dc) = record.dublin_core {
            self.write_dublin_core_metadata(dc);
        }
        if let Some(ref datacite) = record.datacite {
            self.write_datacite_metadata(datacite);
        }
        self.end_element("record");
    }

    /// Finishes building the XML and returns the result as a string.
    pub fn finish(mut self) -> String {
        self.write(Event::End(BytesEnd::new("OAI-PMH")));
        String::from_utf8(self.writer.into_inner().into_inner()).expect("Invalid UTF-8 in XML output")
    }
}

impl Default for OaiXmlBuilder {
    fn default() -> Self {
        Self::new()
    }
}
