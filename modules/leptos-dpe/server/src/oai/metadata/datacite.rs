//! Transformation of Research Projects into DataCite 4.6 metadata.

use app::domain::{Discipline, Funding, Project, TemporalCoverage};

use super::helpers::{
    access_rights_to_string, extract_year, format_date_range, get_multilingual_value,
    infer_subject_scheme, is_creator, license_identifier_to_label, map_contributor_type,
};
use super::types::{
    DataCiteContributor, DataCiteCreator, DataCiteDate, DataCiteDescription,
    DataCiteFundingReference, DataCiteGeoLocation, DataCiteRecord, DataCiteRights,
    DataCiteSubject, DataCiteTitle,
};

const PUBLISHER: &str = "DaSCH";

pub fn project_to_datacite(project: &Project) -> DataCiteRecord {
    let mut record = DataCiteRecord::default();

    // Identifier (mandatory) - use PID or generate from shortcode
    if project.pid != "MISSING" && !project.pid.is_empty() {
        record.identifier = project.pid.clone();
        record.identifier_type = "ARK".to_string();
    } else {
        record.identifier = format!("ark:/72163/1/{}", project.shortcode);
        record.identifier_type = "ARK".to_string();
    }

    // Creators (mandatory) - principal investigators and project leaders
    for attr in &project.attributions {
        if is_creator(&attr.contributor_type) {
            record.creators.push(DataCiteCreator {
                name: attr.contributor.clone(),
                name_type: Some("Personal".to_string()),
            });
        }
    }
    // Ensure at least one creator
    if record.creators.is_empty() {
        record.creators.push(DataCiteCreator {
            name: "DaSCH".to_string(),
            name_type: Some("Organizational".to_string()),
        });
    }

    // Contributors - non-creator attributions mapped to DataCite vocabulary
    for attr in &project.attributions {
        if !is_creator(&attr.contributor_type) {
            let datacite_type = attr
                .contributor_type
                .first()
                .map(|t| map_contributor_type(t))
                .unwrap_or("Other");
            record.contributors.push(DataCiteContributor {
                name: attr.contributor.clone(),
                name_type: Some("Personal".to_string()),
                contributor_type: datacite_type.to_string(),
            });
        }
    }

    // Titles (mandatory)
    // Use the longer of name/officialName as primary, shorter as AlternativeTitle
    let name_valid = project.name != "MISSING" && !project.name.is_empty();
    let official_valid = project.official_name != "MISSING" && !project.official_name.is_empty();

    match (name_valid, official_valid) {
        (true, true) => {
            let (primary, alternative) = if project.official_name.len() >= project.name.len() {
                (&project.official_name, &project.name)
            } else {
                (&project.name, &project.official_name)
            };
            record.titles.push(DataCiteTitle {
                title: primary.clone(),
                title_type: None,
                lang: None,
            });
            if primary != alternative {
                record.titles.push(DataCiteTitle {
                    title: alternative.clone(),
                    title_type: Some("AlternativeTitle".to_string()),
                    lang: None,
                });
            }
        }
        (false, true) => {
            record.titles.push(DataCiteTitle {
                title: project.official_name.clone(),
                title_type: None,
                lang: None,
            });
        }
        _ => {
            record.titles.push(DataCiteTitle {
                title: project.name.clone(),
                title_type: None,
                lang: None,
            });
        }
    }

    // Additional alternative names
    if let Some(ref alt_names) = project.alternative_names {
        for alt_name_map in alt_names {
            if let Some(alt_name) = get_multilingual_value(alt_name_map) {
                let already_present = record.titles.iter().any(|t| t.title == alt_name);
                if !already_present {
                    record.titles.push(DataCiteTitle {
                        title: alt_name,
                        title_type: Some("AlternativeTitle".to_string()),
                        lang: None,
                    });
                }
            }
        }
    }

    // Publisher (mandatory)
    record.publisher = PUBLISHER.to_string();

    // PublicationYear (mandatory)
    if let Some(ref pub_year) = project.data_publication_year {
        record.publication_year = extract_year(pub_year);
    } else {
        record.publication_year = extract_year(&project.start_date);
    }

    // ResourceType (mandatory)
    record.resource_type = "Research Project".to_string();
    record.resource_type_general = "Project".to_string();

    // Subjects (recommended) - keywords without scheme info
    for kw in &project.keywords {
        if let Some(keyword) = get_multilingual_value(kw) {
            record.subjects.push(DataCiteSubject {
                subject: keyword,
                subject_scheme: None,
                scheme_uri: None,
                lang: None,
            });
        }
    }

    // Subjects from disciplines - with scheme info when available
    for discipline in &project.disciplines {
        match discipline {
            Discipline::Reference(ref_data) => {
                if let Some(ref text) = ref_data.text {
                    let (scheme, scheme_uri) = infer_subject_scheme(&ref_data.url);
                    record.subjects.push(DataCiteSubject {
                        subject: text.clone(),
                        subject_scheme: scheme,
                        scheme_uri,
                        lang: None,
                    });
                }
            }
            Discipline::Text(text_map) => {
                if let Some(text) = get_multilingual_value(text_map) {
                    record.subjects.push(DataCiteSubject {
                        subject: text,
                        subject_scheme: None,
                        scheme_uri: None,
                        lang: None,
                    });
                }
            }
        }
    }

    // Descriptions (recommended)
    if let Some(ref abstract_map) = project.abstract_text {
        if let Some(abstract_text) = get_multilingual_value(abstract_map) {
            record.descriptions.push(DataCiteDescription {
                description: abstract_text,
                description_type: "Abstract".to_string(),
                lang: None,
            });
        }
    }
    if let Some(desc) = get_multilingual_value(&project.description) {
        record.descriptions.push(DataCiteDescription {
            description: desc,
            description_type: "Other".to_string(),
            lang: None,
        });
    }

    // Dates - use startDate/endDate range as dateType="Collected"
    if let Some(date_range) = format_date_range(&project.start_date, &project.end_date) {
        record.dates.push(DataCiteDate {
            date: date_range,
            date_type: "Collected".to_string(),
        });
    }

    // Dates - temporal coverage as dateType="Coverage"
    for tc in &project.temporal_coverage {
        let coverage_text = match tc {
            TemporalCoverage::Reference(ref_data) => ref_data.text.clone(),
            TemporalCoverage::Text(text_map) => get_multilingual_value(text_map),
        };
        if let Some(text) = coverage_text {
            record.dates.push(DataCiteDate {
                date: text,
                date_type: "Coverage".to_string(),
            });
        }
    }

    // Language - from data_language (use English value as-is since we don't
    // have ISO codes in the data)
    if let Some(ref languages) = project.data_language {
        if let Some(first_lang) = languages.first() {
            if let Some(lang_value) = get_multilingual_value(first_lang) {
                record.language = Some(lang_value);
            }
        }
    }

    // RelatedIdentifiers -- should contain parent Project Cluster ARK.
    // TODO: Populate once Project Cluster data is available.

    // Rights - with SPDX identifier
    for legal in &project.legal_info {
        let rights_uri = if legal.license.license_uri != "MISSING" {
            Some(legal.license.license_uri.clone())
        } else {
            None
        };
        let has_identifier = legal.license.license_identifier != "MISSING"
            && !legal.license.license_identifier.is_empty();
        record.rights_list.push(DataCiteRights {
            rights: if has_identifier {
                license_identifier_to_label(&legal.license.license_identifier)
            } else {
                access_rights_to_string(&project.access_rights.access_rights).to_string()
            },
            rights_uri,
            rights_identifier: if has_identifier {
                Some(legal.license.license_identifier.clone())
            } else {
                None
            },
            rights_identifier_scheme: if has_identifier {
                Some("SPDX".to_string())
            } else {
                None
            },
        });
    }

    // GeoLocations from spatial_coverage
    for sc in &project.spatial_coverage {
        if let Some(ref text) = sc.text {
            record.geo_locations.push(DataCiteGeoLocation {
                geo_location_place: text.clone(),
            });
        }
    }

    // FundingReferences from grants
    // Note: funder names are currently internal IDs (e.g., "0801-organization-000"),
    // not human-readable names. This is a data quality issue to resolve upstream.
    if let Funding::Grants(ref grants) = project.funding {
        for grant in grants {
            for funder in &grant.funders {
                record.funding_references.push(DataCiteFundingReference {
                    funder_name: funder.clone(),
                    award_number: grant.number.clone(),
                    award_title: grant.name.clone(),
                    award_uri: grant.url.clone(),
                });
            }
        }
    }

    record
}
