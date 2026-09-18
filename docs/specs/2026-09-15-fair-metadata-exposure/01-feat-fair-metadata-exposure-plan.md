---
title: "feat: Machine-readable project metadata for FAIR assessment"
type: feat
date: 2026-09-15
author: "Ivan Subotic"
status: reviewed(2)
repository: dasch-swiss/dsp-repository
linear: DEV-7268
---

# feat: Machine-readable project metadata for FAIR assessment

## Overview

This plan implements [ADR-0005](../../adr/0005-fair-landing-pages-in-the-access-area.md),
*Every landing page in the Access Area is FAIR-assessable by machine*, for the
first landing page: the DPE project page `/dpe/projects/{shortcode}`. The ADR
owns the decision and its rationale; this plan owns the how. Where the two
disagree, the ADR wins and the plan is wrong.

Four mechanisms, each building on the previous one:

1. schema.org JSON-LD and Dublin Core `<meta>` tags embedded in the page head.
2. FAIR Signposting `Link` headers and matching `<link>` elements on the landing
   page.
3. Machine-readable representations at their own URLs under the project path
   (JSON-LD, DataCite JSON), advertised from the landing page via `describedby`,
   plus the one negotiation step the ADR allows: a `303 See Other` from the
   landing page when `Accept` prefers one of them.
4. Optionally, a Turtle representation on the same footing.

The data already exists and is already mapped to DataCite kernel 4 for OAI-PMH.
The work is exposure, not modelling: move the existing mapping into a shared
crate, resolve each project and each record into one graph that every writer
reads, add two serialisations, and wire them into the page and three routes.

## Problem Statement / Motivation

On 2026-09-15 F-UJI 3.5.0 and FAIR Champion (OSTrails Core tests) were run against
`https://ark.dasch.swiss/ark:/72163/1/0862`. Both resolved the ARK, followed the
redirect to `https://repository.dasch.swiss/dpe/projects/0862`, and found an HTML
page with no structured metadata: no JSON-LD, no meta tags, no `Link` header, and
the same HTML for every `Accept` value.

| Assessor | Result | Detail |
|----------|--------|--------|
| F-UJI 3.5.0 | 3 of 24 (12.5%) | F 2/7, A 1/3, I 0/4, R 0/10. Only F1 (identifier) and A1-02M (protocol) pass. |
| FAIR Champion | 6 of 15 pass | 4 real passes (identifier unique, persistent, open protocol, auth). 2 hollow: "linked data found" with 0 of 0 triples. 6 fail, 3 indeterminate. |

Every failure downstream of F1 follows from the one gap. The live OAI-PMH record
for the same project already carries creators with ORCID, publisher, year,
subjects, contributors, SPDX rights and funding, so the fix is to put what the OAI
endpoint knows onto the page the ARK lands on.

Repository FAIRness is a differentiator DaSCH claims against Zenodo and
SWISSUbase. A 12.5% F-UJI score on a project landing page undercuts that claim.
ADR-0005 records this as the reason the decision was taken.

## Proposed Solution

**Decided in ADR-0005, taken as given here.** Its eight decision bullets
(resolver stays a redirect, embedded metadata, Signposting, one URL per
representation with the `303` as the only negotiation step, one resolved graph
in a shared crate, `Dataset` root, nothing invented for a score, FAIRness
measured) and its *Considered Options* are not restated; this plan only adds
the how.

**`shared-fair` is the engine; capabilities are its adapters.** A new shared
crate `shared-fair` (`shared/fair`) holds three layers, none of which knows
DPE. `shared/` is the shared root ADR-0002 names after its 2026-09-17
amendment (the earlier `platform/` collides with Bazel's platform vocabulary);
today the tree still says `modules/platform/` and `platform-{role}`. Phase 0
of this plan executes that half of ADR-0002 first, moving `modules/platform/`
to `shared/` and renaming `platform-metadata` and `platform-telemetry`, so the
new crate is born in its final place with its final name and never carries the
old prefix.

- The **graph types**, `ProjectGraph` and `RecordGraph`: an object's facts after
  resolution, the concept every FAIR consumer speaks. This is the type a port
  will return once the Access Area's data owner is a capability of its own.
- The **builders**, `ProjectGraph::build(&ProjectRaw, &ResolveContext, &[Record])`
  and `RecordGraph::build(&Record)`, over the contract types of
  `shared-metadata`. `ResolveContext` bundles what resolution needs and DPE
  owns today: the `ContributorLookup` (the trait moves from `dpe-core` to
  `shared-metadata`, beside the `Person` and `Organization` it is already
  expressed in, `contributors.rs:32-35`) and the two temporal-coverage tables
  that `project_to_datacite` currently reads from `dpe-core`'s process-global
  caches (`datacite.rs:283-288`), as the `shared-metadata` map types
  `chronontology::load_from` and `temporal_enrichment::load_from` return. The
  only DPE-specific logic in the whole pipeline is the adapter: `dpe-core`'s
  cached lookup implements the trait, and DPE hands over its tables and the
  `ProjectRaw` and `Record`s it already parses.
- The **writers**: the moved DataCite and Dublin Core mappings, re-plumbed as
  writers over the graphs, and the new schema.org JSON-LD, Dublin Core meta,
  Signposting link set, DataCite JSON and Turtle writers. Every writer takes a
  graph plus the consumer's URL layout as options. Nothing in the crate knows
  where a page is rendered, so it has no routes, no Maud, no Axum.

The crate is a shared crate from the start although DPE is its only consumer
today (ADR-0005, *Consequences*): the next consumers are named, record landing
pages and CPE in the Access Area cannot import a `dpe-*` crate (ADR-0003), and
the Deposit Area will assess a project's FAIRness before submission. That
assessment engine is not designed here; the graph and the writers are its seed.

**Two resolved graphs, read by every writer.** `ProjectGraph` is the
project resolved once: canonical ARK and shortcode, titles, description, agents
with ORCID and affiliation, licenses, access rights, dates, keywords, coverage,
funding, parts. `RecordGraph` is the same for a record: ARK, project ARK,
titles, creators with their inferred kind, description, dates, one general data
type, license, access rights, keywords, publisher, citation text and size. The existing `project_to_datacite`,
`project_to_dublin_core`, `record_to_datacite` and `record_to_dublin_core`
become writers over the graphs in the same phase as the crate move, and the
OAI hash test proves that their output did not change. No writer sees a raw
`ProjectRaw` or `Record`, so the representations cannot disagree on who the
creators are, which license applies, or which title is preferred. A corpus-wide
test enforces the agreement over every committed project and record as a second
line of defence.

Record landing pages are out of scope for this plan (ADR-0005 names them as the
next landing page). `RecordGraph` is built now so that a record landing page is
writers plus routes, not a second resolver, and so that the project's `hasPart`
entries and a future record page share one title rule.

**DPE's adapter is the seam for the capability split.** Today DPE reads the
corpus through its own caches and hands `ProjectRaw`, `Record`s and the
`ResolveContext` (lookup plus tables) to the builder. When the corpus readers move into their own Access-Area
capability (ADR-0003; a separate plan), those three arrive through a port DPE
declares, and `shared-fair` does not change. That refactor is not part of
this plan.

**Head extras are rendered in `dpe-server`.** The server already owns the
document shell in `view.rs` and is the composition root; it gains a
`head_extras` slot and a `metadata` module that builds the graph from DPE's
caches, calls the writers, wraps the JSON-LD in the `<script>` element and
turns the link set into a `HeaderValue`.

## Alternative Approaches Considered

Only implementation-level alternatives are listed here; the architectural ones,
including keeping the crate inside DPE until a second consumer appears, are in
ADR-0005.

**Export the mapping from `dpe-api-oai` instead of extracting a crate.** Cheapest
change: make the `metadata` module public and add the new writers there. Rejected
because an OAI crate would then own schema.org JSON-LD that has nothing to do
with OAI, and `dpe-server` would depend on an API crate for non-API code. The
repo's own rule is "where the shared thing is a subset of a crate, extract the
subset" (`docs/src/repo_structure.md`). The extraction is mechanical and the OAI
hash test pins its output.

**Build the graph from DPE's view model, keeping the builder in `dpe-core`.**
Same code, but the resolution logic would be DPE's, and the Deposit Area would
need a second builder from `ProjectRaw`: two resolvers of one graph, the exact
duplication the graph exists to remove. The view model is a field copy of
`ProjectRaw` except for the `url` reading rule (`project.rs:96-142`), so
building from the contract costs one moved function.

**Put the graph and the writers into `shared-metadata`.** It already holds
the reading rules and the per-project checks both areas apply. Rejected: the
contract crate would grow four vocabularies and their fixtures, and a change to
schema.org output would look like a contract change. Contract and exposure stay
two crates, the second depending on the first.

**Use `DataCiteRecord` as the resolved intermediate.** It already exists and
already carries resolved agents. Rejected: it is vocabulary-mapped
(`nameType`, `contributorType`, `resourceTypeGeneral` as DataCite strings,
rights as label plus URI), so JSON-LD would have to un-map DataCite terms back to
facts, and the access-rights enum the JSON-LD and COAR mapping need is already
flattened to text. A graph of facts with vocabulary mapping in each writer is
the simpler shape.

**Keep three independent resolvers and rely on the agreement test.** Smaller
Phase 1. Rejected because ADR-0005 says one resolved graph feeds every
representation, and a plan that ships three resolvers would make the ADR false
on the day it lands; the agreement test only covers the fields it names.

**Build head extras in `dpe-web`.** Rejected because `dpe-web` would then depend
on `shared-fair` for no gain; the server already owns the shell and is where
the graph is assembled from DPE's caches.

## Technical Considerations

### Dependency graph after Phase 1

```
shared-metadata → shared-fair → { dpe-api-oai, dpe-server }
shared-metadata → dpe-core       → { dpe-api-oai, dpe-web, dpe-server }   (unchanged)
```

`shared-fair` depends on `shared-metadata` (`ProjectRaw`, `Record`,
`Person`, `Organization`, `ContributorLookup`, `Multilingual`,
`AccessRightsType`, `is_placeholder`, `multilingual_value`, the temporal
tables' map types and `temporal_coverage::resolve_in`), `serde` and
`serde_json`. It depends on no service crate (a Cargo cycle would stop it), has
no path into a service module (`check-shared-paths.sh`, today
`check-platform-paths.sh`, whose pathspec Phase 0 points at
`shared/*/src/*.rs`), and pulls in no web framework: `LinkSet` renders header *strings*, the JSON-LD escaper
returns a `String`, and `dpe-server` turns them into `HeaderValue`s and Maud.

`dpe-core`'s dependencies do not change: `ContributorLookup` lands in
`shared-metadata`, which `dpe-core` already depends on, so the domain crate
never depends on the exposure engine. `dpe-api-oai` depends on
`shared-metadata`, `dpe-core` and `shared-fair`, and keeps `xml.rs`,
`handlers/`, `resumption.rs`, `error.rs` and the OAI half of
`metadata/mod.rs`; only the mapping moves. `dpe-web` is untouched.

Everything keyed on the shared root is re-pointed once, in Phase 0, and then
covers the new crate by glob: the CI path filters (`modules/platform/**` in six
workflows becomes `shared/**`), the `bacon.toml` watch lists (lines 19 and
30), and the paths gate's pathspec. The Dockerfiles copy prebuilt binaries and
`just test` runs the whole workspace, so neither enumerates crates. The
workspace `Cargo.toml` members list, the commit-scope table and the
agent-context layer are the places that enumerate crates by name.

### Where things live

| Concern | Crate / file |
|---------|--------------|
| `ContributorLookup` trait (moved from `dpe-core`, a reading rule over the contract's `Person` and `Organization`) | `shared-metadata/src/contributors.rs` |
| Resolved intermediates `ProjectGraph`, `RecordGraph`, the `ResolveContext` (lookup plus temporal tables) and the builders over the contract types | `shared-fair/src/graph.rs` |
| DataCite and Dublin Core record models and writers over the graphs | `shared-fair/src/datacite.rs`, `dublin_core.rs`, `record_datacite.rs`, `record_dublin_core.rs`, `types.rs` (moved, then re-plumbed) |
| Vocabulary helpers that need no lookup (`map_contributor_type`, `license_identifier_to_label`, `infer_subject_scheme`, `extract_year`, …) | `shared-fair/src/helpers.rs` (moved) |
| schema.org JSON-LD builder (`serde_json::Value`) and the `<script>`-safe escaper (`String`) | `shared-fair/src/schema_org.rs` |
| Dublin Core meta-tag writer | `shared-fair/src/dublin_core_meta.rs`, over the `DublinCoreRecord` the re-plumbed `project_to_dublin_core` produces plus `DC.accessRights` from the graph |
| DataCite JSON writer; its output is shape-checked in tests against `datacite_4.3_schema.json`, the only JSON schema DataCite publishes (pinned to a `datacite/schema` commit; the official kernel 4.6 is XSD-only) | `shared-fair/src/datacite_json.rs`, `shared-fair/testdata/schemas/` |
| Turtle writer (Phase 4) | `shared-fair/src/turtle.rs` |
| Signposting link set model (rel, href, type; header string) | `shared-fair/src/signposting.rs` |
| `Accept` decision (pure: header string and candidate list in, decision out) | `shared-fair/src/negotiate.rs` |
| The `url` reading rule (`parse_url_value`, moved from `dpe-core/src/project.rs:31`) | `shared-metadata/src/utils.rs`, beside `is_placeholder`; `dpe-core`'s `From<ProjectRaw>` calls it there |
| `CachedContributorLookup` implementing the `shared-metadata` trait; the project cache keeping `ProjectRaw` beside the view model with raw accessors on `ProjectRepository`; the OAI test double `InMemoryProjectRepository` built from `ProjectRaw` | `dpe-core/src/contributors.rs`, `project_cache.rs`, `project_repository.rs`; `modules/dpe/api-oai/src/handlers/test_utils.rs` |
| `dpe_core::resolve_inputs() -> (&'static dyn ContributorLookup, &'static HashMap<String, W3cdtfRange>, &'static HashMap<String, EnrichedDate>)`: the one source of what resolution needs in DPE, over `CachedContributorLookup`, `chronontology_cache::all_periods()` and `temporal_enrichment_cache::all_enriched()`; both call sites wrap it in `ResolveContext::new`, so they cannot drift; `dpe-core` never depends on `shared-fair` because the tuple names only `shared-metadata` types | `dpe-core/src/lib.rs` (the function), `modules/dpe/api-oai/src/metadata/mod.rs`, `modules/dpe/server/src/metadata.rs` |
| Per-shortcode record index `records_for_shortcode(&str) -> &'static [Record]`, grouped once at cache build | `dpe-core/src/record_cache.rs` |
| Corpus-wide tests over the committed data (hash test, agreement test) | `dpe-api-oai`, beside the data they read; a shared crate holds no path into DPE |
| Head extras rendering (`<script>`, `<meta>`, `<link>`), `HeaderValue` assembly, graph assembly from DPE's caches | `modules/dpe/server/src/view.rs` (new `head_extras` slot) and a new `modules/dpe/server/src/metadata.rs` |
| Representation route handlers and the 303 | `modules/dpe/server/src/metadata.rs` |
| Public base URL config | `modules/dpe/server/src/config.rs` (`DPE_PUBLIC_BASE_URL`), threaded through `AppState` like `css_href` (`main.rs:19-21`), never a process-global, so handler tests can inject it |
| OAI-only concerns that stay behind | `modules/dpe/api-oai/src/metadata/mod.rs`: `make_oai_identifier`, `parse_oai_identifier`, `to_oai_record*`, `membership_set_specs`, `matches_date_filter*`, `OaiRecord`, `OaiRecordHeader` |

The representation routes are not full-page routes: they load no
`telemetry.js`, so `KNOWN_ROUTES` in `dpe-server/src/page_url.rs` gets no
entry (`REVIEW.md:33`).

### The resolved graphs

`ProjectGraph::build` and `RecordGraph::build` are the only functions in
`shared-fair` that take a `&ProjectRaw` or a `&Record`. Everything else takes
a graph. The graph holds resolved **facts**; each writer holds its
**vocabulary**. Concretely:

| Resolved in the graph (once) | Left to the writer |
|------------------------------|------------------------|
| ARK from `pid` or built from the shortcode (`datacite.rs:23-30` today) | DataCite `identifierType`, schema.org `PropertyValue` shape |
| Agents: id → name, given/family, ORCID and GND URLs, affiliation names (`resolve.rs:68-83`), keeping the raw contributor-type strings | DataCite `contributorType` (`map_contributor_type`), `nameType`; schema.org `Person`/`Organization` |
| Creator set (`is_creator`) and the `DaSCH` fallback when empty (`datacite.rs:47-53`) | none |
| Preferred title and alternatives (`multilingual_value`, placeholder-filtered) | DataCite `titleType`, DC repetition |
| Description then abstract, English-preferring | truncation for `DC.description` |
| Distinct non-placeholder license URIs with their labels (`license_identifier_to_label`) | DC `rights` text, DataCite `rightsList`, schema.org `license`, Signposting `license` cardinality |
| `AccessRightsType` as the enum, embargo date | text (`access_rights_to_string`), COAR URI, `isAccessibleForFree` |
| Temporal coverage as ISO interval or text, through `shared_metadata::temporal_coverage::resolve_in` over the tables in the `ResolveContext` (today `datacite.rs:283-288` reads `dpe-core`'s globals) | DataCite `dates`, schema.org `temporalCoverage` |
| External website and secondary URL through the `url` reading rule | schema.org `producer.url` |
| Spatial coverage, funding, publications, data language, start/end dates, publication year, keywords, disciplines | per-vocabulary layout |
| Parts: one `PartRef { ark, title }` per contract Record in the committed record dump for the shortcode (three projects have one today: 4,198, 19,770 and 27,026 records), through `PartRef::from_record`, which shares `RecordGraph`'s title helper. `ProjectRaw.records` (a declared list of ids) is not read | `hasPart` cap. The project's DataCite `relatedIdentifiers` do not come from parts and are unchanged by this plan (`datacite.rs:227`) |

`RecordGraph` resolves: ARK and project ARK (`record.rs:29-42,155`), preferred
title and per-language alternatives, creators from `authorship` with the kind
inferred once (`DaSCH` is an organisation, everything else a person, as
`record_datacite.rs:17-24` does today) and the `DaSCH` fallback, description,
created/modified/published dates and the year, one general data type from
`typeOfData` (the identical match currently duplicated in
`record_datacite.rs:27-34` and `record_dublin_core.rs:11-18`), license URI and
label, access rights, keywords, publisher, `how_to_cite` and `size`. The
record's file pointer (`mime`, download URL) is not carried: the OAI Dublin
Core deliberately publishes the ARK only (`record_dublin_core.rs:24`,
`docs/src/dpe/oai-pmh.md`), no representation in this plan describes a record,
and the record landing-page plan adds the field when it emits `distribution`.

`hasPart` does not build a `RecordGraph` per record. `PartRef::from_record`
takes the ARK and the preferred title through the same title helper
`RecordGraph::build` uses, so the two cannot disagree on the title while a
27,000-record project costs two small strings per part instead of a full
graph.

Re-plumbing the four existing mappings onto the graphs happens in Phase 1, under
the byte-identical constraint, so the hash test proves the move, the switch from
the view model to `ProjectRaw`, and the re-plumbing at once. The mappings today
read no field the view model transforms (`url`, `clusters`, `collections`), so
the switch is safe by inspection and the test confirms it. Where a mapping today
makes a choice the graph does not carry (for example Dublin Core repeating every
alternative title while DataCite marks them `AlternativeTitle`), the graph
carries the underlying fact and the writer keeps the choice. If re-plumbing
a mapping would change its output, the mapping is wrong or the graph is missing
a fact; the output does not move in Phase 1.

### JSON-LD content

Built from `ProjectGraph`, which `head_extras_for_project` and the
representation handlers construct from the project cache's `ProjectRaw`, the
resolve inputs and `record_cache::records_for_shortcode()`. That index is new:
`all_records()` (`record_cache.rs:21-23`) is one flat vector of every record of
every project (50,994 today), and filtering it per request would put an
O(corpus) scan on the landing page, three times per visit. The index groups
records by upper-cased shortcode once at cache build, in the shape of
`project_cache.rs`'s `SHORTCODE_INDEX`, so a page costs O(its own records).

**Every identifier, URL and header value derives from the resolved project's
canonical `shortcode` and `pid`, never from the raw path segment.** The lookup is
case-insensitive, so `/dpe/projects/080c` and `/dpe/projects/080C` produce
byte-identical JSON-LD, `Link` headers and redirect targets. Axum percent-decodes
the path, so the raw segment is attacker-controlled and must not reach a header.

| schema.org property | Graph field | Rule |
|---------------------|-------------|------|
| `@id`, `identifier` (PropertyValue, propertyID `ARK`) | `ark` | Same fallback as DataCite, resolved once in the graph |
| `name`, `alternateName` | `title`, `alternative_titles` | Same precedence as DataCite titles |
| `description` | `description` | English-preferring; the graph already applied `multilingual_value` |
| `keywords` | `keywords` | One string per keyword |
| `license` | `licenses[].uri` | Every distinct URI; array when several; omitted when none |
| `conditionsOfAccess`, `isAccessibleForFree` | `access_rights` | See the access-rights table below |
| `datePublished` | `publication_year` | Year string; omitted when absent |
| `creator` | `creators` | Person with `identifier` PropertyValue for ORCID and `affiliation`; Organization otherwise. The graph already applied the `DaSCH` fallback |
| `contributor` | `contributors` | Same shape; DataCite role kept in `roleName` via a `Role` wrapper only if cheap, else omitted |
| `publisher` | constant | `{ "@type": "Organization", "name": "DaSCH", "url": "https://dasch.swiss" }` |
| `funder`, `funding` | `funding` | `MonetaryGrant` with `identifier` (grant number), `url`, `funder` Organization |
| `spatialCoverage` | `spatial_coverage` | `Place` with `sameAs` = authority URL and `name` |
| `temporalCoverage` | `temporal_coverage` | ISO 8601 interval or name-only text, as resolved in the graph |
| `inLanguage` | `data_languages` | As given |
| `url` | landing page | `{public_base_url}/dpe/projects/{canonical shortcode}`, passed in as an option; schema.org and Google expect the dataset's own page here, not the project website |
| `citation` | `publications` | Text, with `@id` when a PID exists |
| `producer` | project identity | `ResearchProject` node: `name`, `startDate`, `endDate`, `url` (the project's external website, when present), `member` (creators). schema.org's `producer` is unrelated to the OAIS Producer of the Deposit Area; the new documentation page says so once |
| `includedInDataCatalog` | option | `DataCatalog` node for the DPE at `{public_base_url}/dpe/projects` |
| `hasPart` | `parts` | `Dataset` nodes with `@id` = the contract Record's ARK and `name`; **capped at 100 in the embedded block** (the OAI default page size), uncapped in the standalone representation. The complete list is harvestable from the OAI set `project:{shortcode}`; no schema.org property is a truthful fit for that pointer, so none is emitted |
| `sameAs` | `pid` when it differs from `@id` | Rarely set |

No project-level `distribution`. There is no project-level download, and
inventing one would be false (ADR-0005, *Nothing is invented for a score*).
This is a known residual against F-UJI F3 and A1-03D for a project ARK; record
landing pages are the correct assessment target for those tests.

The workspace enables `serde_json`'s `preserve_order`. The JSON-LD builder
inserts keys in the order it wants them emitted and never calls `Map::remove`
(which silently re-sorts the map); use `retain` or `shift_remove` if a key must
go.

### Access rights mapping

`AccessRightsType` has four variants (`shared_metadata::project`, line 206).
`license` and `hasPart` are emitted for every variant: a license is a property of
the data whatever its access level, and record ARKs resolve to metadata landing
pages, not downloads, so listing them claims nothing about downloadability.

| `AccessRightsType` | `isAccessibleForFree` | `conditionsOfAccess` | `DC.accessRights` |
|--------------------|-----------------------|----------------------|-------------------|
| `FullOpenAccess` | `true` | `"Full Open Access"` | `http://purl.org/coar/access_right/c_abf2` (open access) |
| `OpenAccessWithRestrictions` | `false` | `"Open Access with Restrictions"` | `http://purl.org/coar/access_right/c_16ec` (restricted) |
| `EmbargoedAccess` | `false` | `"Embargoed Access until {embargo_date}"` when the date is set, else `"Embargoed Access"` | `http://purl.org/coar/access_right/c_f1cf` (embargoed) |
| `MetadataOnlyAccess` | `false` | `"Metadata only Access"` | `http://purl.org/coar/access_right/c_14cb` (metadata only) |

The text values are what `access_rights_to_string` (`helpers.rs:26-33`) already
returns, which are also the enum's serde names, so the human-readable and
machine-readable representations use one vocabulary. The COAR URIs are the
standard terms F-UJI's access-level test recognises.

### Descriptions and language

The graph's `description` is resolved with `shared_metadata::multilingual_value`
(English first, then the deterministic fallback already used by DataCite), so a
project with only German text yields the same German text in JSON-LD, Dublin Core
meta and DataCite. The JSON-LD carries the full text. `DC.description` is
truncated to 1000 characters on a character boundary with an ellipsis; a `<meta>`
attribute of several kilobytes is legal but pointless.

### Signposting link set

Emitted identically as an HTTP `Link` header and as `<link>` elements in the
head. Level 1 of the FAIR Signposting profile.

| rel | target | cardinality |
|-----|--------|-------------|
| `cite-as` | ARK | exactly 1 |
| `type` | `https://schema.org/Dataset`, `https://schema.org/AboutPage` | 2 |
| `describedby` | OAI `GetRecord` for `oai_datacite` and `oai_dc` (`type="application/xml"`); after Phase 3 the JSON-LD (`application/ld+json`) and DataCite JSON (`application/vnd.datacite.datacite+json`) representations; after Phase 4 Turtle (`text/turtle`) | 2 to 5 |
| `license` | SPDX URI | 0 or 1: emitted only when exactly one distinct URI exists in the graph; otherwise omitted (the profile allows at most one). JSON-LD still lists all licenses and Dublin Core emits one `DC.rights` per license |
| `author` | ORCID URLs of creators | 0 or more; the `DaSCH` fallback creator has no ORCID and produces no `author` link |

Each representation answers with `Link: <landing>; rel="describes"`. The
profile's `collection` and `item` relations describe content resources, not
metadata resources, so they are not used here. The link graph is symmetric
among the page and its representations and one-directional towards OAI-PMH:
the page links the OAI records, the OAI envelope is an established protocol
that carries no Signposting back, and that is left as it is. Signposting itself
is a community profile from 2020; the parts this plan depends on, RFC 8288
`Link` syntax and IANA-registered relations, are the stable ones.

`LinkSet` in `shared-fair` renders the header as a `String` (RFC 8288
syntax) from the graph plus a `UrlLayout` option carrying the landing URL, the
representation URLs and the OAI `GetRecord` URLs. `dpe-server` builds the
`HeaderValue` with `HeaderValue::from_str` and drops the header if that fails
rather than panicking; every target is a URI assembled from the graph's
canonical fields and the layout, so failure means a bug, not bad data. Never
place free text (names, titles) or the raw path segment in a header.

**Two base URLs, on purpose.** OAI `describedby` targets are built from
`oai_base_url`, because that is the URL the OAI endpoint itself advertises as
`baseURL` and it must stay truthful; on DEV it lives on a different host
(`https://api.dev.dasch.swiss/dpe/oai`, `config.rs:40-44`). Landing-page and
representation URLs are built from `DPE_PUBLIC_BASE_URL`. The two may differ,
and nothing derives one from the other. The operations page says so.

A note on Phase 2's `describedby` targets: the OAI `GetRecord` URLs return an
OAI-PMH envelope around the DataCite or Dublin Core payload. `application/xml`
is an accurate type for that document, and repositories commonly link it, but a
harvester wanting bare DataCite gets it only from the Phase 3 representations.
Phase 2 therefore claims embedded metadata plus Signposting structure, not the
full Level 1 value; Phase 3 completes it.

### Embedding JSON-LD in Maud

Maud escapes text inside `script {}`, so the JSON is spliced with `PreEscaped`.
That is trusted, server-generated content and becomes a sanctioned `PreEscaped`
site beside the Mosaic icon SVG (ADR-0005, *Consequences*). Before splicing,
every `<`, `>` and `&` in the serialised JSON is replaced with its `\u00XX`
escape. That is still valid JSON, and it neutralises both `</script>` and
`<!--`, the two sequences that move the HTML parser out of script-data state.
`serde_json::to_string` already escapes control characters and quotes. The
escaper lives in `shared-fair` (a `String` in, a `String` out) so every
consumer gets the same rule; the `html!` wrapper lives in `dpe-server`.

```rust
// shared-fair
pub fn script_safe_json(graph: &serde_json::Value) -> String {
    serde_json::to_string(graph)
        .expect("serde_json::Value always serialises")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

// dpe-server
fn json_ld_script(json: String) -> Markup {
    html! {
        script type="application/ld+json" { (PreEscaped(json)) }
    }
}
```

Dublin Core `<meta>` values and `<link href>` attributes go through Maud's
default escaping. A test in `dpe-server` greps its own sources so that
`PreEscaped(` occurs in `metadata.rs` exactly once, on the `script_safe_json`
splice; the escaping rule is then held by a test, not only by prose.

The existing rule text says the icon SVG is the *only* `PreEscaped` site. That
is already inaccurate: `modules/mosaic/tiles/src/components/form/textarea/mod.rs:142`
splices `PreEscaped("\n")` for a leading newline. The rewritten rule in
`modules/dpe/CLAUDE.md` and `ARCH-MAP.md` lists the sanctioned sites instead of
saying "only".

### Head extras slot

`view::head()` gains a `head_extras: HeadExtras` parameter, rendered last in
`<head>`, after `title { (title) }` and the scripts, and `view::page()` takes
it and passes it through. `HeadExtras(Markup)` is a one-line newtype in
`view.rs` whose only job is to make a swap with the same-typed `content:
Markup` parameter a compile error instead of a page whose JSON-LD lands in
`<main>`. Rendering last keeps the title and stylesheet at the top of the head
for parsers that read only its first kilobytes. Callers without extras pass
`HeadExtras(html! {})`. This touches the four
callers in `main.rs` (lines 38, 55, 87, 103) and the four `page()` tests in
`view.rs` (lines 72, 98, 106, 117). `maudfmt` skips a non-trivial `html!` block
nested as a call argument, so the head extras are bound to a local first
(`modules/dpe/CLAUDE.md`, *Formatting*).

### Accept handling on the landing page

The landing page is never rendered differently by header (ADR-0004); the one
negotiation step is the `303` ADR-0005 allows. A small q-value-aware decision
function in `shared-fair` (no new dependency): header string and the
consumer's candidate list in, `Html` or `Redirect(url)` out. Its rules are
written down, not implied: media types compare case-insensitively; parameters
other than `q` are ignored; an entry that does not parse, or whose `q` is not
a number in 0..=1, is skipped while the rest are still considered; HTML's
effective `q` is the maximum over every entry matching `text/html`, `text/*`
or `*/*`, with no specificity weighting; among several non-HTML candidates that
beat HTML at the same `q`, the one listed first in the `Accept` header wins; at
most 20 media ranges are read and a header over 2048 bytes counts as absent.
The candidate list is derived from the same `UrlLayout` that feeds the
`describedby` links, so the page can never advertise a representation it would
not redirect to. Datastar's fragment fetches send
`Accept: text/event-stream, text/html, application/json` and never match a
candidate, and they target the `/tab/{tab}` route, not the landing page, so no
special case is needed. The
candidate set DPE passes is `application/ld+json`,
`application/vnd.datacite.datacite+json` and, in Phase 4, `text/turtle`. A
supported non-HTML type wins only when its effective q is strictly greater than
the best q that covers HTML (`text/html`, `text/*` or `*/*`), and the handler
answers `303 See Other` to the matching representation URL. Browsers send
`text/html` at q=1 and `*/*` at q=0.8, so nothing changes for people. F-UJI
caches parsed bodies by final URL and content type, so a redirect to a distinct
URL with a distinct content type is a fresh parse for it.

This route never produces a 4xx from `Accept`. Decision table:

| `Accept` | Result |
|----------|--------|
| absent, `*/*`, `text/html,*/*;q=0.8` | 200 HTML |
| `application/ld+json` | 303 to `metadata.jsonld` |
| F-UJI's RDF list (`text/turtle, ..., application/ld+json`, no q on JSON-LD) | 303 to `metadata.jsonld` (Turtle wins once Phase 4 ships, since it is listed first at equal q) |
| `application/vnd.datacite.datacite+json` | 303 to `metadata.datacite.json` |
| `text/turtle` before Phase 4 | 200 HTML |
| `application/json`, `application/xml` | 200 HTML (not aliases; JSON-LD is only served for its own type) |
| `text/html;q=1, application/ld+json;q=1` | 200 HTML (tie) |
| `*/*;q=0`, malformed, unparseable q | 200 HTML |
| unknown shortcode, any `Accept` | 200 HTML "Project Not Found", never a redirect |

Every response from the landing page route, 200 or 303, GET or HEAD, carries
`Vary: Accept`. Traefik and browser caches must not replay a 303 to a browser or
HTML to a harvester.

Representation routes validate the shortcode with `is_valid_shortcode` (400) and
look the project up (404), mirroring `project_json_handler`
(`modules/dpe/server/src/fragments.rs:196-209`). Errors are `text/plain` with an
empty body, never HTML. The HTML page handler keeps its existing always-200
"Project Not Found" behaviour; that gap predates this work and is out of scope.
When the project does not resolve, the page emits no JSON-LD and no `Link`
header. The `cite-as` link is therefore only ever emitted on a page that
describes a real project.

Representation routes send no `Cache-Control` or `ETag` for now, consistent with
the other DPE routes; data changes only on deploy. Revisit when the standalone
JSON-LD for record-heavy projects shows up in egress metrics (see *Size and
abuse* for the measured size).

### Response headers precedent

The idiomatic shape in this codebase is the Axum tuple response, as at
`modules/dpe/api-oai/src/handlers/mod.rs:75` (one `CONTENT_TYPE` header there;
the snippet below is the shape to follow, widened to the headers this plan
adds). For the `303`, `axum::response::Redirect::to` already answers
`SEE_OTHER`; `Vary: Accept` is added by wrapping it in the same tuple shape.

```rust
(
    StatusCode::OK,
    [
        (header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8")),
        (header::LINK, link_header_value),
        (header::VARY, HeaderValue::from_static("Accept")),
    ],
    body,
)
```

Axum's `get` also serves `HEAD` with the body stripped, which is what the
Signposting profile requires.

### Public base URL

Signposting targets must be absolute. `DpeConfig` (`config.rs:18-53`) has no
public base URL; its one URL field, `oai_base_url`, is scoped to the OAI endpoint
and differs per environment in host, not just path. Add `DPE_PUBLIC_BASE_URL`
(default `https://repository.dasch.swiss`) next to it, validated at load:
scheme `http` or `https`, no path, no query, no trailing slash. A bad value
fails startup with a clear message, as `DPE_SITE_ADDR` does today
(`main.rs:337`); a silently accepted trailing slash would put a double slash in
every `Link` target and JSON-LD `@id` for the life of the process. Until ops-deploy sets it
per environment, a DEV page emits `describedby` links into production; that is a
human action with a gate before verification.

### Size and abuse

Projects can have thousands of records. The embedded block is bounded by the
`hasPart` cap of 100, which keeps it under 64 KB for every committed project;
the corpus-wide test pins that bound. The standalone JSON-LD representation is uncapped: for the
largest committed project (081C, 27,026 records, titles averaging 27
characters, ARKs 65) the `hasPart` array alone is about 4 MB per response,
built in memory with no I/O but served with no `Cache-Control` or `ETag`. That
is the number to remember, not a hypothetical. The representation routes
therefore sit behind the same per-IP `tower_governor` limiter the OAI route
already has (`router.rs:65-79`) from day one rather than after measurement, and
caching headers are revisited when egress, not latency, shows the route being
crawled.
The `Accept` decision function is bounded as well (at most 20 media ranges,
headers over 2048 bytes treated as absent). The OAI set is the escape hatch for
harvesters that want every record.

### Security

The controls are stated where they apply: escaping under *Embedding JSON-LD in
Maud*, header assembly and the canonical-fields rule under *Signposting link
set* and *JSON-LD content*, redirect targets, shortcode validation and error
paths under *Accept handling*, bounds and the limiter under *Size and abuse*,
base-URL validation under *Public base URL*. The security reviewer's pass runs
on every phase.

### Consistency across representations

Every writer, old and new, reads `ProjectGraph` or `RecordGraph`, and only
three functions in `shared-fair` take a contract type: `ProjectGraph::build`,
`RecordGraph::build` and `PartRef::from_record`, the last only to reach the
title helper the record builder uses. A corpus-wide test, in the shape of the
existing `every_committed_temporal_coverage_resolves` (`datacite.rs:458`),
asserts for every committed project that title, ARK, license URIs, creator
names and creator ORCIDs agree across JSON-LD, Dublin Core meta, the link set
and DataCite, and for a fixed sample of records per committed dump (the first
and last 100 of each) that title, ARK, creators and license agree across
DataCite and Dublin Core; and that every writer runs without panicking. The
deleted Phase 1 hash test covers every record once; the permanent test stays
bounded. Reviewers check that no fourth function in `shared-fair` takes
`&ProjectRaw` or `&Record`; ARCH-MAP records the rule as a boundary rule of the
`shared/fair` component, enforcement review.

Tests read `Link` headers through an RFC 8288 parser (a small test-support
function or a dev-dependency), not substring matching, so malformed quoting or
parameter escaping is caught.

### Agent-context layer

The dune layer (`ARCH-MAP.md`, `CONTEXT.md`, `modules/dpe/CONTEXT.md`,
`docs/adr/`) landed on 2026-09-16 and describes this repository to agents and
reviewers. ADR-0005 was amended alongside this plan on 2026-09-17 to name
`shared-fair` and the builder-per-capability rule; ADR-0002 was amended the
same day to rename the shared root from `platform/` to `shared/` and to let
that half of the move go first, and ADR-0003 follows the new prefix. Several
other lines are made stale by this plan and are updated in the phase that
changes the fact:

| Artefact | Line today | Changes in |
|----------|------------|------------|
| `ARCH-MAP.md` | no component for `shared/fair`, although every tracked file must map to exactly one entry; add the entry with paths, purpose, key entities (`ProjectGraph`, `RecordGraph`, `PartRef`, `ResolveContext`, `LinkSet`, `UrlLayout`, the writers), public interface, kit (≤7 files), depends on, used by, boundary rules (no service crate, no service path, no framework; only the two builders and `PartRef::from_record` take a contract type) | Phase 1 |
| `ARCH-MAP.md` DPE *Key entities*: `ContributorLookup / CachedContributorLookup` | `CachedContributorLookup` only (the trait is `shared-metadata`'s and joins its key entities), plus `resolve_inputs` (Phase 1) and `records_for_shortcode`, `HeadExtras`, `head_extras_for_project` (Phase 2) | Phase 1, Phase 2 |
| `ARCH-MAP.md` DPE *Used by*: committed-data tests live in `api-oai/src/metadata/datacite.rs` | the path where the hash and agreement tests land | Phase 1 |
| `ARCH-MAP.md` DPE *Depends on* and *Boundary rules*: "`dpe-api-oai` depends on `dpe-core` and `shared-metadata` only" | add `shared/fair`; `dpe-api-oai` depends on `dpe-core`, `shared-metadata` and `shared-fair` | Phase 1 |
| `ARCH-MAP.md` shared-metadata *Used by* | add `shared-fair` | Phase 1 |
| `ARCH-MAP.md` DPE *Boundary rules*: "The only sanctioned `PreEscaped` site is the Mosaic `IconData` SVG" | list the sanctioned sites | Phase 2 |
| `ARCH-MAP.md` DPE *Public interface* route list | add the representation URLs | Phase 3 |
| `ARCH-MAP.md` *Conventions*, FAIR landing pages: "docs-only until DEV-7268 lands, then static-analysis" | static-analysis, naming the tests and `check-shared-paths.sh` | Phase 3 |
| `ARCH-MAP.md` *Banned constructs*, "Rendering a landing page differently by `Accept`": "static-analysis (handler tests, once DEV-7268 lands)" | static-analysis, naming the tests | Phase 3 |
| root `CONTEXT.md` *Shared infrastructure* | add `shared-fair` beside `shared-telemetry`: the FAIR exposure engine, mechanism not domain, terms in `shared/README.md` | Phase 1 |
| root `CONTEXT.md` *Boundary rules*, ADR-0005 line: "docs-only until DEV-7268 lands" | static-analysis | Phase 3 |
| `shared/README.md` | tree and a `shared-fair` section in the shape of the `shared-metadata` one | Phase 1 |
| `docs/adr/0005-…md` *Enforced by*: "until it lands, none (docs-only)" | the tests, `check-shared-paths.sh` and the `just fair-check` review step, by name | Phase 3 |
| `ARCH-MAP.md` component paths `modules/platform/metadata`, `modules/platform/telemetry`; *Conventions* "Shared code lives under `modules/platform/` as `platform-{role}`"; *Banned constructs* "A relative path from a `platform-*` crate"; *Cross-cutting concerns* script and README names; every `platform-*` crate name | `shared/…`, `shared-{role}`, `shared-*`, `check-shared-paths.sh`, `shared/README.md` | Phase 0 |
| root `CONTEXT.md` *Shared infrastructure* and `## Shared` paths and crate names; *Flagged ambiguities* "platform" | `shared/…`, `shared-*`; the ambiguity entry records the rename and that "platform" now means DSP only | Phase 0 |
| `modules/dpe/CLAUDE.md`, `modules/editor/CLAUDE.md` and the `docs/src/` pages: `platform-*` mentions | `shared-*` | Phase 0 |
| `modules/dpe/CONTEXT.md` | no vocabulary change: it already defines *Landing page* and *Machine-readable representation*; this plan and the new documentation page use those terms | none |
| `modules/dpe/CLAUDE.md` *Escaping* and the stale "`main.rs` for all routes" pointer ARCH-MAP flags | rewrite the escaping rule; point at `router.rs` | Phase 2 |

The DPE local-context kit is at ARCH-MAP's cap of seven files and gains nothing
from this plan: `dpe-server/src/metadata.rs` is reached from `router.rs`, which
is in the kit, and swapping it in for `fragments.rs` would trade one route
family for another. `shared/fair` gets its own kit.

Vocabulary, per `modules/dpe/CONTEXT.md` and the root `CONTEXT.md`: this plan
says *landing page* for `/dpe/projects/{shortcode}`, *machine-readable
representation* for the JSON-LD, DataCite JSON and Turtle URLs, and qualifies
*Record* (the contract's `Record`, DPE's `OaiRecord`, the OAI-PMH record, the
`records/` dump) wherever the sense is not obvious. It avoids *content
negotiation*, which the context lists as a term to avoid for the `303` step.

### Where results are recorded

ADR-0005 says every assessment result is recorded with the assessor's version.
The 2026-09-15 baseline and every later local run land in an *Assessment
results* section of `docs/src/dpe/machine-readable-metadata.md`, one row per
run: date, target URL, assessor and version, score, link to the raw output if
kept. Results from DEV after merge (post-merge verification) are posted as a comment on DEV-7268
with the same fields and folded into the table on the next touch of that page.

## Implementation Phases

This plan is one PR, executed end to end by `/eng:workflows:work-orchestrate`
with no human input between the start of Phase 0 and the end of Phase 4. Each
phase is one or more commits inside that PR, in phase order; the PR body ticks
`allow-many-commits`, and the "Review Notes" section says the commits are to be
read phase by phase. Phase 0 is a mechanical move with no behaviour change and
every later phase assumes it; Phases 1 to 4 are each a complete, self-contained
change. A bug found in an earlier phase while working on a later one is folded
into the commit that introduced it, following `eng/references/git-history-recipes.md`,
never stacked as a `fix:` on top. Commit scopes: `shared-metadata` and
`shared-telemetry` (renamed in the scope table in Phase 0), `shared-fair` (new,
added in Phase 1), `dpe-core`, `dpe-api-oai`, `dpe-server`, `ci`, `docs`.

**Preconditions, checked once at intake and never mid-run.** The machine the
orchestration runs on has Docker with outbound access to `ghcr.io` (for
`just fair-check` and for reading F-UJI's source out of the pulled image),
outbound HTTPS to `raw.githubusercontent.com` (one fetch of the DataCite JSON
schema fixture in Phase 3, and `cargo` fetching `oxttl` if Phase 4 runs), the
Rust toolchain of `rust-toolchain.toml`, `just`, and `jq` (which the
`fair-check` recipe uses to read F-UJI's JSON; Phase 3 adds it to `flake.nix`'s
dev shell and to `just install-requirements`). There is no `Gate:` on any
phase: the human actions H1 to H3 all fall after the PR merges, and the
post-merge verification below is not part of the orchestrated run.

**Every phase ends with the complete reviewer set.** Each phase's last
checkbox runs `eng:reviewing` on that phase's diff with all of
`rust-reviewer`, `maud-datastar-reviewer`, `security-reviewer`,
`ivan-reviewer`, `performance-reviewer`, `devops-reviewer`, `dune-reviewer`,
`consistency-reviewer` and `code-simplicity-reviewer`. A reviewer with nothing
to say about a phase returns nothing; that is cheaper than deciding per phase
who is relevant and being wrong. Every finding is either folded into the
commit that introduced it before the next phase starts (all of those commits
belong to the current phase, so nothing later sits on top;
`eng/references/git-history-recipes.md` has the non-interactive recipe), never
stacked as a new `fix:` commit, or recorded as a follow-up in the PR body with
a reason. `work-orchestrate`'s own adversarial review of the
whole diff runs after Phase 4 in addition, not instead.

#### Phase 0: Move the shared root to `shared/`

The shared-root half of ADR-0002. No behaviour change; nothing in the moved
crates carries its own path, and the boundary is enforced by Cargo cycles and
the paths gate, neither of which depends on the layout, so this does not wait
for Bazel (ADR-0001). One or more commits. The first two checkboxes are one
worker chunk and one commit, scoped `chore(shared-metadata,shared-telemetry)`:
the tree does not build between the directory move and the import rename, so
the orchestrator must not commit between them. The rename is mechanical
(`git mv` plus `sed` over about 105 files) and stays within one worker's turn
budget. Every later checkbox may land as its own chunk and commit
(`chore(ci)`, `docs(docs)`).

- [ ] One chunk, one commit: `git mv modules/platform shared`; rename the `[package]` names to `shared-metadata` and `shared-telemetry`; update the workspace `Cargo.toml` members list (root-relative, a plain swap); update every dependent `Cargo.toml` (`dpe-*`, `editor-*`, the DPE fuzz crate) and every `use platform_metadata::` / `platform_telemetry::` path to the `shared_*` crate names. The dependency paths are relative and `shared/` sits one level shallower than `modules/platform/`, so each gains one `../`: `path = "../../platform/metadata"` in the nine service manifests becomes `"../../../shared/metadata"`, and the fuzz crate's three-up path becomes four-up; this is depth arithmetic, not a text substitution. `cargo build` plus a `cargo check` inside `modules/dpe/server/fuzz` (the fuzz crate sits outside the workspace) is the checklist
- [ ] Confirm `shared-metadata`'s and `shared-telemetry`'s `Cargo.toml` still take `serde_json` from the workspace (`serde_json = { workspace = true }`), which carries `preserve_order` (`Cargo.toml:43`)
- [ ] Rename `.github/scripts/check-platform-paths.sh` and its `.test.sh` to `check-shared-paths.sh` and `check-shared-paths.test.sh`; set the pathspec to `shared/*/src/*.rs`; keep the forbidden-path regex `(\.\./|modules/)(dpe|editor|mosaic)/` (a `../` escape from `shared/fair/src` into `modules/dpe/` still contains `modules/dpe/`); rename the internal identifiers `PLATFORM_PATHSPECS` and `non_platform_modules`/`non_platform_path_pattern` to `SHARED_*`/`non_shared_*` and reword the comments naming `modules/platform/README.md`; restructure the test's `make_repo()` so `editor/core` stays under `$dir/modules/` while `shared/metadata` and `shared/telemetry` are created at `$dir/shared/`, and change test 6's `git rm -rq modules/platform` to `git rm -rq shared`; update the three `justfile` touch points (the `check-platform-paths` recipe at lines 57-58, its entry in `check`'s dependency list at line 65, and the `test` recipe's comment and invocation at lines 133-134)
- [ ] Replace `modules/platform/**` with `shared/**` in the path filters of the six workflows that name it, and `modules/platform` with `shared` in both `bacon.toml` watch lists
- [ ] Rewrite `shared/README.md` (the moved `modules/platform/README.md`): tree, crate names, the rule's wording ("lands here as soon as a second area depends on it, named `shared-{role}`")
- [ ] Update `ARCH-MAP.md`: the two component entries' paths and purposes, the *Conventions* bullet "Shared code lives under `modules/platform/` as `platform-{role}`", the *Banned constructs* row on paths from a `platform-*` crate, the *Cross-cutting concerns* script and README names, and every `platform-*` crate name
- [ ] Update the root `CONTEXT.md`: *Shared infrastructure* and `## Shared` paths and names, and the *Flagged ambiguities* "platform" entry, which now records the rename (2026-09-17, Bazel's platform vocabulary) and that "platform" means the DaSCH Service Platform only
- [ ] Update `platform-*` mentions in `modules/dpe/CLAUDE.md`, `modules/editor/CLAUDE.md` and the developer documentation under `docs/src/` (16 files; `repo_structure.md` tree, crate table and *Shared Crates* section first); the `CONTEXT.md` files and ADR-0003 carry none
- [ ] Rename the scopes in `CONVENTIONS.md`'s table to `shared-metadata` and `shared-telemetry`, and update the three `REVIEW.md` bullets that name `platform-metadata` (lines 15, 16 and 40; one embeds the old recipe name `just check-platform-paths`)
- [ ] Update ADR-0002's own consequence line and *Enforced by* to name `check-shared-paths.sh`
- [ ] Grep the tree, case-insensitively, for `platform_metadata`, `platform-metadata`, `platform-telemetry`, `modules/platform`, `PLATFORM_PATHSPECS` and `non_platform` outside `CHANGELOG.md` and `.git/`; the result is empty
- [ ] Run `just check`, `just test` and `.github/scripts/check-shared-paths.test.sh`
- [ ] Run `eng:reviewing` on this phase's diff with the complete reviewer set; fold every finding into the commit that introduced it before Phase 1 starts, never a new `fix:` commit, or record it as a follow-up in the PR body with a reason

#### Phase 1: Extract `shared-fair` and resolve once

Move plus re-plumb. No behaviour change: the OAI XML output for every committed
project and every committed record must be byte-identical before and after.
The checkboxes are ordered so that the tree builds and the tests pass after
each one, except where a checkbox says "one chunk, one commit": those bundle
steps between which the tree does not build, and the orchestrator commits them
together. The "unit tests" checkboxes list several assertions but are each one
chunk; the bullet density is not a size estimate.

- [ ] Capture the baseline before any move in this phase: a test in `dpe-api-oai` renders `GetRecord` for `oai_datacite` and `oai_dc` for every committed project and every record in the committed record dumps, hashes each payload, and writes the per-identifier hash list to a committed fixture file; commit the fixture
- [ ] Create `shared/fair` (`shared-fair`) with `Cargo.toml` depending on `shared-metadata`, `serde` and `serde_json` from the workspace (`{ workspace = true }`, which carries `preserve_order`) and nothing else, an empty `lib.rs`; add it to the workspace `Cargo.toml` members and to the fixture list in `check-shared-paths.test.sh`
- [ ] Move `ContributorLookup` from `dpe-core/src/contributors.rs:32-35` to `shared/metadata/src/contributors.rs` and export it from the crate root; update `CachedContributorLookup`'s `impl` block in `dpe-core` and every `use dpe_core::ContributorLookup` in `dpe-core`, `dpe-api-oai` and its `test_utils.rs` to the `shared_metadata` path (`grep -rn ContributorLookup modules` is the list)
- [ ] Move `parse_url_value` from `dpe-core/src/project.rs:31` to `shared-metadata/src/utils.rs` as the `url` reading rule, with its tests; `dpe-core`'s `From<ProjectRaw> for Project` calls it there
- [ ] Keep the parsed `ProjectRaw` in `dpe-core`'s project cache beside the view model, add `get_all_raw(&self) -> &[ProjectRaw]` and `get_raw_by_shortcode(&self, shortcode: &str) -> Option<&ProjectRaw>` to `ProjectRepository` (`project_repository.rs:4-7`) and implement them on `FsProjectRepository`; change `InMemoryProjectRepository` in `modules/dpe/api-oai/src/handlers/test_utils.rs:101-119` to take `Vec<ProjectRaw>` and derive the view model from it, and update its fixture builders (`incunabula_project()`, `project_with_shortcode()`) to build `ProjectRaw`; `cargo test -p dpe-api-oai` is the checklist
- [ ] One chunk, one commit: move `modules/dpe/api-oai/src/metadata/{datacite.rs,record_datacite.rs,dublin_core.rs,record_dublin_core.rs,helpers.rs,resolve.rs}` and the `DataCiteRecord` / `DublinCoreRecord` half of `types.rs` into `shared-fair/src/` with a public module surface; in the same chunk, add `ResolveContext<'a> { lookup: &'a dyn ContributorLookup, periods: &'a HashMap<String, W3cdtfRange>, enriched: &'a HashMap<String, EnrichedDate> }` to `shared-fair/src/graph.rs`, switch the four moved mappings from `&dpe_core::Project` to `&ProjectRaw` (they read no field the view model transforms) and `resolve_temporal_coverage` (`datacite.rs:283-288`) from `dpe-core`'s globals to the context's tables; point `dpe-api-oai`'s `metadata/mod.rs` at `shared_fair::*`, building the context at the call site from `CachedContributorLookup`, `chronontology_cache::all_periods()` and `temporal_enrichment_cache::all_enriched()` and passing `ProjectRaw` from the raw accessor; delete the moved code; add `shared-fair` to `dpe-api-oai`'s and `dpe-server`'s dependencies (`dpe-core`'s do not change). `OaiRecord`, `OaiRecordHeader` and the OAI functions in `metadata/mod.rs` stay in `dpe-api-oai`. This chunk's acceptance test is the hash comparison test against the committed fixture, which must pass with the move done and the mappings still reading `ProjectRaw` directly (hash checkpoint 1)
- [ ] Add `RecordGraph` and `RecordGraph::build(&Record) -> RecordGraph` to `graph.rs`, carrying ARK, project ARK, titles, creators with inferred kind, description, dates, general data type, license, access rights, keywords, publisher, citation text and size (no file pointer); the `typeOfData` match lives here once; in the same chunk add the record title helper and `PartRef { ark: String, title: String }` with `PartRef::from_record(&Record) -> PartRef`, which takes only the ARK and the preferred title through that helper
- [ ] Add `ProjectGraph` and `ProjectGraph::build(&ProjectRaw, &ResolveContext, &[Record]) -> ProjectGraph` to `graph.rs`, carrying the resolved facts in the table above, built with the moved helpers; `parts: Vec<PartRef>` is populated by calling `PartRef::from_record` per record of the shortcode, never by inline title logic
- [ ] Add `dpe_core::resolve_inputs()` returning the cached lookup and the two temporal tables as `&'static` references, and switch `dpe-api-oai`'s `metadata/mod.rs` call site to `ResolveContext::new(lookup, periods, enriched)` over it; `dpe-server`'s `metadata.rs` (Phase 2) calls `resolve_inputs()` from the start, so no later switch is needed
- [ ] One chunk: re-plumb `project_to_datacite` and `project_to_dublin_core` to take `&ProjectGraph`, keeping DataCite and Dublin Core vocabulary in the writers, and update their callers in `dpe-api-oai` to build the graph first
- [ ] One chunk: re-plumb `record_to_datacite` and `record_to_dublin_core` to take `&RecordGraph`, update their callers, and delete the duplicated `typeOfData` and name-kind helpers. This chunk's acceptance test is the hash comparison test again, which must pass with all four mappings reading the graphs (hash checkpoint 2)
- [ ] Delete the hash fixture and its test, their job done. Byte-for-byte pinning of the OAI output is a migration check for this phase only: from Phase 2 on, the corpus-wide agreement test guards that the representations agree, not their exact bytes, and the OAI XML keeps the unit and snapshot tests it has today
- [ ] Unit tests in `shared-fair` (one chunk): `ProjectGraph::build` applies the ARK fallback, the `DaSCH` creator fallback, placeholder filtering, English-first titles and the `url` reading rule; `RecordGraph::build` infers `DaSCH` as an organisation and other authorship as persons, and maps `typeOfData` once; `PartRef::from_record` yields the same title as `RecordGraph::build` for the same record; a `ResolveContext` over ad-hoc fixture maps builds without `'static` data
- [ ] Add the `shared/fair` component entry to `ARCH-MAP.md` (paths, purpose, key entities, public interface, local-context kit of at most seven files, depends on, used by, boundary rules including "only `ProjectGraph::build`, `RecordGraph::build` and `PartRef::from_record` take a contract type", enforcement review); update the `modules/dpe` entry (key entities: drop `ContributorLookup`, add `resolve_inputs`; *Used by* test path; *Depends on*; the `dpe-api-oai` boundary rule) and the `shared/metadata` entry (`ContributorLookup` under key entities, `shared-fair` under *Used by*)
- [ ] Add `shared-fair` to the *Shared infrastructure* list of the root `CONTEXT.md`, in the shape of the `shared-telemetry` entry
- [ ] Add a `shared-fair` section and tree entry to `shared/README.md`, in the shape of the `shared-metadata` section: contents, what stays behind in DPE and why, dependencies
- [ ] Update `docs/src/repo_structure.md` (tree and crate table, line 43's "only") and `docs/src/dpe/project_structure.md` (lines 32-34, 67, 96-98: dependency graph and the "API crates depend on … only" rule) for the new crate and the moved trait
- [ ] Update `docs/src/dpe/architecture.md:29` so `dpe-api-oai` depends on `shared-metadata`, `dpe-core` and `shared-fair`
- [ ] Add `shared-fair` to the commit-scope table in `CONVENTIONS.md`
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with the complete reviewer set; fold every finding into the commit that introduced it before Phase 2 starts, never a new `fix:` commit, or record it as a follow-up in the PR body with a reason

#### Phase 2: Embedded metadata and Signposting on the landing page

- [ ] Add `DPE_PUBLIC_BASE_URL` to `DpeConfig` in `modules/dpe/server/src/config.rs` with default `https://repository.dasch.swiss`, validated at config load (scheme `http` or `https`, no path, no query, no trailing slash; a bad value fails startup with a clear message, as `DPE_SITE_ADDR` does at `main.rs:337`), thread it through `AppState` (not a process-global), and document it in `docs/src/dpe/operations.md`: that it is independent of `DPE_OAI_BASE_URL` and why, and that a local run assessed from a container must set it to a host the container can reach (`http://host.docker.internal:4000`)
- [ ] Add `records_for_shortcode(shortcode: &str) -> &'static [Record]` to `dpe-core/src/record_cache.rs`, grouping `all_records()` by upper-cased shortcode once at cache build in the shape of `project_cache.rs`'s `SHORTCODE_INDEX`; point the OAI set filter in `modules/dpe/api-oai/src/handlers/mod.rs:333` (`collect_filtered_records`, today a scan of `record_repo.get_all()` per `project:{shortcode}` request) at the same index so the corpus has one shortcode lookup; a unit test shows the largest committed dump (081C) is served from the index without a scan of the other dumps
- [ ] Add `UrlLayout` to `shared-fair/src/signposting.rs`: the consumer's URL layout (landing URL, the representation URLs present so far as `(media type, URL)` pairs, the OAI `GetRecord` URLs for `oai_datacite` and `oai_dc`) and, declared beside it, `Candidate { media_type: String, url: String }` with `UrlLayout::candidates() -> Vec<Candidate>` deriving the `Accept` candidate list from the same pairs (the type exists from this phase so the workspace builds; Phase 3's decision function consumes it), consumed by the JSON-LD writer, the link set and the decision function alike
- [ ] Add `shared-fair/src/schema_org.rs`: `to_schema_org(&ProjectGraph, &UrlLayout, opts) -> serde_json::Value` implementing the property table above, with `opts.has_part_cap: Option<usize>`; keys inserted in emission order, no `Map::remove`; plus `script_safe_json(&Value) -> String` applying the `<`, `>`, `&` unicode escapes
- [ ] Add `shared-fair/src/dublin_core_meta.rs`: a `<meta name="DC.*">` writer (`DC.title`, `DC.creator`, `DC.identifier`, `DC.rights` one per license, `DC.accessRights` COAR URI, `DC.type`, `DC.date`, `DC.description` truncated at 1000 characters, `DC.publisher`, `DC.language`) as a list of name/content pairs over the `DublinCoreRecord` that `project_to_dublin_core(&graph)` produces, with `DC.accessRights` taken from the graph
- [ ] Add `shared-fair/src/signposting.rs`: a `LinkSet` type (rel, href, optional media type) with `to_header_string() -> String` in RFC 8288 syntax and an iterator for rendering `<link>` elements, plus `project_link_set(&ProjectGraph, &UrlLayout)` implementing the cardinality rules above
- [ ] Add `pub struct HeadExtras(pub Markup)` and a `head_extras: HeadExtras` parameter to `view::page()` and `head()` in `modules/dpe/server/src/view.rs`, rendered last in `<head>` after `<title>` and the scripts; update the four callers in `main.rs` (`HeadExtras(html! {})`) and the four tests in `view.rs`, plus one test that the extras render after the title
- [ ] Add `modules/dpe/server/src/metadata.rs` with `head_extras_for_project(shortcode, &AppState) -> Option<(Markup, HeaderMap)>` that builds the graph from the raw project, the cached lookup and the records, renders the JSON-LD script (`script_safe_json` spliced with `PreEscaped`), the DC meta tags and the `<link>` elements, and turns the link set's header string into a `HeaderValue`, dropping it with a `tracing::warn!` naming the shortcode if `from_str` fails (a persistent failure is a data-quality bug that must surface in logs)
- [ ] Wire `project_page_handler` (`main.rs:66-89`) to emit the head extras and the `Link` header when the project resolves, and nothing when it does not; its return type changes from `Html<String>` to `axum::response::Response` so the headers now and the `303` branch in Phase 3 fit without a second restructuring
- [ ] Add a small hand-rolled `Link` header parser to `dpe-server`'s test support, scoped to what Signposting emits (comma-separated `<url>; rel="x"; type="y"` entries, quoted parameters, no relative-URI resolution, no RFC 8187 encoding), so header tests assert on parsed relations, not substrings; no new dependency
- [ ] Confirm in F-UJI's `fuji_server/helper/metadata_mapper.py`, read out of the pulled `ghcr.io/pangaea-data-publisher/fuji` image at `/usr/src/app/fuji_server/helper/metadata_mapper.py` (`docker run --rm --entrypoint cat <image> <path>`), that `identifier` as a `PropertyValue` (and the `@id` fallback) is read as the object identifier; if it needs a plain string, emit `identifier` as an array of the PropertyValue and the ARK string
- [ ] Unit tests in `shared-fair`: JSON-LD for a fixture project has `@type: Dataset`, ARK `@id`, `url` equal to the landing page, license URI, creator with ORCID `identifier`, `producer` of type `ResearchProject` carrying the external website; placeholders are absent; multiple licenses produce an array; a project with no creator attribution gets the `DaSCH` Organization creator; no `distribution` key is emitted for any fixture (ADR-0005, *Nothing is invented for a score*)
- [ ] Unit test in `dpe-server`: `DpeConfig` rejects a `DPE_PUBLIC_BASE_URL` with a trailing slash, a path, a query or a scheme other than `http`/`https`, and accepts the default
- [ ] Unit tests: one fixture per `AccessRightsType` variant asserting `isAccessibleForFree`, `conditionsOfAccess` (including the embargo date) and the COAR `DC.accessRights` URI from the access-rights table
- [ ] Unit test: a project whose description has only non-English keys yields the same text in JSON-LD, Dublin Core meta and DataCite; `DC.description` for a description over 1000 characters is truncated on a character boundary and still escapes correctly
- [ ] Unit test: `hasPart` is capped at 100 entries when `has_part_cap` is set and uncapped otherwise; the serialised JSON-LD's top-level keys appear in the builder's insertion order (`@context`, `@type`, `@id`, `name`, …), which is what `preserve_order` guarantees and a stray `Map::remove` would silently break
- [ ] Corpus-wide test in `dpe-api-oai` (shape of `every_committed_temporal_coverage_resolves`): for every committed project, every writer runs without panicking, and title, ARK, license URIs, creator names and creator ORCIDs agree across JSON-LD, Dublin Core meta, the link set and DataCite; for a fixed sample of records per committed dump (the first and last 100 of each, so the permanent test stays bounded while the deleted Phase 1 hash test covered every record once), title, ARK, creators and license agree across DataCite and Dublin Core; the embedded JSON-LD for the committed project with the most records stays under 64 KB
- [ ] Unit test: `script_safe_json` on a graph containing `</script><script>alert(1)</script>` and `<!--` in a description yields JSON in which `<`, `>`, `&` appear only as `\u00XX`, and the rendered page has a single `<script>` element
- [ ] Unit test: link set emits `license` only when exactly one distinct URI exists; `cite-as` always exactly once; the `DaSCH` fallback creator produces no `author` link; a header string that fails `HeaderValue::from_str` is dropped in `dpe-server`
- [ ] Handler tests in `dpe-server` (oneshot, following `fragments.rs` tests): GET landing page contains one `application/ld+json` script and the DC meta tags; the parsed `Link` header has `cite-as`, `type`, `describedby` with `type` attributes; the set of `<link rel href type>` elements in the head equals the parsed header set, relation for relation; unknown shortcode has none of them
- [ ] Handler test: HEAD and GET for the same shortcode return byte-identical headers (`Link`, `Content-Type`) and HEAD has an empty body
- [ ] Handler test: `/dpe/projects/080c` and `/dpe/projects/080C` (or the committed mixed-case shortcode) return identical JSON-LD `@id` and `Link` header values
- [ ] Handler test: a shortcode containing `%0D%0A`, `"`, `;` or `>` returns the current 200 "Project Not Found" body with no `Link` header and no JSON-LD, and does not panic
- [ ] Rewrite the *Escaping* rule in `modules/dpe/CLAUDE.md` to list the sanctioned `PreEscaped` sites (Mosaic `IconData` SVG, the textarea leading newline, the JSON-LD script in `dpe-server/src/metadata.rs`, whose only permitted input is the output of `script_safe_json`; a test in `dpe-server` greps its sources so that `PreEscaped(` appears in `metadata.rs` exactly once and on that call), and fix the stale "`main.rs` for all routes" pointer to name `router.rs`
- [ ] Rewrite the `PreEscaped` boundary rule in `ARCH-MAP.md`'s `modules/dpe` entry the same way; add `records_for_shortcode`, `HeadExtras` and `head_extras_for_project` to the DPE key entities and `LinkSet`, `UrlLayout` and the writers to the `shared/fair` entry's key entities
- [ ] Write `docs/src/dpe/machine-readable-metadata.md` covering the embedded metadata and Signposting, using the `modules/dpe/CONTEXT.md` terms *landing page* and *machine-readable representation*, disambiguating schema.org `producer` from the OAIS Producer in one line, qualifying *Record* where used, and opening the *Assessment results* table with the 2026-09-15 baseline rows (F-UJI 3.5.0, FAIR Champion 1.1.11); add the page to `docs/src/SUMMARY.md`
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with the complete reviewer set; fold every finding into the commit that introduced it before Phase 3 starts, never a new `fix:` commit, or record it as a follow-up in the PR body with a reason

#### Phase 3: Machine-readable representations and the `303` step

- [ ] Add `shared-fair/src/datacite_json.rs`: `to_datacite_json(&DataCiteRecord) -> serde_json::Value` in the DataCite kernel 4 JSON shape (`identifiers[]` with `identifierType: "ARK"`, `types`, `creators[].nameIdentifiers[]`, `titles`, `publisher`, `publicationYear`, `subjects`, `contributors`, `descriptions`, `dates`, `language`, `rightsList`, `geoLocations`, `fundingReferences`, `relatedIdentifiers`, `schemaVersion`); the `DataCiteRecord` is itself written from the graph, so the writer takes it as input
- [ ] Add `shared-fair/src/negotiate.rs`: `decide(accept: Option<&str>, candidates: &[Candidate]) -> Decision` over the `Candidate` type `UrlLayout::candidates()` already returns (never hand-written at a call site) and `Decision::{Html, Redirect(url)}`, implementing the decision table above with these written rules: among several candidates that beat HTML at the same `q`, the one listed first in the `Accept` header wins; media types compare case-insensitively; parameters other than `q` are ignored; an entry whose `q` is missing counts as 1, one whose `q` is non-numeric or outside 0..=1 is skipped, and an entry that does not parse is skipped while the rest are still considered; HTML's effective `q` is the maximum over every entry matching `text/html`, `text/*` or `*/*`, without specificity weighting; at most 20 media ranges are considered and a header longer than 2048 bytes is treated as absent, so a hostile header costs bounded work and always yields `Html`
- [ ] Add routes `GET /dpe/projects/{shortcode}/metadata.jsonld` (`application/ld+json`, uncapped `hasPart`) and `GET /dpe/projects/{shortcode}/metadata.datacite.json` (`application/vnd.datacite.datacite+json`) in `metadata.rs`, registered in `router.rs` behind the same per-IP `tower_governor` layer the OAI router uses (`router.rs:65-79`, same `DPE_OAI_RATE_LIMIT_*` settings) from day one, because the JSON-LD representation is uncapped and public; each body is `serde_json::to_string` output returned in the tuple shape with an explicit `Content-Type` (`axum::Json` would hardcode `application/json`); 400 on malformed shortcode, 404 on unknown project, both `text/plain`
- [ ] Each representation response carries `Link: <landing>; rel="describes"` pointing at the canonical landing page URL
- [ ] Extend `UrlLayout` and the landing page link set with `describedby` entries for both representations, typed with their media types
- [ ] Wire the `303 See Other` from `project_page_handler` when the decision function picks a representation; add `Vary: Accept` to every landing page response (200 and 303, GET and HEAD)
- [ ] Unit tests for the decision function: one per row of the decision table, including malformed headers and `*/*;q=0`; one bad entry beside a valid `application/ld+json` entry still redirects; `text/html;q=0.5, text/*;q=0.9` gives HTML an effective `q` of 0.9; `text/html;level=1;q=0.7` parses; `application/vnd.datacite.datacite+json, application/ld+json` at equal `q` redirects to the DataCite JSON representation because it is listed first; a header with thousands of media ranges and one over 2048 bytes both yield `Html` without panicking
- [ ] Handler tests: both representations return the right `Content-Type`, `describes` link and body shape; requests past the per-IP burst answer 429 as the OAI route does; 400 and 404 paths are `text/plain`; landing page with `Accept: application/ld+json` returns 303 with the JSON-LD `Location` built from the canonical shortcode; with a browser `Accept` returns 200 HTML; `Vary: Accept` present on 200, 303 and HEAD
- [ ] Handler test matrix: unknown shortcode × {no `Accept`, `application/ld+json`, `text/html`} returns 200 HTML with no `Link` header and no JSON-LD in every cell
- [ ] Add DataCite's JSON schema as a shape-check fixture under `shared-fair/testdata/schemas/`: DataCite publishes JSON schemas only for kernel 4.2 and 4.3 (`datacite/schema`, `source/json/kernel-4.3/datacite_4.3_schema.json`, Invenio-derived; the official kernel is XSD-only), so fetch that file pinned to a `datacite/schema` commit with a `download-schemas.sh` header in the shape of `modules/dpe/api-oai/src/handlers/testdata/schemas/download-schemas.sh`, and validate the DataCite JSON writer's output for every committed project against it from the corpus-wide test in DPE. Kernel 4.6 additions over 4.3 are optional properties, so the 4.3 schema checks shape, not version; F-UJI's own parse in `just fair-check` is the acceptance for the 4.6 output
- [ ] Unit test: DataCite JSON output for a fixture project has `identifiers[0].identifierType == "ARK"` and the same title, creators and rights as the XML
- [ ] Check F-UJI's metadata merge order in `fuji_server/helper/metadata_mapper.py` (the static mapping table) and `fuji_server/harvester/metadata_harvester.py` (`merge_metadata()`), read out of the pulled image at `/usr/src/app/fuji_server/…` with `docker run --rm --entrypoint cat <image> <path>`; if the DataCite JSON `Project` type can override the JSON-LD `Dataset` type in F-UJI's merged view, keep the `describedby` link anyway (a true link is not hidden for one assessor's merge order, the mirror of "nothing is invented for a score"), record the resource-type regression as a known residual in the *Assessment results* table and in `docs/src/dpe/machine-readable-metadata.md`, and note any gap against the Success Metrics targets as a follow-up in the PR body; the plan file itself is not edited
- [ ] Amend `docs/src/dpe/architecture.md:72` to say that the landing page's `303` on `Accept` is the one exception, decided in ADR-0005, rather than inventing new carve-out language
- [ ] Update `docs/src/dpe/oai-pmh.md` to reconcile its "no content negotiation" statement for the file endpoint with the new representations (the file endpoint is unchanged)
- [ ] Update `docs/src/dpe/json-api.md` with a pointer to the representations and a note that they are standards-shaped, unlike `/dpe/api/v2`
- [ ] Extend `docs/src/dpe/machine-readable-metadata.md` with the representation URLs, media types and the `303` rule
- [ ] Add `just fair-check <url>`: starts the F-UJI container from an image pinned by digest (`ghcr.io/pangaea-data-publisher/fuji@sha256:…`, the digest of the current release recorded in the recipe and in the *Assessment results* table next to `software_version`; bumping it is a reviewed change made when F-UJI publishes a release and checked at least with every change to a landing page, as for the Tailwind CLI pin in `docs/src/security.md`), publishing port 1071 on `127.0.0.1` only, with `--rm` and `--add-host=host.docker.internal:host-gateway` so it also works on Linux, and a `trap` that stops the container on any exit; waits for the port for at most 60 seconds; POSTs `{"object_identifier": "<url>", "test_debug": true, "use_datacite": true}` to `http://localhost:1071/fuji/api/v1/evaluate` with F-UJI's default basic-auth user (`marvel` / `wonderwoman`, from `fuji_server/config/users.py`) and `curl --max-time 600`; prints the per-metric score table plus `software_version` and the image digest with `jq`; takes an optional second argument `min_score` (default 0) and exits non-zero when the total score is below it, so the orchestrator gets a mechanical pass/fail; add `jq` to `flake.nix`'s dev shell and `just install-requirements`; document the recipe in `docs/src/dpe/machine-readable-metadata.md`, including that the container can reach the host's port 4000 during the run, that F-UJI's default credentials are its own published defaults for a loopback-only container, and that the digest pin has no `tailwind.pins`-style refresh recipe because `fair-check` never runs in CI
- [ ] Start `DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000 just dev` in the background, poll `http://localhost:4000/dpe/projects/0862` until it answers 200 (at most 120 seconds), run `just fair-check http://host.docker.internal:4000/dpe/projects/0862 12`, and stop the dev server whatever the outcome; the recipe's exit code is the check against the Success Metrics target of 12; add the run, with `software_version` and the image digest, as a row in the *Assessment results* table
- [ ] Add a `REVIEW.md` checklist line: a change to a landing page or to `shared-fair` runs `just fair-check` against project 0862 and records the result before it is called done
- [ ] Add the representation URLs to the `modules/dpe` *Public interface* route list in `ARCH-MAP.md`
- [ ] Flip the enforcement level in the four places that defer to DEV-7268: `ARCH-MAP.md` *Conventions* (FAIR landing pages, "docs-only until DEV-7268 lands"), `ARCH-MAP.md` *Banned constructs* (rendering a landing page differently by `Accept`, "once DEV-7268 lands"), root `CONTEXT.md` *Boundary rules* ("docs-only until DEV-7268 lands"), and the *Enforced by* line of `docs/adr/0005-fair-landing-pages-in-the-access-area.md`. Attribute each mechanism to its clause: the corpus-wide agreement test for one graph feeding every representation, the `Link` / `Vary` / `303` handler tests for the headers and the negotiation step, `check-shared-paths.sh` for `shared-fair` holding no path into an area, and the `REVIEW.md` `fair-check` step for FAIRness being measured
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with the complete reviewer set; fold every finding into the commit that introduced it before Phase 4 starts, never a new `fix:` commit, or record it as a follow-up in the PR body with a reason

#### Phase 4: Turtle representation

Skip this phase when the Phase 3 local `just fair-check` run recorded in the
*Assessment results* table shows F-UJI I1 at 2/2, I2 at 1/1 and I3 at 1/1 for
project 0862; JSON-LD is RDF and F-UJI accepts it. Otherwise execute it. The
decision is read from that recorded result and needs no one's input; the
orchestrator reads the row before decomposing this phase.

**Decision (2026-09-18): skipped, and not on the condition above, which is not
met.** The Phase 3 run scored I1 2/2 and I3 1/1 but **I2-01M 0/1**, so the
literal skip condition fails. Turtle is skipped nevertheless, on evidence the
condition did not anticipate: two independent reads of the pinned F-UJI 3.5.0
image show I2 cannot be earned by any serialisation of this graph. F-UJI's
default-namespace list excludes schema.org and both Dublin Core namespaces —
exactly what this page emits — and strips them before either I2 sub-test runs;
and the sub-test that checks namespace availability adds its own status to the
score while that status is still false, so it earns zero whatever it found.
Both obstacles are about *which* vocabulary namespaces appear, not about how
they are serialised, and a Turtle rendering of the same graph carries the same
namespaces. Moving I2 needs a controlled-vocabulary link F-UJI's registry
recognises, which is new scope. The phase would therefore add a hand-rolled
Turtle writer, or an `oxrdf`/`oxttl` dependency, for a measured gain of zero,
against a total that already exceeds its target at 14/24. The checkboxes below
are ticked as skipped on that reason, **not** on the suggested wording "skipped,
Phase 3 passed I1 to I3", which would be false. Full evidence in the journal's
*Phase 4 decision* section.

- [x] *(skipped)* Add `shared-fair/src/turtle.rs`: a small Turtle writer over the same graph the JSON-LD builder produces (subjects are the ARK and blank nodes; predicates from schema.org and Dublin Core Terms); if the hand-rolled writer exceeds roughly 200 lines, switch to `oxrdf` plus `oxttl` (the Oxigraph crates, pure Rust) as regular dependencies of `shared-fair` instead
- [x] *(skipped)* Add route `GET /dpe/projects/{shortcode}/metadata.ttl` (`text/turtle`) with the same 400/404 and `describes` behaviour
- [x] *(skipped)* Add the Turtle representation to DPE's `UrlLayout`, which extends both the `describedby` set and the derived candidate list at once; unit test: F-UJI's RDF `Accept` list (`text/turtle` before `application/ld+json`, equal `q`) now redirects to Turtle
- [x] *(skipped)* Unit test: Turtle output for a fixture parses with `oxttl` (already a regular dependency if the writer switched to `oxrdf` plus `oxttl`; otherwise added here as a dev-dependency of `shared-fair` for this test alone) and yields the same triples the JSON-LD carries for ARK, title, license and creators
- [x] *(skipped)* Handler test: `Accept: text/turtle` on the landing page redirects to the Turtle representation
- [x] *(skipped)* Update `docs/src/dpe/machine-readable-metadata.md` and the `ARCH-MAP.md` route list
- [x] *(skipped)* Run `just check` and `just test`
- [x] *(skipped)* Run `eng:reviewing` on this phase's diff with the complete reviewer set; fold every finding into the commit that introduced it, never a new `fix:` commit, or record it as a follow-up in the PR body with a reason

#### Phase 5: Fixes found by the first live assessment

Added 2026-09-18, after Phases 0 to 4 had landed and the branch was pushed. The
first assessment against a *deployed* page — the Cloud Run PR preview for #391,
not the local run of Phase 3 — scored F-UJI 3.5.0 at **13 of 24** and FAIR
Champion 1.1.11 at **7 of 15** passing, and surfaced three things the local run
could not: two defects in what the JSON-LD asserts, and one deployment gap.
Each is a fix, not an investigation. The evidence, including the assessor log
lines that prove each one, is in the journal under *Round 11*.

**This phase starts from `6f53d3c7`, which is already pushed.** Every commit
here is a *new commit on top*; nothing at or below that SHA may be amended or
rebased. That is the one way this phase differs from Phases 0 to 4.

- [ ] Emit each schema.org `license` as an IRI node (`{"@id": "…"}`) instead of a bare string, keeping the cardinality rule: one object for a single licence, an array for several, the key omitted when the graph carries none. FAIR Champion's *LicenseStrong* reads only Resources, and schema.org's remote context does not coerce `license` to `@id`, so a string parses as a literal and the test fails with "Found the Schema license predicate, but it does not have a Resource as its value". `shared/fair/src/schema_org.rs`, the `insert_list(&mut root, "license", graph.license_uris())` call
- [ ] Update the two unit tests in `schema_org.rs` that pin the old shape (the single-licence assertion on `doc["license"]`, and `several_licenses_become_an_array`), and the licence comparison in the corpus-wide agreement test in `modules/dpe/api-oai/src/metadata/corpus.rs`, which must now read `@id` out of the JSON-LD side
- [ ] Add the canonical landing page URL to schema.org `identifier`, alongside the existing ARK `PropertyValue`, so the property becomes an array. FAIR Champion's *MetadataIdentifierFound* checks `schema:identifier` only — it does not consider `url` — so with the ARK alone it can only match an assessor pointed at the ARK itself. schema.org's `identifier` accepts `Text | URL | PropertyValue`, and F-UJI reads the `PropertyValue` form, so keep the ARK exactly as it is. Take the URL from the `UrlLayout` the writer already holds; never reconstruct it
- [ ] Update the unit test asserting `doc["identifier"]["propertyID"] == "ARK"` and any corpus assertion that indexes `identifier` as a single object
- [ ] Point the Cloud Run PR preview at its own URL: in `.github/workflows/cloud-run-dpe-pull-request.yml`, after the deploy step and before the comment steps, run `gcloud run services update "$SERVICE_NAME"` with `--region` and `--update-env-vars="DPE_PUBLIC_BASE_URL=<the deploy step's url output>"`. Without it the preview falls back to the compiled default of `https://repository.dasch.swiss`, so every `describedby` link and the `303` target point at production, which does not carry this code and answers 404. Comment *why* it is a second step: Cloud Run only knows the service URL once the service exists, and one extra revision on an ephemeral preview is the right price for identifiers that are absolute and self-consistent. The reported URL has no trailing slash and an `https` scheme, so it satisfies `validate_public_base_url` unmodified — do not post-process it
- [ ] **Do not** add a request-host fallback in DPE as an alternative to the above. Deriving the base URL from `Host` would make `@id` and `Link` vary by how the page was reached and would put an attacker-influenceable value into an identifier — the exact thing the canonical-shortcode rule exists to prevent. If the workflow change cannot be made to work, stop and report rather than reaching for it
- [ ] Check whether `cloud-run-editor-pull-request.yml` has the same shape and record the answer as a follow-up in the PR body; do **not** change it in this phase
- [ ] Update `docs/src/dpe/machine-readable-metadata.md`: the property table for the new `license` and `identifier` shapes, and a row in the *Assessment results* table for the 2026-09-18 preview run (F-UJI 3.5.0, 13/24; FAIR Champion 1.1.11, 7/15), noting that it predates these fixes and that the missing base URL accounted for `I1-01M-2`
- [ ] Update `docs/src/dpe/operations.md` to say that PR previews set `DPE_PUBLIC_BASE_URL` to their own Cloud Run URL automatically
- [ ] Update this plan's Success Metrics rows for *LicenseStrong* and *MetadataIdentifierFound* to record that the gap was found by assessment and fixed. **Do not assert a new score anywhere** — nothing is re-measured until a preview carrying these fixes is assessed again
- [ ] Prove OAI output is unchanged: neither JSON-LD fix touches DataCite or Dublin Core. The baseline is at `.claude/tmp/oai-baseline-hashes.txt` (102,158 entries) and must **not** be regenerated. Recover the hash test (the journal records where it was last restored from), run it, confirm it prints `compared 102158 entries`, then remove it again. A skipped test is not a passed checkpoint
- [ ] Run the standing gate: `git rebase --exec 'env -u GIT_DIR just check' --exec 'env -u GIT_DIR just test' 6f53d3c7`. The `env -u GIT_DIR` is load-bearing — see the journal's side findings
- [ ] Run `eng:reviewing` on this phase's diff with the complete reviewer set; fold every finding into the commit that introduced it — but only for commits created *in this phase*, since everything at or below `6f53d3c7` is pushed

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Set `DPE_PUBLIC_BASE_URL` per environment in ops-deploy (DEV, TEST, STAGE, PROD) and deploy the merged DPE to DEV | Infrastructure (Lukas Stöckli or Samuel Börlin) | after the PR merges, before the post-merge verification | Deployment to shared infrastructure and a change in another repository |
| H2 | Run FAIR Champion (`https://tools.ostrails.eu/champion/`) against the DEV landing page for project 0862 and share the result set | Ivan Subotic | after H1, before the post-merge verification | Hosted external service with no CLI |
| H3 | Decide whether DaSCH publishes a metadata persistence policy URL; if yes, provide it so a `persistencePolicy` link can be added | Co-Directors | anytime; not blocking | Organisational decision |

None of these falls inside Phases 0 to 4, so the orchestrated run has no gate.

## Post-merge verification (not orchestrated)

Not a phase, not part of the orchestrated run, and not a checklist the
orchestrator ticks: it happens after the PR has merged and DEV carries it, and
it changes no code. **Gate: H1, H2** must be resolved before it starts. The
local F-UJI run in Phase 3 already proves the code; this proves the deployment,
where the base URL, Traefik and caching are real.

1. Run `just fair-check` against the DEV landing page for project 0862 and compare against the 2026-09-15 baseline and the Success Metrics targets.
2. Compare the FAIR Champion result from H2 against the baseline and the Success Metrics targets.
3. Verify with `curl -I` against DEV that `Link` and `Vary: Accept` survive Traefik unchanged and that `HEAD` returns them.
4. Confirm each remaining failure is one of the known residuals (project-level data pointer, DataCite registration, persistence policy, search indexing); anything else is a defect for a follow-up plan.
5. Post both DEV results, with assessor versions, as a comment on DEV-7268 in the *Assessment results* row format.

## Acceptance Criteria

### Phases 0 to 4 (checked before the PR is called done)

- [ ] `GET /dpe/projects/0862` contains exactly one `<script type="application/ld+json">` whose root has `@type: Dataset`, `@id` equal to the ARK, `url` equal to the landing page, `license`, `creator` with an ORCID `identifier`, `producer` of type `ResearchProject`, `isAccessibleForFree` and `conditionsOfAccess` per the access-rights table, and no `MISSING` or `CALCULATED` strings anywhere
- [ ] The same response carries a `Link` header with exactly one `cite-as`, two `type`, at least two `describedby` with `type` attributes, and matching `<link>` elements in the head; `Link` is asserted through an RFC 8288 parser
- [ ] `HEAD /dpe/projects/0862` returns headers byte-identical to GET with an empty body
- [ ] Requests differing only in shortcode casing produce identical JSON-LD `@id`, `Link` values and redirect targets
- [ ] A shortcode containing header-injection characters yields the existing 200 "Project Not Found" body with no `Link` header and no JSON-LD
- [ ] `GET /dpe/projects/0862/metadata.jsonld` returns `application/ld+json` with the uncapped graph and `Link: rel="describes"`
- [ ] `GET /dpe/projects/0862/metadata.datacite.json` returns `application/vnd.datacite.datacite+json` that validates against DataCite's kernel 4 JSON schema
- [ ] `GET /dpe/projects/0862` with `Accept: application/ld+json` returns 303 to the JSON-LD representation; with a browser `Accept` returns 200 HTML; `Vary: Accept` is present on 200, 303 and HEAD
- [ ] Malformed shortcode on a representation route returns 400 `text/plain`; unknown shortcode returns 404 `text/plain`; the representation routes sit behind the same per-IP limiter as `/dpe/oai` and answer 429 past its burst (handler test)
- [ ] A `DPE_PUBLIC_BASE_URL` with a trailing slash or a path fails startup; the `Accept` decision function yields HTML without panicking for a header with thousands of media ranges
- [ ] Unknown shortcode on the landing page emits no JSON-LD and no `Link` header and never redirects, whatever the `Accept` header
- [ ] `modules/platform/` no longer exists; `shared/metadata`, `shared/telemetry` and `shared/fair` do; no `platform_metadata`, `platform-metadata`, `platform-telemetry` or `modules/platform` string remains outside `CHANGELOG.md`; `check-shared-paths.sh` runs from `just check` over `shared/*/src/*.rs`
- [ ] OAI-PMH `GetRecord` output for every committed project and every committed record is byte-identical before and after Phase 1
- [ ] `shared-fair` depends on `shared-metadata`, `serde` and `serde_json` only, holds no path into a service module (`just check`), and contains no Axum, Maud or `http` type; `dpe-core`'s dependency list is unchanged
- [ ] In `shared-fair`, only `ProjectGraph::build`, `RecordGraph::build` and `PartRef::from_record` take a `&ProjectRaw` or a `&Record`, and nothing in the crate reads a process-global; every DataCite, Dublin Core, JSON-LD, meta-tag, link-set and Turtle writer takes a graph or a record written from one
- [ ] Title, ARK, license URIs, creator names and creator ORCIDs agree across JSON-LD, Dublin Core meta, the link set and DataCite for every committed project, and title, ARK, creators and license agree across DataCite and Dublin Core for the first and last 100 records of every committed dump; every writer runs without panicking over that set (test)
- [ ] `just fair-check` against a local `just dev` started with `DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000` scores F-UJI at or above 12 of 24 for project 0862, with F2, F4-01M-1, I1, I2, I3 and R1.1 passing, and the run is recorded in the *Assessment results* table with its `software_version`
- [ ] `ARCH-MAP.md` has a `shared/fair` component entry, the root `CONTEXT.md` lists `shared-fair` under shared infrastructure, and `ARCH-MAP.md`, the root `CONTEXT.md` and ADR-0005 no longer say "docs-only until DEV-7268 lands"; each names the tests and the review step that enforce the decision
- [ ] `just check` and `just test` pass on every commit; documentation listed in each phase is updated
- [ ] Every phase ended with the complete reviewer set run on that phase's diff before the next phase started, and every finding is either folded into the commit that introduced it or recorded as a follow-up in the PR body with a reason
- [ ] Phases 0 to 4 ran without a gate or a human input; H1 to H3 and the post-merge verification all fall after the merge

### Post-merge (after H1 and H2; not part of the orchestrated run)

Numbered, not checkboxes, so no orchestrator scan mistakes them for work.

1. `just fair-check` against the DEV landing page for project 0862 meets the Success Metrics targets, and `Link` and `Vary: Accept` survive Traefik unchanged on GET and HEAD.
2. FAIR Champion on DEV passes LicenseStrong, LicenseWeak, QualifiedRefs and MetadataIdentifierFound for project 0862.

## Dependencies & Risks

- **Cross-repo dependency on ops-deploy** for `DPE_PUBLIC_BASE_URL` (H1). Until set, DEV pages point `describedby` links at production. Acceptable for a short window; verification is gated on it.
- **Re-plumbing the four mappings and switching from the view model to `ProjectRaw` in Phase 1 can change OAI output.** The hash test over every committed project and record is the guard; a diff means the graph is missing a fact or a projection dropped a choice, and the output does not move. The mappings read no field the view model transforms, so the switch is expected to be silent. If a mapping turns out to make a choice that cannot be expressed as graph fact plus projection, that mapping keeps reading the fact it needs from the graph in whatever shape it needs, and the plan does not widen the graph to mirror `ProjectRaw`.
- **`dpe-core` grows a second copy of every project in memory** (the `ProjectRaw` beside the view model). Eighty-five projects; negligible. If the view model later becomes a projection of the raw, the copy goes away.
- **Phase 0 is a wide mechanical rename.** About 105 files mention `platform_metadata`, `platform-metadata`, `platform-telemetry` or `modules/platform` (52 under `modules/dpe`, 31 under `modules/editor`, 16 under `docs/src`, 6 workflows, the root docs and manifests; counted 2026-09-17). The compiler catches every missed Rust path and every wrong dependency-path depth at once (`cargo build` fails to resolve the crate), `just check` catches formatting and the paths gate, and a final grep catches prose. `CHANGELOG.md` is generated history and is not edited. What can survive to review is a stale prose mention, and the consistency and dune reviewers read for exactly that.
- **Phase 0 conflicts with every open Rust branch.** Roughly ninety `use` lines change across `dpe-*` and `editor-*`, so any open branch touching imports (dependabot bumps included) conflicts on rebase. This PR should merge ahead of other Rust work, or Phase 0 is rebased repeatedly; the order is a coordination point, not a code risk.
- **`just fair-check` needs Docker and registry access.** It pulls and runs `ghcr.io/pangaea-data-publisher/fuji`, and the two F-UJI source checks read out of the same image. Both are preconditions of the machine running the orchestration, verified at intake; a machine without them cannot run this plan unattended, and the run should not start there.
- **The mdBook build is not a link checker.** CI runs `mdbook build docs` with no linkcheck preprocessor, so a stale cross-reference among the sixteen `docs/src/` pages Phase 0 and Phase 1 touch fails nothing. The consistency reviewer's pass and the Phase 0 grep are the backstop.
- **A shared crate with one consumer.** Deliberate, and recorded in ADR-0005 with the consumers it anticipates. The cost of being wrong is a crate that could have been `dpe-*`; the cost of the reverse mistake is a second resolver in the Deposit Area.
- **F-UJI merges metadata from several sources with a priority order.** JSON-LD says `Dataset`, DataCite JSON says `Project`. If DataCite wins the merge, F-UJI's resource-type test can regress after Phase 3. Check the merge order in `fuji_server/helper/metadata_mapper.py` during Phase 3; if it does, the link stays (nothing is hidden for a score any more than invented for one) and the regression is recorded as a residual. Do not change the DataCite mapping.
- **F-UJI's DataCite JSON parser expects an object** (its log shows it failing on bytes). The writer must produce the flat kernel-4 JSON shape, and the Phase 3 test must exercise F-UJI's own reading of it via `just fair-check`.
- **F-UJI version drift and supply chain.** The baseline is 3.5.0; the current container is 4.x. The recipe pins the image by digest and every result records `software_version` and the digest; the two source checks in Phases 2 and 3 read from that pinned image, never from a floating tag. Bumping the pin is a reviewed change.
- **Phase 2's `describedby` targets are OAI envelopes.** Accurate as `application/xml`, but a harvester wanting bare DataCite gets it only from Phase 3. Do not claim full Signposting Level 1 before Phase 3 ships.
- **Project-level data pointer.** F3, A1-03D and FAIR Champion's data-identifier tests need a data identifier. Projects without records have none, and no project-level download exists. This is a residual, not a defect; record landing pages are the follow-up, and `RecordGraph` gains the file pointer when that plan emits `distribution`.
- **JSON-LD size** for record-heavy projects on the standalone representation. Mitigation: per-IP limiter reuse if measured to matter.
- **Maud formatting.** `maudfmt` rejects a non-trivial `html!` block passed directly as a function argument; keep the head extras as a local binding.
- **No RDF library in the tree.** Turtle is last and droppable for that reason.
- **Google Dataset Search** requires `Dataset` root, `name` and `description`; indexing is not under our control and is not an acceptance criterion.
- **The dune layer is one day old.** `ARCH-MAP.md` and the ADRs were written with this plan in view, and ADR-0005 was amended with it; any disagreement found while implementing is resolved in favour of the ADR, and the plan is amended, not the ADR.

## Success Metrics

| Metric | Baseline (2026-09-15) | Target after Phase 3 |
|--------|-----------------------|----------------------|
| F-UJI total | 3 / 24 (12.5%) | ≥ 12 / 24 |
| F-UJI F2 core metadata | 0 / 2 | 2 / 2 |
| F-UJI F4-01M-1 search-engine ingestion | 0 / 1 | 1 / 1 |
| F-UJI I1 formal representation | 0 / 2 | 2 / 2 |
| F-UJI I2 semantic resources | 0 / 1 | 1 / 1 |
| F-UJI I3 related entities | 0 / 1 | 1 / 1 |
| F-UJI R1.1 license | 0 / 2 | 2 / 2 |
| F-UJI A1-01M access level | 0 / 1 | ≥ 0.5 / 1 |
| FAIR Champion passes | 6 / 15 (2 hollow) | ≥ 10 / 15 |
| FAIR Champion license (strong, weak) | fail, fail | pass, pass |
| FAIR Champion MetadataIdentifierFound | fail | pass |
| FAIR Champion QualifiedRefs | fail (0 of 0 triples) | pass |

Targets are conservative. Out of reach in code and excluded: DataCite and re3data
registration (keyed on a DOI prefix), Bing indexing, and metadata persistence
(needs H3).

**One target is missed and is not reachable by anything in this plan: F-UJI I2
semantic resources, target 1/1, measured 0/1.** The total target is met (14/24
against ≥ 12) and every other F-UJI row above is at or above its target. I2 is
not a serialisation gap, which is why Phase 4 does not close it: F-UJI strips
schema.org and the Dublin Core namespaces before scoring the test, and one of
its two sub-tests earns zero unconditionally in 3.5.0. See *Phase 4: Turtle
representation* and the *Known residuals* section of
`docs/src/dpe/machine-readable-metadata.md`. Recorded as a follow-up — a
controlled-vocabulary link F-UJI's registry recognises — not as a silent miss.

## References

- The decision this plan implements: `docs/adr/0005-fair-landing-pages-in-the-access-area.md` (amended 2026-09-17 alongside this plan); the hypermedia rule it carves into: `docs/adr/0004-hypermedia-frontends.md`; the layout and layering it must respect: `docs/adr/0002-areas-at-the-repository-root.md` (amended 2026-09-17: shared root `shared/`, moved first by this plan's Phase 0), `docs/adr/0003-one-modulith-per-area.md`
- Agent-context layer to keep current: `ARCH-MAP.md` (`modules/dpe` and `shared/metadata` entries, *Conventions*, *Banned constructs*), `CONTEXT.md` (*Shared infrastructure*, *Boundary rules*), `modules/dpe/CONTEXT.md` (*Landing page*, *Machine-readable representation*), `shared/README.md`
- Landing page handler and document shell: `modules/dpe/server/src/main.rs:66-89`, `modules/dpe/server/src/view.rs:10-64`
- JSON handler pattern to mirror for 400/404: `modules/dpe/server/src/fragments.rs:196-209`
- Header tuple response precedent: `modules/dpe/api-oai/src/handlers/mod.rs:75`
- DataCite mapping and helpers to move: `modules/dpe/api-oai/src/metadata/datacite.rs:20` (`project_to_datacite`), `datacite.rs:47-53` (creator fallback), `helpers.rs`, `resolve.rs:68-83`, `types.rs:24-43`; record mappings `record_datacite.rs:17-34`, `record_dublin_core.rs:11-24`; OAI-only code that stays: `metadata/mod.rs`
- Existing corpus-wide test to copy: `modules/dpe/api-oai/src/metadata/datacite.rs:458`
- Contributor lookup: the trait to move, `modules/dpe/core/src/contributors.rs:32-35`; `CachedContributorLookup`, which stays in `dpe-core`, `contributors.rs:37-49`; view-model conversion and the `url` reading rule to move: `modules/dpe/core/src/project.rs:31,96-142`; temporal globals the mapping reads today: `modules/dpe/api-oai/src/metadata/datacite.rs:283-288`, `modules/dpe/core/src/temporal_enrichment_cache.rs:15`
- Project cache, repository trait and record cache: `modules/dpe/core/src/project_cache.rs:23-60`, `modules/dpe/core/src/project_repository.rs:4-7`, `modules/dpe/core/src/record_cache.rs:21-23`
- ARK construction: `shared/metadata/src/record.rs:29-42` and `record.rs:155` (project ARK from a record)
- Placeholder and multilingual helpers: `shared/metadata/src/utils.rs:16-35`; `AccessRightsType`: `shared/metadata/src/project.rs:206`
- Shared crate rules: `modules/platform/README.md` and `.github/scripts/check-platform-paths.sh` (pathspec `modules/platform/*/src/*.rs`, module discovery from `git ls-files -- modules`), which Phase 0 moves to `shared/README.md` and `check-shared-paths.sh`; its fixture-building test `.github/scripts/check-platform-paths.test.sh:43` lists the shared crate directories and moves with it
- Paths written as `shared/…` in this plan are today's `modules/platform/…` until Phase 0 lands; line numbers refer to the files as they are today
- Config and state threading: `modules/dpe/server/src/config.rs:18-53`, `modules/dpe/server/src/main.rs:19-21`
- OAI per-IP limiter: `modules/dpe/server/src/router.rs:65-79`
- Deliberate no-negotiation precedent to reconcile: `modules/dpe/server/src/downloads.rs:56-62,152-170`
- Architecture statement to amend: `docs/src/dpe/architecture.md:72`; dependency statements to update: `docs/src/repo_structure.md:43`, `docs/src/dpe/project_structure.md:32-34,67,96-98`, `docs/src/dpe/architecture.md:29`
- Handler test harness: `modules/dpe/server/src/fragments.rs:291-320`, `modules/dpe/server/src/router.rs:293-334`
- Pre-existing third `PreEscaped` site: `modules/mosaic/tiles/src/components/form/textarea/mod.rs:142`
- FAIR Signposting profile: https://signposting.org/FAIR/
- Science-on-Schema.org Dataset guide: https://github.com/ESIPFed/science-on-schema.org/blob/main/guides/Dataset.md
- Google Dataset structured data: https://developers.google.com/search/docs/appearance/structured-data/dataset
- schema.org `license` domain: https://schema.org/license
- F-UJI methods and source: https://www.f-uji.net/index.php?action=methods, https://github.com/pangaea-data-publisher/fuji (container `ghcr.io/pangaea-data-publisher/fuji`, port 1071; cache key in `fuji_server/helper/request_helper.py` is final URL plus content type)
- OSTrails FAIR Champion tests: https://tools.ostrails.eu/champion/tests/
- DataCite content resolver and JSON media type: https://support.datacite.org/docs/datacite-content-resolver
- Baseline assessment results (F-UJI 3.5.0, FAIR Champion 1.1.11) recorded 2026-09-15 for `https://ark.dasch.swiss/ark:/72163/1/0862`; Linear DEV-7268
