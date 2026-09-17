//! Transformation of Research Projects into Dublin Core metadata.

use crate::helpers::access_rights_to_string;
use crate::project_graph::ProjectGraph;
use crate::types::DublinCoreRecord;

const PUBLISHER: &str = "DaSCH";

pub fn project_to_dublin_core(graph: &ProjectGraph) -> DublinCoreRecord {
    let mut dc = DublinCoreRecord::default();

    // dc:title - prefer officialName, fallback to name
    let title = graph.official_name.as_deref().unwrap_or(&graph.raw_name);
    dc.titles.push(title.to_string());

    // dc:title - additional alternative names
    for alt_name in &graph.alternative_names {
        if !dc.titles.contains(alt_name) {
            dc.titles.push(alt_name.clone());
        }
    }

    // dc:description - prefer English from description field
    if let Some(ref desc) = graph.description {
        dc.descriptions.push(desc.clone());
    }
    // Also include abstract if available
    if let Some(ref abstract_text) = graph.abstract_text {
        if !dc.descriptions.contains(abstract_text) {
            dc.descriptions.push(abstract_text.clone());
        }
    }

    // dc:subject from keywords
    for keyword in &graph.keywords {
        dc.subjects.push(keyword.clone());
    }

    // dc:subject from disciplines
    for discipline in &graph.disciplines {
        dc.subjects.push(discipline.text.clone());
    }

    // dc:creator from attributions (principal investigators and project leaders),
    // resolved to display names
    for agent in &graph.creators {
        dc.creators.push(agent.name.clone());
    }

    // dc:contributor from other attributions, resolved to display names
    for agent in &graph.contributors {
        dc.contributors.push(agent.name.clone());
    }

    // dc:publisher
    dc.publisher = PUBLISHER.to_string();

    // dc:date from startDate
    if !shared_metadata::is_placeholder(&graph.start_date) && !graph.start_date.is_empty() {
        dc.dates.push(graph.start_date.clone());
    }

    // dc:type
    dc.resource_type = "Project".to_string();

    // dc:identifier - canonical ARK URL from pid, or derived from shortcode as fallback
    dc.identifiers.push(graph.ark.clone());

    // dc:language (BCP 47 codes)
    for lang in &graph.data_language {
        dc.languages.push(lang.clone());
    }

    // dc:relation -- should be the parent Project Cluster ARK.
    // TODO: Populate once Project Cluster data is available.

    // dc:coverage from temporal and spatial coverage
    for tc in &graph.temporal_coverage {
        if let Some(ref name) = tc.name {
            dc.coverages.push(name.clone());
        }
    }
    for sc in &graph.spatial_coverage {
        if let Some(ref text) = sc.text {
            dc.coverages.push(text.clone());
        }
    }

    // dc:rights
    dc.rights.push(access_rights_to_string(&graph.access_rights).to_string());
    for legal in &graph.legal_info {
        if !shared_metadata::is_placeholder(&legal.license_uri) && !legal.license_uri.is_empty() {
            dc.rights.push(legal.license_uri.clone());
        }
    }

    dc
}
