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
| Production | Not yet deployed; see [Known Limitations](#known-limitations) regarding the advertised `baseURL` |

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
  <request verb="Identify">https://meta.dasch.swiss/oai</request>
  <Identify>
    <repositoryName>DaSCH Service Platform Repository</repositoryName>
    <baseURL>https://meta.dasch.swiss/oai</baseURL>
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

```xml
<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/" ...>
  <request verb="ListSets">https://meta.dasch.swiss/oai</request>
  <ListSets>
    <set>
      <setSpec>entityType:ProjectCluster</setSpec>
      <setName>Project Clusters</setName>
    </set>
    <set>
      <setSpec>entityType:ResearchProject</setSpec>
      <setName>Research Projects</setName>
    </set>
  </ListSets>
</OAI-PMH>
```

## Metadata Formats

| `metadataPrefix` | Schema | Namespace |
|------------------|--------|-----------|
| `oai_dc` | `http://www.openarchives.org/OAI/2.0/oai_dc.xsd` | `http://www.openarchives.org/OAI/2.0/oai_dc/` |
| `oai_datacite` | `http://schema.datacite.org/oai/oai-1.1/oai.xsd` | `http://schema.datacite.org/oai/oai-1.1/` |

The `oai_datacite` payload contains DataCite kernel 4 metadata (`schemaVersion` 4.6, `datacentreSymbol` `DASCH.DSP`), following the DaSCH Metadata to DataCite mapping specification. `oai_dc` is the simpler Dublin Core representation; `oai_datacite` carries richer structured metadata (contributors, related identifiers, rights, geolocations, funding references).

## Identifiers

OAI identifiers are derived from DaSCH [ARK](https://arks.org/) identifiers:

| Item type | Pattern | Example |
|-----------|---------|---------|
| Research project | `oai:meta.dasch.swiss:ark:/72163/1/{shortcode}` | `oai:meta.dasch.swiss:ark:/72163/1/0803` |
| Record | `oai:meta.dasch.swiss:ark:/72163/1/{shortcode}/{record_id}` | `oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh` |

These differ from the resolvable ARK URLs (`https://ark.dasch.swiss/ark:/72163/1/...`), which appear in the metadata payloads as `dc:identifier` / DataCite `identifier`.

## Sets

Sets group items by entity type. Selective harvesting uses the `set` argument on `ListIdentifiers` and `ListRecords`.

| `setSpec` | Contents | Status |
|-----------|----------|--------|
| `entityType:ResearchProject` | Research project metadata entries | Harvestable |
| `entityType:ProjectCluster` | Project clusters | Advertised but currently empty — always returns `noRecordsMatch` |
| `entityType:Record` | Record-level metadata entries | Accepted as a filter and stamped on record headers, but not advertised by `ListSets`; subject to change |

An unknown `set` value is not rejected — it matches nothing and returns `noRecordsMatch`, the same response as an empty repository.

## Harvesting

Full harvest of all items:

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListRecords&metadataPrefix=oai_dc"
```

Selective harvest of research projects only:

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListRecords&metadataPrefix=oai_datacite&set=entityType:ResearchProject"
```

Headers only (no metadata payloads):

```bash
curl "https://api.dev.dasch.swiss/dpe/oai?verb=ListIdentifiers&metadataPrefix=oai_dc"
```

List responses are returned complete and unpaged — there is no `resumptionToken` element, and its absence means the response is complete, not truncated.

### Fetching a single item

```bash
# A research project
curl "https://api.dev.dasch.swiss/dpe/oai?verb=GetRecord&metadataPrefix=oai_dc&identifier=oai:meta.dasch.swiss:ark:/72163/1/0803"

# A record within a project
curl "https://api.dev.dasch.swiss/dpe/oai?verb=GetRecord&metadataPrefix=oai_dc&identifier=oai:meta.dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh"
```

Abbreviated example response:

```xml
<OAI-PMH xmlns="http://www.openarchives.org/OAI/2.0/" ...>
  <request verb="GetRecord" identifier="oai:meta.dasch.swiss:ark:/72163/1/0803" metadataPrefix="oai_dc">https://meta.dasch.swiss/oai</request>
  <GetRecord>
    <record>
      <header>
        <identifier>oai:meta.dasch.swiss:ark:/72163/1/0803</identifier>
        <datestamp>2008-06-01</datestamp>
        <setSpec>entityType:ResearchProject</setSpec>
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
| `badArgument` | An argument is missing, repeated, or not allowed for the verb (e.g. `set` on `GetRecord`). |
| `badResumptionToken` | A `resumptionToken` argument is supplied (resumption tokens are not supported). |
| `cannotDisseminateFormat` | `metadataPrefix` is not `oai_dc` or `oai_datacite`. |
| `idDoesNotExist` | The `identifier` does not resolve — including malformed identifiers (wrong prefix, bare shortcode), which are not reported as `badArgument`. |
| `noRecordsMatch` | A list request matches nothing: empty result, unknown `set` value, or a `from`/`until` window with no matches. An empty list is never returned. |

## Known Limitations

- **No resumption tokens.** List responses are complete and unpaged; any `resumptionToken` argument yields `badResumptionToken`.
- **Advertised `baseURL` differs from the actual endpoint.** `Identify` reports `https://meta.dasch.swiss/oai`, while the endpoint is mounted at `/dpe/oai`. Automated OAI validators and harvest managers that compare the `baseURL` against the request URL will flag this.
- **No deleted-record tracking** (`deletedRecord: no`). Items that disappear are not announced; see the re-harvesting recommendation above.
- **GET only.** OAI-PMH 2.0 also requires POST; harvesters that default to POST will not get an OAI response.
- **`ListMetadataFormats` with a record identifier** returns `idDoesNotExist`, even for records that `GetRecord` resolves.
- **`entityType:ProjectCluster`** is advertised by `ListSets` but contains no items yet.
