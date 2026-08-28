//! DPE's project view model and the conversions between it and the wire
//! contract.
//!
//! The contract types themselves (`ProjectRaw` and everything it is built from)
//! live in `platform-metadata`, shared with the editor. `Project` is DPE's
//! rendering shape and stays here: both conversions are lossy in ways only DPE
//! can accept — `From<&Project>` writes `url` back in the object form
//! regardless of how it was read, and drops `clusters` — so the editor works
//! `ProjectRaw` -> draft -> `ProjectRaw` and never passes through this type.

use platform_metadata::models::AuthorityFileReference;
use platform_metadata::project::{
    AccessRights, Attribution, Discipline, Funding, LegalInfo, ProjectRaw, ProjectStatus, Publication, TemporalCoverage,
};
use platform_metadata::utils::{is_placeholder, Multilingual};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cluster::ClusterRef;
use super::collection::CollectionRef;

/// Valid tab names for project detail pages.
pub const VALID_TABS: &[&str] = &["overview", "publications", "contributors"];

fn make_ref(url: String) -> AuthorityFileReference {
    AuthorityFileReference { type_: "URL".to_string(), url, text: None }
}

/// Parses the `"url"` JSON value — either a structured object (new format)
/// or a legacy string array — into primary and secondary references.
fn parse_url_value(value: Option<Value>) -> (Option<AuthorityFileReference>, Option<AuthorityFileReference>) {
    match value {
        Some(Value::Object(_)) => {
            let reference = serde_json::from_value::<AuthorityFileReference>(value.unwrap())
                .ok()
                .filter(|r| !is_placeholder(&r.url));
            (reference, None)
        }
        Some(Value::Array(arr)) => {
            // Placeholders ("MISSING"/"CALCULATED") signal "no URL yet" and must not
            // become live links — e.g. a "Discover Project Data" button to nowhere.
            let mut strings = arr
                .into_iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .filter(|s| !is_placeholder(s));
            (strings.next().map(make_ref), strings.next().map(make_ref))
        }
        _ => (None, None),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub pid: String,
    pub name: String,
    pub shortcode: String,
    pub official_name: String,
    pub status: ProjectStatus,
    pub short_description: String,
    pub description: Multilingual,
    pub start_date: String,
    pub end_date: String,
    pub url: Option<AuthorityFileReference>,
    pub secondary_url: Option<AuthorityFileReference>,
    pub how_to_cite: String,
    pub access_rights: AccessRights,
    pub legal_info: Vec<LegalInfo>,
    pub data_management_plan: Option<String>,
    pub data_publication_year: Option<String>,
    pub type_of_data: Option<Vec<String>>,
    pub data_language: Option<Vec<String>>,
    pub clusters: Vec<ClusterRef>,
    pub collections: Vec<CollectionRef>,
    /// Raw collection IDs from JSON, used to resolve `collections` on demand.
    pub collection_ids: Vec<String>,
    pub records: Option<Vec<String>>,
    pub keywords: Vec<Multilingual>,
    pub disciplines: Vec<Discipline>,
    pub temporal_coverage: Vec<TemporalCoverage>,
    pub spatial_coverage: Vec<AuthorityFileReference>,
    pub attributions: Vec<Attribution>,
    pub abstract_text: Option<Multilingual>,
    pub contact_point: Option<Vec<String>>,
    pub publications: Option<Vec<Publication>>,
    pub funding: Funding,
    pub alternative_names: Option<Vec<Multilingual>>,
    pub documentation_material: Option<Vec<String>>,
    pub provenance: Option<String>,
    pub additional_material: Option<Vec<String>>,
    /// Optional credit line for the project's cover image (e.g. a photographer
    /// copyright). Stored verbatim; distinct from `legal_info` (dataset rights).
    pub image_credit: Option<String>,
}

impl From<ProjectRaw> for Project {
    fn from(raw: ProjectRaw) -> Self {
        let (url, secondary_url_from_array) = parse_url_value(raw.url);
        // New-format files have `secondaryURL` as a separate key; legacy files encode
        // it as the second element of the `url` array.
        let secondary_url = raw.secondary_url.or(secondary_url_from_array);
        Project {
            id: raw.id,
            pid: raw.pid,
            name: raw.name,
            shortcode: raw.shortcode,
            official_name: raw.official_name,
            status: raw.status,
            short_description: raw.short_description,
            description: raw.description,
            start_date: raw.start_date,
            end_date: raw.end_date,
            url,
            secondary_url,
            how_to_cite: raw.how_to_cite,
            access_rights: raw.access_rights,
            legal_info: raw.legal_info,
            data_management_plan: raw.data_management_plan,
            data_publication_year: raw.data_publication_year,
            type_of_data: raw.type_of_data,
            data_language: raw.data_language,
            clusters: Vec::new(),
            collections: Vec::new(),
            collection_ids: raw.collections.unwrap_or_default(),
            records: raw.records,
            keywords: raw.keywords,
            disciplines: raw.disciplines,
            temporal_coverage: raw.temporal_coverage,
            spatial_coverage: raw.spatial_coverage,
            attributions: raw.attributions,
            abstract_text: raw.abstract_text,
            contact_point: raw.contact_point,
            publications: raw.publications,
            funding: raw.funding,
            alternative_names: raw.alternative_names,
            documentation_material: raw.documentation_material,
            provenance: raw.provenance,
            additional_material: raw.additional_material,
            image_credit: raw.image_credit,
        }
    }
}

impl From<&Project> for ProjectRaw {
    fn from(p: &Project) -> Self {
        ProjectRaw {
            id: p.id.clone(),
            pid: p.pid.clone(),
            name: p.name.clone(),
            shortcode: p.shortcode.clone(),
            official_name: p.official_name.clone(),
            status: p.status.clone(),
            short_description: p.short_description.clone(),
            description: p.description.clone(),
            start_date: p.start_date.clone(),
            end_date: p.end_date.clone(),
            url: p.url.as_ref().and_then(|u| serde_json::to_value(u).ok()),
            secondary_url: p.secondary_url.clone(),
            how_to_cite: p.how_to_cite.clone(),
            access_rights: p.access_rights.clone(),
            legal_info: p.legal_info.clone(),
            data_management_plan: p.data_management_plan.clone(),
            data_publication_year: p.data_publication_year.clone(),
            type_of_data: p.type_of_data.clone(),
            data_language: p.data_language.clone(),
            clusters: None,
            collections: Some(p.collection_ids.clone()),
            records: p.records.clone(),
            keywords: p.keywords.clone(),
            disciplines: p.disciplines.clone(),
            temporal_coverage: p.temporal_coverage.clone(),
            spatial_coverage: p.spatial_coverage.clone(),
            attributions: p.attributions.clone(),
            abstract_text: p.abstract_text.clone(),
            contact_point: p.contact_point.clone(),
            publications: p.publications.clone(),
            funding: p.funding.clone(),
            alternative_names: p.alternative_names.clone(),
            documentation_material: p.documentation_material.clone(),
            provenance: p.provenance.clone(),
            additional_material: p.additional_material.clone(),
            image_credit: p.image_credit.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn array_placeholder_url_yields_no_reference() {
        let (primary, secondary) = parse_url_value(Some(json!(["MISSING"])));
        assert!(primary.is_none());
        assert!(secondary.is_none());
    }

    #[test]
    fn array_real_url_yields_primary_reference() {
        let (primary, secondary) = parse_url_value(Some(json!(["https://example.org/data"])));
        assert_eq!(primary.unwrap().url, "https://example.org/data");
        assert!(secondary.is_none());
    }

    #[test]
    fn array_filters_placeholders_keeps_real_urls() {
        // A placeholder primary must not shift a real URL into the primary slot
        // incorrectly, nor become a link itself.
        let (primary, secondary) = parse_url_value(Some(json!(["MISSING", "https://example.org/site"])));
        assert_eq!(primary.unwrap().url, "https://example.org/site");
        assert!(secondary.is_none());
    }

    #[test]
    fn array_calculated_placeholder_is_filtered() {
        let (primary, _) = parse_url_value(Some(json!(["CALCULATED"])));
        assert!(primary.is_none());
    }

    #[test]
    fn object_placeholder_url_yields_no_reference() {
        let (primary, secondary) = parse_url_value(Some(json!({"type": "URL", "url": "MISSING"})));
        assert!(primary.is_none());
        assert!(secondary.is_none());
    }

    #[test]
    fn object_real_url_yields_primary_reference() {
        let (primary, _) = parse_url_value(Some(json!({"type": "URL", "url": "https://example.org/data"})));
        assert_eq!(primary.unwrap().url, "https://example.org/data");
    }

    #[test]
    fn missing_url_yields_no_reference() {
        let (primary, secondary) = parse_url_value(None);
        assert!(primary.is_none());
        assert!(secondary.is_none());
    }

    /// A file without `imageCredit` (all committed files predate the field) must
    /// deserialize to `None` — the field is purely additive.
    #[test]
    fn image_credit_defaults_to_none_for_existing_file() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects/0102_tanner.json");
        let json = std::fs::read_to_string(path).expect("sample project file should be readable");
        let project = serde_json::from_str::<ProjectRaw>(&json).map(Project::from).expect("parses");
        assert!(project.image_credit.is_none());
    }

    /// When present, `imageCredit` round-trips verbatim through `ProjectRaw` → `Project`.
    #[test]
    fn image_credit_is_parsed_when_present() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects/0102_tanner.json");
        let json = std::fs::read_to_string(path).expect("sample project file should be readable");
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("valid json");
        value["imageCredit"] = json!("© Someone, Somewhere");
        let project = Project::from(serde_json::from_value::<ProjectRaw>(value).expect("parses"));
        assert_eq!(project.image_credit.as_deref(), Some("© Someone, Somewhere"));
    }

    /// Classify an authority-file URL as a place gazetteer, a period gazetteer,
    /// or neither, based on its host. Used to guard against temporal/spatial
    /// coverage being swapped (a known data-migration defect, see DEV history).
    fn url_kind(url: &str) -> &'static str {
        let host = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("");
        // Period (temporal) gazetteers.
        if host.contains("chronontology.dainst.org") || host.contains("perio.do") || host.contains("n2t.net") {
            "period"
        }
        // Place (spatial) gazetteers.
        else if host.contains("pleiades.stoa.org")
            || host.contains("geonames.org")
            || host.contains("gazetteer.dainst.org")
            || host.contains("vocab.getty.edu")
        {
            "place"
        } else {
            "unknown"
        }
    }

    /// Guard against the temporal/spatial coverage inversion introduced during
    /// the old-model migration: a place reference must never sit in
    /// `temporalCoverage`, and a period reference must never sit in
    /// `spatialCoverage`. Ambiguous URLs (free-form `URL` type) are ignored.
    #[test]
    fn coverage_fields_are_not_inverted() {
        // Resolve the data dir relative to this crate, independent of the test
        // process working directory.
        let projects_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects");

        let mut violations = Vec::new();
        let entries = std::fs::read_dir(projects_dir).expect("projects data directory should be readable");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            let json = std::fs::read_to_string(&path).expect("project file should be readable");
            let project = serde_json::from_str::<ProjectRaw>(&json)
                .map(Project::from)
                .unwrap_or_else(|e| panic!("failed to parse {filename}: {e}"));

            for coverage in &project.temporal_coverage {
                if let TemporalCoverage::Reference(reference) = coverage {
                    if url_kind(&reference.url) == "place" {
                        violations.push(format!(
                            "{filename}: temporalCoverage holds a place reference (type={}, url={})",
                            reference.type_, reference.url
                        ));
                    }
                }
            }
            for reference in &project.spatial_coverage {
                if url_kind(&reference.url) == "period" {
                    violations.push(format!(
                        "{filename}: spatialCoverage holds a period reference (type={}, url={})",
                        reference.type_, reference.url
                    ));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "temporal/spatial coverage inversions found:\n{}",
            violations.join("\n")
        );
    }

    /// Every published shortcode must pass the shared shape predicate — the
    /// five BEOL sub-codes (`0801a`–`0801e`) included, since they are the reason
    /// it is looser than DSP-API's four-hex-digit rule.
    #[test]
    fn every_committed_shortcode_passes_the_shape_check() {
        let projects_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../server/data/projects");
        let entries = std::fs::read_dir(projects_dir).expect("projects data directory should be readable");
        let mut checked = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let json = std::fs::read_to_string(&path).expect("project file should be readable");
            let raw: ProjectRaw = serde_json::from_str(&json).expect("parses");
            assert!(
                platform_metadata::is_valid_shortcode(&raw.shortcode),
                "{:?} has a shortcode the shape check rejects: {:?}",
                path.file_name(),
                raw.shortcode
            );
            checked += 1;
        }
        assert!(checked > 0, "no project files were checked");
    }
}
