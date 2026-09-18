//! Graph fixtures shared by the writers' tests.
//!
//! One project, one lookup and one pair of temporal tables, so that the
//! DataCite, Dublin Core, JSON-LD and Signposting tests all describe the same
//! object and a change to the fixture shows up in every writer at once.

use std::collections::HashMap;

use shared_metadata::temporal_enrichment::EnrichedDate;
use shared_metadata::w3cdtf::{to_w3cdtf_range, W3cdtfRange};
use shared_metadata::{
    AccessRights, AccessRightsType, AuthorityFileReference, ContributorLookup, Funding, LegalInfo, License,
    Multilingual, Organization, Person, ProjectRaw, ProjectStatus, Record, RecordLegalInfo, RecordLicense, RecordPid,
};

use crate::graph::ResolveContext;
use crate::project_graph::ProjectGraph;

/// A second small copy of `resolve.rs`'s test lookup: that one is private to
/// its own test module, and neither is worth a public helper.
#[derive(Default)]
pub(crate) struct InMemoryContributorLookup {
    persons: HashMap<String, Person>,
    organizations: HashMap<String, Organization>,
}

impl ContributorLookup for InMemoryContributorLookup {
    fn person(&self, id: &str) -> Option<Person> {
        self.persons.get(id).cloned()
    }

    fn organization(&self, id: &str) -> Option<Organization> {
        self.organizations.get(id).cloned()
    }
}

pub(crate) fn lookup() -> InMemoryContributorLookup {
    let mut lookup = InMemoryContributorLookup::default();
    lookup.persons.insert(
        "person-001".to_string(),
        Person {
            id: "person-001".to_string(),
            given_names: vec!["Anna".to_string()],
            family_names: vec!["Müller".to_string()],
            job_titles: vec![],
            affiliations: vec![],
            same_as: vec![],
            email: None,
        },
    );
    // The one person carrying an identifier and an affiliation, so a writer's
    // ORCID and `affiliation` branches have something to read.
    lookup.persons.insert(
        "person-002".to_string(),
        Person {
            id: "person-002".to_string(),
            given_names: vec!["Jean".to_string()],
            family_names: vec!["Dupont".to_string()],
            job_titles: vec![],
            affiliations: vec!["organization-001".to_string()],
            same_as: vec![AuthorityFileReference {
                type_: "ORCID".to_string(),
                url: "https://orcid.org/0000-0002-1825-0097".to_string(),
                text: None,
            }],
            email: None,
        },
    );
    lookup.organizations.insert(
        "organization-001".to_string(),
        Organization {
            id: "organization-001".to_string(),
            name: "Schweizerischer Nationalfonds".to_string(),
            same_as: vec![],
            url: String::new(),
            address: None,
            email: None,
            alternative_name: None,
        },
    );
    lookup
}

/// A period cache keyed by bare id (as the real one is), so tests exercise
/// the real `/period/` URL-stripping in `timespan_for_in`.
pub(crate) fn periods() -> HashMap<String, W3cdtfRange> {
    HashMap::from([(
        "0vGXxVln724L".to_string(),
        to_w3cdtf_range(Some("98"), Some("117")).expect("a valid range"),
    )])
}

pub(crate) fn enrichment(entries: &[(&str, Option<&str>, &str)]) -> HashMap<String, EnrichedDate> {
    entries
        .iter()
        .map(|(key, date, name)| {
            (
                key.to_string(),
                EnrichedDate {
                    date: date.map(str::to_string),
                    original_name: name.to_string(),
                    source: "llm".to_string(),
                },
            )
        })
        .collect()
}

/// The default enrichment fixture for tests that are not specifically about
/// an empty or missing table. It does carry a `"Trajanic"` row with a
/// *different* range than the period cache, so a URL-tier test can prove the
/// URL wins over a same-named enrichment row.
pub(crate) fn default_enrichment() -> HashMap<String, EnrichedDate> {
    enrichment(&[
        ("Trajanic", Some("1111/2222"), "Trajanic"),
        ("Bronze Age", Some("-3300/-1200"), "Bronze Age"),
    ])
}

pub(crate) fn english(text: &str) -> Multilingual {
    Multilingual::from([("en".to_string(), text.to_string())])
}

pub(crate) fn temporal_reference(url: &str, text: Option<&str>) -> shared_metadata::TemporalCoverage {
    shared_metadata::TemporalCoverage::Reference(AuthorityFileReference {
        type_: "Chronontology".to_string(),
        url: url.to_string(),
        text: text.map(str::to_string),
    })
}

pub(crate) fn temporal_text(en: &str) -> shared_metadata::TemporalCoverage {
    shared_metadata::TemporalCoverage::Text(english(en))
}

pub(crate) fn legal(identifier: &str, uri: &str) -> LegalInfo {
    LegalInfo {
        license: License {
            license_identifier: identifier.to_string(),
            license_date: "2012-08-31".to_string(),
            license_uri: uri.to_string(),
        },
        copyright_holder: "Universität Basel".to_string(),
        authorship: vec!["person-001".to_string()],
    }
}

pub(crate) fn project() -> ProjectRaw {
    ProjectRaw {
        id: "0001".to_string(),
        pid: "https://ark.dasch.swiss/ark:/72163/1/0001".to_string(),
        name: "Rural Land Use".to_string(),
        shortcode: "0001".to_string(),
        official_name: "Rural Land Use in the Swiss Midlands, 1920-1950".to_string(),
        status: ProjectStatus::Finished,
        short_description: "Land use in the Swiss Midlands.".to_string(),
        description: english("A study of rural land use."),
        start_date: "2008-06-01".to_string(),
        end_date: "2012-08-31".to_string(),
        url: None,
        secondary_url: None,
        how_to_cite: "Rural Land Use (2012) DaSCH.".to_string(),
        access_rights: AccessRights {
            access_rights: AccessRightsType::FullOpenAccess,
            embargo_date: None,
        },
        legal_info: vec![legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/")],
        data_management_plan: None,
        data_publication_year: None,
        type_of_data: Some(vec!["Image".to_string()]),
        data_language: Some(vec!["de".to_string(), "fr".to_string()]),
        clusters: None,
        collections: None,
        records: None,
        keywords: vec![english("land use")],
        disciplines: vec![],
        temporal_coverage: vec![],
        spatial_coverage: vec![],
        attributions: vec![],
        abstract_text: None,
        contact_point: None,
        publications: None,
        funding: Funding::Grants(vec![]),
        alternative_names: None,
        documentation_material: None,
        provenance: None,
        additional_material: None,
        image_credit: None,
    }
}

pub(crate) fn record(id: &str, label: Multilingual) -> Record {
    Record {
        id: id.to_string(),
        pid: RecordPid::new("https://ark.dasch.swiss", "0001", id),
        label,
        access_rights: "Full Open Access".to_string(),
        legal_info: RecordLegalInfo {
            license: RecordLicense {
                license_identifier: "CC-BY-4.0".to_string(),
                license_date: "2024-01-15".to_string(),
                license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
            },
            copyright_holder: "Universität Basel".to_string(),
            authorship: vec!["Müller, Anna".to_string()],
        },
        how_to_cite: String::new(),
        publisher: "DaSCH".to_string(),
        source: String::new(),
        description: Multilingual::new(),
        date_created: "2024-01-15".to_string(),
        date_modified: "2024-06-30".to_string(),
        date_published: "2024-02-01".to_string(),
        type_of_data: "Text".to_string(),
        size: "2.3 GB".to_string(),
        keywords: vec![],
        file: None,
    }
}

/// Builds a graph over ad-hoc tables, proving the builder needs no `'static`
/// data and no process-global caches.
pub(crate) fn build(raw: &ProjectRaw, records: &[Record]) -> ProjectGraph {
    let lookup = lookup();
    let periods = periods();
    let enriched = default_enrichment();
    let ctx = ResolveContext::new(&lookup, &periods, &enriched);
    ProjectGraph::build(raw, &ctx, records)
}

pub(crate) fn build_with_enrichment(raw: &ProjectRaw, enriched: HashMap<String, EnrichedDate>) -> ProjectGraph {
    let lookup = lookup();
    let periods = periods();
    let ctx = ResolveContext::new(&lookup, &periods, &enriched);
    ProjectGraph::build(raw, &ctx, &[])
}
