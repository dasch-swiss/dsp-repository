//! A project's facts after resolution, one resolved object per project.
//!
//! The graph carries facts; each writer keeps its own vocabulary and its own
//! cardinality choices. Where two writers read the same field but disagree
//! about it, the graph carries the *underlying* value — placeholders and
//! emptiness included — so each writer can keep branching on it.

use std::borrow::Cow;

use shared_metadata::temporal_coverage::Resolution;
use shared_metadata::{
    AccessRightsType, AuthorityFileReference, Discipline, Funding, LegalInfo, ProjectRaw, ProjectStatus, Record,
};

#[cfg(test)]
use crate::graph::ArkHost;
use crate::graph::{AgentKind, PartRef, ResolveContext, DASCH, FALLBACK_PUBLICATION_YEAR};
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

/// The `nameIdentifier` scheme that identifies an agent well enough to be its
/// IRI. A GND string or a bare name does not.
const ORCID: &str = "ORCID";

impl ProjectAgent {
    /// The agent's ORCID, when it has a usable one.
    ///
    /// One rule, one place. Both the JSON-LD `@id` and Signposting's `author`
    /// link ask "does this agent have an IRI of its own", and they used to ask
    /// it separately — with two `const ORCID` declarations and two answers. The
    /// JSON-LD side filtered placeholders and took the first match; the link
    /// set did neither, so a placeholder ORCID became an `author` link the
    /// graph did not back, and an agent with two ORCIDs got two links and one
    /// `@id`. Two representations of one graph disagreeing is what ADR-0005's
    /// single-graph rule exists to prevent.
    pub fn orcid(&self) -> Option<&str> {
        self.name_identifiers
            .iter()
            .find(|id| id.scheme == ORCID)
            .and_then(|id| crate::helpers::real(&id.identifier))
    }
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
    ///
    /// Its host is [`ResolveContext::ark_host`]'s, which is the recorded one
    /// unless the deployment configured otherwise.
    pub ark: String,
    pub shortcode: String,
    /// The recorded `pid`, host-substituted alongside [`ark`](Self::ark) so the
    /// two are comparable: the `sameAs` this feeds exists to show a recorded
    /// PID differing from the resolved ARK as a *fact about the data*, and a
    /// difference in host alone is a fact about the deployment.
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
    /// Possibly empty. The mandatory-creator fallback is
    /// [`ProjectGraph::creators_with_fallback`], not this field: applying it
    /// here would change `oai_dc` output for every project with no attributed
    /// creator.
    pub creators: Vec<ProjectAgent>,
    pub contributors: Vec<ProjectAgent>,
    pub keywords: Vec<String>,
    pub disciplines: Vec<DisciplineRef>,
    /// Verbatim: DataCite formats the two into one `Collected` range, while
    /// Dublin Core emits `start_date` alone and only when it is real.
    pub start_date: String,
    pub end_date: String,
    /// The year the declared publication year or the start date yields, or
    /// `None` when neither yields one. The mandatory-year fallback is
    /// [`ProjectGraph::publication_year_with_fallback`], not this field:
    /// resolving it here would make every writer assert a publication year for
    /// a project that records no usable date.
    pub publication_year: Option<String>,
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
    /// Resolves one project into a graph.
    ///
    /// `records` is an iterator rather than a slice so a caller can bound what
    /// the builder materialises. One `PartRef` per record is two small strings
    /// plus the record's file when it has a publishable one, which is nothing
    /// for a project of forty records and 27,026 of them for the largest
    /// committed one — and the embedded JSON-LD on its landing page caps
    /// `hasPart` and `distribution` at a hundred. So the landing page hands in
    /// a bounded iterator, the standalone representation hands in all of them,
    /// and the OAI writers, which read no part at all, hand in an empty slice.
    pub fn build<'a>(
        raw: &ProjectRaw,
        ctx: &ResolveContext<'_>,
        records: impl IntoIterator<Item = &'a Record>,
    ) -> Self {
        let (website, secondary_from_array) = shared_metadata::utils::parse_url_value(raw.url.clone());

        Self {
            ark: ctx.ark_host.apply(canonical_ark(raw)),
            shortcode: raw.shortcode.clone(),
            // Host-substituted like `ark`, deliberately: the `sameAs` this
            // feeds exists to show a *recorded* PID differing from the resolved
            // ARK, and F-UJI folds `schema:sameAs` into its object-identifier
            // pool. Leaving the recorded host here would put the very
            // identifier the substitution removes back into that pool.
            pid: ctx.ark_host.apply(raw.pid.clone()),
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
            parts: records
                .into_iter()
                .map(|record| PartRef::from_record(record, ctx.ark_host))
                .collect(),
        }
    }

    /// `publication_year`, or the fallback year when neither the declared
    /// publication year nor the start date yields one.
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

    /// The project's usable license URIs: placeholders and empties dropped,
    /// deduplicated, in file order.
    ///
    /// `legal_info` keeps every element verbatim because DataCite emits one
    /// `rightsList` entry per element and tests the URI differently. Everything
    /// that wants "which licenses does this project carry" — the Dublin Core
    /// `rights` URIs, the schema.org `license` list and the Signposting
    /// cardinality rule — wants this list instead, and reading it here rather
    /// than filtering `legal_info` again is what keeps them from disagreeing.
    pub fn license_uris(&self) -> Vec<&str> {
        let mut uris: Vec<&str> = Vec::new();
        for legal in &self.legal_info {
            let uri = legal.license_uri.as_str();
            if uri.is_empty() || shared_metadata::is_placeholder(uri) || uris.contains(&uri) {
                continue;
            }
            uris.push(uri);
        }
        uris
    }

    /// The project's preferred title, and its alternatives in emission order.
    ///
    /// The precedence is DataCite's, because it was written there first: the
    /// longer of `name` and `officialName` wins, the other becomes an
    /// alternative, and the recorded alternative names follow, deduplicated
    /// against everything already chosen. When neither title is real the raw
    /// `name` carries its placeholder through, which is what the DataCite
    /// output has always done — a writer that must not emit a placeholder
    /// filters it out itself.
    ///
    /// Resolved here rather than in each writer so that "the same precedence as
    /// DataCite" is a call rather than a second implementation.
    pub fn titles(&self) -> (String, Vec<String>) {
        let (primary, alternative) = match (self.name.as_ref(), self.official_name.as_ref()) {
            (Some(name), Some(official_name)) if official_name.len() >= name.len() => {
                (official_name.clone(), Some(name.clone()))
            }
            (Some(name), Some(official_name)) => (name.clone(), Some(official_name.clone())),
            (None, Some(official_name)) => (official_name.clone(), None),
            _ => (self.raw_name.clone(), None),
        };

        let mut alternatives = Vec::new();
        if let Some(alternative) = alternative {
            if alternative != primary {
                alternatives.push(alternative);
            }
        }
        for name in &self.alternative_names {
            if *name != primary && !alternatives.contains(name) {
                alternatives.push(name.clone());
            }
        }
        (primary, alternatives)
    }

    /// `creators`, or a single organizational `DaSCH` when the project credits
    /// nobody as one.
    ///
    /// DataCite makes at least one creator mandatory, so every representation
    /// that rule governs needs this fallback and resolves it here rather than
    /// writing it out again (ADR-0005). Dublin Core deliberately does not call
    /// it: `oai_dc` has no such rule, and naming DaSCH as the creator of a
    /// project nobody is credited with would invent attribution (ADR-0005,
    /// *Nothing is invented for a score*). That is also why `build` leaves
    /// `creators` holding only the agents the project actually attributes.
    pub fn creators_with_fallback(&self) -> Cow<'_, [ProjectAgent]> {
        if self.creators.is_empty() {
            Cow::Owned(vec![ProjectAgent {
                name: DASCH.to_string(),
                kind: AgentKind::Organization,
                given_name: None,
                family_name: None,
                name_identifiers: Vec::new(),
                affiliations: Vec::new(),
                contributor_type: Vec::new(),
            }])
        } else {
            Cow::Borrowed(&self.creators)
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
/// `start_date`.
///
/// Presence rather than validity, deliberately: a recorded but unusable
/// `dataPublicationYear` yields `None` and does **not** fall through to the
/// start date. That is what the byte-identical DataCite output has always done,
/// and a project that declared a publication year is not making a claim about
/// its start date.
fn publication_year(raw: &ProjectRaw) -> Option<String> {
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
    use shared_metadata::{Attribution, Grant, Multilingual, ProjectRaw};

    use super::*;
    use crate::test_support::*;

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

    #[test]
    fn a_substituted_host_reaches_the_ark_the_pid_and_every_part() {
        let record = record("record-0001", english("Survey Responses"));
        let graph = build_with_ark_host(
            &project(),
            std::slice::from_ref(&record),
            ArkHost::Substituted("https://dpe-pr-391.a.run.app"),
        );
        assert_eq!(graph.ark, "https://dpe-pr-391.a.run.app/ark:/72163/1/0001");
        // `pid` too, or the `sameAs` it feeds would put the recorded host back
        // into the identifier pool an assessor reads.
        assert_eq!(graph.pid, "https://dpe-pr-391.a.run.app/ark:/72163/1/0001");
        assert_eq!(graph.parts[0].ark, "https://dpe-pr-391.a.run.app/ark:/72163/1/0001/record-0001");
    }

    #[test]
    fn a_substituted_host_reaches_the_shortcode_fallback_and_leaves_a_placeholder_pid_alone() {
        for pid in ["MISSING", "CALCULATED", ""] {
            let raw = ProjectRaw { pid: pid.to_string(), ..project() };
            let graph = build_with_ark_host(&raw, &[], ArkHost::Substituted("https://dpe-pr-391.a.run.app"));
            assert_eq!(graph.ark, "https://dpe-pr-391.a.run.app/ark:/72163/1/0001", "{pid:?}");
            assert_eq!(graph.pid, pid, "{pid:?}");
        }
    }

    #[test]
    fn a_placeholder_empty_or_duplicate_license_uri_is_not_a_usable_one() {
        let raw = ProjectRaw {
            legal_info: vec![
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                legal("MISSING", "MISSING"),
                legal("", ""),
            ],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(graph.license_uris(), vec!["https://creativecommons.org/licenses/by/4.0/"]);
        // `legal_info` still holds every element: DataCite emits one per element.
        assert_eq!(graph.legal_info.len(), 4);
    }

    /// `build` records the attributed creators as they are; the
    /// mandatory-creator fallback is the accessor below, so that it cannot
    /// reach `oai_dc`.
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
    fn an_empty_creator_set_falls_back_to_the_dasch_organization() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-001".to_string(),
                contributor_type: vec!["Researcher".to_string()],
            }],
            ..project()
        };
        let creators = build(&raw, &[]).creators_with_fallback().into_owned();
        assert_eq!(creators.len(), 1);
        assert_eq!(creators[0].name, "DaSCH");
        assert_eq!(creators[0].kind, AgentKind::Organization);
        assert!(creators[0].name_identifiers.is_empty());
        assert!(creators[0].affiliations.is_empty());
    }

    #[test]
    fn the_fallback_leaves_attributed_creators_alone() {
        let raw = ProjectRaw {
            attributions: vec![Attribution {
                contributor: "person-001".to_string(),
                contributor_type: vec!["Project Leader".to_string()],
            }],
            ..project()
        };
        let graph = build(&raw, &[]);
        assert!(!graph.creators.is_empty());
        assert_eq!(graph.creators_with_fallback().as_ref(), graph.creators.as_slice());
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
        assert_eq!(
            graph.parts,
            records
                .iter()
                .map(|r| PartRef::from_record(r, ArkHost::Recorded))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            graph.parts.iter().map(|p| p.title.as_str()).collect::<Vec<_>>(),
            vec!["Survey Responses", "Feldnotizen"]
        );
    }

    #[test]
    fn publication_year_prefers_the_declared_year_over_the_start_date() {
        let graph = build(&project(), &[]);
        assert_eq!(graph.publication_year.as_deref(), Some("2008"));

        let raw = ProjectRaw {
            data_publication_year: Some("2014-03-01".to_string()),
            ..project()
        };
        assert_eq!(build(&raw, &[]).publication_year.as_deref(), Some("2014"));
    }

    /// Presence, not validity: a declared but unusable year does not fall
    /// through to the start date, and the graph reports no year at all rather
    /// than one it made up.
    #[test]
    fn an_unusable_declared_year_yields_no_year_and_does_not_fall_back_to_the_start_date() {
        let raw = ProjectRaw {
            data_publication_year: Some("MISSING".to_string()),
            start_date: "2008-06-01".to_string(),
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(graph.publication_year, None);
        assert_eq!(graph.publication_year_with_fallback(), "2015");
    }

    #[test]
    fn an_unusable_start_date_yields_no_year_when_none_is_declared() {
        let raw = ProjectRaw {
            data_publication_year: None,
            start_date: "MISSING".to_string(),
            ..project()
        };
        let graph = build(&raw, &[]);
        assert_eq!(graph.publication_year, None);
        assert_eq!(graph.publication_year_with_fallback(), "2015");
    }

    #[test]
    fn the_year_fallback_leaves_a_real_year_alone() {
        assert_eq!(build(&project(), &[]).publication_year_with_fallback(), "2008");
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
