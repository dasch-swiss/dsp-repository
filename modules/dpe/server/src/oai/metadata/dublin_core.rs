//! Transformation of Research Projects into Dublin Core metadata.

use dpe_app::domain::{Discipline, Project, TemporalCoverage};

use super::helpers::{access_rights_to_string, get_multilingual_value, is_creator};
use super::make_oai_identifier;
use super::types::DublinCoreRecord;

const PUBLISHER: &str = "DaSCH";

pub fn project_to_dublin_core(project: &Project) -> DublinCoreRecord {
    let mut record = DublinCoreRecord::default();

    // dc:title - prefer officialName, fallback to name
    let title = if project.official_name != "MISSING" && !project.official_name.is_empty() {
        &project.official_name
    } else {
        &project.name
    };
    record.titles.push(title.clone());

    // dc:title - additional alternative names
    if let Some(ref alt_names) = project.alternative_names {
        for alt_name_map in alt_names {
            if let Some(alt_name) = get_multilingual_value(alt_name_map) {
                if !record.titles.contains(&alt_name) {
                    record.titles.push(alt_name);
                }
            }
        }
    }

    // dc:description - prefer English from description field
    if let Some(desc) = get_multilingual_value(&project.description) {
        record.descriptions.push(desc);
    }
    // Also include abstract if available
    if let Some(ref abstract_map) = project.abstract_text {
        if let Some(abstract_text) = get_multilingual_value(abstract_map) {
            if !record.descriptions.contains(&abstract_text) {
                record.descriptions.push(abstract_text);
            }
        }
    }

    // dc:subject from keywords
    for kw in &project.keywords {
        if let Some(keyword) = get_multilingual_value(kw) {
            record.subjects.push(keyword);
        }
    }

    // dc:subject from disciplines
    for discipline in &project.disciplines {
        match discipline {
            Discipline::Reference(ref_data) => {
                if let Some(ref text) = ref_data.text {
                    record.subjects.push(text.clone());
                }
            }
            Discipline::Text(text_map) => {
                if let Some(text) = get_multilingual_value(text_map) {
                    record.subjects.push(text);
                }
            }
        }
    }

    // dc:creator from attributions (principal investigators and project leaders)
    for attr in &project.attributions {
        if is_creator(&attr.contributor_type) {
            record.creators.push(attr.contributor.clone());
        }
    }

    // dc:contributor from other attributions
    for attr in &project.attributions {
        if !is_creator(&attr.contributor_type) {
            record.contributors.push(attr.contributor.clone());
        }
    }

    // dc:publisher
    record.publisher = PUBLISHER.to_string();

    // dc:date from startDate
    if project.start_date != "MISSING" && !project.start_date.is_empty() {
        record.dates.push(project.start_date.clone());
    }

    // dc:type
    record.resource_type = "Project".to_string();

    // dc:identifier - use PID or shortcode
    if project.pid != "MISSING" && !project.pid.is_empty() {
        record.identifiers.push(project.pid.clone());
    }
    record.identifiers.push(make_oai_identifier(&project.shortcode));

    // dc:language
    if let Some(ref languages) = project.data_language {
        for lang_map in languages {
            if let Some(lang) = get_multilingual_value(lang_map) {
                record.languages.push(lang);
            }
        }
    }

    // dc:relation -- should be the parent Project Cluster ARK.
    // TODO: Populate once Project Cluster data is available.

    // dc:coverage from temporal and spatial coverage
    for tc in &project.temporal_coverage {
        match tc {
            TemporalCoverage::Reference(ref_data) => {
                if let Some(ref text) = ref_data.text {
                    record.coverages.push(text.clone());
                }
            }
            TemporalCoverage::Text(text_map) => {
                if let Some(text) = get_multilingual_value(text_map) {
                    record.coverages.push(text);
                }
            }
        }
    }
    for sc in &project.spatial_coverage {
        if let Some(ref text) = sc.text {
            record.coverages.push(text.clone());
        }
    }

    // dc:rights
    record
        .rights
        .push(access_rights_to_string(&project.access_rights.access_rights).to_string());
    for legal in &project.legal_info {
        if legal.license.license_uri != "MISSING" && !legal.license.license_uri.is_empty() {
            record.rights.push(legal.license.license_uri.clone());
        }
    }

    record
}
