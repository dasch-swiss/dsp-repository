# Every landing page in the Access Area is FAIR-assessable by machine

Every page in the Access Area that a persistent identifier resolves to — the DPE project page
`/dpe/projects/{shortcode}`, record pages, and the pages CPE publishes — carries its metadata in a form a machine
reads without a human and without executing a client: standards-shaped, in the served HTML and in the HTTP
headers, with dedicated URLs for each machine-readable representation. Repository FAIRness is the differentiator
DaSCH claims against general-purpose repositories such as Zenodo and SWISSUbase, and a landing page that a FAIR
assessor scores at 3 of 24 (F-UJI 3.5.0 against project 0862, the 2026-09-15 baseline: no JSON-LD, no `<meta>`,
no `Link` header, the same HTML for every `Accept`) undercuts that claim. The metadata already exists and is
already mapped for OAI-PMH, so the work is exposure, not modelling. The decision, phrased so a violation is
describable in code:

- **The persistent identifier resolves to the landing page, and the resolver stays a plain redirect.** The ARK
  resolver passes the request through unchanged; everything below happens in the Access Area's response. The
  landing page's `cite-as` is the ARK, exactly once.
- **A deployment answers the ARKs it publishes.** A deployment that is not the one the recorded ARKs resolve to,
  such as a PR preview, sets `DPE_ARK_RESOLVER_BASE_URL`. It rewrites the host of every emitted identifier ARK
  to the deployment's own and mounts a plain redirect resolver that answers it, so an assessor harvesting the
  preview assesses the preview. The rewrite happens once, where corpus data enters the Access Area, so no reader
  knows a resolver exists and `shared-fair` reads only the graph it is handed. The ARK path is the identifier
  and is never touched; the host in front of it is a fact about the deployment, like `DPE_PUBLIC_BASE_URL`. An
  ARK the corpus records as project data rather than as the object's identifier (project 083D records its own
  ARK as the project's website) is not rewritten, because that would change a recorded fact.
- **The served HTML embeds the metadata.** A single `<script type="application/ld+json">` with a schema.org
  graph, Dublin Core `<meta>` tags, and `<link>` elements mirroring the Signposting relations, all rendered on the
  server into the page head. No placeholder value (`MISSING`, `CALCULATED`) ever appears in them.
- **The HTTP response carries FAIR Signposting.** A `Link` header with `cite-as`, `type`, `describedby` (typed
  with the media type of each target), `license` when exactly one applies, and `author` for creators with an
  ORCID; identical on `GET` and `HEAD`. Header values are URIs built from the resolved object's canonical
  identifiers, never from the raw path segment and never from free text.
- **Each machine-readable representation is its own URL beside the page** — JSON-LD and DataCite JSON —
  answering with its own media type and a `Link: rel="describes"` back to the page. The landing page's `GET` is
  never rendered differently by header; the one negotiation step is a `303 See Other` from the landing page to
  the matching dedicated URL when the client's `Accept` prefers a supported non-HTML type, with `Vary: Accept` on
  every landing-page response.
- **One resolved graph per object feeds every representation.** The graph of a project or a record — its facts
  after agent resolution, placeholder filtering, multilingual preference, creator fallback and temporal-coverage
  resolution — is built once, by one builder over the contract types, and every representation (JSON-LD, Dublin
  Core, the Signposting link set, DataCite XML and JSON, the OAI-PMH output) is a projection of that graph, so
  they cannot disagree. Graph types, builders and writers live in one shared crate, `shared-fair`, which knows no
  area's store, routes or URLs: a capability adapts its own store through the contributor lookup and the
  temporal tables the builder accepts, and passes its URL layout in as options; the routes stay with the
  capability. A corpus-wide test asserts the agreement for every committed object.
- **The schema.org root of a project page is `Dataset`**, describing the project's data holdings, with the
  research project attached as a `producer` of type `ResearchProject`; the DataCite mapping keeps
  `resourceTypeGeneral="Project"`. The two vocabularies do not need to agree, and a `ResearchProject` root cannot
  carry the properties assessors test. A `DataCatalog` node for the whole DPE is emitted as
  `includedInDataCatalog`.
- **Nothing is invented for a score.** No representation asserts a fact the corpus does not record. A project
  whose records carry no file emits no `distribution`, and none is fabricated for it. A project whose records do
  carry files describes them: one `DataDownload` per file, under the licence and the access level that record
  itself declares, which reports what exists rather than inventing it; a file the corpus does not record as
  fully open is never advertised. Record pages are the right target for record-level tests. Residuals that a
  code change cannot fix (DataCite registration keyed on a DOI prefix, a metadata persistence policy,
  search-engine indexing) are recorded as such, not worked around.
- **What a representation leaves out is bounded, not chosen.** `/metadata.jsonld` is capped at 4,000,000 bytes,
  a margin under the 5,000,000 at which F-UJI truncates a download. The bound is assessor-shaped, and it is
  stated as such. It is on the right side of the rule above because a consumer that truncates at its size limit
  gets an unparseable document, not a lower score, and because nothing false is added: the document does not
  claim to be complete, and where the complete list lives is documented. Choosing which records to include so
  as to move a metric stays ruled out, so `hasPart` and `distribution` share one prefix in document order rather
  than the budget being spent on the downloads an assessor rewards.
- **FAIRness is measured, not asserted.** F-UJI runs locally and against DEV through a `just` recipe; FAIR
  Champion is run by hand; every result is recorded with the assessor's version, against the baseline above.

How FAIR assessment of a project before submission works in the Deposit Area is not decided here; it gets its
own decision record when it is designed. `shared-fair` is the seed of that engine.

## Considered Options

- **Embedded metadata plus Signposting plus dedicated representation URLs (chosen).**
- **Render JSON-LD or DataCite inline from the landing page URL when `Accept` asks for it** — rejected: it
  contradicts the rule that a page is never rendered differently by header (ADR-0004), leaves the machine
  representations without stable URLs to put in `describedby`, and forces every error path to be typed per
  media type. The redirect keeps the URLs canonical and the carve-out to one line.
- **Serve machine metadata only through OAI-PMH** — insufficient: assessors and search engines read the landing
  page the identifier resolves to, not a harvesting endpoint; OAI-PMH stays and is linked as one `describedby`
  target.
- **Type the project page `ResearchProject` and attach `CreativeWork` properties anyway** — rejected:
  semantically invalid schema.org that validators and the resource-type test both reject.
- **Type the project page `DataCatalog`** — rejected as the root: assessors and search engines treat a catalog as
  a container, so dataset-level tests keep failing.
- **Make the ARK resolver negotiate content** — rejected: the resolver is a redirect and stays one; both
  assessors read the final response.
- **Serve previews from a `*.dasch.swiss` host** — rejected: it would satisfy F-UJI's domain comparison while
  still sending the assessor to production, which is gaming the metric.
- **Keep the graph and the writers in a `dpe-*` crate until a second consumer appears** — rejected: the
  consumers are named (record pages and CPE in the Access Area, the Deposit Area for assessment before
  submission), a `dpe-*` crate would bind the vocabularies to one capability's view model, and ADR-0003 forbids
  the sibling import CPE would need. A shared crate that depends on the contract alone costs the same and is
  what a port can speak.
- **A Turtle representation** — *deferred*. It comes back when a consumer needs it; until then a `text/turtle`
  `Accept` gets the HTML page (`shared-fair`'s `negotiate` module).

## Consequences

- `shared-fair` is a shared crate although DPE is its only consumer, because the decision names the next ones:
  record pages and CPE in the Access Area, and the Deposit Area, where a depositor assesses the FAIRness of a
  project before it is submitted (ADR-0002, `shared/`).
- Each capability that serves a landing page owns three things and only three: a builder adapter (its store
  behind the lookup and tables `shared-fair` accepts, its objects as contract types), its route handlers, and
  its URL layout. Nothing in `shared-fair` knows where it is rendered.
- The landing page's document shell has a head-extras slot, and the JSON-LD `<script>` is a sanctioned
  `PreEscaped` site beside the Mosaic icon SVG and the textarea's leading newline, with `<`, `>` and `&`
  neutralised as JSON unicode escapes before splicing.
- Signposting targets must be absolute, so the Access Area's binary learns its public base URL from
  configuration and threads it through application state, never a process-global.
- Every new landing page in the Access Area — record pages, CPE presentations — inherits these obligations; a
  page that lacks them is a defect, not a later feature. Which ARKs resolve to CPE, and when, is ADR-0007's.

Not yet in code: DPE serves no record pages; `/dpe/records/{shortcode}/{record_id}/file` is a download, not a
landing page (not scheduled).
Not yet in code: the ARK host rewrite runs as corpus data enters `dpe-core`'s caches; it belongs to `sync`,
where corpus data enters the Access Area once DPE reads through it (ADR-0003) (not scheduled).
Not yet in code: no Deposit-Area crate depends on `shared-fair` (not scheduled).

Enforced by, clause by clause: **one resolved graph feeding every representation** —
`every_representation_of_a_committed_object_agrees_with_the_others` in `dpe-api-oai`, which also validates the
DataCite JSON against DataCite's own schema (static-analysis); **the headers and the one negotiation step** —
the `dpe-server` handler tests `a_landing_page_carries_the_metadata_and_the_link_header`,
`the_page_links_every_representation_it_serves`, `every_landing_page_answer_varies_on_accept`,
`a_harvester_asking_for_a_representation_is_redirected_to_it` and
`an_unknown_shortcode_never_redirects_whatever_it_was_asked_for`, over the decision table in `shared-fair`'s
`negotiate` module (static-analysis); **the bound on `/metadata.jsonld`** — the `JSON_LD_BYTE_BUDGET` test in
`dpe-server` (static-analysis); **`shared-fair` holding no hardcoded path into an area**, in its sources and its
test fixtures alike — `.github/scripts/check-shared-paths.sh`, run by `just check` (static-analysis);
**FAIRness being measured rather than asserted** — the `just fair-check` step in `REVIEW.md`, run against
projects 0862 and 0868 before a change to a landing page or to `shared-fair` is called done (review). Two
projects, because no committed record dump exists for 0862: it can never emit a `distribution`, so run against
it alone the gate cannot see the file-describing paths at all. 0868 carries 7,716 files and exercises them; 0862
stays as the fast control that must not move.
