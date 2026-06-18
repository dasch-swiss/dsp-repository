//! Shared `#[cfg(test)]` fixtures for view-rendering unit tests.
//!
//! Building a `Project` (and its sub-aggregates) inline in every component test
//! would mean repeating ~40 fields per file; a single well-populated builder is
//! clearer. Tests that need a variant (e.g. no publications) clone and override
//! the relevant field: `Project { publications: None, ..sample_project() }`.

use std::collections::HashMap;

use dpe_core::contributors::ResolvedContributor;
use dpe_core::models::AuthorityFileReference;
use dpe_core::organization::Organization;
use dpe_core::person::Person;
use dpe_core::project::{
    AccessRights, AccessRightsType, Attribution, Funding, Grant, LegalInfo, License, Project, ProjectStatus,
    Publication, Pid,
};

fn lang_map(value: &str) -> HashMap<String, String> {
    HashMap::from([("en".to_string(), value.to_string())])
}

fn url_ref(url: &str) -> AuthorityFileReference {
    AuthorityFileReference { type_: "URL".to_string(), url: url.to_string(), text: None }
}

/// A richly populated project covering the fields exercised by the detail-page
/// and card views.
pub(crate) fn sample_project() -> Project {
    Project {
        id: "sample".to_string(),
        pid: "10.5072/sample".to_string(),
        name: "Sample Research Project".to_string(),
        shortcode: "0ABC".to_string(),
        official_name: "Sample Research Project (official)".to_string(),
        status: ProjectStatus::Ongoing,
        short_description: "A short description of the sample project.".to_string(),
        description: lang_map("A longer description of the sample project."),
        start_date: "2020-01-01".to_string(),
        end_date: "2024-12-31".to_string(),
        url: Some(url_ref("https://example.org/project")),
        secondary_url: None,
        how_to_cite: "Sample, A. (2020). Sample Research Project.".to_string(),
        access_rights: AccessRights { access_rights: AccessRightsType::FullOpenAccess, embargo_date: None },
        legal_info: vec![LegalInfo {
            license: License {
                license_identifier: "CC BY 4.0".to_string(),
                license_date: "2020-01-01".to_string(),
                license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
            },
            copyright_holder: "DaSCH".to_string(),
            authorship: vec!["Sample, A.".to_string()],
        }],
        data_management_plan: None,
        data_publication_year: Some("2021".to_string()),
        type_of_data: Some(vec!["Text".to_string(), "Image".to_string()]),
        data_language: Some(vec!["en".to_string(), "de".to_string()]),
        clusters: vec![],
        collections: vec![],
        collection_ids: vec![],
        records: None,
        keywords: vec![lang_map("archaeology"), lang_map("history")],
        disciplines: vec![],
        temporal_coverage: vec![],
        spatial_coverage: vec![],
        attributions: vec![Attribution {
            contributor: "person-001".to_string(),
            contributor_type: vec!["Author".to_string()],
        }],
        abstract_text: Some(lang_map("An abstract of the sample project.")),
        contact_point: None,
        publications: Some(vec![Publication {
            text: "Sample, A. (2021). A paper.".to_string(),
            pid: Some(Pid { url: "https://doi.org/10.0/sample".to_string(), text: Some("DOI".to_string()) }),
        }]),
        funding: Funding::Grants(vec![Grant {
            funders: vec!["SNSF".to_string()],
            number: Some("12345".to_string()),
            name: Some("Sample Grant".to_string()),
            url: Some("https://example.org/grant".to_string()),
        }]),
        alternative_names: None,
        documentation_material: None,
        provenance: None,
        additional_material: None,
    }
}

/// A person with two affiliations resolved.
pub(crate) fn sample_person() -> Person {
    Person {
        id: "person-001".to_string(),
        given_names: vec!["Ada".to_string()],
        family_names: vec!["Lovelace".to_string()],
        job_titles: vec!["Researcher".to_string()],
        affiliations: vec!["organization-001".to_string()],
        same_as: vec![url_ref("https://orcid.org/0000-0000-0000-0000")],
        email: Some("ada@example.org".to_string()),
    }
}

/// An organization fixture.
pub(crate) fn sample_organization() -> Organization {
    Organization {
        id: "organization-001".to_string(),
        name: "Sample University".to_string(),
        same_as: vec![],
        url: "https://example.org/org".to_string(),
        address: None,
        email: None,
        alternative_name: None,
    }
}

/// A resolved person contributor with one affiliation and a role.
pub(crate) fn sample_person_contributor() -> ResolvedContributor {
    ResolvedContributor::Person {
        person: sample_person(),
        affiliations: vec![sample_organization()],
        roles: Some("Author".to_string()),
    }
}
