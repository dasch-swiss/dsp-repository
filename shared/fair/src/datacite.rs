//! Transformation of Research Projects into DataCite 4.6 metadata.

use shared_metadata::temporal_coverage::Resolution;

use crate::helpers::{access_rights_to_string, format_date_range, infer_subject_scheme, map_contributor_type};
use crate::project_graph::ProjectGraph;
use crate::types::{
    DataCiteContributor, DataCiteCreator, DataCiteDate, DataCiteDescription, DataCiteFundingReference,
    DataCiteGeoLocation, DataCiteRecord, DataCiteRights, DataCiteSubject, DataCiteTitle,
};

const PUBLISHER: &str = "DaSCH";

pub fn project_to_datacite(graph: &ProjectGraph) -> DataCiteRecord {
    let mut datacite = DataCiteRecord {
        // Identifier (mandatory) - use PID or generate from shortcode
        identifier: graph.ark.clone(),
        identifier_type: "ARK".to_string(),
        ..Default::default()
    };

    // Creators (mandatory) - principal investigators and project leaders, or the
    // graph's organizational fallback when the project attributes none
    for agent in graph.creators_with_fallback().iter() {
        datacite.creators.push(DataCiteCreator {
            name: agent.name.clone(),
            name_type: Some(agent.kind.name_type().to_string()),
            given_name: agent.given_name.clone(),
            family_name: agent.family_name.clone(),
            name_identifiers: agent.name_identifiers.clone(),
            affiliations: agent.affiliations.clone(),
        });
    }

    // Contributors - non-creator attributions mapped to DataCite vocabulary
    for agent in &graph.contributors {
        let datacite_type = agent
            .contributor_type
            .first()
            .map(|t| map_contributor_type(t))
            .unwrap_or("Other");
        datacite.contributors.push(DataCiteContributor {
            name: agent.name.clone(),
            name_type: Some(agent.kind.name_type().to_string()),
            contributor_type: datacite_type.to_string(),
            given_name: agent.given_name.clone(),
            family_name: agent.family_name.clone(),
            name_identifiers: agent.name_identifiers.clone(),
            affiliations: agent.affiliations.clone(),
        });
    }

    // Titles (mandatory). The precedence — the longer of name/officialName
    // first, the rest as AlternativeTitle — is the graph's, so the JSON-LD
    // `name` and this one cannot pick differently.
    let (title, alternatives) = graph.titles();
    datacite.titles.push(DataCiteTitle { title, title_type: None, lang: None });
    for alternative in alternatives {
        datacite.titles.push(DataCiteTitle {
            title: alternative,
            title_type: Some("AlternativeTitle".to_string()),
            lang: None,
        });
    }

    // Publisher (mandatory)
    datacite.publisher = PUBLISHER.to_string();

    // PublicationYear (mandatory)
    datacite.publication_year = graph.publication_year.clone();

    // ResourceType (mandatory)
    datacite.resource_type = "Research Project".to_string();
    datacite.resource_type_general = "Project".to_string();

    // Subjects (recommended) - keywords without scheme info
    for keyword in &graph.keywords {
        datacite.subjects.push(DataCiteSubject {
            subject: keyword.clone(),
            subject_scheme: None,
            scheme_uri: None,
            lang: None,
        });
    }

    // Subjects from disciplines - with scheme info when available
    for discipline in &graph.disciplines {
        let (scheme, scheme_uri) = match discipline.authority_url {
            Some(ref url) => infer_subject_scheme(url),
            None => (None, None),
        };
        datacite.subjects.push(DataCiteSubject {
            subject: discipline.text.clone(),
            subject_scheme: scheme,
            scheme_uri,
            lang: None,
        });
    }

    // Descriptions (recommended)
    if let Some(ref abstract_text) = graph.abstract_text {
        datacite.descriptions.push(DataCiteDescription {
            description: abstract_text.clone(),
            description_type: "Abstract".to_string(),
            lang: None,
        });
    }
    if let Some(ref desc) = graph.description {
        datacite.descriptions.push(DataCiteDescription {
            description: desc.clone(),
            description_type: "Other".to_string(),
            lang: None,
        });
    }

    // Dates - use startDate/endDate range as dateType="Collected"
    // (kept on format_date_range: project start/end are full ISO YYYY-MM-DD dates,
    // so the year-only w3cdtf formatter used for Coverage would lose precision here.)
    if let Some(date_range) = format_date_range(&graph.start_date, &graph.end_date) {
        datacite.dates.push(DataCiteDate {
            date: date_range,
            date_type: "Collected".to_string(),
            ..Default::default()
        });
    }

    // Dates - temporal coverage as dateType="Coverage". A project may cover
    // several distinct periods, each emitted as its own Coverage date.
    for tc in &graph.temporal_coverage {
        if let Some(ref resolution) = tc.resolution {
            datacite.dates.push(coverage_date(resolution));
        }
    }

    // Language - from data_language (BCP 47 codes)
    if let Some(first_lang) = graph.data_language.first() {
        datacite.language = Some(first_lang.clone());
    }

    // RelatedIdentifiers -- should contain parent Project Cluster ARK.
    // TODO: Populate once Project Cluster data is available.

    // Rights - with SPDX identifier
    for legal in &graph.legal_info {
        // Placeholder only, deliberately: `helpers::real` would also drop an
        // empty URI, and an empty `rightsURI` is in the committed OAI output.
        // Nor is this `graph.license_uris()` — DataCite emits one entry per
        // `legalInfo` element, duplicates included.
        let rights_uri = if !shared_metadata::is_placeholder(&legal.license_uri) {
            Some(legal.license_uri.clone())
        } else {
            None
        };
        let has_identifier = legal.license_label.is_some();
        datacite.rights_list.push(DataCiteRights {
            rights: match legal.license_label {
                Some(ref label) => label.clone(),
                None => access_rights_to_string(&graph.access_rights).to_string(),
            },
            rights_uri,
            rights_identifier: has_identifier.then(|| legal.license_identifier.clone()),
            rights_identifier_scheme: has_identifier.then(|| "SPDX".to_string()),
        });
    }

    // GeoLocations from spatial_coverage
    for sc in &graph.spatial_coverage {
        if let Some(ref text) = sc.text {
            datacite
                .geo_locations
                .push(DataCiteGeoLocation { geo_location_place: text.clone() });
        }
    }

    // FundingReferences from grants; funder IDs resolved to organization names
    for grant in &graph.funding {
        for funder_name in &grant.funder_names {
            datacite.funding_references.push(DataCiteFundingReference {
                funder_name: funder_name.clone(),
                award_number: grant.number.clone(),
                award_title: grant.name.clone(),
                award_uri: grant.url.clone(),
            });
        }
    }

    datacite
}

/// The DataCite `Coverage` date shape for one resolved `temporalCoverage`
/// entry. The resolution chain behind it (ChronOntology URL → enrichment table
/// → name-only fallback) is `ProjectGraph::build`'s.
fn coverage_date(resolution: &Resolution) -> DataCiteDate {
    DataCiteDate {
        date: resolution.date.clone(),
        date_type: "Coverage".to_string(),
        date_information: resolution.date_information.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The resolution chain itself belongs to `ProjectGraph::build` and is
    /// tested there; this pins only the shape the writer gives a resolved entry.
    #[test]
    fn a_resolution_becomes_a_coverage_date() {
        let resolution = Resolution {
            date: "0098/0117".to_string(),
            date_information: Some("Trajanic".to_string()),
        };
        let date = coverage_date(&resolution);
        assert_eq!(date.date, "0098/0117");
        assert_eq!(date.date_type, "Coverage");
        assert_eq!(date.date_information.as_deref(), Some("Trajanic"));
    }
}
