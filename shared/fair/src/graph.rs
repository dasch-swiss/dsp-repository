//! The resolution context a writer reads: everything resolution needs that the
//! consuming service owns, passed in rather than reached for.
//!
//! Also the record graph: a record's facts after resolution, so that every
//! representation of a record reads one resolved object instead of re-deriving
//! from `Record`.

use std::borrow::Cow;
use std::collections::HashMap;

use shared_metadata::temporal_enrichment::EnrichedDate;
use shared_metadata::w3cdtf::W3cdtfRange;
use shared_metadata::{AccessRightsType, ContributorLookup, Multilingual, Record};

use crate::helpers::{access_rights_to_string, extract_year, license_identifier_to_label, real};

/// The organization's own name, used both to infer that an authorship entry is
/// DaSCH itself and as the mandatory-creator fallback of both graphs.
pub(crate) const DASCH: &str = "DaSCH";

/// The year both graphs report when the input carries no readable one.
///
/// DataCite makes `publicationYear` mandatory, so a record with no usable date
/// still has to name a year, and this is the one the committed OAI output has
/// always carried. It is a fallback, not a fact: only the representations
/// DataCite's rule governs may use it, which is why it is reached through
/// `publication_year_with_fallback` rather than resolved into either graph.
pub(crate) const FALLBACK_PUBLICATION_YEAR: &str = "2015";

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
    ///
    /// Which resolver is in front is settled before the record reaches this
    /// crate: `dpe-core` normalises the host as the corpus enters its caches.
    pub ark: String,
    pub project_ark: String,
    pub title: Option<String>,
    /// Every non-`en` label entry as `(language tag, text)`, sorted by tag. These
    /// are not deduplicated against `title`: when there is no `en` key the
    /// preferred title is one of these entries, and the DataCite writer emits it
    /// both as the title and as an `AlternativeTitle`. That duplication is in the
    /// committed output.
    pub alternative_titles: Vec<(String, String)>,
    /// Exactly what `authorship` holds, possibly empty. The mandatory-creator
    /// fallback is [`RecordGraph::creators_with_fallback`], not this field:
    /// applying it here would change `oai_dc` output for every record with
    /// empty authorship.
    pub creators: Vec<RecordCreator>,
    pub description: Option<String>,
    pub date_created: String,
    pub date_modified: String,
    pub date_published: String,
    /// The year `date_published` yields, or `None` when it yields none. The
    /// mandatory-year fallback is [`RecordGraph::publication_year_with_fallback`],
    /// not this field: resolving it here would make every writer assert a
    /// publication year for a record that records no date.
    pub publication_year: Option<String>,
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

    /// `publication_year`, or the fallback year when the record's
    /// `date_published` yields none.
    ///
    /// DataCite makes `publicationYear` mandatory, so the representations that
    /// rule governs need this and resolve it here rather than writing the
    /// fallback out again. schema.org's `datePublished` is optional and reads
    /// the raw `Option` instead: a landing page asserting a publication year a
    /// project never recorded is an invented fact (ADR-0005, *Nothing is
    /// invented for a score*), and one a FAIR assessor would read.
    pub fn publication_year_with_fallback(&self) -> &str {
        self.publication_year.as_deref().unwrap_or(FALLBACK_PUBLICATION_YEAR)
    }

    /// `creators`, or a single organizational `DaSCH` when the record has no
    /// authorship at all.
    ///
    /// DataCite makes at least one creator mandatory, so every representation
    /// that rule governs needs this fallback and resolves it here rather than
    /// writing it out again (ADR-0005). Dublin Core deliberately does not call
    /// it: `oai_dc` has no such rule, and naming DaSCH as the creator of a
    /// record nobody is credited with would invent authorship (ADR-0005,
    /// *Nothing is invented for a score*). That is also why `build` leaves
    /// `creators` exactly as `authorship` holds it.
    pub fn creators_with_fallback(&self) -> Cow<'_, [RecordCreator]> {
        if self.creators.is_empty() {
            Cow::Owned(vec![RecordCreator { name: DASCH.to_string(), kind: AgentKind::Organization }])
        } else {
            Cow::Borrowed(&self.creators)
        }
    }
}

/// A reference to a record from its parent project: an ARK, a title, and the
/// record's file when there is one that may be published.
///
/// A project's `hasPart` and `distribution` lists need nothing more, and the
/// largest committed project holds 27 026 records: two strings and a small
/// optional struct per part rather than a whole `RecordGraph`. It derives its
/// title through the same helper `RecordGraph` uses, which is what keeps a
/// project page and a record page from disagreeing about a record's title.
#[derive(Clone, Debug, PartialEq)]
pub struct PartRef {
    pub ark: String,
    /// Empty when the record carries no label at all.
    pub title: String,
    /// `None` for a record with no file, and for one the corpus does not record
    /// as fully open — see [`publishable_file`].
    pub file: Option<FileRef>,
}

impl PartRef {
    pub fn from_record(record: &Record) -> Self {
        Self {
            ark: record.pid.as_url(),
            title: preferred_title(&record.label).unwrap_or_default(),
            file: publishable_file(record),
        }
    }
}

/// A record's file, as much of it as a download pointer needs.
///
/// No checksum: schema.org has no property carrying one on a `DataDownload`,
/// and `/dpe/records/{shortcode}/{record_id}/file` already serves it beside the
/// same URL.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRef {
    /// dsp-ingest's public address for the bitstream.
    pub url: String,
    /// Absent from every file of some projects — 0803's 4 062 files carry no
    /// `mimeType` at all — so a consumer has to be able to describe a file
    /// without one.
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<u64>,
    /// The *record's* licence, which is not always the project's: 0868 licenses
    /// the project CC BY 4.0 and every one of its 7 716 file-carrying records
    /// CC0 1.0. A file described under the project's licence alone would
    /// misstate all of them, so the fact travels with the file.
    pub license_uri: Option<String>,
}

/// The record's file when a representation may advertise it, and nothing
/// otherwise.
///
/// This governs what `shared-fair` writes into a representation. It is not an
/// access control and makes nothing unreachable: dsp-ingest serves these URLs
/// to anyone who has one, and DPE's own `/dpe/records/{shortcode}/{record_id}/file`
/// returns the same URL for any record with a file. What it controls is whether
/// a landing page *publishes* one, and the rule lives here rather than in a
/// writer so that a writer cannot be added that forgets it. It fails closed: an
/// access level spelled in a way this does not recognise yields no file.
/// `Record::access_rights` is free text, unlike a project's typed
/// `AccessRightsType`, which is why this is a comparison and not a match.
fn publishable_file(record: &Record) -> Option<FileRef> {
    if record.access_rights != access_rights_to_string(&AccessRightsType::FullOpenAccess) {
        return None;
    }
    let file = record.file.as_ref()?;
    Some(FileRef {
        url: real(&file.url)?.to_string(),
        mime_type: file.mime_type.as_deref().and_then(real).map(str::to_string),
        file_name: file.file_name.as_deref().and_then(real).map(str::to_string),
        file_size: file.file_size,
        license_uri: real(&record.legal_info.license.license_uri).map(str::to_string),
    })
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

    /// `build` records authorship as it is; the mandatory-creator fallback is
    /// the accessor below, so that it cannot reach `oai_dc`.
    #[test]
    fn empty_authorship_yields_no_creators() {
        let mut record = test_record();
        record.legal_info.authorship = vec![];
        let graph = RecordGraph::build(&record);
        assert!(graph.creators.is_empty());
    }

    #[test]
    fn publication_year_comes_from_date_published() {
        let graph = RecordGraph::build(&test_record());
        assert_eq!(graph.publication_year.as_deref(), Some("2024"));
        assert_eq!(graph.publication_year_with_fallback(), "2024");
    }

    /// A record with no usable `datePublished` records no year. DataCite's
    /// mandatory field reaches the fallback through the accessor; nothing else
    /// may.
    #[test]
    fn an_unusable_date_published_yields_no_year() {
        for date_published in ["", "MISSING", "CALCULATED", "20"] {
            let record = Record { date_published: date_published.to_string(), ..test_record() };
            let graph = RecordGraph::build(&record);
            assert_eq!(graph.publication_year, None, "{date_published:?}");
            assert_eq!(graph.publication_year_with_fallback(), "2015", "{date_published:?}");
        }
    }

    #[test]
    fn empty_authorship_falls_back_to_the_dasch_organization() {
        let mut record = test_record();
        record.legal_info.authorship = vec![];
        let graph = RecordGraph::build(&record);
        assert_eq!(
            graph.creators_with_fallback().into_owned(),
            vec![RecordCreator { name: "DaSCH".to_string(), kind: AgentKind::Organization }]
        );
    }

    #[test]
    fn the_fallback_leaves_real_authorship_alone() {
        let graph = RecordGraph::build(&test_record());
        assert_eq!(graph.creators_with_fallback().as_ref(), graph.creators.as_slice());
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

    /// A record carrying every file field, as 0868's do.
    fn record_with_a_file() -> Record {
        Record {
            file: Some(RecordFile {
                mime_type: Some("image/png".to_string()),
                url: "https://ingest.dasch.swiss/projects/0001/assets/abc/original".to_string(),
                checksum: Some("9ab438922efe".to_string()),
                checksum_algorithm: Some("SHA-256".to_string()),
                file_name: Some("Screenshot.png".to_string()),
                file_size: Some(377_685),
                date_created: Some("2024-01-15".to_string()),
            }),
            ..test_record()
        }
    }

    #[test]
    fn a_part_carries_the_file_of_a_fully_open_record() {
        let file = PartRef::from_record(&record_with_a_file()).file.expect("a file");
        assert_eq!(file.url, "https://ingest.dasch.swiss/projects/0001/assets/abc/original");
        assert_eq!(file.mime_type.as_deref(), Some("image/png"));
        assert_eq!(file.file_name.as_deref(), Some("Screenshot.png"));
        assert_eq!(file.file_size, Some(377_685));
        assert_eq!(
            file.license_uri.as_deref(),
            Some("https://creativecommons.org/licenses/by/4.0/")
        );
    }

    #[test]
    fn a_part_carries_no_file_when_the_record_has_none() {
        assert_eq!(PartRef::from_record(&test_record()).file, None);
    }

    /// The rule that keeps a restricted record's ingest URL out of every
    /// representation: dsp-ingest serves it to anyone.
    #[test]
    fn a_part_carries_no_file_for_a_record_that_is_not_fully_open() {
        for access_rights in [
            "Open Access with Restrictions",
            "Embargoed Access",
            "Metadata only Access",
            "",
            "full open access",
        ] {
            let record = Record {
                access_rights: access_rights.to_string(),
                ..record_with_a_file()
            };
            assert_eq!(
                PartRef::from_record(&record).file,
                None,
                "a file was published for access rights {access_rights:?}"
            );
        }
    }

    #[test]
    fn a_part_carries_no_file_without_a_url() {
        let mut record = record_with_a_file();
        record.file.as_mut().expect("a file").url = String::new();
        assert_eq!(PartRef::from_record(&record).file, None);
    }

    /// 0803's 4,062 files carry no MIME type, and 0868's project licence is not
    /// the licence its records carry. Both are facts about the file, absent or
    /// dissenting, and neither is filled in.
    #[test]
    fn a_files_absent_and_dissenting_facts_are_reported_as_they_stand() {
        let mut record = record_with_a_file();
        record.file.as_mut().expect("a file").mime_type = None;
        record.file.as_mut().expect("a file").file_name = Some(String::new());
        record.file.as_mut().expect("a file").file_size = None;
        record.legal_info.license.license_uri = "https://creativecommons.org/publicdomain/zero/1.0/".to_string();
        let file = PartRef::from_record(&record).file.expect("a file");
        assert_eq!(file.mime_type, None);
        assert_eq!(file.file_name, None);
        assert_eq!(file.file_size, None);
        assert_eq!(
            file.license_uri.as_deref(),
            Some("https://creativecommons.org/publicdomain/zero/1.0/")
        );
    }

    #[test]
    fn a_placeholder_license_yields_no_license_for_the_file() {
        let mut record = record_with_a_file();
        record.legal_info.license.license_uri = "MISSING".to_string();
        assert_eq!(PartRef::from_record(&record).file.expect("a file").license_uri, None);
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
