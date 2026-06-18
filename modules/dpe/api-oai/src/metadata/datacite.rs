//! Transformation of Research Projects into DataCite 4.6 metadata.

use std::collections::HashMap;

use dpe_core::{ContributorLookup, Discipline, Funding, Project, TemporalCoverage};

use super::helpers::{
    access_rights_to_string, extract_year, format_date_range, get_multilingual_value, infer_subject_scheme, is_creator,
    license_identifier_to_label, map_contributor_type,
};
use super::resolve::resolve_agent;
use super::types::{
    DataCiteContributor, DataCiteCreator, DataCiteDate, DataCiteDescription, DataCiteFundingReference,
    DataCiteGeoLocation, DataCiteRecord, DataCiteRights, DataCiteSubject, DataCiteTitle,
};

const PUBLISHER: &str = "DaSCH";

pub fn project_to_datacite(project: &Project, lookup: &dyn ContributorLookup) -> DataCiteRecord {
    let mut record = DataCiteRecord::default();

    // Identifier (mandatory) - use PID or generate from shortcode
    if !dpe_core::is_placeholder(&project.pid) && !project.pid.is_empty() {
        record.identifier = project.pid.clone();
        record.identifier_type = "ARK".to_string();
    } else {
        record.identifier = format!("https://ark.dasch.swiss/ark:/72163/1/{}", project.shortcode);
        record.identifier_type = "ARK".to_string();
    }

    // Creators (mandatory) - principal investigators and project leaders
    for attr in &project.attributions {
        if is_creator(&attr.contributor_type) {
            let agent = resolve_agent(&attr.contributor, lookup);
            record.creators.push(DataCiteCreator {
                name: agent.name,
                name_type: Some(agent.name_type.to_string()),
                given_name: agent.given_name,
                family_name: agent.family_name,
                name_identifiers: agent.name_identifiers,
                affiliations: agent.affiliations,
            });
        }
    }
    // Ensure at least one creator
    if record.creators.is_empty() {
        record.creators.push(DataCiteCreator {
            name: "DaSCH".to_string(),
            name_type: Some("Organizational".to_string()),
            ..Default::default()
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
            let agent = resolve_agent(&attr.contributor, lookup);
            record.contributors.push(DataCiteContributor {
                name: agent.name,
                name_type: Some(agent.name_type.to_string()),
                contributor_type: datacite_type.to_string(),
                given_name: agent.given_name,
                family_name: agent.family_name,
                name_identifiers: agent.name_identifiers,
                affiliations: agent.affiliations,
            });
        }
    }

    // Titles (mandatory)
    // Use the longer of name/officialName as primary, shorter as AlternativeTitle
    let name_valid = !dpe_core::is_placeholder(&project.name) && !project.name.is_empty();
    let official_valid = !dpe_core::is_placeholder(&project.official_name) && !project.official_name.is_empty();

    match (name_valid, official_valid) {
        (true, true) => {
            let (primary, alternative) = if project.official_name.len() >= project.name.len() {
                (&project.official_name, &project.name)
            } else {
                (&project.name, &project.official_name)
            };
            record
                .titles
                .push(DataCiteTitle { title: primary.clone(), title_type: None, lang: None });
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
            record
                .titles
                .push(DataCiteTitle { title: project.name.clone(), title_type: None, lang: None });
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
    // (kept on format_date_range: project start/end are full ISO YYYY-MM-DD dates,
    // so the year-only w3cdtf formatter used for Coverage would lose precision here.)
    if let Some(date_range) = format_date_range(&project.start_date, &project.end_date) {
        record.dates.push(DataCiteDate {
            date: date_range,
            date_type: "Collected".to_string(),
            ..Default::default()
        });
    }

    // Dates - temporal coverage as dateType="Coverage". A project may cover
    // several distinct periods, each emitted as its own Coverage date.
    for tc in &project.temporal_coverage {
        if let Some(date) = resolve_temporal_coverage(tc) {
            record.dates.push(date);
        }
    }

    // Language - from data_language (BCP 47 codes)
    if let Some(ref languages) = project.data_language {
        if let Some(first_lang) = languages.first() {
            record.language = Some(first_lang.clone());
        }
    }

    // RelatedIdentifiers -- should contain parent Project Cluster ARK.
    // TODO: Populate once Project Cluster data is available.

    // Rights - with SPDX identifier
    for legal in &project.legal_info {
        let rights_uri = if !dpe_core::is_placeholder(&legal.license.license_uri) {
            Some(legal.license.license_uri.clone())
        } else {
            None
        };
        let has_identifier = !dpe_core::is_placeholder(&legal.license.license_identifier)
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
            rights_identifier_scheme: if has_identifier { Some("SPDX".to_string()) } else { None },
        });
    }

    // GeoLocations from spatial_coverage
    for sc in &project.spatial_coverage {
        if let Some(ref text) = sc.text {
            record
                .geo_locations
                .push(DataCiteGeoLocation { geo_location_place: text.clone() });
        }
    }

    // FundingReferences from grants; funder IDs resolved to organization names
    if let Funding::Grants(ref grants) = project.funding {
        for grant in grants {
            for funder in &grant.funders {
                record.funding_references.push(DataCiteFundingReference {
                    funder_name: resolve_agent(funder, lookup).name,
                    award_number: grant.number.clone(),
                    award_title: grant.name.clone(),
                    award_uri: grant.url.clone(),
                });
            }
        }
    }

    record
}

/// Resolve one `temporalCoverage` entry to a DataCite `Coverage` date, using the
/// process-global ChronOntology and enrichment caches.
fn resolve_temporal_coverage(tc: &TemporalCoverage) -> Option<DataCiteDate> {
    resolve_temporal_coverage_in(
        tc,
        dpe_core::chronontology_cache::all_periods(),
        dpe_core::temporal_enrichment_cache::all_enriched(),
    )
}

/// Pure resolution of a `temporalCoverage` entry over the given lookup maps, so
/// the fallback chain can be unit-tested without the process-global caches (the
/// same `*_in` shape used by the cache modules themselves).
///
/// Resolution order:
/// 1. A ChronOntology reference URL → its timespan range (`periods`).
/// 2. Otherwise (or on a cache miss) the offline enrichment table (`enrichment`), keyed by the same
///    name the mapping computes here.
/// 3. Otherwise a name-only date (`dateInformation` with an empty `date`), so the original
///    information is never silently dropped.
///
/// Returns `None` only when there is neither a resolvable range nor any name to
/// carry — a date with neither value nor information would be useless.
fn resolve_temporal_coverage_in(
    tc: &TemporalCoverage,
    periods: &HashMap<String, dpe_core::w3cdtf::W3cdtfRange>,
    enrichment: &HashMap<String, dpe_core::temporal_enrichment_cache::EnrichedDate>,
) -> Option<DataCiteDate> {
    use dpe_core::{chronontology_cache, temporal_enrichment_cache};

    // The display name, used both as the enrichment key and as dateInformation.
    let name = match tc {
        TemporalCoverage::Reference(ref_data) => ref_data.text.clone(),
        TemporalCoverage::Text(text_map) => get_multilingual_value(text_map),
    };

    // 1. ChronOntology URL → timespan.
    if let TemporalCoverage::Reference(ref_data) = tc {
        if !ref_data.url.is_empty() {
            if let Some(range) = chronontology_cache::timespan_for_in(periods, &ref_data.url) {
                return Some(DataCiteDate {
                    date: range.into(),
                    date_type: "Coverage".to_string(),
                    date_information: name,
                });
            }
        }
    }

    // 2. Enrichment table, keyed by the display name.
    if let Some(ref key) = name {
        if let Some(enriched) = temporal_enrichment_cache::enriched_for_in(enrichment, key) {
            return Some(DataCiteDate {
                date: enriched.date.unwrap_or_default(),
                date_type: "Coverage".to_string(),
                date_information: Some(enriched.original_name),
            });
        }
    }

    // 3. Name-only fallback (empty date body, dateInformation set).
    name.map(|n| DataCiteDate {
        date: String::new(),
        date_type: "Coverage".to_string(),
        date_information: Some(n),
    })
}

#[cfg(test)]
mod temporal_tests {
    use std::collections::HashMap;

    use dpe_core::temporal_enrichment_cache::EnrichedDate;
    use dpe_core::w3cdtf::{to_w3cdtf_range, W3cdtfRange};
    use dpe_core::AuthorityFileReference;

    use super::*;

    fn reference(url: &str, text: Option<&str>) -> TemporalCoverage {
        TemporalCoverage::Reference(AuthorityFileReference {
            type_: "Chronontology".to_string(),
            url: url.to_string(),
            text: text.map(str::to_string),
        })
    }

    fn text(en: &str) -> TemporalCoverage {
        let mut map = HashMap::new();
        map.insert("en".to_string(), en.to_string());
        TemporalCoverage::Text(map)
    }

    /// A period cache keyed by bare id (as the real one is), so tests exercise the
    /// real `/period/` URL-stripping in `timespan_for_in`.
    fn periods() -> HashMap<String, W3cdtfRange> {
        let mut map = HashMap::new();
        map.insert("0vGXxVln724L".to_string(), to_w3cdtf_range(Some("98"), Some("117")).unwrap());
        map
    }

    fn enrichment(entries: &[(&str, Option<&str>, &str)]) -> HashMap<String, EnrichedDate> {
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

    #[test]
    fn chronontology_url_resolves_to_range() {
        let tc = reference("https://chronontology.dainst.org/period/0vGXxVln724L", Some("Trajanic"));
        let date = resolve_temporal_coverage_in(&tc, &periods(), &HashMap::new()).unwrap();
        assert_eq!(date.date, "0098/0117");
        assert_eq!(date.date_type, "Coverage");
        assert_eq!(date.date_information.as_deref(), Some("Trajanic"));
    }

    #[test]
    fn free_text_resolves_via_enrichment() {
        let tc = text("Early Christianity");
        let enrich = enrichment(&[("Early Christianity", Some("0030/0451"), "Early Christianity")]);
        let date = resolve_temporal_coverage_in(&tc, &HashMap::new(), &enrich).unwrap();
        assert_eq!(date.date, "0030/0451");
        assert_eq!(date.date_information.as_deref(), Some("Early Christianity"));
    }

    #[test]
    fn stale_url_falls_through_to_enrichment() {
        // URL present but unknown to the period cache; enrichment by name resolves.
        let tc = reference("https://chronontology.dainst.org/period/stale", Some("Late Middle Ages"));
        let enrich = enrichment(&[("Late Middle Ages", Some("1250/1500"), "Late Middle Ages")]);
        let date = resolve_temporal_coverage_in(&tc, &periods(), &enrich).unwrap();
        assert_eq!(date.date, "1250/1500");
        assert_eq!(date.date_information.as_deref(), Some("Late Middle Ages"));
    }

    #[test]
    fn unresolved_emits_name_only_empty_date() {
        let tc = text("Mysterious Era");
        let date = resolve_temporal_coverage_in(&tc, &HashMap::new(), &HashMap::new()).unwrap();
        assert_eq!(date.date, "");
        assert_eq!(date.date_type, "Coverage");
        assert_eq!(date.date_information.as_deref(), Some("Mysterious Era"));
    }

    #[test]
    fn no_name_and_no_resolution_is_none() {
        let tc = reference("", None);
        assert!(resolve_temporal_coverage_in(&tc, &HashMap::new(), &HashMap::new()).is_none());
    }

    #[test]
    fn enrichment_without_range_emits_name_only() {
        let tc = text("Vague Period");
        let enrich = enrichment(&[("Vague Period", None, "Vague Period")]);
        let date = resolve_temporal_coverage_in(&tc, &HashMap::new(), &enrich).unwrap();
        assert_eq!(date.date, "");
        assert_eq!(date.date_information.as_deref(), Some("Vague Period"));
    }
}
