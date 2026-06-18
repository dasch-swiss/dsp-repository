# OAI-PMH Endpoint

Usage guide for the DPE [OAI-PMH 2.0](https://www.openarchives.org/OAI/openarchivesprotocol.html) data provider, which exposes DaSCH research project and record metadata for harvesting. The implementation lives in the `dpe-api-oai` crate (see [Project Structure](./project_structure.md)).

## Endpoint

- **Path**: `GET /dpe/oai` — GET only; POST requests are not supported
- **Response**: always `200 OK` with `Content-Type: text/xml; charset=utf-8`. Protocol errors are reported as OAI `<error>` elements inside the XML body, not as HTTP error codes.
- **Protocol version**: 2.0

| Environment | Base URL |
|-------------|----------|
| Local development (`just watch-dpe`) | `http://localhost:4000/dpe/oai` |
| DEV | `https://api.dev.dasch.swiss/dpe/oai` |
| Production | `https://repository.dasch.swiss/dpe/oai` |

The `baseURL` advertised by `Identify` (and echoed in every `<request>` element) is configured per environment via the `DPE_OAI_BASE_URL` environment variable, so it matches the URL harvesters actually use (see [operations](./operations.md)). If unset it defaults to the production endpoint above.

## Verbs

All six OAI-PMH 2.0 verbs are implemented:

| Verb | Required arguments | Optional arguments |
|------|--------------------|--------------------|
| `Identify` | — | — |
| `ListMetadataFormats` | — | `identifier` |
| `ListSets` | — | — |
| `ListIdentifiers` | `metadataPrefix` | `from`, `until`, `set` |
| `ListRecords` | `metadataPrefix` | `from`, `until`, `set` |
| `GetRecord` | `identifier`, `metadataPrefix` | — |

Arguments outside these lists are rejected with `badArgument`, with two exceptions that are silently ignored: `set` on `ListSets` and `metadataPrefix` on `ListMetadataFormats`. A `resumptionToken` is rejected with `badResumptionToken` on every verb — see [Known Limitations](#known-limitations).

### Identify

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=Identify"
```

Abbreviated example response (values vary by environment):

```xml
<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/" ...>
  <request verb="Identify">https://repository.dasch.swiss/dpe/oai</request>
  <Identify>
    <repositoryName>DaSCH Service Platform Repository</repositoryName>
    <baseURL>https://repository.dasch.swiss/dpe/oai</baseURL>
    <protocolVersion>2.0</protocolVersion>
    <adminEmail>info@dasch.swiss</adminEmail>
    <earliestDatestamp>2008-06-01</earliestDatestamp>
    <deletedRecord>no</deletedRecord>
    <granularity>YYYY-MM-DD</granularity>
  </Identify>
</OAI-PMH>
```

### ListMetadataFormats

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListMetadataFormats"
```

With an `identifier` argument, the supported formats for that item are returned. Note: identifiers are currently validated against projects only — a record identifier that works with `GetRecord` returns `idDoesNotExist` here.

### ListSets

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListSets"
```

Abbreviated example response — the full response also contains one `project:{shortcode}` set per project and one `cluster:{id}` set per cluster:

```xml
<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/" ...>
  <request verb="ListSets">https://repository.dasch.swiss/dpe/oai</request>
  <ListSets>
    <set>
      <setSpec>entityType:ProjectCluster</setSpec>
      <setName>Project Clusters</setName>
    </set>
    <set>
      <setSpec>entityType:ResearchProject</setSpec>
      <setName>Research Projects</setName>
    </set>
    <set>
      <setSpec>project:0803</setSpec>
      <setName>Die Bilderfolgen der Basler Frühdrucke</setName>
    </set>
    <set>
      <setSpec>cluster:cluster-003</setSpec>
      <setName>Bernoulli-Euler Online (BEOL)</setName>
    </set>
    <!-- further project: and cluster: sets omitted -->
  </ListSets>
</OAI-PMH>
```

The response is a single un-paginated list and always includes the static `entityType:*` sets, so it is never empty.

## Metadata Formats

| `metadataPrefix` | Schema | Namespace |
|------------------|--------|-----------|
| `oai_dc` | `http://www.openarchives.org/OAI/2.0/oai_dc.xsd` | `http://www.openarchives.org/OAI/2.0/oai_dc/` |
| `oai_datacite` | `http://schema.datacite.org/oai/oai-1.1/oai.xsd` | `http://schema.datacite.org/oai/oai-1.1/` |

The `oai_datacite` payload contains DataCite kernel 4 metadata (`schemaVersion` 4.6, `datacentreSymbol` `DASCH.DSP`), following the DaSCH Metadata to DataCite mapping specification. `oai_dc` is the simpler Dublin Core representation; `oai_datacite` carries richer structured metadata (contributors, related identifiers, rights, geolocations, funding references).

### Temporal coverage

A project's `temporalCoverage` is emitted as a DataCite `date` element with `dateType="Coverage"`. The DataCite schema requires a structured W3CDTF value here (a year or a `start/end` interval, e.g. `1250/1500`, with negative years for BCE and `..` for an open bound), so the human-readable period name is carried in the `dateInformation` attribute rather than in the element body:

```xml
<date dateType="Coverage" dateInformation="Late Middle Ages">1250/1500</date>
```

Ranges are resolved offline (no network or LLM calls at request time), in two tiers:

1. **ChronOntology references** — entries with a `https://chronontology.dainst.org/period/...` URL resolve against `modules/dpe/server/data/chronontology-periods.json` (a slimmed mirror of ChronOntology timespans, regenerated by `scripts/fetch-chronontology-periods.py`).
2. **Everything else** — free-text names resolve against `modules/dpe/server/data/temporal-coverage-enrichment.json`, a reviewed lookup table built by `scripts/build-temporal-coverage-enrichment.py`. The tool parses common forms (numeric ranges, centuries, BC/AD/BCE/CE, decades) deterministically and uses curated ranges for named historical periods (tagged `source: "llm"`). Re-running merges new entries without overwriting reviewed rows; `--check` fails if the table is stale.

When neither tier yields a range (e.g. a cultural style like "Swiss" that is not a time period), the element is emitted with the `dateInformation` attribute only and an empty body, so the original label is never dropped.

## Identifiers

OAI identifiers are derived from DaSCH [ARK](https://arks.org/) identifiers:

| Item type | Pattern | Example |
|-----------|---------|---------|
| Research project | `oai:dasch.swiss:ark:/72163/1/{shortcode}` | `oai:dasch.swiss:ark:/72163/1/0803` |
| Record | `oai:dasch.swiss:ark:/72163/1/{shortcode}/{record_id}` | `oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh` |

These differ from the resolvable ARK URLs (`https://ark.dasch.swiss/ark:/72163/1/...`), which appear in the metadata payloads as `dc:identifier` / DataCite `identifier`.

## Sets

Selective harvesting uses the `set` argument on `ListIdentifiers` and `ListRecords`. Two kinds of sets exist: static **entity-type** sets and dynamic **project**/**cluster** sets.

| `setSpec` | Contents | Notes |
|-----------|----------|-------|
| `entityType:ResearchProject` | All research project metadata entries | Advertised by `ListSets` |
| `entityType:ProjectCluster` | Project clusters | Advertised but currently empty — always returns `noRecordsMatch` |
| `entityType:Record` | All record-level metadata entries | Accepted as a filter and stamped on record headers, but not advertised by `ListSets`; subject to change |
| `project:{shortcode}` | The Records belonging to one research project | One set per project, `setName` = project name. Shortcode matching is case-insensitive. |
| `cluster:{id}` | All entities under one project cluster: the cluster's research project metadata entries **plus** all of those projects' Records | One set per cluster, `setName` = cluster name. The `{id}` is the stable cluster id (e.g. `cluster-003`). |

The hierarchy is **Cluster → Projects → Records**, and the two dynamic set kinds deliberately differ in breadth: `project:{shortcode}` is a record-harvesting scope (it does *not* include the project's own metadata entry), while `cluster:{id}` is the discovery container for a whole cluster and therefore also surfaces the project metadata entries a harvester needs to navigate. To fetch a single project's *metadata entry*, use `entityType:ResearchProject` or the project's `cluster:{id}` set.

A project belongs to a cluster if the cluster's member list contains the project's shortcode (case-insensitive); records inherit their parent project's cluster membership.

**Set membership on headers.** Every item header lists its full set membership, so membership can be determined from an item alone: record headers carry `entityType:Record`, their `project:{shortcode}`, and a `cluster:{id}` for each cluster of the parent project; project headers carry `entityType:ResearchProject`, their own `project:{shortcode}`, and their `cluster:{id}` sets.

**Set validation.** An unrecognised `set` value — bad prefix, empty value, or a `project:`/`cluster:` value matching no known project or cluster — is rejected with `badArgument`. A recognised set that matches zero items (e.g. a known project with no records yet, possibly after date filtering) returns `noRecordsMatch`.

## Harvesting

Full harvest of all items:

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListRecords&metadataPrefix=oai_dc"
```

Selective harvest of research projects only:

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListRecords&metadataPrefix=oai_datacite&set=entityType:ResearchProject"
```

Selective harvest of one project's records, or of everything under one cluster:

```bash
# All records of project 0803
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListRecords&metadataPrefix=oai_dc&set=project:0803"

# All project entries and records under cluster cluster-003
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListRecords&metadataPrefix=oai_dc&set=cluster:cluster-003"
```

Headers only (no metadata payloads):

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListIdentifiers&metadataPrefix=oai_dc"
```

List responses are returned complete and unpaged — there is no `resumptionToken` element, and its absence means the response is complete, not truncated.

### Fetching a single item

```bash
# A research project
curl "https://api.dev.dasch.swiss/dpe/oai?verb=GetRecord&metadataPrefix=oai_dc&identifier=oai:dasch.swiss:ark:/72163/1/0803"

# A record within a project
curl "https://api.dev.dasch.swiss/dpe/oai?verb=GetRecord&metadataPrefix=oai_dc&identifier=oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"
```

Abbreviated example response:

```xml
<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/" ...>
  <request verb="GetRecord" identifier="oai:dasch.swiss:ark:/72163/1/0803" metadataPrefix="oai_dc">https://repository.dasch.swiss/dpe/oai</request>
  <GetRecord>
    <record>
      <header>
        <identifier>oai:dasch.swiss:ark:/72163/1/0803</identifier>
        <datestamp>2008-06-01</datestamp>
        <setSpec>entityType:ResearchProject</setSpec>
        <setSpec>project:0803</setSpec>
        <!-- plus a cluster:{id} setSpec for each cluster the project belongs to -->
      </header>
      <metadata>
        <oai_dc:dc xmlns:oai_dc="http://www.openarchives.org/OAI/2.0/oai_dc/" xmlns:dc="http://purl.org/dc/elements/1.1/" ...>
          <dc:title>Die Bilderfolgen der Basler Frühdrucke</dc:title>
          <dc:publisher>DaSCH</dc:publisher>
          <dc:date>2008-06-01</dc:date>
          <dc:type>Project</dc:type>
          <dc:identifier>https://ark.dasch.swiss/ark:/72163/1/0803</dc:identifier>
          <!-- further Dublin Core elements omitted -->
        </oai_dc:dc>
      </metadata>
    </record>
  </GetRecord>
</OAI-PMH>
```

## Datestamps and Date Filtering

`ListIdentifiers` and `ListRecords` accept `from` and `until` arguments. Both bounds are inclusive. Before relying on them, understand what the datestamps mean:

- **Project datestamp**: the project's research *start date* (fallback `2015-01-01`) — not the date the metadata was created or last modified.
- **Record datestamp**: the record's `dateModified`, falling back to `datePublished`, then `dateCreated`, then `2015-01-01`. Record datestamps may carry a full timestamp (e.g. `2012-06-19T14:33:33Z`) even though `Identify` advertises `YYYY-MM-DD` granularity.

Filtering compares `from`/`until` against datestamps as plain strings (lexicographic comparison):

- Use `YYYY-MM-DD` values. Other formats are not rejected with an error but compare incorrectly.
- Because record datestamps may include a time component, `until=2012-06-19` excludes a record stamped `2012-06-19T14:33:33Z`. To include a full day, filter with `until` set to the following day.

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListIdentifiers&metadataPrefix=oai_dc&from=2010-01-01&until=2020-12-31"
```

**Incremental harvesting is not reliable with these semantics.** Project datestamps reflect research start dates, so new or updated project metadata does not surface through `from`-based incremental harvests, and deletions are not tracked (`deletedRecord: no`). Harvesters should periodically re-harvest the full repository instead of relying on date-based increments.

## Errors

Errors are returned as OAI `<error>` elements with HTTP status `200`:

| Code | Returned when |
|------|---------------|
| `badVerb` | The `verb` argument is missing or not one of the six verbs. The `<request>` element omits the verb attribute in this case. |
| `badArgument` | An argument is missing, repeated, or not allowed for the verb (e.g. `set` on `GetRecord`); also returned for an unrecognised `set` value (bad prefix, empty value, or unknown project/cluster). |
| `badResumptionToken` | A `resumptionToken` argument is supplied (resumption tokens are not supported). |
| `cannotDisseminateFormat` | `metadataPrefix` is not `oai_dc` or `oai_datacite`. |
| `idDoesNotExist` | The `identifier` does not resolve — including malformed identifiers (wrong prefix, bare shortcode), which are not reported as `badArgument`. |
| `noRecordsMatch` | A list request matches nothing: empty result, a recognised `set` with no items, or a `from`/`until` window with no matches. An empty list is never returned. |

## Known Limitations

- **No resumption tokens.** List responses are complete and unpaged; any `resumptionToken` argument yields `badResumptionToken`.
- **`baseURL` is a fixed configured value.** It is set per environment via `DPE_OAI_BASE_URL` rather than derived from each incoming request. Deploying behind a hostname that does not match the configured value will make the advertised `baseURL` disagree with the request URL, which OAI validators flag — keep the env var in sync with the public endpoint.
- **No deleted-record tracking** (`deletedRecord: no`). Items that disappear are not announced; see the re-harvesting recommendation above.
- **GET only.** OAI-PMH 2.0 also requires POST; harvesters that default to POST will not get an OAI response.
- **`ListMetadataFormats` with a record identifier** returns `idDoesNotExist`, even for records that `GetRecord` resolves.
- **`entityType:ProjectCluster`** is advertised by `ListSets` but contains no items yet.
