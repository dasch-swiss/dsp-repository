# Machine-Readable Metadata

A project's **landing page** — `/dpe/projects/{shortcode}`, the page the project's
ARK resolves to — carries its metadata in the served HTML, so that a FAIR
assessor or a harvester reading the page gets the same facts a person does.

Three things are emitted, all built from one resolved graph per project:

| What | Where | Vocabulary |
|------|-------|------------|
| A JSON-LD block | `<script type="application/ld+json">` at the end of `<head>` | schema.org `Dataset` |
| Meta tags | `<meta name="DC.*">` in `<head>` | Dublin Core elements, plus a COAR access-right URI |
| Typed links | `<link>` elements in `<head>` and an HTTP `Link` header | FAIR Signposting, Level 1 |

The graph is `shared_fair::ProjectGraph`. Every representation reads it and none
re-derives a fact from the source data, which is what keeps them from disagreeing
(ADR-0005, `docs/adr/0005-fair-landing-pages-in-the-access-area.md`). A
corpus-wide test in `dpe-api-oai` asserts that agreement over every committed
project.

## What is emitted

### schema.org JSON-LD

The project is a `Dataset` whose `@id` and `identifier` are its ARK, `url` its
landing page, and `hasPart` a list of its records. `publisher` is DaSCH;
`producer` is a `ResearchProject` node carrying the project's own name, dates,
external website and members.

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
| `describedby` | the OAI `GetRecord` URLs for `oai_datacite` and `oai_dc`, typed `application/xml` | 2 |
| `license` | the SPDX URI | 0 or 1: only when the project records exactly one distinct URI, because the profile allows no more. The JSON-LD still lists them all |
| `author` | the ORCID of each credited creator | 0 or more |

The `describedby` targets return an OAI-PMH envelope around the DataCite or
Dublin Core payload, so `application/xml` is the accurate type for them. A
harvester wanting bare DataCite does not get it from these links.

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

## Assessment results

Scores for project 0862, the reference project.

| Date | Assessor | Version | Target | Result |
|------|----------|---------|--------|--------|
| 2026-09-15 | F-UJI | 3.5.0 | `https://ark.dasch.swiss/ark:/72163/1/0862` (PROD) | 3 of 24 (12.5%). F 2/7, A 1/3, I 0/4, R 0/10; only F1 and A1-02M passed. Baseline, before this work |
| 2026-09-15 | FAIR Champion | 1.1.11 | same | 6 of 15 pass, of which 2 hollow ("linked data found", 0 of 0 triples); 6 fail, 3 indeterminate. Baseline, before this work |

F-UJI is run from a pinned image. The mapping this work targets was read out of
`ghcr.io/pangaea-data-publisher/fuji@sha256:3eca94076b2272a18dcd8cddd1adad7675c2ac98a390799dd60f62369688a6fa`:
its schema.org mapping reads the object identifier from `identifier.value`,
which is why `identifier` is emitted as a `PropertyValue`.

### Known residuals

These fail by design, because the alternative would be to state something untrue.

- **No project-level `distribution`.** There is no project-level download. F-UJI
  F3 and A1-03D want one; record landing pages are the right assessment target
  for those tests.
- **No DataCite registration.** The ARKs are DaSCH's own and are not registered
  with DataCite, so tests that resolve metadata through a DOI registry cannot
  pass.
- **No metadata persistence policy URL.** A `persistencePolicy` link needs a
  published policy to point at.
