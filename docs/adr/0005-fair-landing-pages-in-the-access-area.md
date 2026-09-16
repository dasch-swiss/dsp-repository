---
status: accepted
date: 2026-09-16
---

# Every landing page in the Access Area is FAIR-assessable by machine

Every page in the Access Area that a persistent identifier resolves to — today the DPE project page `/dpe/projects/{shortcode}`, later record pages and whatever CPE publishes — carries its metadata in a form a machine reads without a human and without executing a client: standards-shaped, in the served HTML and in the HTTP headers, with dedicated URLs for each machine-readable representation. Repository FAIRness is the differentiator DaSCH claims against general-purpose repositories such as Zenodo and SWISSUbase, and a landing page that a FAIR assessor scores at 3 of 24 (F-UJI 3.5.0 against project 0862, 2026-09-15: no JSON-LD, no `<meta>`, no `Link` header, the same HTML for every `Accept`) undercuts that claim; the metadata already exists and is already mapped for OAI-PMH, so the work is exposure, not modelling (rationale as stated in the FAIR metadata exposure plan, DEV-7268). The decision, phrased so a violation is describable in code:

- **The persistent identifier resolves to the landing page, and the resolver stays a plain redirect.** The ARK resolver passes the request through unchanged; everything below happens in the Access Area's response. The landing page's `cite-as` is the ARK, exactly once.
- **The served HTML embeds the metadata.** A single `<script type="application/ld+json">` with a schema.org graph, Dublin Core `<meta>` tags, and `<link>` elements mirroring the Signposting relations, all rendered on the server into the page head. No placeholder value (`MISSING`, `CALCULATED`) ever appears in them.
- **The HTTP response carries FAIR Signposting.** A `Link` header with `cite-as`, `type`, `describedby` (typed with the media type of each target), `license` when exactly one applies, and `author` for creators with an ORCID; identical on `GET` and `HEAD`. Header values are URIs built from the resolved object's canonical identifiers, never from the raw path segment and never from free text.
- **Each machine-readable representation is its own URL beside the page** — JSON-LD, DataCite JSON, and Turtle if it earns its place — answering with its own media type and a `Link: rel="describes"` back to the page. The landing page's `GET` is never rendered differently by header; the one negotiation step is a `303 See Other` from the landing page to the matching dedicated URL when the client's `Accept` prefers a supported non-HTML type, with `Vary: Accept` on every landing-page response.
- **One resolved graph per object feeds every representation.** The mapping lives in one library crate; JSON-LD, Dublin Core, the Signposting link set and DataCite are written from the same resolved intermediate and share the same helpers for multilingual preference, placeholder filtering, creator fallback and temporal-coverage resolution, so they cannot disagree. A corpus-wide test asserts the agreement for every committed object.
- **The schema.org root of a project page is `Dataset`**, describing the project's data holdings, with the research project attached as a `producer` of type `ResearchProject`; the DataCite mapping keeps `resourceTypeGeneral="Project"`. The two vocabularies do not need to agree, and a `ResearchProject` root cannot carry the properties assessors test.
- **Nothing is invented for a score.** A project has no project-level download, so no `distribution` is emitted; record pages are the right target for data-level tests. Residuals that a code change cannot fix (DataCite registration keyed on a DOI prefix, a metadata persistence policy, search-engine indexing) are recorded as such, not worked around.
- **FAIRness is measured, not asserted.** F-UJI runs locally and against DEV through a `just` recipe; FAIR Champion is run by hand; every result is recorded with the assessor's version, against the 2026-09-15 baseline.

## Considered Options

- **Embedded metadata plus Signposting plus dedicated representation URLs (chosen).**
- **Render JSON-LD or DataCite inline from the landing page URL when `Accept` asks for it** — rejected: it contradicts the rule that a page is never rendered differently by header (ADR-0004), leaves the machine representations without stable URLs to put in `describedby`, and forces every error path to be typed per media type. The redirect keeps the URLs canonical and the carve-out to one line.
- **Serve machine metadata only through OAI-PMH** — insufficient: assessors and search engines read the landing page the identifier resolves to, not a harvesting endpoint; OAI-PMH stays and is linked as one `describedby` target.
- **Type the project page `ResearchProject` and attach `CreativeWork` properties anyway** — rejected: semantically invalid schema.org that validators and the resource-type test both reject.
- **Type the project page `DataCatalog`** — rejected as the root: assessors and search engines treat a catalog as a container, so dataset-level tests keep failing; a `DataCatalog` node for the whole DPE is still emitted as `includedInDataCatalog`.
- **Make the ARK resolver negotiate content** — rejected: the resolver is a redirect and stays one; both assessors read the final response.

## Consequences

- The DataCite mapping and its helpers leave the OAI crate for a shared formats crate that the OAI crate and the server both depend on; the OAI XML output stays byte-identical across the move.
- The landing page's document shell gains a head-extras slot, and the JSON-LD `<script>` becomes the second sanctioned `PreEscaped` site after the Mosaic icon SVG, with `<`, `>` and `&` neutralised as JSON unicode escapes before splicing.
- Signposting targets must be absolute, so the Access Area's binary learns its public base URL from configuration and threads it through application state, never a process-global.
- Every new landing page in the Access Area — record pages, CPE presentations — inherits these obligations; a page that lacks them is a defect, not a later feature.

Enforced by: the corpus-wide representation-agreement test and the handler tests on `Link`, `Vary` and the `303` decision table (static-analysis); `just fair-check` against the 2026-09-15 baseline before a change to a landing page is called done (review). Both arrive with the plan that implements this decision (DEV-7268); until it lands, none (docs-only).
