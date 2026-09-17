//! A project's facts after resolution, one resolved object per project.
//!
//! The graph carries facts; each writer keeps its own vocabulary and its own
//! cardinality choices. Where two writers read the same field but disagree
//! about it, the graph carries the *underlying* value — placeholders and
//! emptiness included — so each writer can keep branching on it.

use shared_metadata::temporal_coverage::Resolution;
use shared_metadata::{
    AccessRightsType, AuthorityFileReference, Discipline, Funding, LegalInfo, ProjectRaw, ProjectStatus, Record,
};

use crate::graph::{AgentKind, PartRef, ResolveContext};
use crate::helpers::{extract_year, is_creator, license_identifier_to_label};
use crate::resolve::resolve_agent;
use crate::types::DataCiteNameIdentifier;

/// A resolved agent credited with the project.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectAgent {
    pub name: String,
    pub kind: AgentKind,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub name_identifiers: Vec<DataCiteNameIdentifier>,
    pub affiliations: Vec<String>,
    /// Verbatim `contributorType`, possibly empty. DataCite maps the first entry
    /// through `map_contributor_type` and falls back to `Other`; Dublin Core
    /// emits only the name, so the raw strings cannot be resolved away here.
    pub contributor_type: Vec<String>,
}

/// One `legalInfo` element, in file order.
///
/// Identifier and URI stay verbatim because the two writers test them
/// differently: DataCite emits one entry per element and only checks the URI
/// for a placeholder, while Dublin Core skips a URI that is a placeholder *or*
/// empty. A deduplicated list of non-placeholder URIs would silently drop
/// DataCite entries.
#[derive(Clone, Debug, PartialEq)]
pub struct LicenseRef {
    pub license_identifier: String,
    pub license_uri: String,
    /// `None` when the identifier is a placeholder or empty — the same test
    /// DataCite uses to decide whether the entry gets a `rightsIdentifier` at
    /// all, and whether its `rights` text falls back to the access rights.
    pub license_label: Option<String>,
}

/// One `temporalCoverage` entry, carrying both readings the writers need.
#[derive(Clone, Debug, PartialEq)]
pub struct TemporalRef {
    /// The entry's display name, which Dublin Core emits as `dc:coverage`.
    pub name: Option<String>,
    /// `None` only for an entry with neither a resolvable range nor a name —
    /// the only case DataCite drops. An entry that resolves to no date still
    /// yields a `Resolution` with an empty `date` and is emitted.
    pub resolution: Option<Resolution>,
}

/// A discipline subject.
#[derive(Clone, Debug, PartialEq)]
pub struct DisciplineRef {
    pub text: String,
    /// `Some` only for the authority-reference variant, which is the only one
    /// DataCite infers a subject scheme from.
    pub authority_url: Option<String>,
}

/// A place the project covers. Both writers read only `text` today; the URL is
/// what a schema.org `Place.sameAs` needs.
#[derive(Clone, Debug, PartialEq)]
pub struct SpatialRef {
    pub text: Option<String>,
    pub url: String,
}

/// One grant, with its funder IDs resolved to names in grant order.
#[derive(Clone, Debug, PartialEq)]
pub struct FundingRef {
    pub funder_names: Vec<String>,
    pub number: Option<String>,
    pub name: Option<String>,
    pub url: Option<String>,
}

/// A publication about the project.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicationRef {
    pub text: String,
    pub pid: Option<String>,
}

/// A project's facts after resolution.
#[derive(Clone, Debug)]
pub struct ProjectGraph {
    /// The canonical ARK: `pid` when it is neither a placeholder nor empty,
    /// otherwise built from the shortcode. Both writers perform that fallback
    /// today; `pid` stays beside it because the OAI identifier is derived from
    /// the raw value and a `sameAs` needs to see the two differ.
    pub ark: String,
    pub shortcode: String,
    pub pid: String,
    /// `None` for a placeholder or empty raw value, which is exactly DataCite's
    /// `name_valid` / `official_valid` test. No preferred title is computed:
    /// DataCite takes the longer of the two, Dublin Core prefers
    /// `official_name`, and the graph must not pick for them.
    pub name: Option<String>,
    pub official_name: Option<String>,
    /// The raw `name`, placeholder and all. Both writers fall back to it
    /// verbatim when neither title is real, so the placeholder string itself
    /// reaches the output and cannot be resolved away here.
    pub raw_name: String,
    /// Not deduplicated: each writer deduplicates against its own title list.
    pub alternative_names: Vec<String>,
    /// Kept apart and unordered relative to each other. DataCite emits the
    /// abstract first as `Abstract` then the description as `Other`; Dublin
    /// Core emits the description first and drops an abstract equal to it.
    pub description: Option<String>,
    pub abstract_text: Option<String>,
    /// Possibly empty. DataCite requires one creator and its writer appends an
    /// organizational `DaSCH` when there is none; Dublin Core emits no creator
    /// at all, so applying that fallback here would change `oai_dc` output.
    pub creators: Vec<ProjectAgent>,
    pub contributors: Vec<ProjectAgent>,
    pub keywords: Vec<String>,
    pub disciplines: Vec<DisciplineRef>,
    /// Verbatim: DataCite formats the two into one `Collected` range, while
    /// Dublin Core emits `start_date` alone and only when it is real.
    pub start_date: String,
    pub end_date: String,
    pub publication_year: String,
    pub temporal_coverage: Vec<TemporalRef>,
    pub spatial_coverage: Vec<SpatialRef>,
    /// Every entry: DataCite takes the first as its single `language`, Dublin
    /// Core emits all of them.
    pub data_language: Vec<String>,
    pub legal_info: Vec<LicenseRef>,
    pub access_rights: AccessRightsType,
    pub embargo_date: Option<String>,
    pub funding: Vec<FundingRef>,
    /// The `url` reading rule applied once, with `secondaryURL` taking
    /// precedence over the second element of a legacy `url` array.
    pub website: Option<AuthorityFileReference>,
    pub secondary_website: Option<AuthorityFileReference>,
    pub how_to_cite: String,
    pub publications: Vec<PublicationRef>,
    pub status: ProjectStatus,
    /// One entry per record handed in, through `PartRef::from_record`, which is
    /// what keeps a project page and a record page agreeing about a title.
    pub parts: Vec<PartRef>,
}

impl ProjectGraph {
    pub fn build(raw: &ProjectRaw, ctx: &ResolveContext<'_>, records: &[Record]) -> Self {
        let (website, secondary_from_array) = shared_metadata::utils::parse_url_value(raw.url.clone());

        Self {
            ark: canonical_ark(raw),
            shortcode: raw.shortcode.clone(),
            pid: raw.pid.clone(),
            name: real_value(&raw.name),
            official_name: real_value(&raw.official_name),
            raw_name: raw.name.clone(),
            alternative_names: raw
                .alternative_names
                .iter()
                .flatten()
                .filter_map(shared_metadata::multilingual_value)
                .collect(),
            description: shared_metadata::multilingual_value(&raw.description),
            abstract_text: raw.abstract_text.as_ref().and_then(shared_metadata::multilingual_value),
            creators: attributed_agents(raw, ctx, true),
            contributors: attributed_agents(raw, ctx, false),
            keywords: raw.keywords.iter().filter_map(shared_metadata::multilingual_value).collect(),
            disciplines: raw.disciplines.iter().filter_map(discipline_ref).collect(),
            start_date: raw.start_date.clone(),
            end_date: raw.end_date.clone(),
            publication_year: publication_year(raw),
            temporal_coverage: raw
                .temporal_coverage
                .iter()
                .map(|tc| TemporalRef {
                    name: shared_metadata::temporal_coverage::coverage_name(tc),
                    resolution: shared_metadata::temporal_coverage::resolve_in(tc, ctx.periods, ctx.enriched),
                })
                .collect(),
            spatial_coverage: raw
                .spatial_coverage
                .iter()
                .map(|sc| SpatialRef { text: sc.text.clone(), url: sc.url.clone() })
                .collect(),
            data_language: raw.data_language.clone().unwrap_or_default(),
            legal_info: raw.legal_info.iter().map(license_ref).collect(),
            access_rights: raw.access_rights.access_rights.clone(),
            embargo_date: raw.access_rights.embargo_date.clone(),
            funding: funding_refs(raw, ctx),
            website,
            secondary_website: raw.secondary_url.clone().or(secondary_from_array),
            how_to_cite: raw.how_to_cite.clone(),
            publications: raw
                .publications
                .iter()
                .flatten()
                .map(|publication| PublicationRef {
                    text: publication.text.clone(),
                    pid: publication.pid.as_ref().map(|pid| pid.url.clone()),
                })
                .collect(),
            status: raw.status.clone(),
            parts: records.iter().map(PartRef::from_record).collect(),
        }
    }
}

fn canonical_ark(raw: &ProjectRaw) -> String {
    match real_value(&raw.pid) {
        Some(pid) => pid,
        None => format!("https://ark.dasch.swiss/ark:/72163/1/{}", raw.shortcode),
    }
}

/// The value unless it is a placeholder or empty, in which case there is none.
fn real_value(value: &str) -> Option<String> {
    if shared_metadata::is_placeholder(value) || value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// `data_publication_year` when the key is present at all, otherwise
/// `start_date`. Presence rather than validity: `extract_year` already turns an
/// unusable value into its own fallback year.
fn publication_year(raw: &ProjectRaw) -> String {
    match raw.data_publication_year {
        Some(ref year) => extract_year(year),
        None => extract_year(&raw.start_date),
    }
}

/// The attributions whose creator status matches `creators`, in attribution order.
fn attributed_agents(raw: &ProjectRaw, ctx: &ResolveContext<'_>, creators: bool) -> Vec<ProjectAgent> {
    raw.attributions
        .iter()
        .filter(|attr| is_creator(&attr.contributor_type) == creators)
        .map(|attr| {
            let agent = resolve_agent(&attr.contributor, ctx.lookup);
            ProjectAgent {
                name: agent.name,
                kind: AgentKind::from_name_type(agent.name_type),
                given_name: agent.given_name,
                family_name: agent.family_name,
                name_identifiers: agent.name_identifiers,
                affiliations: agent.affiliations,
                contributor_type: attr.contributor_type.clone(),
            }
        })
        .collect()
}

/// `None` for a reference without a label or a text entry with no readable
/// language, both of which every writer skips.
fn discipline_ref(discipline: &Discipline) -> Option<DisciplineRef> {
    match discipline {
        Discipline::Reference(reference) => reference
            .text
            .clone()
            .map(|text| DisciplineRef { text, authority_url: Some(reference.url.clone()) }),
        Discipline::Text(map) => {
            shared_metadata::multilingual_value(map).map(|text| DisciplineRef { text, authority_url: None })
        }
    }
}

fn license_ref(legal: &LegalInfo) -> LicenseRef {
    let identifier = &legal.license.license_identifier;
    LicenseRef {
        license_identifier: identifier.clone(),
        license_uri: legal.license.license_uri.clone(),
        license_label: real_value(identifier).map(|id| license_identifier_to_label(&id)),
    }
}

/// One entry per grant. `Funding::Text` is free prose rather than grants and no
/// writer emits it today, so it yields nothing.
fn funding_refs(raw: &ProjectRaw, ctx: &ResolveContext<'_>) -> Vec<FundingRef> {
    match raw.funding {
        Funding::Grants(ref grants) => grants
            .iter()
            .map(|grant| FundingRef {
                funder_names: grant
                    .funders
                    .iter()
                    .map(|funder| resolve_agent(funder, ctx.lookup).name)
                    .collect(),
                number: grant.number.clone(),
                name: grant.name.clone(),
                url: grant.url.clone(),
            })
            .collect(),
        Funding::Text(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use shared_metadata::temporal_enrichment::EnrichedDate;
    use shared_metadata::w3cdtf::{to_w3cdtf_range, W3cdtfRange};
    use shared_metadata::{
        AccessRights, Attribution, ContributorLookup, Grant, License, Multilingual, Organization, Person, Record,
        RecordLegalInfo, RecordLicense, RecordPid,
    };

    use super::*;

    /// A second small copy of `resolve.rs`'s test lookup: that one is private to
    /// its own test module, and neither is worth a public helper.
    #[derive(Default)]
    struct InMemoryContributorLookup {
        persons: HashMap<String, Person>,
        organizations: HashMap<String, Organization>,
    }

    impl ContributorLookup for InMemoryContributorLookup {
        fn person(&self, id: &str) -> Option<Person> {
            self.persons.get(id).cloned()
        }

        fn organization(&self, id: &str) -> Option<Organization> {
            self.organizations.get(id).cloned()
        }
    }

    fn lookup() -> InMemoryContributorLookup {
        let mut lookup = InMemoryContributorLookup::default();
        lookup.persons.insert(
            "person-001".to_string(),
            Person {
                id: "person-001".to_string(),
                given_names: vec!["Anna".to_string()],
                family_names: vec!["Müller".to_string()],
                job_titles: vec![],
                affiliations: vec![],
                same_as: vec![],
                email: None,
            },
        );
        lookup.organizations.insert(
            "organization-001".to_string(),
            Organization {
                id: "organization-001".to_string(),
                name: "Schweizerischer Nationalfonds".to_string(),
                same_as: vec![],
                url: String::new(),
                address: None,
                email: None,
                alternative_name: None,
            },
        );
        lookup
    }

    /// A period cache keyed by bare id (as the real one is), so tests exercise
    /// the real `/period/` URL-stripping in `timespan_for_in`.
    fn periods() -> HashMap<String, W3cdtfRange> {
        HashMap::from([(
            "0vGXxVln724L".to_string(),
            to_w3cdtf_range(Some("98"), Some("117")).expect("a valid range"),
        )])
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

    /// The default enrichment fixture for tests that are not specifically about
    /// an empty or missing table. It does carry a `"Trajanic"` row with a
    /// *different* range than the period cache, so a URL-tier test can prove the
    /// URL wins over a same-named enrichment row.
    fn default_enrichment() -> HashMap<String, EnrichedDate> {
        enrichment(&[
            ("Trajanic", Some("1111/2222"), "Trajanic"),
            ("Bronze Age", Some("-3300/-1200"), "Bronze Age"),
        ])
    }

    fn english(text: &str) -> Multilingual {
        Multilingual::from([("en".to_string(), text.to_string())])
    }

    fn temporal_reference(url: &str, text: Option<&str>) -> shared_metadata::TemporalCoverage {
        shared_metadata::TemporalCoverage::Reference(AuthorityFileReference {
            type_: "Chronontology".to_string(),
            url: url.to_string(),
            text: text.map(str::to_string),
        })
    }

    fn temporal_text(en: &str) -> shared_metadata::TemporalCoverage {
        shared_metadata::TemporalCoverage::Text(english(en))
    }

    fn legal(identifier: &str, uri: &str) -> LegalInfo {
        LegalInfo {
            license: License {
                license_identifier: identifier.to_string(),
                license_date: "2012-08-31".to_string(),
                license_uri: uri.to_string(),
            },
            copyright_holder: "Universität Basel".to_string(),
            authorship: vec!["person-001".to_string()],
        }
    }

    fn project() -> ProjectRaw {
        ProjectRaw {
            id: "0001".to_string(),
            pid: "https://ark.dasch.swiss/ark:/72163/1/0001".to_string(),
            name: "Rural Land Use".to_string(),
            shortcode: "0001".to_string(),
            official_name: "Rural Land Use in the Swiss Midlands, 1920-1950".to_string(),
            status: ProjectStatus::Finished,
            short_description: "Land use in the Swiss Midlands.".to_string(),
            description: english("A study of rural land use."),
            start_date: "2008-06-01".to_string(),
            end_date: "2012-08-31".to_string(),
            url: None,
            secondary_url: None,
            how_to_cite: "Rural Land Use (2012) DaSCH.".to_string(),
            access_rights: AccessRights {
                access_rights: AccessRightsType::FullOpenAccess,
                embargo_date: None,
            },
            legal_info: vec![legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/")],
            data_management_plan: None,
            data_publication_year: None,
            type_of_data: Some(vec!["Image".to_string()]),
            data_language: Some(vec!["de".to_string(), "fr".to_string()]),
            clusters: None,
            collections: None,
            records: None,
            keywords: vec![english("land use")],
            disciplines: vec![],
            temporal_coverage: vec![],
            spatial_coverage: vec![],
            attributions: vec![],
            abstract_text: None,
            contact_point: None,
            publications: None,
            funding: Funding::Grants(vec![]),
            alternative_names: None,
            documentation_material: None,
            provenance: None,
            additional_material: None,
            image_credit: None,
        }
    }

    fn record(id: &str, label: Multilingual) -> Record {
        Record {
            id: id.to_string(),
            pid: RecordPid::new("https://ark.dasch.swiss", "0001", id),
            label,
            access_rights: "Full Open Access".to_string(),
            legal_info: RecordLegalInfo {
                license: RecordLicense {
                    license_identifier: "CC-BY-4.0".to_string(),
                    license_date: "2024-01-15".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                },
                copyright_holder: "Universität Basel".to_string(),
                authorship: vec!["Müller, Anna".to_string()],
            },
            how_to_cite: String::new(),
            publisher: "DaSCH".to_string(),
            source: String::new(),
            description: Multilingual::new(),
            date_created: "2024-01-15".to_string(),
            date_modified: "2024-06-30".to_string(),
            date_published: "2024-02-01".to_string(),
            type_of_data: "Text".to_string(),
            size: "2.3 GB".to_string(),
            keywords: vec![],
            file: None,
        }
    }

    /// Builds a graph over ad-hoc tables, proving the builder needs no `'static`
    /// data and no process-global caches.
    fn build(raw: &ProjectRaw, records: &[Record]) -> ProjectGraph {
        let lookup = lookup();
        let periods = periods();
        let enriched = default_enrichment();
        let ctx = ResolveContext::new(&lookup, &periods, &enriched);
        ProjectGraph::build(raw, &ctx, records)
    }

    fn build_with_enrichment(raw: &ProjectRaw, enriched: HashMap<String, EnrichedDate>) -> ProjectGraph {
        let lookup = lookup();
        let periods = periods();
        let ctx = ResolveContext::new(&lookup, &periods, &enriched);
        ProjectGraph::build(raw, &ctx, &[])
    }

    #[test]
    fn ark_uses_the_pid_when_it_is_real() {
        let graph = build(&project(), &[]);
        assert_eq!(graph.ark, "https://ark.dasch.swiss/ark:/72163/1/0001");
        assert_eq!(graph.pid, "https://ark.dasch.swiss/ark:/72163/1/0001");
    }

    #[test]
    fn ark_falls_back_to_the_shortcode_when_the_pid_is_a_placeholder() {
        for pid in ["MISSING", "CALCULATED", ""] {
            let raw = ProjectRaw { pid: pid.to_string(), ..project() };
            let graph = build(&raw, &[]);
            assert_eq!(graph.ark, "https://ark.dasch.swiss/ark:/72163/1/0001", "{pid:?}");
            // The raw value survives beside the resolved one.
            assert_eq!(graph.pid, pid);
        }
    }

    /// The DataCite writer owns the mandatory-creator fallback; adding it here
    /// would put a DaSCH creator into `oai_dc` too.
    #[test]
    fn an_empty_creator_set_stays_empty() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-001".to_string(),
                contributor_type: vec!["Researcher".to_string()],
            }],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert!(graph.creators.is_empty());
        assert_eq!(graph.contributors.len(), 1);
    }

    #[test]
    fn creators_and_contributors_keep_their_raw_contributor_types() {
        let raw = ProjectRaw {
            attributions: vec![
                Attribution {
                    contributor: "person-001".to_string(),
                    contributor_type: vec!["Project Leader".to_string()],
                },
                Attribution {
                    contributor: "organization-001".to_string(),
                    contributor_type: vec!["Data Collector".to_string(), "Editor".to_string()],
                },
                Attribution {
                    contributor: "person-002".to_string(),
                    contributor_type: vec![],
                },
            ],
            ..project()
        };
        let graph = build(&raw, &[]);

        assert_eq!(graph.creators.len(), 1);
        assert_eq!(graph.creators[0].name, "Müller, Anna");
        assert_eq!(graph.creators[0].kind, AgentKind::Person);
        assert_eq!(graph.creators[0].contributor_type, vec!["Project Leader".to_string()]);

        assert_eq!(graph.contributors.len(), 2);
        assert_eq!(graph.contributors[0].name, "Schweizerischer Nationalfonds");
        assert_eq!(graph.contributors[0].kind, AgentKind::Organization);
        assert_eq!(
            graph.contributors[0].contributor_type,
            vec!["Data Collector".to_string(), "Editor".to_string()]
        );
        // An empty type list is carried as such: DataCite's "Other" fallback is
        // the writer's, not the graph's.
        assert!(graph.contributors[1].contributor_type.is_empty());
    }

    #[test]
    fn both_titles_are_carried_when_both_are_real() {
        let graph = build(&project(), &[]);
        assert_eq!(graph.name.as_deref(), Some("Rural Land Use"));
        assert_eq!(
            graph.official_name.as_deref(),
            Some("Rural Land Use in the Swiss Midlands, 1920-1950")
        );
    }

    #[test]
    fn a_placeholder_or_empty_title_is_absent() {
        for value in ["MISSING", "CALCULATED", ""] {
            let raw = ProjectRaw {
                name: value.to_string(),
                official_name: value.to_string(),
                ..project()
            };
            let graph = build(&raw, &[]);
            assert_eq!(graph.name, None, "{value:?}");
            assert_eq!(graph.official_name, None, "{value:?}");
            // Both writers fall back to the raw name here, so it survives beside
            // the resolved one.
            assert_eq!(graph.raw_name, value);
        }
    }

    /// DataCite emits one rights entry per `legalInfo` element even when both
    /// its identifier and its URI are placeholders, so neither may be filtered.
    #[test]
    fn placeholder_license_values_survive_verbatim() {
        let raw = ProjectRaw {
            legal_info: vec![
                legal("MISSING", "MISSING"),
                legal("", ""),
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
            ],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(
            graph.legal_info,
            vec![
                LicenseRef {
                    license_identifier: "MISSING".to_string(),
                    license_uri: "MISSING".to_string(),
                    license_label: None,
                },
                LicenseRef {
                    license_identifier: String::new(),
                    license_uri: String::new(),
                    license_label: None,
                },
                LicenseRef {
                    license_identifier: "CC-BY-4.0".to_string(),
                    license_uri: "https://creativecommons.org/licenses/by/4.0/".to_string(),
                    license_label: Some("Creative Commons Attribution 4.0 International".to_string()),
                },
            ]
        );
    }

    #[test]
    fn a_resolving_temporal_entry_carries_its_name_and_its_date() {
        let raw = ProjectRaw {
            temporal_coverage: vec![temporal_reference(
                "https://chronontology.dainst.org/period/0vGXxVln724L",
                Some("Trajanic"),
            )],
            ..project()
        };
        // Enrichment is present and even has a "Trajanic" row with a different
        // range; the URL tier must still win, proving its precedence.
        let graph = build(&raw, &[]);
        assert_eq!(graph.temporal_coverage[0].name.as_deref(), Some("Trajanic"));
        let resolution = graph.temporal_coverage[0].resolution.as_ref().expect("it resolves");
        assert_eq!(resolution.date, "0098/0117");
        assert_eq!(resolution.date_information.as_deref(), Some("Trajanic"));
    }

    #[test]
    fn free_text_resolves_via_enrichment() {
        let raw = ProjectRaw {
            temporal_coverage: vec![temporal_text("Early Christianity")],
            ..project()
        };
        let enriched = enrichment(&[("Early Christianity", Some("0030/0451"), "Early Christianity")]);
        let graph = build_with_enrichment(&raw, enriched);
        let resolution = graph.temporal_coverage[0].resolution.as_ref().expect("it resolves");
        assert_eq!(resolution.date, "0030/0451");
        assert_eq!(resolution.date_information.as_deref(), Some("Early Christianity"));
    }

    #[test]
    fn a_stale_url_falls_through_to_enrichment() {
        // URL present but unknown to the period cache; enrichment by name resolves.
        let raw = ProjectRaw {
            temporal_coverage: vec![temporal_reference(
                "https://chronontology.dainst.org/period/stale",
                Some("Late Middle Ages"),
            )],
            ..project()
        };
        let enriched = enrichment(&[("Late Middle Ages", Some("1250/1500"), "Late Middle Ages")]);
        let graph = build_with_enrichment(&raw, enriched);
        let resolution = graph.temporal_coverage[0].resolution.as_ref().expect("it resolves");
        assert_eq!(resolution.date, "1250/1500");
        assert_eq!(resolution.date_information.as_deref(), Some("Late Middle Ages"));
    }

    #[test]
    fn an_enrichment_row_without_a_range_carries_its_name_only() {
        let raw = ProjectRaw {
            temporal_coverage: vec![temporal_text("Vague Period")],
            ..project()
        };
        let enriched = enrichment(&[("Vague Period", None, "Vague Period")]);
        let graph = build_with_enrichment(&raw, enriched);
        let resolution = graph.temporal_coverage[0].resolution.as_ref().expect("a name-only resolution");
        assert!(resolution.date.is_empty());
        assert_eq!(resolution.date_information.as_deref(), Some("Vague Period"));
    }

    /// An entry that resolves to no date is not `None`: DataCite still emits it
    /// with an empty `date`, and Dublin Core still emits the name.
    #[test]
    fn an_unresolved_temporal_entry_still_carries_its_name() {
        let raw = ProjectRaw {
            temporal_coverage: vec![temporal_text("Mysterious Era")],
            ..project()
        };
        // Enrichment is populated but has no row for "Mysterious Era": resolution
        // must fall through both the URL and enrichment tiers to the name-only tier.
        let graph = build(&raw, &[]);
        assert_eq!(graph.temporal_coverage[0].name.as_deref(), Some("Mysterious Era"));
        let resolution = graph.temporal_coverage[0].resolution.as_ref().expect("a name-only resolution");
        assert!(resolution.date.is_empty());
        assert_eq!(resolution.date_information.as_deref(), Some("Mysterious Era"));
    }

    #[test]
    fn a_nameless_unresolvable_temporal_entry_has_no_resolution() {
        let raw = ProjectRaw {
            temporal_coverage: vec![temporal_reference("", None)],
            ..project()
        };
        // Even with the period cache and enrichment populated, an entry that
        // carries neither a resolvable URL nor any name yields nothing.
        let graph = build(&raw, &[]);
        assert_eq!(graph.temporal_coverage[0].name, None);
        assert_eq!(graph.temporal_coverage[0].resolution, None);
    }

    #[test]
    fn parts_come_from_part_ref_from_record() {
        let records = vec![
            record("record-0001", english("Survey Responses")),
            record(
                "record-0002",
                Multilingual::from([("de".to_string(), "Feldnotizen".to_string())]),
            ),
        ];
        let graph = build(&project(), &records);
        assert_eq!(graph.parts, records.iter().map(PartRef::from_record).collect::<Vec<_>>());
        assert_eq!(
            graph.parts.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(),
            vec!["Survey Responses", "Feldnotizen"]
        );
    }

    #[test]
    fn publication_year_prefers_the_declared_year_over_the_start_date() {
        let graph = build(&project(), &[]);
        assert_eq!(graph.publication_year, "2008");

        let raw = ProjectRaw {
            data_publication_year: Some("2014-03-01".to_string()),
            ..project()
        };
        assert_eq!(build(&raw, &[]).publication_year, "2014");
    }

    #[test]
    fn dates_and_languages_stay_whole() {
        let raw = ProjectRaw { start_date: "MISSING".to_string(), ..project() };
        let graph = build(&raw, &[]);
        assert_eq!(graph.start_date, "MISSING");
        assert_eq!(graph.end_date, "2012-08-31");
        assert_eq!(graph.data_language, vec!["de".to_string(), "fr".to_string()]);
    }

    #[test]
    fn disciplines_carry_an_authority_url_only_for_references() {
        let raw = ProjectRaw {
            disciplines: vec![
                Discipline::Reference(AuthorityFileReference {
                    type_: "URL".to_string(),
                    url: "https://d-nb.info/gnd/4066562-8".to_string(),
                    text: Some("Economics".to_string()),
                }),
                // Skipped by every writer, so the graph does not carry it.
                Discipline::Reference(AuthorityFileReference {
                    type_: "URL".to_string(),
                    url: "https://d-nb.info/gnd/1".to_string(),
                    text: None,
                }),
                Discipline::Text(english("10404 Visual arts and Art history")),
            ],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(
            graph.disciplines,
            vec![
                DisciplineRef {
                    text: "Economics".to_string(),
                    authority_url: Some("https://d-nb.info/gnd/4066562-8".to_string()),
                },
                DisciplineRef {
                    text: "10404 Visual arts and Art history".to_string(),
                    authority_url: None,
                },
            ]
        );
    }

    #[test]
    fn funders_resolve_to_names_and_text_funding_yields_nothing() {
        let raw = ProjectRaw {
            funding: Funding::Grants(vec![Grant {
                funders: vec!["organization-001".to_string(), "organization-999".to_string()],
                number: Some("123456".to_string()),
                name: Some("Rural Land Use".to_string()),
                url: None,
            }]),
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(
            graph.funding,
            vec![FundingRef {
                // An unresolvable ID falls back to itself so no funder is lost.
                funder_names: vec![
                    "Schweizerischer Nationalfonds".to_string(),
                    "organization-999".to_string()
                ],
                number: Some("123456".to_string()),
                name: Some("Rural Land Use".to_string()),
                url: None,
            }]
        );

        let raw = ProjectRaw {
            funding: Funding::Text("Self-funded".to_string()),
            ..project()
        };
        assert!(build(&raw, &[]).funding.is_empty());
    }

    #[test]
    fn alternative_names_are_not_deduplicated_against_the_titles() {
        let raw = ProjectRaw {
            alternative_names: Some(vec![english("Rural Land Use"), english("RLU"), Multilingual::new()]),
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(graph.alternative_names, vec!["Rural Land Use".to_string(), "RLU".to_string()]);
    }

    /// Every multilingual value goes through `shared_metadata::multilingual_value`:
    /// English when present, otherwise the lexicographically smallest tag. The rule
    /// is deterministic on purpose — it also keys the temporal-enrichment lookup.
    #[test]
    fn multilingual_values_prefer_english_then_the_smallest_tag() {
        let raw = ProjectRaw {
            description: Multilingual::from([
                ("de".to_string(), "Eine Studie.".to_string()),
                ("en".to_string(), "A study.".to_string()),
            ]),
            abstract_text: Some(Multilingual::from([
                ("fr".to_string(), "Un résumé.".to_string()),
                ("de".to_string(), "Eine Zusammenfassung.".to_string()),
            ])),
            keywords: vec![Multilingual::from([
                ("it".to_string(), "uso del suolo".to_string()),
                ("de".to_string(), "Landnutzung".to_string()),
            ])],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(graph.description.as_deref(), Some("A study."));
        assert_eq!(graph.abstract_text.as_deref(), Some("Eine Zusammenfassung."));
        assert_eq!(graph.keywords, vec!["Landnutzung".to_string()]);
    }

    /// The `url` reading rule, applied once here instead of in every writer: a
    /// legacy string array yields the first two real entries, a placeholder
    /// yields nothing, and an explicit `secondaryURL` wins over the array's
    /// second element.
    #[test]
    fn the_url_reading_rule_is_applied_once() {
        let raw = ProjectRaw {
            url: Some(serde_json::json!(["https://example.org/site", "https://example.org/data"])),
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(graph.website.map(|r| r.url).as_deref(), Some("https://example.org/site"));
        assert_eq!(
            graph.secondary_website.map(|r| r.url).as_deref(),
            Some("https://example.org/data")
        );

        let raw = ProjectRaw { url: Some(serde_json::json!(["MISSING"])), ..project() };
        let graph = build(&raw, &[]);
        assert!(graph.website.is_none());
        assert!(graph.secondary_website.is_none());

        let raw = ProjectRaw {
            url: Some(serde_json::json!(["https://example.org/site", "https://example.org/data"])),
            secondary_url: Some(AuthorityFileReference {
                type_: "URL".to_string(),
                url: "https://example.org/explicit".to_string(),
                text: None,
            }),
            ..project()
        };
        assert_eq!(
            build(&raw, &[]).secondary_website.map(|r| r.url).as_deref(),
            Some("https://example.org/explicit")
        );
    }
}
