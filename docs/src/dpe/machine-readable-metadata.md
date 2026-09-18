# Machine-Readable Metadata

A project's **landing page** — `/dpe/projects/{shortcode}`, the page the project's
ARK resolves to — carries its metadata in the served HTML, so that a FAIR
assessor or a harvester reading the page gets the same facts a person does.

Three things are emitted in the page, and two **machine-readable
representations** are served beside it, all built from one resolved graph per
project:

| What | Where | Vocabulary |
|------|-------|------------|
| A JSON-LD block | `<script type="application/ld+json">` at the end of `<head>` | schema.org `Dataset` |
| Meta tags | `<meta name="DC.*">` in `<head>` | Dublin Core elements, plus a COAR access-right URI |
| Typed links | `<link>` elements in `<head>` and an HTTP `Link` header | FAIR Signposting, Level 1 |
| A JSON-LD document | `GET /dpe/projects/{shortcode}/metadata.jsonld` | schema.org `Dataset`, `hasPart` uncapped |
| A DataCite document | `GET /dpe/projects/{shortcode}/metadata.datacite.json` | DataCite kernel 4 JSON |

The graph is `shared_fair::ProjectGraph`. Every representation reads it and none
re-derives a fact from the source data, which is what keeps them from disagreeing
(ADR-0005, `docs/adr/0005-fair-landing-pages-in-the-access-area.md`). A
corpus-wide test in `dpe-api-oai` asserts that agreement over every committed
project.

## What is emitted

### schema.org JSON-LD

The project is a `Dataset` whose `@id` is its ARK, `url` its landing page, and
`hasPart` a list of its records. `publisher` is DaSCH; `producer` is a
`ResearchProject` node carrying the project's own name, dates, external website
and members.

Two properties have a shape worth naming, both of them because an assessor reads
them as RDF rather than as JSON:

| Property | Shape | Why |
|----------|-------|-----|
| `identifier` | always two entries: the ARK as a `PropertyValue` with `propertyID: "ARK"`, and the landing page URL as a plain string | F-UJI reads the object identifier from `identifier.value`, so the ARK keeps the `PropertyValue` form. FAIR Champion's *MetadataIdentifierFound* reads `schema:identifier` alone and does not consider `url`, so an assessor pointed at the page needs the page's own URL here too |
| `license` | a node object, `{"@id": "<SPDX URI>"}`; one object for a single licence, an array for several, no key for none | schema.org's remote context does not coerce `license` to `@id`, so a bare string parses as an RDF literal. FAIR Champion's *LicenseStrong* wants a Resource |
| `prov:wasAttributedTo` | references to agents the graph already names: DaSCH, and every credited creator with an ORCID | PROV-O is the W3C vocabulary for provenance, and `creator` and `publisher` already assert exactly these agents. See *Provenance* below |

An agent node carries an `@id` when it has an IRI of its own — its ORCID, by the
same rule Signposting's `author` link uses — so a statement about that agent
points at the node rather than describing a second one. The publisher's `@id` is
`https://dasch.swiss`. An agent without an ORCID stays a blank node.

### Provenance

The `@context` binds `prov:` to `http://www.w3.org/ns/prov#`, and the graph
asserts `prov:wasAttributedTo` over DaSCH and the credited creators that have an
ORCID.

Nothing here is a new fact. `creator` and `publisher` already name those agents;
PROV-O restates that relation in the vocabulary a consumer asking about
provenance reads. That is the line ADR-0005's "nothing is invented for a score"
draws: re-expressing a true statement in a second standard vocabulary is
interoperability, and asserting a fact the corpus does not record would not be.

Two things are deliberately not emitted:

- **`prov:generatedAtTime`.** F-UJI's own mapping treats `publication_date` that
  way, but what the corpus records is a publication *year*, not a generation
  time, so the statement would be approximately true at best.
- **Attribution to a creator with no ORCID.** There is no node to point at, and
  inlining a copy would put a second, unidentified agent in the graph — the
  opposite of what the statement means. Naming some agents does not deny the
  others, so the shorter statement stays true.

> schema.org's `producer` is the research project that made the data. It is not
> the OAIS Producer of the Deposit Area, which is the depositing agent.

Three rules hold throughout:

- **Nothing is invented.** A value the corpus records as a Placeholder, or does
  not record at all, yields no key. There is no project-level `distribution`,
  because there is no project-level download.
- **`hasPart` is capped at 100 in the embedded block.** The largest committed
  project has 27,026 records. The complete list is harvestable from the OAI set
  `project:{shortcode}`.
- **Key order is the builder's.** The workspace enables `serde_json`'s
  `preserve_order`, so insertion order is emission order. `Map::remove`
  silently re-sorts the map and must not be used; `retain` or `shift_remove`
  instead.

### Dublin Core meta tags

A second rendering of the Dublin Core record the `oai_dc` metadata prefix already
serves, not a second mapping. Two differences, both deliberate:

- Placeholders are dropped. The committed `oai_dc` output carries them through,
  but a landing page telling a harvester `DC.title = "MISSING"` states something
  false.
- `DC.description` is cut to 1000 characters, on a character boundary. A `<meta>`
  attribute of several kilobytes is legal and pointless.

`DC.accessRights` is the one value the OAI record has no field for: the COAR
access-right URI for the project's access level.

| Access level | `isAccessibleForFree` | `conditionsOfAccess` | `DC.accessRights` |
|--------------|-----------------------|----------------------|-------------------|
| Full Open Access | `true` | `Full Open Access` | `http://purl.org/coar/access_right/c_abf2` |
| Open Access with Restrictions | `false` | `Open Access with Restrictions` | `http://purl.org/coar/access_right/c_16ec` |
| Embargoed Access | `false` | `Embargoed Access until {date}`, or `Embargoed Access` | `http://purl.org/coar/access_right/c_f1cf` |
| Metadata only Access | `false` | `Metadata only Access` | `http://purl.org/coar/access_right/c_14cb` |

### Signposting

The same set of typed links goes out twice, as `<link>` elements and as an RFC
8288 `Link` header.

| Relation | Target | Cardinality |
|----------|--------|-------------|
| `cite-as` | the project's ARK | exactly 1 |
| `type` | `https://schema.org/Dataset`, `https://schema.org/AboutPage` | 2 |
| `describedby` | the two representations, typed `application/ld+json` and `application/vnd.datacite.datacite+json`, then the OAI `GetRecord` URLs for `oai_datacite` and `oai_dc`, typed `application/xml` | 4 |
| `license` | the SPDX URI | 0 or 1: only when the project records exactly one distinct URI, because the profile allows no more. The JSON-LD still lists them all |
| `author` | the ORCID of each credited creator | 0 or more |

Each representation answers with the other half of that: `Link: <landing page>;
rel="describes"`.

The OAI `describedby` targets return an OAI-PMH envelope around the DataCite or
Dublin Core payload, so `application/xml` is the accurate type for them. A
harvester wanting bare DataCite gets it from `metadata.datacite.json`.

## The representations, and the one negotiation step

```
GET /dpe/projects/{shortcode}/metadata.jsonld        → application/ld+json
GET /dpe/projects/{shortcode}/metadata.datacite.json → application/vnd.datacite.datacite+json
```

The JSON-LD one is the same schema.org graph the page embeds, with `hasPart`
**uncapped** — every record, where the embedded block stops at 100. The DataCite
one is the same record the `oai_datacite` prefix serves, in DataCite's kernel-4
JSON shape rather than inside an OAI envelope. Its output is validated against
DataCite's own JSON schema for every committed project by a corpus-wide test.

A malformed shortcode is `400` and an unknown one `404`, both `text/plain` with
an empty body: a client that asked for JSON-LD is a machine, and the content
type is how it learns that what came back is not the document it asked for.

Both routes share the per-IP bucket `/dpe/oai` uses, under the same
`DPE_OAI_RATE_LIMIT_*` settings, because the uncapped JSON-LD for the largest
committed project is a few megabytes built in memory per request. That limit
bounds the request *rate*, not the per-request size; see
[Operations](./operations.md) for what a burst costs. Neither route sends
`Cache-Control` or `ETag`, consistent with the rest of DPE: the data changes
only on deploy.

**The landing page redirects to a representation when `Accept` prefers one.**
This is the single negotiation step ADR-0005 allows, and the only thing about
the route that varies by header — the page itself is byte-identical for every
`Accept` value (ADR-0004, `docs/adr/0004-hypermedia-frontends.md`).

| `Accept` | Result |
|----------|--------|
| absent, `*/*`, `text/html,*/*;q=0.8` | 200 HTML |
| `application/ld+json` | 303 to `metadata.jsonld` |
| `application/vnd.datacite.datacite+json` | 303 to `metadata.datacite.json` |
| `text/turtle`, `application/json`, `application/xml`, `application/*` | 200 HTML — these are not aliases; each representation is served for its own media type and no other |
| `text/html;q=1, application/ld+json;q=1` | 200 HTML — a tie goes to the page |
| `*/*;q=0`, malformed, unparseable `q` | 200 HTML |
| unknown shortcode, any `Accept` | 200 HTML "Project Not Found", never a redirect |

The table above is the acceptance specification. The rules that produce it —
how `q` is read, how wildcards count, what a hostile header costs — are written
out once, in `shared-fair`'s `negotiate` module, and are not restated here. The
route never produces a 4xx from `Accept`.

**Every answer from the route carries `Vary: Accept`** — 200 and 303, GET and
HEAD. Without it a cache in front of DPE could replay a 303 to a person or the
HTML to a harvester.

The candidate list and the representation `describedby` links come from the
same `UrlLayout` rows, so the page cannot redirect to a representation it does
not link. The converse does not hold: the two OAI-record `describedby` links are
never candidates, because a harvester is pointed at them rather than redirected
to them.

## Two base URLs

`DPE_PUBLIC_BASE_URL` gives the site's own URLs — the landing page and the
catalogue. `DPE_OAI_BASE_URL` gives the `describedby` targets, because that is
the URL the OAI endpoint advertises for itself. Neither is derived from the
other: on DEV the OAI endpoint answers on a different host. See
[Operations](./operations.md).

## How it is kept safe

Machine-readable metadata puts corpus data into HTML and into HTTP headers. The
rules that govern that — identifiers derived from the resolved project and never
from the request path, and the JSON-LD `<script>` as a sanctioned `PreEscaped`
site whose only permitted input is `shared_fair::script_safe_json` — are
ADR-0005's decision (`docs/adr/0005-fair-landing-pages-in-the-access-area.md`),
and are not restated here. What the implementation adds to them:

- **The grep is crate-wide.** A test in `dpe-server` reads every file under its
  own `src/` and asserts `PreEscaped(` appears exactly once in the whole crate,
  on the JSON-LD splice. Scoped to `metadata.rs` it could never have seen the
  site the rule actually worries about, the search-query echo in `fragments.rs`.
- **A rejected `Link` header is dropped, not panicked.** A control character in
  a recorded PID makes `HeaderValue::from_str` refuse the value; the header goes
  with a `tracing::warn!` naming the shortcode, and the page stays. The same
  links are in the head either way, and a persistent failure is a data-quality
  bug that has to surface in logs rather than take the page down.
- **RFC 8288 field syntax is percent-encoded out of every href.** `<`, `>`, `"`,
  `;`, `,` and space are encoded when the header is serialized, so a curator-
  entered ARK or license URI cannot end a link-value and start another. It is
  done at serialization, so the identifier in the JSON-LD and the `<link>`
  elements stays what the corpus records.
- **Casing changes nothing.** The shortcode lookup is case-insensitive, so
  `/dpe/projects/080c` and `/dpe/projects/080C` produce byte-identical JSON-LD
  and `Link` headers.

A project that does not resolve gets nothing: no JSON-LD, no `Link` header, no
redirect. The existing always-200 "Project Not Found" body is unchanged.

## Measuring it: `just fair-check`

```
just fair-check <url> [min_score]
```

Starts F-UJI in a container, POSTs the URL to its evaluation API, prints the
per-metric table with the assessor's `software_version` and the image digest,
and exits non-zero when the total is below `min_score`. The exit code is the
check, so a reviewer gets a pass or a fail rather than a table to read.

Against a local server:

```
DPE_PUBLIC_BASE_URL=http://host.docker.internal:4000 DPE_SITE_ADDR=0.0.0.0:4000 just dev
just fair-check http://host.docker.internal:4000/dpe/projects/0862 12
```

Both environment variables are needed. `DPE_PUBLIC_BASE_URL` makes the page's
own links point at a host the container can resolve, and `DPE_SITE_ADDR` binds
the server on all interfaces — the default `127.0.0.1` is not reachable from a
container under a VM-backed Docker runtime such as colima.

Notes on the recipe:

- **The image is pinned by digest**, not by `:latest`. A score is only
  comparable against the rows below if the assessor is the same build. Bumping
  the pin is a reviewed change, made when F-UJI publishes a release and checked
  at least with every change to a landing page — the same discipline as the
  Tailwind CLI pin in [Security](../security.md). There is no
  `tailwind-pins-refresh`-style refresh recipe, because `fair-check` never runs
  in CI.
- **The container can reach the host's port 4000** during the run, through
  `--add-host=host.docker.internal:host-gateway`. That is the point: F-UJI
  fetches the URL it is given. The container is published on `127.0.0.1` only
  and is destroyed when the recipe returns, because an API that fetches
  arbitrary URLs should not be listening on the developer's network.
- **`marvel` / `wonderwoman` are F-UJI's own published defaults**
  (`fuji_server/config/users.py`), not a DaSCH credential. They are in the
  recipe because the container is loopback-only and short-lived.
- **`DOCKER_CONFIG` points at a scratch directory** holding a bare `{}`. A
  developer's own `~/.docker/config.json` may name a credential helper that is
  not on `PATH` under the active context, and `docker pull` then fails on a
  public image that needs no credentials at all.
- **`jq` is required**, and is in the Nix dev shell and `just install-requirements`.

`REVIEW.md` carries this as a checklist step: a change to a landing page or to
`shared-fair` runs it against project 0862 and records the result.

## Assessment results

Scores for project 0862, the reference project.

| Date | Assessor | Version | Target | Result |
|------|----------|---------|--------|--------|
| 2026-09-15 | F-UJI | 3.5.0 | `https://ark.dasch.swiss/ark:/72163/1/0862` (PROD) | 3 of 24 (12.5%). F 2/7, A 1/3, I 0/4, R 0/10; only F1 and A1-02M passed. Baseline, before this work |
| 2026-09-15 | FAIR Champion | 1.1.11 | same | 6 of 15 pass, of which 2 hollow ("linked data found", 0 of 0 triples); 6 fail, 3 indeterminate. Baseline, before this work |
| 2026-09-18 | F-UJI | 3.5.0 (`sha256:3cde9d30bc14…`) | `http://host.docker.internal:4000/dpe/projects/0862` (local `dpe-server serve`) | **14 of 24**, against a target of 12. F1-01D 1/1, F2-01M 2/2, F4-01M 1/2, A1-01M 1/1, A1-02M 1/1, I1-01M 2/2, I3-01M 1/1, R1-01MD 1/4, R1.1-01M 2/2, R1.2-01M 1/2, R1.3-01M 1/1. Failing: F1-02D, F3-01M, A1-03D, R1.3-02D, and I2-01M scores 0/1 |
| 2026-09-18 | F-UJI | 3.5.0 | `https://dpe-pr-391-…run.app/dpe/projects/0862` (Cloud Run PR preview) | 13 of 24. One point below the local run, and the whole difference is `I1-01M-2`: the preview did not set `DPE_PUBLIC_BASE_URL`, so the typed links pointed at production, which does not carry this code and answered 404. Predates the `identifier`, `license` and workflow fixes |
| 2026-09-18 | FAIR Champion | 1.1.11 | same preview | 7 of 15 passing. *LicenseStrong* and *MetadataIdentifierFound* among the failures; both are fixed by the `license` and `identifier` shapes above, and both predate them |
| 2026-09-18 | F-UJI | 3.5.0 (`sha256:3cde9d30bc14…`) | same local target, after the `identifier` and `license` fixes | **14 of 24 again.** A re-confirmation, not an improvement: every per-metric value matches the local row above, so neither fix cost a point and neither earned one. It was run to prove that putting `identifier` in an array did not break F-UJI's reading of the ARK — it does not, and `F1-01D` still passes |
| 2026-09-18 | F-UJI | 3.5.0 (`sha256:3cde9d30bc14…`) | same local target, after PROV-O | **16 of 24.** Two metrics moved and no others: `R1.2-01M` 1/2 → 2/2 (`Found use of dedicated provenance ontologies`) and `I2-01M` 0/1 → 1/1 (`Namespace matches found -: ['http://www.w3.org/ns/prov']`). One statement earns both — PROV is a provenance ontology *and* a vocabulary F-UJI's LOD registry lists |

F-UJI is run from a pinned image, at 3.5.0 — the version the baseline was taken
with, so the rows are comparable.

The FAIR Champion tests named on this page by their short reference —
*LicenseStrong*, *MetadataIdentifierFound*, *DataIdentifierFound* — are defined
in the [FAIR Maturity Indicators
index](https://fairmetrics.github.io/Metrics/landingpages/general/index.html),
which is where each `FM_*` indicator a FAIR Champion result cites resolves to
prose. It covers that set only; F-UJI's metrics are a separate set, cited in its
own result payload as <https://doi.org/10.5281/zenodo.6461229>.

Every row above is a local or preview measurement. Neither assessor has yet run
against a *deployment* carrying the PROV-O, `identifier`, `license` and base-URL
changes, and nothing on this page claims a score for one.

**Two representations of one project do not fight over its resource type.**
F-UJI's JSON-LD mapping reads `object_type` from schema.org's `@type`
(`Dataset`) and its DataCite JSON mapping from `types.resourceTypeGeneral`
(`Project`). It merges `object_type` as a *list* and its resource-type test
passes if **any** entry matches, so linking the DataCite representation costs
nothing there. This was read out of the pinned image, and holds in 4.0.0 as
well.

### Known residuals

Where the eight points F-UJI does not award go. The per-metric totals are the
local run that scored 16 of 24; the sub-test detail is the preview run of
2026-09-18, whose `test_debug` output records which half of a metric failed and
why. That run differs from the current one in `I1-01M-2`, `I2-01M` and
`R1.2-01M-2`, all three of which are fixed above; the sub-test evidence for the
eight points below is unaffected by all three.

| Cause | Where the points go | Points |
|-------|---------------------|--------|
| No data pointer *for this project* | F3-01M, A1-03D, R1-01MD sub-tests 2 to 4, R1.3-02D | 6 |
| ARKs are not registered with DataCite | F4-01M-2 | 1 |
| The assessment ran against a non-production host | F1-02D | 1 |

**No data pointer for this project — six points.** F-UJI distinguishes the
metadata record from retrievable *data content*, and asks for a pointer it can
fetch and inspect.

This is a fact about project 0862, not about the repository. 0862's records carry
no file, and neither do 081C's; 0868's records carry 7,716 files with public
URLs, and 0803's carry 4,062. So the six points below are what a project with no
files scores, and they are not a repository-wide ceiling.

For 0862 as measured there is no `distribution` to describe, and six points hang
off that one absence. R1-01MD-1 ("minimal information about available data content")
passes on the resource type alone. R1-01MD-2 does run and comes back with an
empty `data_content_descriptor`, because nothing populates it; R1-01MD-3 and
R1-01MD-4 cannot run at all — `NO data object content available/accessible to
perform file descriptors (type and size) tests`. A1-03D skips for the same
reason (`Skipping protocol test for data since NO content (data) identifier is
given in metadata`), and R1.3-02D reports `Could not perform file format checks
as data content identifier(s) unavailable/inaccesible`.

Naming a download that this project does not have would earn the six points and
state something untrue, which ADR-0005 rules out: nothing is invented for a
score. That is the narrow point. Surfacing the files a project *does* have is a
different thing entirely and is not invention.

Record landing pages describe objects that *do* have retrievable content, and
they are the right assessment target for all six.

**ARKs are not registered with DataCite — one point.** F4-01M-1 passes: the
metadata is offered through a harvesting endpoint. F4-01M-2 asks for
registration in a major research data registry, and the ARKs are DaSCH's own.

**The assessment ran against a non-production host — one point.** F1-02D is
about a persistent identifier, and F-UJI did find one. It harvested the ARK from
the metadata twice, once through the Signposting `cite-as` link, confirmed the
syntax as `ark` and resolved it successfully (`resolvable_status: true`, to
`https://repository.dasch.swiss/dpe/projects/0862`). It then discarded it:
`Landing page domain resolved from PID found in metadata does not match with
input URL domain -: run.app <> dasch.swiss`, followed by `PID syntax is OK but
the PID seems to resolve to a different entity, will not use this PID for
content negotiation` and finally `Could not find any persistent identifier for
metadata which complies with a known PID syntax`. The PID was found, was
well-formed and did resolve; it was rejected because the domain it resolves to
is not the domain the assessment was run against. Every 2026-09-18 run targeted a
non-`dasch.swiss` host, and the 2026-09-15 baseline that did target the ARK
predates this work, when the page carried no metadata for a PID to be harvested
from. **No run against a build carrying this work has been made from a host under
`dasch.swiss`, so F1-02D's status there is unknown.** Assessing DEV is the
post-merge step that would measure it.

Two further residuals sit outside this arithmetic, because no F-UJI metric
carries them:

- **No metadata persistence policy URL.** A `persistencePolicy` link needs a
  published policy to point at. FAIR Champion's *MetadataPersistence* asks for
  one.
- **Not indexed by a search engine.** FAIR Champion's *DiscoverableInBing*.
