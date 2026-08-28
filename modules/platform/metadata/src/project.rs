//! The project wire contract: the shape a `projects/*.json` file deserializes
//! into, unchanged from what is on disk.
//!
//! DPE's view model (`Project`) and the conversions into and out of it stay in
//! `dpe-core` — they are lossy in exactly the places the editor must preserve
//! (`url`'s original form, `clusters`), so the editor's path is
//! `ProjectRaw` -> draft -> `ProjectRaw` and never goes through the view model.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::AuthorityFileReference;
use crate::utils::Multilingual;

/// Whether `candidate` could be a project shortcode.
///
/// Non-empty, ASCII alphanumeric, and at most [`MAX_SHORTCODE_LEN`] characters.
///
/// **Deliberately looser than the authoritative rule.** DSP-API's `Shortcode`
/// (`KnoraProject.scala`) is `^\p{XDigit}{4}$` — exactly four hex digits,
/// uppercased. Of the 85 published projects, 80 match it and five do not:
/// `0801a` through `0801e`. Those five are a temporary split of BEOL, which is
/// one VRE project but is represented as several in the metadata, and no real
/// shortcodes have been assigned to the parts yet. Encoding either the hex
/// shape or the five-character exception here would bake a transitional state
/// into a predicate that outlives it, so this checks shape only and leaves
/// existence to the lookup that follows it.
///
/// The length bound is not a metadata rule; it is input hygiene for the
/// editor, where this predicate also gates a hand-typed form field whose value
/// becomes half a primary key.
#[must_use]
pub fn is_valid_shortcode(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate.len() <= MAX_SHORTCODE_LEN
        && candidate.chars().all(|c| c.is_ascii_alphanumeric())
}

/// The longest shortcode [`is_valid_shortcode`] accepts. The published set runs
/// to five characters; this leaves room without letting a paste become a row.
pub const MAX_SHORTCODE_LEN: usize = 16;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRaw {
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
    /// Raw value — either a structured object or a legacy string array.
    #[serde(default)]
    pub url: Option<Value>,
    /// New-format secondary URL (absent in legacy files).
    pub secondary_url: Option<AuthorityFileReference>,
    pub how_to_cite: String,
    pub access_rights: AccessRights,
    pub legal_info: Vec<LegalInfo>,
    pub data_management_plan: Option<String>,
    pub data_publication_year: Option<String>,
    pub type_of_data: Option<Vec<String>>,
    pub data_language: Option<Vec<String>>,
    #[serde(default)]
    pub clusters: Option<Vec<String>>,
    #[serde(default)]
    pub collections: Option<Vec<String>>,
    #[serde(default)]
    pub records: Option<Vec<String>>,
    pub keywords: Vec<Multilingual>,
    pub disciplines: Vec<Discipline>,
    pub temporal_coverage: Vec<TemporalCoverage>,
    pub spatial_coverage: Vec<AuthorityFileReference>,
    pub attributions: Vec<Attribution>,
    #[serde(rename = "abstract", default)]
    pub abstract_text: Option<Multilingual>,
    pub contact_point: Option<Vec<String>>,
    #[serde(default)]
    pub publications: Option<Vec<Publication>>,
    pub funding: Funding,
    pub alternative_names: Option<Vec<Multilingual>>,
    pub documentation_material: Option<Vec<String>>,
    #[serde(default)]
    pub provenance: Option<String>,
    pub additional_material: Option<Vec<String>>,
    /// Optional credit line for the project's cover image (e.g. a photographer
    /// copyright). Stored verbatim; distinct from `legal_info` (dataset rights).
    pub image_credit: Option<String>,
}

pub const ACCESS_RIGHTS_VALUES: &[&str] = &[
    "Full Open Access",
    "Open Access with Restrictions",
    "Embargoed Access",
    "Metadata only Access",
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProjectStatus {
    Ongoing,
    Finished,
}

impl ProjectStatus {
    pub fn is_ongoing(&self) -> bool {
        *self == ProjectStatus::Ongoing
    }

    pub fn is_finished(&self) -> bool {
        *self == ProjectStatus::Finished
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectStatus::Ongoing => "ongoing",
            ProjectStatus::Finished => "finished",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AccessRightsType {
    #[serde(rename = "Full Open Access")]
    FullOpenAccess,
    #[serde(rename = "Open Access with Restrictions")]
    OpenAccessWithRestrictions,
    #[serde(rename = "Embargoed Access")]
    EmbargoedAccess,
    #[serde(rename = "Metadata only Access")]
    MetadataOnlyAccess,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemporalCoverage {
    Reference(AuthorityFileReference),
    Text(Multilingual),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Discipline {
    Reference(AuthorityFileReference),
    Text(Multilingual),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Funding {
    Grants(Vec<Grant>),
    Text(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessRights {
    #[serde(rename = "accessRights")]
    pub access_rights: AccessRightsType,
    #[serde(rename = "embargoDate", default)]
    pub embargo_date: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegalInfo {
    pub license: License,
    #[serde(rename = "copyrightHolder")]
    pub copyright_holder: String,
    pub authorship: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct License {
    #[serde(rename = "licenseIdentifier")]
    pub license_identifier: String,
    #[serde(rename = "licenseDate")]
    pub license_date: String,
    #[serde(rename = "licenseURI")]
    pub license_uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attribution {
    pub contributor: String,
    #[serde(rename = "contributorType")]
    pub contributor_type: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Publication {
    pub text: String,
    #[serde(default)]
    pub pid: Option<Pid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pid {
    pub url: String,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Grant {
    pub funders: Vec<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_shortcode_alphanumeric() {
        assert!(is_valid_shortcode("0803"));
        assert!(is_valid_shortcode("080C"));
        assert!(is_valid_shortcode("abc123"));
    }

    /// `080C` and `080E` are four hex digits; `0801a` through `0801e` are the
    /// BEOL sub-codes and are five characters. All are published today and must
    /// keep resolving, which is why this predicate is not DSP-API's rule.
    #[test]
    fn valid_shortcode_accepts_the_published_set_including_beol_sub_codes() {
        for shortcode in [
            "0101", "0801", "080C", "080E", "0801a", "0801b", "0801c", "0801d", "0801e",
        ] {
            assert!(is_valid_shortcode(shortcode), "{shortcode}");
        }
    }

    #[test]
    fn invalid_shortcode_empty() {
        assert!(!is_valid_shortcode(""));
    }

    /// Path traversal, separators, the percent sign and a trailing newline all
    /// matter: this predicate gates both a URL path segment and a hand-typed
    /// form field whose value becomes half a primary key.
    #[test]
    fn invalid_shortcode_special_chars() {
        for candidate in [
            "08/03", "../etc", "..", "<script>", "ab cd", "ab-cd", "ab_cd", "0801/", "0801-a", "08%30", "0801\n",
        ] {
            assert!(!is_valid_shortcode(candidate), "{candidate:?}");
        }
    }

    #[test]
    fn invalid_shortcode_xss_attempt() {
        assert!(!is_valid_shortcode("0803'OR'1'='1"));
        assert!(!is_valid_shortcode("0803&tab=overview"));
    }

    #[test]
    fn invalid_shortcode_over_length_bound() {
        assert!(!is_valid_shortcode(&"a".repeat(MAX_SHORTCODE_LEN + 1)));
        assert!(is_valid_shortcode(&"a".repeat(MAX_SHORTCODE_LEN)));
    }
}
