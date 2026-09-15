---
title: "feat: Machine-readable project metadata for FAIR assessment"
type: feat
date: 2026-09-15
author: "Ivan Subotic"
status: draft
repository: dasch-swiss/dsp-repository
linear: DEV-7268
---

# feat: Machine-readable project metadata for FAIR assessment

## Overview

Make the DPE project landing page `/dpe/projects/{shortcode}` machine-readable so
FAIR assessment tools and harvesters can read the project's metadata without a
human. Four mechanisms, each building on the previous one:

1. schema.org JSON-LD and Dublin Core `<meta>` tags embedded in the page head.
2. FAIR Signposting `Link` headers and matching `<link>` elements on the landing
   page.
3. Dedicated metadata resources under the project path (JSON-LD, DataCite JSON),
   linked from the landing page via `describedby`, plus an Accept-driven redirect
   from the landing page to them.
4. Optionally, a Turtle resource on the same footing.

The data already exists and is already mapped to DataCite kernel 4 for OAI-PMH.
The work is exposure, not modelling: extract the existing mapping into a shared
crate, add two serialisations, and wire them into the page and three routes.

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

## Proposed Solution

**Repository side only.** The ARK resolver stays a plain redirect. It already
passes `Accept` through, and both assessors read the final response, so nothing
in this plan needs the resolver to change.

**Root type `Dataset`.** schema.org defines `license`, `keywords`,
`datePublished`, `conditionsOfAccess` and `distribution` on `CreativeWork` only.
`ResearchProject` is an `Organization`, so a page typed that way cannot carry the
properties the assessors test. The JSON-LD root is therefore a `Dataset`
describing the project's data holdings, with the research project attached as a
linked `producer` node of type `ResearchProject`. The DataCite mapping keeps
`resourceTypeGeneral="Project"` unchanged; it follows the DaSCH-to-DataCite
mapping specification and the two vocabularies do not need to agree.

**Dedicated metadata URLs, not header-based rendering.** The architecture
document rejects header-based route discrimination, and the record-file endpoint
pins that `Accept` is ignored. The plan respects that: each machine-readable
representation gets its own URL under the project path, in the same path-depth
style as `/tab/{tab}`. The landing page advertises them via `describedby`. The
only header-based decision is a `303 See Other` from the landing page to the
matching metadata URL when a client asks for a non-HTML type, which is the W3C
"Cool URIs" pattern and needs a one-line carve-out in the architecture document.

**One home for the mapping, one resolved graph.** The DataCite mapping, its
helpers and the agent resolver move out of the OAI crate's private module into a
new library crate `dpe-formats`, which the OAI crate and the server both depend
on. A single resolved intermediate, `ProjectGraph` (canonical ARK and shortcode,
titles, description, resolved agents with ORCID, licenses, access, dates,
keywords, coverage, funding, parts), is built once per project. The JSON-LD,
Dublin Core meta, Signposting link set and DataCite JSON writers all consume it,
and `project_to_datacite` reads the same helpers, so the representations cannot
disagree on multilingual preference, placeholder filtering, creator fallback or
temporal-coverage resolution. A corpus-wide test enforces the agreement.

## Alternative Approaches Considered

**Export the mapping from `dpe-api-oai` instead of extracting a crate.** Cheapest
change: make the `metadata` module public and add the new writers there. Rejected
because an OAI crate would then own schema.org JSON-LD that has nothing to do
with OAI, and `dpe-server` would depend on an API crate for non-API code. The
repo's own rule is "where the shared thing is a subset of a crate, extract the
subset". The extraction is mechanical and the existing OAI tests pin its output.

**Accept-based rendering inline on the landing page route.** Serve JSON-LD or
DataCite JSON directly from `/dpe/projects/{shortcode}` when `Accept` asks for
it. Rejected: it contradicts the documented preference against header-based
discrimination, gives the machine-readable representations no stable URL to put
in `describedby` links, and forces every error path to be typed per media type.
The redirect variant keeps the URLs canonical and the carve-out to one line.

**Type the page `ResearchProject` and attach CreativeWork properties anyway.**
Semantically invalid schema.org. Validators flag it and F-UJI's resource-type
test keeps failing. Rejected.

**Type the page `DataCatalog`.** Fits the "database of databases" framing, but
F-UJI and Google treat a catalog as a container, not a dataset, so the
dataset-level tests still fail. A `DataCatalog` node for the whole DPE is still
emitted as `includedInDataCatalog`; it just is not the root.

**Build head extras in `dpe-web`.** Rejected because `dpe-web` would then depend
on `dpe-formats`, changing the documented dependency graph twice. The server
already owns the document shell in `view.rs` and is the right place to inject
head content.

## Technical Considerations

### Dependency graph after Phase 1

```
platform-metadata → dpe-core → dpe-formats → { dpe-api-oai, dpe-web*, dpe-server }
                                              (* dpe-web does not depend on dpe-formats)
```

`dpe-formats` depends on `dpe-core` because `ContributorLookup` and the runtime
`Project` type live there (`modules/dpe/core/src/contributors.rs:32-35`,
`modules/dpe/core/src/project.rs:52-94`). It depends on no API crate and on
neither service binary. `dpe-api-oai` keeps `xml.rs`, `handlers/`, `resumption.rs`
and `error.rs`; only `src/metadata/` moves.

### Where things live

| Concern | Crate / file |
|---------|--------------|
| DataCite record model, mapping, helpers, agent resolver | `dpe-formats` (moved from `modules/dpe/api-oai/src/metadata/`) |
| Resolved intermediate `ProjectGraph` | `dpe-formats/src/graph.rs` |
| schema.org JSON-LD builder (`serde_json::Value`) | `dpe-formats/src/schema_org.rs` |
| Dublin Core meta-tag model | `dpe-formats/src/dublin_core.rs` |
| DataCite JSON writer | `dpe-formats/src/datacite_json.rs` |
| Turtle writer (Phase 4) | `dpe-formats/src/turtle.rs` |
| Signposting link set model (rel, href, type) | `dpe-formats/src/signposting.rs` |
| Head extras rendering (`<script>`, `<meta>`, `<link>`) | `modules/dpe/server/src/view.rs` (new `head_extras` slot) and a new `modules/dpe/server/src/metadata.rs` |
| Metadata route handlers, Accept parsing, 303 | `modules/dpe/server/src/metadata.rs` |
| Public base URL config | `modules/dpe/server/src/config.rs` (`DPE_PUBLIC_BASE_URL`), threaded through `AppState` like `css_href`, never a process-global, so handler tests can inject it |

### JSON-LD content

Built from `dpe_core::project_cache::project_by_shortcode` (the runtime
`Project`), `CachedContributorLookup` for persons and organisations, and
`record_cache::all_records()` filtered on the upper-cased shortcode for parts.

**Every identifier, URL and header value derives from the resolved project's
canonical `shortcode` and `pid`, never from the raw path segment.** The lookup is
case-insensitive, so `/dpe/projects/080c` and `/dpe/projects/080C` produce
byte-identical JSON-LD, `Link` headers and redirect targets. Axum percent-decodes
the path, so the raw segment is attacker-controlled and must not reach a header.

| schema.org property | Source | Rule |
|---------------------|--------|------|
| `@id`, `identifier` (PropertyValue, propertyID `ARK`) | `pid` or ARK built from shortcode | Same fallback as `project_to_datacite` |
| `name`, `alternateName` | `name` / `official_name`, `alternative_names` | Same precedence as DataCite titles |
| `description` | `description` then `abstract_text` | `platform_metadata::multilingual_value` (English-preferring) |
| `keywords` | `keywords` | One string per keyword, same helper |
| `license` | `legal_info[].license.license_uri` | Every distinct non-placeholder URI; array when several; omitted when none |
| `conditionsOfAccess`, `isAccessibleForFree` | `access_rights` | See the access-rights table below |
| `datePublished` | `data_publication_year` | Year string; omitted when placeholder |
| `creator` | attributions where `is_creator` | Person with `identifier` PropertyValue for ORCID and `affiliation`; Organization otherwise; same shape as `resolve_agent`. **Same fallback as DataCite:** when no attribution is a creator, a single Organization creator `DaSCH` is emitted (`datacite.rs:47-53`) |
| `contributor` | remaining attributions | Same shape; DataCite role kept in `roleName` via a `Role` wrapper only if cheap, else omitted |
| `publisher` | constant | `{ "@type": "Organization", "name": "DaSCH", "url": "https://dasch.swiss" }` |
| `funder`, `funding` | `funding` grants | `MonetaryGrant` with `identifier` (grant number), `url`, `funder` Organization |
| `spatialCoverage` | `spatial_coverage` | `Place` with `sameAs` = authority URL and `name` |
| `temporalCoverage` | `temporal_coverage` | Reuse `platform_metadata::temporal_coverage::resolve_in`; ISO 8601 interval or name-only text |
| `inLanguage` | `data_language` | As given |
| `url` | landing page | `{public_base_url}/dpe/projects/{canonical shortcode}`; schema.org and Google expect the dataset's own page here, not the project website |
| `citation` | `publications` | Text, with `@id` when a PID exists |
| `producer` | project identity | `ResearchProject` node: `name`, `startDate`, `endDate`, `url` (the project's external website from `url`, when present and non-placeholder), `member` (creators) |
| `includedInDataCatalog` | constant | `DataCatalog` node for the DPE at `{public_base_url}/dpe/projects` |
| `hasPart` | records for the shortcode | `Dataset` nodes with `@id` = record ARK and `name` only; **capped at 100 in the embedded block** (the OAI default page size), uncapped in the standalone JSON-LD resource. The complete list is also harvestable from the OAI set `project:{shortcode}`, which the documentation names; no schema.org property is a truthful fit for that pointer, so none is emitted |
| `sameAs` | `pid` when it differs from `@id` | Rarely set |

No project-level `distribution`. There is no project-level download, and
inventing one would be false. This is a known residual against F-UJI F3 and
A1-03D for a project ARK; record landing pages are the correct assessment target
for those tests and are out of scope here.

### Access rights mapping

`AccessRightsType` has four variants (`platform_metadata::project`, line 205).
`license` and `hasPart` are emitted for every variant: a license is a property of
the data whatever its access level, and record ARKs resolve to metadata landing
pages, not downloads, so listing them claims nothing about downloadability.

| `AccessRightsType` | `isAccessibleForFree` | `conditionsOfAccess` | `DC.accessRights` |
|--------------------|-----------------------|----------------------|-------------------|
| `FullOpenAccess` | `true` | `"Full Open Access"` | `http://purl.org/coar/access_right/c_abf2` (open access) |
| `OpenAccessWithRestrictions` | `false` | `"Open Access with Restrictions"` | `http://purl.org/coar/access_right/c_16ec` (restricted) |
| `EmbargoedAccess` | `false` | `"Embargoed Access until {embargo_date}"` when the date is set, else `"Embargoed Access"` | `http://purl.org/coar/access_right/c_f1cf` (embargoed) |
| `MetadataOnlyAccess` | `false` | `"Metadata only Access"` | `http://purl.org/coar/access_right/c_14cb` (metadata only) |

The text values are the existing serde names of the enum, so the human-readable
and machine-readable representations use one vocabulary. The COAR URIs are the
standard terms F-UJI's access-level test recognises.

### Descriptions and language

`description` uses `platform_metadata::multilingual_value` (English first, then
the deterministic fallback already used by DataCite), so a project with only
German text yields the same German text in JSON-LD, Dublin Core meta and
DataCite. The JSON-LD carries the full text. `DC.description` is truncated to
1000 characters on a character boundary with an ellipsis; a `<meta>` attribute
of several kilobytes is legal but pointless.

### Signposting link set

Emitted identically as an HTTP `Link` header and as `<link>` elements in the
head. Level 1 of the FAIR Signposting profile.

| rel | target | cardinality |
|-----|--------|-------------|
| `cite-as` | ARK | exactly 1 |
| `type` | `https://schema.org/Dataset`, `https://schema.org/AboutPage` | 2 |
| `describedby` | OAI `GetRecord` for `oai_datacite` and `oai_dc` (`type="application/xml"`); after Phase 3 the JSON-LD (`application/ld+json`) and DataCite JSON (`application/vnd.datacite.datacite+json`) resources; after Phase 4 Turtle (`text/turtle`) | 2 to 5 |
| `license` | SPDX URI | 0 or 1: emitted only when exactly one distinct non-placeholder URI exists across `legal_info`; otherwise omitted (the profile allows at most one). JSON-LD still lists all licenses and Dublin Core emits one `DC.rights` per license |
| `author` | ORCID URLs of creators | 0 or more; the `DaSCH` fallback creator has no ORCID and produces no `author` link |

Each metadata resource answers with `Link: <landing>; rel="describes"`. The
profile's `collection` and `item` relations describe content resources, not
metadata resources, so they are not used here.

Header values are URIs assembled from the resolved project's canonical fields.
Build each with `HeaderValue::from_str` and drop any that fails rather than
panicking. Never place free text (names, titles) or the raw path segment in a
header.

A note on Phase 2's `describedby` targets: the OAI `GetRecord` URLs return an
OAI-PMH envelope around the DataCite or Dublin Core payload. `application/xml`
is an accurate type for that document, and repositories commonly link it, but a
harvester wanting bare DataCite gets it only from the Phase 3 resources. Phase 2
therefore claims embedded metadata plus Signposting structure, not the full
Level 1 value; Phase 3 completes it.

### Embedding JSON-LD in Maud

Maud escapes text inside `script {}`, so the JSON is spliced with `PreEscaped`.
That is trusted, server-generated content and becomes the second sanctioned
`PreEscaped` site after the Mosaic icon SVG. Before splicing, every `<`, `>` and
`&` in the serialised JSON is replaced with its `\u00XX` escape. That is still
valid JSON, and it neutralises both `</script>` and `<!--`, the two sequences
that move the HTML parser out of script-data state. `serde_json::to_string`
already escapes control characters and quotes.

```rust
fn json_ld_script(graph: &serde_json::Value) -> Markup {
    let json = serde_json::to_string(graph)
        .expect("serde_json::Value always serialises")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026");
    html! {
        script type="application/ld+json" { (PreEscaped(json)) }
    }
}
```

Dublin Core `<meta>` values and `<link href>` attributes go through Maud's
default escaping. Descriptions in `DC.description` are truncated to a bounded
length.

### Head extras slot

`view::page()` gains a `head_extras: Markup` parameter inserted before
`title { (title) }`. Callers without extras pass `html! {}`. This touches the four
callers in `main.rs` and the five `page()` tests in `view.rs`.

### Accept handling on the landing page

Small q-value-aware parser over the `Accept` header (no new dependency). The
candidate set is `text/html`, `application/ld+json`,
`application/vnd.datacite.datacite+json` and, in Phase 4, `text/turtle`. A
supported non-HTML type wins only when its effective q is strictly greater than
the best q that covers HTML (`text/html`, `text/*` or `*/*`), and the handler
answers `303 See Other` to the matching metadata URL. Browsers send `text/html`
at q=1 and `*/*` at q=0.8, so nothing changes for people. F-UJI caches parsed
bodies by final URL and content type, so a redirect to a distinct URL with a
distinct content type is a fresh parse for it.

Content negotiation on this route never produces a 4xx. Decision table:

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

Metadata routes validate the shortcode with `is_valid_shortcode` (400) and look
the project up (404), mirroring `project_json_handler`
(`modules/dpe/server/src/fragments.rs:196-209`). Errors are `text/plain` with an
empty body, never HTML. The HTML page handler keeps its existing always-200
"Project Not Found" behaviour; that gap predates this work and is out of scope.
When the project does not resolve, the page emits no JSON-LD and no `Link`
header. The `cite-as` link is therefore only ever emitted on a page that
describes a real project.

Metadata routes send no `Cache-Control` or `ETag` for now, consistent with the
other DPE routes; data changes only on deploy. Revisit if the standalone JSON-LD
for record-heavy projects shows up in latency metrics.

### Response headers precedent

The idiomatic shape in this codebase is the Axum tuple response, as in
`modules/dpe/api-oai/src/handlers/mod.rs:75`:

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

Signposting targets must be absolute. `DpeConfig` has only `oai_base_url`, scoped
to `/dpe/oai`. Add `DPE_PUBLIC_BASE_URL` (default
`https://repository.dasch.swiss`) next to it. Until ops-deploy sets it per
environment, a DEV page emits `describedby` links into production; that is a
human action with a gate before verification.

### Size and abuse

Projects can have thousands of records. The embedded block is bounded by the
`hasPart` cap. The standalone JSON-LD resource is uncapped but is a single
in-memory serialisation of cached data with no I/O; if it proves heavy, it gets
the same per-IP limiter the OAI route already has. The OAI set is the escape
hatch for harvesters that want every record.

### Security

- Only server-generated JSON reaches `PreEscaped`; `<`, `>` and `&` are
  neutralised as JSON unicode escapes.
- No metadata string enters an HTTP header except as a validated URI built from
  the resolved project's canonical fields. The raw path segment never reaches a
  header or the JSON-LD.
- Metadata routes are read-only, cache-backed, and validate the shortcode shape
  before any lookup.

### Consistency across representations

JSON-LD, Dublin Core meta, the Signposting link set and DataCite JSON are all
written from one `ProjectGraph`; DataCite XML reads the same helpers
(`multilingual_value`, `is_placeholder`, `is_creator`, `resolve_agent`,
`resolve_temporal_coverage_in`, `license_identifier_to_label`). A corpus-wide
test, in the shape of the existing `every_committed_temporal_coverage_resolves`,
asserts for every committed project that title, ARK, license URIs, creator names
and creator ORCIDs agree across all representations, and that every writer runs
without panicking.

Tests read `Link` headers through an RFC 8288 parser (a small test-support
function or a dev-dependency), not substring matching, so malformed quoting or
parameter escaping is caught.

## Implementation Phases

This plan is one PR. Each phase is one commit inside that PR, in phase order,
and the PR body ticks `allow-many-commits` because the phases are independent,
self-contained changes (the crate extraction has value on its own; each later
phase is a complete feature). A bug found in an earlier phase while working on a
later one is amended into the commit that introduced it, never a `fix:` on top.
Phase 5 runs after the PR has merged and deployed to DEV and changes no code.
Commit scopes: `dpe-formats` (new, add to the scope table), `dpe-api-oai`,
`dpe-server`, `docs`.

**Review after every phase.** Each phase ends by running `eng:reviewing` on
that phase's diff with the reviewers relevant to it, drawn from this set:
`rust-reviewer` (crate extraction, ownership in the writers),
`maud-datastar-reviewer` (head extras, the `PreEscaped` site),
`security-reviewer` (header assembly, escaping, redirect target),
`ivan-reviewer` (server authority, MPA, the header-based carve-out),
`consistency-reviewer` (the module move, docs and scope table, stale paths),
`performance-reviewer` (JSON-LD size for record-heavy projects) and
`code-simplicity-reviewer`. Findings are fixed by amending that phase's commit
before the next phase starts. The per-phase checkboxes name the subset.

#### Phase 1: Extract the `dpe-formats` crate

Pure refactor. No behaviour change; the OAI XML output for every committed project
must be byte-identical before and after.

- [ ] Create `modules/dpe/formats` (`dpe-formats`) with `Cargo.toml` depending on `platform-metadata`, `dpe-core`, `serde_json`
- [ ] Move `modules/dpe/api-oai/src/metadata/{datacite.rs,record_datacite.rs,dublin_core.rs,record_dublin_core.rs,helpers.rs,resolve.rs,types.rs,mod.rs}` into `dpe-formats/src/` with a public module surface
- [ ] Point `dpe-api-oai` at `dpe_formats::*` and delete its private `metadata` module
- [ ] Add `dpe-formats` to the workspace `Cargo.toml` members and to `dpe-server`'s dependencies
- [ ] Add a snapshot test in `dpe-api-oai` that renders `GetRecord` for `oai_datacite` and `oai_dc` for one committed project and compares to a fixture captured before the move
- [ ] Update `docs/src/repo_structure.md` (tree and crate table) and `docs/src/dpe/project_structure.md` (dependency graph) for the new crate
- [ ] Update `docs/src/dpe/architecture.md` crate graph line so `dpe-api-oai` depends on `platform-metadata`, `dpe-core` and `dpe-formats`
- [ ] Add `dpe-formats` to the commit-scope table in `CONVENTIONS.md`
- [ ] Extend `.github/scripts/check-platform-paths.sh` or `bacon.toml` watch lists if either enumerates DPE crate directories
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with `rust-reviewer`, `consistency-reviewer` and `code-simplicity-reviewer`; amend findings into this phase's commit

#### Phase 2: Embedded metadata and Signposting on the landing page

- [ ] Add `DPE_PUBLIC_BASE_URL` to `DpeConfig` in `modules/dpe/server/src/config.rs` with default `https://repository.dasch.swiss`, thread it through `AppState` (not a process-global), and document it in `docs/src/dpe/operations.md`, including that a local run assessed from a container must set it to a host the container can reach (`http://host.docker.internal:4000`)
- [ ] Add `dpe-formats/src/graph.rs`: `ProjectGraph::build(project, lookup, records) -> ProjectGraph`, the resolved intermediate (canonical ARK and shortcode, titles, description, agents with ORCID, licenses, access rights, dates, keywords, coverage, funding, parts) using the existing helpers and the DataCite creator fallback
- [ ] Add `dpe-formats/src/schema_org.rs`: `to_schema_org(&ProjectGraph, opts) -> serde_json::Value` implementing the property table above, with `opts.has_part_cap: Option<usize>` and `opts.public_base_url`
- [ ] Add `dpe-formats/src/dublin_core.rs` project meta model (`DC.title`, `DC.creator`, `DC.identifier`, `DC.rights` one per license, `DC.accessRights` COAR URI, `DC.type`, `DC.date`, `DC.description` truncated at 1000 characters, `DC.publisher`, `DC.language`) written from `ProjectGraph`
- [ ] Add `dpe-formats/src/signposting.rs`: a `LinkSet` type (rel, href, optional media type) with `to_header_value() -> Option<HeaderValue>` and an iterator for rendering `<link>` elements, plus `project_link_set(&ProjectGraph, base_url, oai_base_url)` implementing the cardinality rules above
- [ ] Add `head_extras: Markup` to `view::page()` and `head()` in `modules/dpe/server/src/view.rs`, inserted before `<title>`; update the four callers in `main.rs` and the five tests in `view.rs`
- [ ] Add `modules/dpe/server/src/metadata.rs` with `head_extras_for_project(shortcode) -> Option<(Markup, HeaderMap)>` that renders the JSON-LD script (with `</` neutralised and `PreEscaped`), the DC meta tags and the `<link>` elements, and builds the `Link` header
- [ ] Wire `project_page_handler` to emit the head extras and the `Link` header when the project resolves, and nothing when it does not
- [ ] Add an RFC 8288 `Link` header parser to `dpe-server` test support (or a dev-dependency) so header tests assert on parsed relations, not substrings
- [ ] Confirm in F-UJI's `fuji_server/helper/metadata_mapper.py` that `identifier` as a `PropertyValue` (and the `@id` fallback) is read as the object identifier; if it needs a plain string, emit `identifier` as an array of the PropertyValue and the ARK string
- [ ] Unit tests in `dpe-formats`: JSON-LD for a fixture project has `@type: Dataset`, ARK `@id`, `url` equal to the landing page, license URI, creator with ORCID `identifier`, `producer` of type `ResearchProject` carrying the external website; placeholders are absent; multiple licenses produce an array; a project with no creator attribution gets the `DaSCH` Organization creator
- [ ] Unit tests: one fixture per `AccessRightsType` variant asserting `isAccessibleForFree`, `conditionsOfAccess` (including the embargo date) and the COAR `DC.accessRights` URI from the access-rights table
- [ ] Unit test: a project whose description has only non-English keys yields the same text in JSON-LD, Dublin Core meta and DataCite; `DC.description` for a description over 1000 characters is truncated on a character boundary and still escapes correctly
- [ ] Unit test: `hasPart` is capped at 100 entries when `has_part_cap` is set and uncapped otherwise; the embedded JSON-LD for the committed project with the most records stays under 64 KB
- [ ] Corpus-wide test (shape of `every_committed_temporal_coverage_resolves`): for every committed project, every writer runs without panicking, and title, ARK, license URIs, creator names and creator ORCIDs agree across JSON-LD, Dublin Core meta, the link set and DataCite
- [ ] Unit test: JSON-LD containing `</script><script>alert(1)</script>` and `<!--` in a description renders as inert JSON inside a single `<script>` element (`<`, `>`, `&` appear only as `\u00XX`)
- [ ] Unit test: link set emits `license` only when exactly one distinct URI exists; `cite-as` always exactly once; the `DaSCH` fallback creator produces no `author` link; header values that fail `HeaderValue::from_str` are dropped
- [ ] Handler tests in `dpe-server` (oneshot, following `fragments.rs` tests): GET landing page contains one `application/ld+json` script, DC meta tags and `<link rel="cite-as">`; the parsed `Link` header has `cite-as`, `type`, `describedby` with `type` attributes; unknown shortcode has neither
- [ ] Handler test: HEAD and GET for the same shortcode return byte-identical headers (`Link`, `Content-Type`) and HEAD has an empty body
- [ ] Handler test: `/dpe/projects/080c` and `/dpe/projects/080C` (or the committed mixed-case shortcode) return identical JSON-LD `@id` and `Link` header values
- [ ] Handler test: a shortcode containing `%0D%0A`, `"`, `;` or `>` returns the current 200 "Project Not Found" body with no `Link` header and no JSON-LD, and does not panic
- [ ] Update `modules/dpe/CLAUDE.md` escaping rule: the JSON-LD script in `dpe-server/src/metadata.rs` is the second sanctioned `PreEscaped` site, with the `</` rule
- [ ] Update `REVIEW.md` if the review checklist needs a line for the `PreEscaped` sanction or the head-extras slot
- [ ] Document the embedded metadata and Signposting in a new `docs/src/dpe/machine-readable-metadata.md` and add it to `docs/src/SUMMARY.md`
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with `rust-reviewer`, `maud-datastar-reviewer`, `security-reviewer`, `ivan-reviewer`, `performance-reviewer` and `consistency-reviewer`; amend findings into this phase's commit

#### Phase 3: Standalone metadata resources and Accept redirect

- [ ] Add `dpe-formats/src/datacite_json.rs`: `to_datacite_json(&DataCiteRecord) -> serde_json::Value` in the DataCite kernel 4 JSON shape (`identifiers[]` with `identifierType: "ARK"`, `types`, `creators[].nameIdentifiers[]`, `titles`, `publisher`, `publicationYear`, `subjects`, `contributors`, `descriptions`, `dates`, `language`, `rightsList`, `geoLocations`, `fundingReferences`, `relatedIdentifiers`, `schemaVersion`)
- [ ] Add routes `GET /dpe/projects/{shortcode}/metadata.jsonld` (`application/ld+json`, uncapped `hasPart`) and `GET /dpe/projects/{shortcode}/metadata.datacite.json` (`application/vnd.datacite.datacite+json`) in `metadata.rs`, registered in `router.rs`; 400 on malformed shortcode, 404 on unknown project, both `text/plain`
- [ ] Each metadata response carries `Link: <landing>; rel="describes"` pointing at the canonical landing page URL
- [ ] Extend the landing page link set with `describedby` entries for both resources, typed with their media types
- [ ] Add an `Accept` parser in `metadata.rs` implementing the decision table above and a `303 See Other` from `project_page_handler` when a supported non-HTML type wins; add `Vary: Accept` to every landing page response (200 and 303, GET and HEAD)
- [ ] Unit tests for the `Accept` parser: one per row of the decision table, including malformed headers and `*/*;q=0`
- [ ] Handler tests: both resources return the right `Content-Type`, `describes` link and body shape; 400 and 404 paths are `text/plain`; landing page with `Accept: application/ld+json` returns 303 with the JSON-LD `Location` built from the canonical shortcode; with a browser `Accept` returns 200 HTML; `Vary: Accept` present on 200, 303 and HEAD
- [ ] Handler test matrix: unknown shortcode × {no `Accept`, `application/ld+json`, `text/html`} returns 200 HTML with no `Link` header and no JSON-LD in every cell
- [ ] Add DataCite's kernel 4 JSON schema as a test fixture under `dpe-formats` (mirroring the XSD fixtures in `modules/dpe/api-oai/src/handlers/testdata/schemas/`) and validate the DataCite JSON writer's output for every committed project against it
- [ ] Unit test: DataCite JSON output for a fixture project has `identifiers[0].identifierType == "ARK"` and the same title, creators and rights as the XML
- [ ] Check F-UJI's metadata merge order in `fuji_server/helper/metadata_mapper.py` and `metadata_harvester.py`; if the DataCite JSON `Project` type can override the JSON-LD `Dataset` type, omit the DataCite JSON `describedby` from the landing page while keeping the resource, and note the reason in `docs/src/dpe/machine-readable-metadata.md`
- [ ] Add the carve-out to `docs/src/dpe/architecture.md`: the landing page's `Accept` redirect is the only header-based decision, and why
- [ ] Update `docs/src/dpe/oai-pmh.md` to reconcile its "no content negotiation" statement for the file endpoint with the new metadata resources (the file endpoint is unchanged)
- [ ] Update `docs/src/dpe/json-api.md` with a pointer to the metadata resources and a note that they are standards-shaped, unlike `/dpe/api/v2`
- [ ] Extend `docs/src/dpe/machine-readable-metadata.md` with the resource URLs, media types and the redirect rule
- [ ] Add `just fair-check <url>` that runs the F-UJI container (`ghcr.io/pangaea-data-publisher/fuji`, port 1071, with `--add-host=host.docker.internal:host-gateway` so it also works on Linux) against a URL and prints the per-metric score table with the F-UJI version; document it in `docs/src/dpe/machine-readable-metadata.md`
- [ ] Run `DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000 just dev` and `just fair-check http://host.docker.internal:4000/dpe/projects/0862`; compare against the 2026-09-15 baseline in Success Metrics
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with `rust-reviewer`, `security-reviewer`, `ivan-reviewer`, `consistency-reviewer` and `code-simplicity-reviewer`; amend findings into this phase's commit

#### Phase 4: Turtle resource

Drop this phase if Phase 3 already lifts both assessors' interoperability tests;
JSON-LD is RDF and both tools accept it.

- [ ] Add `dpe-formats/src/turtle.rs`: a small Turtle writer over the same graph the JSON-LD builder produces (subjects are the ARK and blank nodes; predicates from schema.org and Dublin Core Terms), or a dependency if a hand-rolled writer exceeds roughly 200 lines
- [ ] Add route `GET /dpe/projects/{shortcode}/metadata.ttl` (`text/turtle`) with the same 400/404 and `describes` behaviour
- [ ] Add `text/turtle` to the landing page `describedby` set and to the `Accept` candidate list
- [ ] Unit test: Turtle output for a fixture parses with a Turtle parser in the test dependencies, or is validated against a committed fixture if no parser is added
- [ ] Handler test: `Accept: text/turtle` on the landing page redirects to the Turtle resource
- [ ] Update `docs/src/dpe/machine-readable-metadata.md`
- [ ] Run `just check` and `just test`
- [ ] Run `eng:reviewing` on this phase's diff with `rust-reviewer`, `consistency-reviewer` and `code-simplicity-reviewer`; amend findings into this phase's commit

#### Phase 5: Verification on DEV

**Gate: H1** — resolve before starting this phase.

This phase runs after the PR has merged and DEV carries it. The local F-UJI run
in Phase 3 already proves the code; this phase proves the deployment, where the
base URL, Traefik and caching are real.

- [ ] Run `just fair-check` against the DEV landing page for project 0862 and compare against the 2026-09-15 baseline and the Success Metrics targets
- [ ] Compare the FAIR Champion result from H2 against the baseline and the Success Metrics targets
- [ ] Verify with `curl -I` against DEV that `Link` and `Vary: Accept` survive Traefik unchanged and that `HEAD` returns them
- [ ] Confirm each remaining failure is one of the known residuals (project-level data pointer, DataCite registration, persistence policy, search indexing); anything else is a defect to fix

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Set `DPE_PUBLIC_BASE_URL` per environment in ops-deploy (DEV, TEST, STAGE, PROD) and deploy the merged DPE to DEV | Infrastructure (Lukas Stöckli or Samuel Börlin) | after the PR merges, before Phase 5 | Deployment to shared infrastructure and a change in another repository |
| H2 | Run FAIR Champion (`https://tools.ostrails.eu/champion/`) against the DEV landing page for project 0862 and share the result set | Ivan Subotic | during Phase 5 | Hosted external service with no CLI |
| H3 | Decide whether DaSCH publishes a metadata persistence policy URL; if yes, provide it so a `persistencePolicy` link can be added | Co-Directors | anytime; not blocking | Organisational decision |

## Acceptance Criteria

- [ ] `GET /dpe/projects/0862` contains exactly one `<script type="application/ld+json">` whose root has `@type: Dataset`, `@id` equal to the ARK, `url` equal to the landing page, `license`, `creator` with an ORCID `identifier`, `producer` of type `ResearchProject`, `isAccessibleForFree` and `conditionsOfAccess` per the access-rights table, and no `MISSING` or `CALCULATED` strings anywhere
- [ ] The same response carries a `Link` header with exactly one `cite-as`, two `type`, at least two `describedby` with `type` attributes, and matching `<link>` elements in the head; `Link` is asserted through an RFC 8288 parser
- [ ] `HEAD /dpe/projects/0862` returns headers byte-identical to GET with an empty body
- [ ] Requests differing only in shortcode casing produce identical JSON-LD `@id`, `Link` values and redirect targets
- [ ] A shortcode containing header-injection characters yields the existing 200 "Project Not Found" body with no `Link` header and no JSON-LD
- [ ] `GET /dpe/projects/0862/metadata.jsonld` returns `application/ld+json` with the uncapped graph and `Link: rel="describes"`
- [ ] `GET /dpe/projects/0862/metadata.datacite.json` returns `application/vnd.datacite.datacite+json` that validates against DataCite's kernel 4 JSON schema
- [ ] `GET /dpe/projects/0862` with `Accept: application/ld+json` returns 303 to the JSON-LD resource; with a browser `Accept` returns 200 HTML; `Vary: Accept` is present on 200, 303 and HEAD
- [ ] Malformed shortcode on a metadata route returns 400 `text/plain`; unknown shortcode returns 404 `text/plain`
- [ ] Unknown shortcode on the landing page emits no JSON-LD and no `Link` header and never redirects, whatever the `Accept` header
- [ ] OAI-PMH `GetRecord` output for every committed project is byte-identical before and after Phase 1
- [ ] Title, ARK, license URIs, creator names and creator ORCIDs agree across JSON-LD, Dublin Core meta, the link set and DataCite for every committed project, and every writer runs without panicking over the whole corpus (test)
- [ ] `just fair-check` against a local `just dev` started with `DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000` scores F-UJI at or above 12 of 24 for project 0862, with F2, F4-01M-1, I1, I2, I3 and R1.1 passing
- [ ] FAIR Champion on DEV passes LicenseStrong, LicenseWeak, QualifiedRefs and MetadataIdentifierFound for project 0862
- [ ] `just check` and `just test` pass on every phase commit; documentation listed in each phase is updated
- [ ] Every phase's reviewer pass has run on that phase's diff before the next phase started, and every finding is either amended into the phase's commit or dismissed with a reason

## Dependencies & Risks

- **Cross-repo dependency on ops-deploy** for `DPE_PUBLIC_BASE_URL` (H1). Until set, DEV pages point `describedby` links at production. Acceptable for a short window; verification is gated on it.
- **F-UJI merges metadata from several sources with a priority order.** JSON-LD says `Dataset`, DataCite JSON says `Project`. If DataCite wins the merge, F-UJI's resource-type test can regress after Phase 3. Check the merge order in `fuji_server/helper/metadata_mapper.py` during Phase 3 and, if needed, drop the DataCite JSON `describedby` from the landing page while keeping the resource. Do not change the DataCite mapping.
- **F-UJI's DataCite JSON parser expects an object** (its log shows it failing on bytes). The writer must produce the flat kernel-4 JSON shape, and the Phase 3 test must exercise F-UJI's own reading of it via `just fair-check`.
- **F-UJI version drift.** The baseline is 3.5.0; the current container is 4.x. Record the version with every result.
- **Phase 2's `describedby` targets are OAI envelopes.** Accurate as `application/xml`, but a harvester wanting bare DataCite gets it only from Phase 3. Do not claim full Signposting Level 1 before Phase 3 ships.
- **Project-level data pointer.** F3, A1-03D and FAIR Champion's data-identifier tests need a data identifier. Projects without records have none, and no project-level download exists. This is a residual, not a defect; record landing pages are the follow-up.
- **JSON-LD size** for record-heavy projects on the standalone resource. Mitigation: per-IP limiter reuse if measured to matter.
- **Maud formatting.** `maudfmt` rejects a non-trivial `html!` block passed directly as a function argument; keep the head extras as a local binding.
- **No RDF library in the tree.** Turtle is last and droppable for that reason.
- **Google Dataset Search** requires `Dataset` root, `name` and `description`; indexing is not under our control and is not an acceptance criterion.

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

## References

- Landing page handler and document shell: `modules/dpe/server/src/main.rs:66-89`, `modules/dpe/server/src/view.rs:10-64`
- JSON handler pattern to mirror for 400/404: `modules/dpe/server/src/fragments.rs:196-209`
- Header tuple response precedent: `modules/dpe/api-oai/src/handlers/mod.rs:75`
- DataCite mapping and helpers to move: `modules/dpe/api-oai/src/metadata/datacite.rs:20`, `helpers.rs`, `resolve.rs:68-83`, `types.rs:24-43`
- Contributor lookup: `modules/dpe/core/src/contributors.rs:32-49`
- Project cache and record cache: `modules/dpe/core/src/project_cache.rs:23-32`, `modules/dpe/core/src/record_cache.rs:21-23`
- ARK construction: `modules/platform/metadata/src/record.rs:29-42`
- Placeholder and multilingual helpers: `modules/platform/metadata/src/utils.rs:16-35`
- Deliberate no-negotiation precedent to reconcile: `modules/dpe/server/src/downloads.rs:56-62,152-170`
- Architecture statement to carve out: `docs/src/dpe/architecture.md:72`
- Handler test harness: `modules/dpe/server/src/fragments.rs:291-320`, `modules/dpe/server/src/router.rs:293-334`
- FAIR Signposting profile: https://signposting.org/FAIR/
- Science-on-Schema.org Dataset guide: https://github.com/ESIPFed/science-on-schema.org/blob/main/guides/Dataset.md
- Google Dataset structured data: https://developers.google.com/search/docs/appearance/structured-data/dataset
- schema.org `license` domain: https://schema.org/license
- F-UJI methods and source: https://www.f-uji.net/index.php?action=methods, https://github.com/pangaea-data-publisher/fuji (container `ghcr.io/pangaea-data-publisher/fuji`, port 1071; cache key in `fuji_server/helper/request_helper.py` is final URL plus content type)
- OSTrails FAIR Champion tests: https://tools.ostrails.eu/champion/tests/
- DataCite content resolver and JSON media type: https://support.datacite.org/docs/datacite-content-resolver
- Baseline assessment results (F-UJI 3.5.0, FAIR Champion 1.1.11) recorded 2026-09-15 for `https://ark.dasch.swiss/ark:/72163/1/0862`; Linear DEV-7268
