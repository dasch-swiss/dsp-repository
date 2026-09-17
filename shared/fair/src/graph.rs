//! The resolution context a writer reads: everything resolution needs that the
//! consuming service owns, passed in rather than reached for.
//!
//! Also the record graph: a record's facts after resolution, so that every
//! representation of a record reads one resolved object instead of re-deriving
//! from `Record`.

use std::collections::HashMap;

use shared_metadata::temporal_enrichment::EnrichedDate;
use shared_metadata::w3cdtf::W3cdtfRange;
use shared_metadata::{ContributorLookup, Multilingual, Record};

use crate::helpers::{extract_year, license_identifier_to_label};

const DASCH: &str = "DaSCH";

/// Borrowed lookup tables a writer resolves against: contributor IDs,
/// ChronOntology period timespans, and the temporal-coverage enrichment table.
pub struct ResolveContext<'a> {
    pub lookup: &'a dyn ContributorLookup,
    pub periods: &'a HashMap<String, W3cdtfRange>,
    pub enriched: &'a HashMap<String, EnrichedDate>,
}

impl<'a> ResolveContext<'a> {
    pub fn new(
        lookup: &'a dyn ContributorLookup,
        periods: &'a HashMap<String, W3cdtfRange>,
        enriched: &'a HashMap<String, EnrichedDate>,
    ) -> Self {
        Self { lookup, periods, enriched }
    }
}

/// Whether an agent is a person or an organization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentKind {
    Person,
    Organization,
}

impl AgentKind {
    /// DataCite's `nameType` spelling for this kind.
    pub fn name_type(self) -> &'static str {
        match self {
            AgentKind::Person => "Personal",
            AgentKind::Organization => "Organizational",
        }
    }

    /// The kind behind a [`crate::resolve::ResolvedAgent`]'s `name_type`, so a
    /// resolved agent and an inferred one carry the same spelling.
    pub(crate) fn from_name_type(name_type: &str) -> Self {
        if name_type == "Organizational" {
            AgentKind::Organization
        } else {
            AgentKind::Person
        }
    }
}

/// A named agent credited with the record, with its inferred kind.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordCreator {
    pub name: String,
    pub kind: AgentKind,
}

/// Infers the kind of an authorship entry.
///
/// Record authorship is a free-text name with no structured person/organization
/// flag, so the kind is inferred from the name. DaSCH itself is an organization;
/// every other authorship name is treated as a person.
fn authorship_kind(name: &str) -> AgentKind {
    if name == DASCH {
        AgentKind::Organization
    } else {
        AgentKind::Person
    }
}

/// Maps `typeOfData` to a general type: DataCite's `resourceTypeGeneral` and
/// Dublin Core's `dc:type` draw on the same vocabulary, so the mapping lives here once.
fn general_data_type(type_of_data: &str) -> String {
    match type_of_data {
        "Image" => "Image".to_string(),
        "Text" | "XML (TEI)" => "Text".to_string(),
        "Video" => "Audiovisual".to_string(),
        "Audio" => "Sound".to_string(),
        other => other.to_string(),
    }
}

/// The record's preferred title: English when present, otherwise the
/// lexicographically smallest language tag.
fn preferred_title(label: &Multilingual) -> Option<String> {
    shared_metadata::multilingual_value(label)
}

/// A record's facts after resolution, one resolved object per record.
///
/// The graph carries facts; each writer keeps its own vocabulary and its own
/// cardinality choices. Where a writer's output differs from the fact recorded
/// here, that difference is the writer's and belongs to the writer.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordGraph {
    /// The resolvable ARK URL. It is the only form any writer exposes: an ARK
    /// without the resolver in front does not dereference for a harvester
    /// (`docs/src/dpe/oai-pmh.md`, *Identifiers*).
    pub ark: String,
    pub project_ark: String,
    pub title: Option<String>,
    /// Every non-`en` label entry as `(language tag, text)`, sorted by tag. These
    /// are not deduplicated against `title`: when there is no `en` key the
    /// preferred title is one of these entries, and the DataCite writer emits it
    /// both as the title and as an `AlternativeTitle`. That duplication is in the
    /// committed output.
    pub alternative_titles: Vec<(String, String)>,
    /// Exactly what `authorship` holds, possibly empty. DataCite requires at
    /// least one creator and its writer appends an organizational `DaSCH` when
    /// this is empty; Dublin Core does not. Applying that fallback here would
    /// change `oai_dc` output for every record with empty authorship.
    pub creators: Vec<RecordCreator>,
    pub description: Option<String>,
    pub date_created: String,
    pub date_modified: String,
    pub date_published: String,
    pub publication_year: String,
    pub type_of_data: String,
    pub general_data_type: String,
    /// Verbatim, so a writer can still see "empty" and branch on it.
    pub license_identifier: String,
    pub license_uri: String,
    pub license_label: Option<String>,
    /// Free text on `Record`, not the `AccessRightsType` enum the project carries.
    /// DataCite uses it as the `rights` text when there is no license identifier.
    pub access_rights: String,
    pub keywords: Vec<String>,
    pub how_to_cite: String,
    pub size: String,
    /// `Record.publisher`. Both record writers deliberately emit the constant
    /// "DaSCH" instead of this value; do not wire it into their output.
    pub publisher: String,
    /// The file's MIME type, the only part of the file pointer carried. The
    /// download URL, checksum, file name and file size are deliberately absent —
    /// see `docs/src/dpe/oai-pmh.md`; that rule is about the URL, not the format.
    pub mime_type: Option<String>,
}

impl RecordGraph {
    pub fn build(record: &Record) -> Self {
        let license = &record.legal_info.license;
        let license_label = if license.license_identifier.is_empty() {
            None
        } else {
            Some(license_identifier_to_label(&license.license_identifier))
        };

        Self {
            ark: record.pid.as_url(),
            project_ark: record.project_ark(),
            title: preferred_title(&record.label),
            alternative_titles: record
                .label
                .iter()
                .filter(|(lang, _)| lang.as_str() != "en")
                .map(|(lang, text)| (lang.clone(), text.clone()))
                .collect(),
            creators: record
                .legal_info
                .authorship
                .iter()
                .map(|name| RecordCreator { name: name.clone(), kind: authorship_kind(name) })
                .collect(),
            description: shared_metadata::multilingual_value(&record.description),
            date_created: record.date_created.clone(),
            date_modified: record.date_modified.clone(),
            date_published: record.date_published.clone(),
            publication_year: extract_year(&record.date_published),
            type_of_data: record.type_of_data.clone(),
            general_data_type: general_data_type(&record.type_of_data),
            license_identifier: license.license_identifier.clone(),
            license_uri: license.license_uri.clone(),
            license_label,
            access_rights: record.access_rights.clone(),
            keywords: record.keywords.iter().filter_map(shared_metadata::multilingual_value).collect(),
            how_to_cite: record.how_to_cite.clone(),
            size: record.size.clone(),
            publisher: record.publisher.clone(),
            mime_type: record.file.as_ref().and_then(|f| f.mime_type.clone()).filter(|m| !m.is_empty()),
        }
    }
}

/// A reference to a record from its parent project, an ARK and a title.
///
/// A project's `hasPart` list needs nothing more, and the largest committed
/// project holds 27 026 records: two strings per part rather than a whole
/// `RecordGraph`. It derives its title through the same helper `RecordGraph`
/// uses, which is what keeps a project page and a record page from disagreeing
/// about a record's title.
#[derive(Clone, Debug, PartialEq)]
pub struct PartRef {
    pub ark: String,
    /// Empty when the record carries no label at all.
    pub title: String,
}

impl PartRef {
    pub fn from_record(record: &Record) -> Self {
        Self {
            ark: record.pid.as_url(),
            title: preferred_title(&record.label).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use shared_metadata::record::Pid;
    use shared_metadata::{RecordFile, RecordLegalInfo, RecordLicense};

    use super::*;

    fn test_record() -> Record {
        Record {
            id: "record-0001".to_string(),
            pid: Pid::new("https://ark.dasch.swiss", "0001", "record-0001"),
            label: {
                let mut m = Multilingual::new();
                m.insert("en".to_string(), "Survey Responses on Rural Land Use, 1920–1950".to_string());
                m.insert(
                    "de".to_string(),
                    "Umfrageantworten zur ländlichen Landnutzung, 1920–1950".to_string(),
                );
                m
            },
            access_rights: "Full Open Access".to_string(),
            legal_info: RecordLegalInfo {
                license: RecordLicense {
                    license_identifier: "CC-BY-4.0".to_string(),
                    license_date: "2024-01-15".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                },
                copyright_holder: "University of Basel".to_string(),
                authorship: vec!["Dr. Anna Müller".to_string(), "Prof. Hans Bauer".to_string()],
            },
            how_to_cite: String::new(),
            publisher: "DaSCH".to_string(),
            source: String::new(),
            description: {
                let mut m = Multilingual::new();
                m.insert("en".to_string(), "A collection of survey responses.".to_string());
                m
            },
            date_created: "2024-01-15".to_string(),
            date_modified: "2024-06-30".to_string(),
            date_published: "2024-02-01".to_string(),
            type_of_data: "Text".to_string(),
            size: "2.3 GB".to_string(),
            keywords: vec![],
            file: None,
        }
    }

    /// A record whose label carries no "en" key, so the preferred title comes
    /// from the fallback path.
    fn record_without_english_label() -> Record {
        Record {
            label: {
                let mut m = Multilingual::new();
                m.insert("fr".to_string(), "Réponses au sondage".to_string());
                m.insert("de".to_string(), "Umfrageantworten".to_string());
                m
            },
            ..test_record()
        }
    }

    #[test]
    fn dasch_authorship_is_an_organization() {
        let mut record = test_record();
        record.legal_info.authorship = vec!["DaSCH".to_string()];
        let graph = RecordGraph::build(&record);
        assert_eq!(
            graph.creators,
            vec![RecordCreator { name: "DaSCH".to_string(), kind: AgentKind::Organization }]
        );
    }

    #[test]
    fn other_authorship_is_a_person() {
        let graph = RecordGraph::build(&test_record());
        assert_eq!(
            graph.creators,
            vec![
                RecordCreator {
                    name: "Dr. Anna Müller".to_string(), kind: AgentKind::Person
                },
                RecordCreator {
                    name: "Prof. Hans Bauer".to_string(),
                    kind: AgentKind::Person
                },
            ]
        );
    }

    /// The DataCite writer owns the mandatory-creator fallback; adding it here
    /// would put a DaSCH creator into `oai_dc` too.
    #[test]
    fn empty_authorship_yields_no_creators() {
        let mut record = test_record();
        record.legal_info.authorship = vec![];
        let graph = RecordGraph::build(&record);
        assert!(graph.creators.is_empty());
    }

    #[test]
    fn general_data_type_mappings() {
        let mut record = test_record();
        for (type_of_data, expected) in [
            ("Image", "Image"),
            ("Text", "Text"),
            ("XML (TEI)", "Text"),
            ("Video", "Audiovisual"),
            ("Audio", "Sound"),
            ("Other", "Other"),
        ] {
            record.type_of_data = type_of_data.to_string();
            let graph = RecordGraph::build(&record);
            assert_eq!(graph.type_of_data, type_of_data);
            assert_eq!(graph.general_data_type, expected);
        }
    }

    #[test]
    fn alternative_titles_are_sorted_and_exclude_english() {
        let mut record = test_record();
        record.label.insert("fr".to_string(), "Réponses au sondage".to_string());
        let graph = RecordGraph::build(&record);
        assert_eq!(
            graph.alternative_titles,
            vec![
                (
                    "de".to_string(),
                    "Umfrageantworten zur ländlichen Landnutzung, 1920–1950".to_string()
                ),
                ("fr".to_string(), "Réponses au sondage".to_string()),
            ]
        );
    }

    #[test]
    fn alternative_titles_still_include_the_chosen_language() {
        let graph = RecordGraph::build(&record_without_english_label());
        assert_eq!(graph.title.as_deref(), Some("Umfrageantworten"));
        assert_eq!(
            graph.alternative_titles,
            vec![
                ("de".to_string(), "Umfrageantworten".to_string()),
                ("fr".to_string(), "Réponses au sondage".to_string()),
            ]
        );
    }

    #[test]
    fn part_ref_title_matches_the_graph_title() {
        for record in [test_record(), record_without_english_label()] {
            let part = PartRef::from_record(&record);
            let graph = RecordGraph::build(&record);
            assert_eq!(part.title, graph.title.clone().unwrap_or_default());
            assert_eq!(part.ark, graph.ark);
        }
    }

    #[test]
    fn part_ref_title_is_empty_without_a_label() {
        let record = Record { label: Multilingual::new(), ..test_record() };
        assert_eq!(PartRef::from_record(&record).title, "");
    }

    #[test]
    fn mime_type_comes_from_the_file() {
        let record = Record {
            file: Some(RecordFile {
                mime_type: Some("image/jp2".to_string()),
                url: "https://ingest.dasch.swiss/projects/0001/assets/abc/original".to_string(),
                ..RecordFile::default()
            }),
            ..test_record()
        };
        assert_eq!(RecordGraph::build(&record).mime_type.as_deref(), Some("image/jp2"));
    }

    #[test]
    fn mime_type_is_absent_without_a_file() {
        assert_eq!(RecordGraph::build(&test_record()).mime_type, None);
    }

    #[test]
    fn mime_type_is_absent_when_empty() {
        let record = Record {
            file: Some(RecordFile {
                mime_type: Some(String::new()),
                url: "https://ingest.dasch.swiss/projects/0001/assets/abc/original".to_string(),
                ..RecordFile::default()
            }),
            ..test_record()
        };
        assert_eq!(RecordGraph::build(&record).mime_type, None);
    }

    #[test]
    fn the_file_pointer_is_not_carried_beyond_the_mime_type() {
        let record = Record {
            file: Some(RecordFile {
                mime_type: Some("image/png".to_string()),
                url: "https://ingest.dasch.swiss/projects/0001/assets/abc/original".to_string(),
                checksum: Some("9ab438922efe".to_string()),
                file_name: Some("Screenshot.png".to_string()),
                file_size: Some(377_685),
                ..RecordFile::default()
            }),
            ..test_record()
        };
        let rendered = format!("{:?}", RecordGraph::build(&record));
        assert!(!rendered.contains("ingest."));
        assert!(!rendered.contains("9ab438922efe"));
        assert!(!rendered.contains("Screenshot.png"));
        assert!(!rendered.contains("377685"));
    }

    #[test]
    fn license_label_is_absent_without_an_identifier() {
        let mut record = test_record();
        record.legal_info.license.license_identifier = String::new();
        let graph = RecordGraph::build(&record);
        assert_eq!(graph.license_label, None);
        assert_eq!(graph.access_rights, "Full Open Access");
    }

    #[test]
    fn license_identifier_and_uri_are_verbatim() {
        let graph = RecordGraph::build(&test_record());
        assert_eq!(graph.license_identifier, "CC-BY-4.0");
        assert_eq!(graph.license_uri, "https://creativecommons.org/licenses/by/4.0/");
        assert_eq!(
            graph.license_label.as_deref(),
            Some("Creative Commons Attribution 4.0 International")
        );
    }

    #[test]
    fn identifiers_are_resolvable_ark_urls() {
        let graph = RecordGraph::build(&test_record());
        assert_eq!(graph.ark, "https://ark.dasch.swiss/ark:/72163/1/0001/record-0001");
        assert_eq!(graph.project_ark, "https://ark.dasch.swiss/ark:/72163/1/0001");
    }

    #[test]
    fn keywords_take_the_preferred_language() {
        let mut record = test_record();
        record.keywords = vec![
            Multilingual::from([("en".to_string(), "land use".to_string())]),
            Multilingual::from([("de".to_string(), "Landnutzung".to_string())]),
            Multilingual::new(),
        ];
        assert_eq!(RecordGraph::build(&record).keywords, vec!["land use", "Landnutzung"]);
    }
}
