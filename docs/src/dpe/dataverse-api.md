# Dataverse-Compatible API

DPE serves a minimal subset of the [Dataverse Native API](https://guides.dataverse.org/en/latest/api/native-api.html) so that EOSC Data Commons can harvest DaSCH datasets with file-level download information.

Their pipeline works in two steps: it harvests our [OAI-PMH](./oai-pmh.md) feed with the `oai_datacite` prefix, and then, for each record, asks a Dataverse API for that record's files. Rather than deploy Dataverse, DPE reimplements the two endpoints that pipeline actually calls.

> **Three fields are placeholders.** The MIME type, creation date, download URL, and file id all come from real record data. But `filename`, `filesize`, and `checksum` have no source in DPE or dsp-api yet, and are served as obvious fakes — every `filesize` is `1` and every checksum is the MD5 of the empty string. A client that verifies a checksum against the downloaded bytes **will** find a mismatch. A forthcoming dsp-api file endpoint will supply the real values. See [Placeholders](#placeholders).

## Endpoints

Both endpoints are unauthenticated — they serve published, public metadata — and both live at the **host root**, not under `/dpe` like the rest of the application. The URL shapes are fixed by the Dataverse contract: the versions endpoint is the URL EOSC Data Commons seeds on their side, and the client builds the download URL itself from the `dataFile.id` we emit.

Both are rate-limited per IP, like the OAI endpoint (see [Operations](./operations.md#environment-variables)).

### `GET /api/datasets/:persistentId/versions/:latest-published`

Returns the file list for one record.

`:persistentId` and `:latest-published` are **literal path segments**, not path parameters — that is genuinely how the Dataverse API spells this route. The dataset is selected by the `persistentId` *query* parameter instead.

| Query parameter | Required | Description |
|---|---|---|
| `persistentId` | Yes | The full OAI header identifier, exactly as emitted by our OAI-PMH feed (e.g. `oai:dasch.swiss:ark:/72163/1/0803/lklK7rVuVOmpBZYWrF8o=gh`). The crawler passes it back verbatim, colons and all. |
| `exporter` | No | Sent by the crawler as `dataverse_json`. Accepted and ignored: this endpoint only speaks that one format. |

Example:

```bash
curl 'http://localhost:4000/api/datasets/:persistentId/versions/:latest-published?exporter=dataverse_json&persistentId=oai:dasch.swiss:ark:/72163/1/0803/_qXHiyf6WsOfLFEXr7Zdng7'
```

```json
{
  "status": "OK",
  "data": {
    "files": [
      {
        "restricted": false,
        "version": 1,
        "dataFile": {
          "id": 8294929545880278,
          "filename": "2vbIabBOEvq-EU9jwmgEe9j.jp2",
          "contentType": "image/jp2",
          "filesize": 1,
          "creationDate": "2011-04-14T07:15:28Z",
          "checksum": { "type": "MD5", "value": "d41d8cd98f00b204e9800998ecf8427e" }
        }
      }
    ]
  }
}
```

That is a verbatim response from the committed data. Note the `filesize` of `1` and the empty-string MD5 — the placeholders described [below](#placeholders).

A record with no file returns the same envelope with an empty array:

```json
{ "status": "OK", "data": { "files": [] } }
```

Field notes:

| Field | Type | Notes |
|---|---|---|
| Field | Type | Source | Notes |
|---|---|---|---|
| `restricted` | bool | constant | Required. Always `false` — all DaSCH data is currently openly accessible. |
| `version` | u64 | constant | Required. Always `1` — DPE has no per-file version concept. |
| `dataFile.id` | u64 | derived | Required. Derived from the record IRI; see [File ids](#file-ids). |
| `dataFile.contentType` | string | record `file.mimeType` | Required, must parse as a MIME type. May carry parameters (`text/csv; charset=UTF-8`). |
| `dataFile.creationDate` | string | record `dateCreated` | Required, ISO 8601. |
| `dataFile.filename` | string | **placeholder** | Required. Synthesised from the asset id plus a MIME-guessed extension — *not* the original filename. |
| `dataFile.filesize` | u64 | **placeholder** | Required, in bytes. Always `1`. |
| `dataFile.checksum.type` | string | **placeholder** | Required. Always `MD5`. |
| `dataFile.checksum.value` | string | **placeholder** | Required, hex. Always the MD5 of the empty string, so it is recognisable as fake. |
| `dataFile.lastUpdateTime` | string | — | Optional. Never emitted: records carry no per-file modification date. |
| `directoryLabel` | string | — | Optional. Never emitted: DPE has no directory concept, and Harvard omits it too. |

Absent optionals are **omitted, not `null`**. That is contractual, not cosmetic: the consuming parser expects a string there, and `null` is a different shape than a missing key.

Responses:

| Status | When |
|---|---|
| `200` | The identifier resolves. `data.files` is empty when the record exists but has no file — the majority case, see below. |
| `400` | `persistentId` is missing or blank. |
| `404` | The identifier matches no known project or record. |

The distinction between a `200` with an empty list and a `404` is deliberate, and it matters at scale: **roughly 77% of records (39,216 of 50,994) are metadata-only** and have no file at all. Answering those with `404` would make a normal harvest look like a broken endpoint, so a known-but-fileless record gets an empty array and only a genuinely unknown identifier gets `404`.

### `GET /api/access/datafile/{id}`

Returns the bytes of one file, selected by the numeric `dataFile.id`. The client constructs this URL itself, so its shape is not ours to choose.

```bash
curl -L 'http://localhost:4000/api/access/datafile/8294929545880278'
# 307 → https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original
```

| Status | When |
|---|---|
| `307` | Redirect to the storage URL (dsp-ingest). Clients follow redirects here; production Dataverse does the same thing, answering with a `303` to presigned object storage. |
| `403` | The file is `restricted`. Currently unreachable — all DaSCH data is open — but kept so the branch is correct once a real restriction signal exists. |
| `404` | No file has that id, or the segment is not numeric. |

`Content-Disposition` is deliberately not set on the redirect — it belongs on the final hop, which dsp-ingest serves.

Errors from both endpoints use Dataverse's own envelope, so a client written against the real API can read them:

```json
{ "status": "ERROR", "message": "file not found: 999999" }
```

## Data source

There is no separate data file and no database. Everything is derived from the records already committed under `modules/dpe/server/data/records/`, so the endpoint stays in step with the record dumps automatically and nothing has to be maintained by hand.

A record's `file` object supplies the MIME type and the storage URL:

```json
{
  "id": "http://rdfh.ch/0803/_qXHiyf6WsOfLFEXr7Zdng",
  "pid": "https://ark.dasch.swiss/ark:/72163/1/0803/_qXHiyf6WsOfLFEXr7Zdng7",
  "dateCreated": "2011-04-14T07:15:28Z",
  "file": {
    "mimeType": "image/jp2",
    "url": "https://ingest.dasch.swiss/projects/0803/assets/2vbIabBOEvq-EU9jwmgEe9j/original"
  }
}
```

`DataverseFile::from_record` maps that to the wire shape; records without a `file` key produce no files at all. Of the 50,994 committed records, 11,778 carry a file.

### File ids

The Dataverse format has **no download-URL field.** Clients derive the URL from `dataFile.id` by string concatenation — `{scheme}://{host}/api/access/datafile/{id}` — and cache the result. (Confirmed against a live Harvard response in `notes/examples/harvard-dataverse-payload.json`, whose only storage-facing field is an internal `storageIdentifier`.) So the id *is* the download address, and it must keep resolving to the same file.

Ids are therefore derived, not assigned: `file_id_for_iri` hashes the record IRI with FNV-1a and masks to 53 bits. That is deterministic and stateless, needs no lookup table, and survives regeneration of the record dumps — which matters, because those dumps are machine-generated and would destroy any hand-written ids.

Two constraints shape the choice:

- **53 bits, not 64.** Many JSON consumers parse numbers as IEEE-754 doubles and lose integer precision above 2⁵³, which would silently round an id and break the derived URL.
- **Collisions are detected, not silent.** Two records hashing to the same id would make one file's download URL serve the other's bytes. The id index rejects duplicates and logs an error, and `committed_records_have_no_id_collisions` asserts the property over the real data, so a colliding dump fails the build. There are currently zero collisions across all 50,994 records.

The record IRI is hashed rather than the ARK because the IRI is what dsp-api uses internally, so it lines up with whatever the forthcoming file endpoint returns.

### Placeholders

Three required fields have no source in DPE or dsp-api today and are filled with obvious fakes, grouped in `dpe_core::FilePlaceholders`:

| Field | Placeholder | Why it is not real |
|---|---|---|
| `filename` | asset id + MIME-guessed extension | dsp-api has the original filename (`FileValueV2.originalFilename`) but discards it on export, keeping only the opaque asset id. |
| `filesize` | `1` | Not present in dsp-api at all; must come from dsp-ingest. |
| `checksum` | MD5 of the empty string | No hashing anywhere in the pipeline. |

The checksum placeholder is the MD5 of the empty string (`d41d8cd9…`) rather than random hex specifically so that anyone inspecting a response can recognise it as fake.

When the dsp-api file endpoint lands it populates `FilePlaceholders`, and the wire types, handlers, and routes do not change. Moving to SHA-256 at that point is a data change too: the serializer emits whatever `checksum_type` says.

`restricted` is *not* a placeholder — it is a constant `false`, because every DaSCH record is `Full Open Access` and dsp-api hardcodes that on export. If DaSCH ever holds restricted assets this must become a real per-file value; a per-record access-rights string cannot express per-file restriction.

## Implementation

| Piece | Location |
|---|---|
| Endpoint handlers, DTOs, errors | `modules/dpe/api-dataverse/` |
| File model, id derivation, placeholders | `modules/dpe/core/src/dataverse_file.rs` |
| Record-backed lookup and id index | `modules/dpe/core/src/dataverse_file_cache.rs` |
| Route wiring and rate limiting | `modules/dpe/server/src/router.rs` |

Two implementation details are worth knowing before editing the routes:

- **The literal colons are intentional.** Axum 0.8 spells path parameters `{name}` and panics on any segment starting with `:` to catch un-migrated 0.7-style routes, so registering these literal segments requires `without_v07_checks()`. On merge, Axum keeps that check unless *both* routers disabled it — so every router in the merge chain (`build_router`, `oai_router_with`, `dataverse_router_with`) opts out. Removing it from any one of them makes the application panic at startup.
- **`files_for` returns a three-way answer.** `None` means no such record (`404`), `Some(vec![])` means a record with no file (`200`, empty array), and a non-empty vec is the normal case. Collapsing the first two would break the majority of harvest requests.
