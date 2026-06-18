# JSON API

Read-only JSON endpoints that expose DaSCH research project metadata. They serve the same project data as the DPE pages and the [OAI-PMH endpoint](./oai-pmh.md), sourced from the in-process project cache that is loaded from the data directory at startup. The handlers live in the `dpe-server` crate (`server/src/fragments.rs`).

## Endpoints

| Method | Path | Returns |
|--------|------|---------|
| GET | `/dpe/api/v2/projects` | JSON array of all projects |
| GET | `/dpe/api/v2/projects/{shortcode}` | A single project object |

- **Method**: GET only.
- **Response**: `Content-Type: application/json`.
- **Authentication**: none.

| Environment | Base URL |
|-------------|----------|
| Local development (`just watch-dpe`) | `http://localhost:4000` |
| DEV | `https://api.dev.dasch.swiss` |
| Production | Not yet deployed |

## List all projects

```bash
curl "https://api.dev.dasch.swiss/dpe/api/v2/projects"
```

Returns a JSON array containing every project. The list is **not paginated and not filtered** — the entire collection is returned in one response. (The HTML listing at `/dpe/projects` supports search and faceting; this JSON endpoint does not.)

## Fetch a single project

The path segment is the project **shortcode**, not the `id` field. Matching is case-insensitive, so `0803` and any case variant of an alphanumeric shortcode (e.g. `080c` for `080C`) resolve to the same project.

```bash
curl "https://api.dev.dasch.swiss/dpe/api/v2/projects/0803"
```

| Status | Returned when |
|--------|---------------|
| `200 OK` | The shortcode resolves to a project. |
| `400 Bad Request` | The shortcode is not alphanumeric (e.g. contains `/`, `-`, `_`, or other characters). Empty body. |
| `404 Not Found` | The shortcode is well-formed but matches no project. Empty body. |

## Response format

Both endpoints serialize the project metadata with **camelCase** keys. The shape mirrors the stored project JSON files; the single-project endpoint returns one object, the list endpoint an array of the same. For the meaning and cardinality of each field, see the [Metadata Model (v2)](./metadata-model.md); the table below describes only the wire format.

| Key | Type | Notes |
|-----|------|-------|
| `id` | string | Internal project id |
| `pid` | string | Persistent identifier (ARK URL) |
| `name` | string | Display name |
| `shortcode` | string | Project shortcode (the lookup key for the single-project endpoint) |
| `officialName` | string | |
| `status` | string | `"Ongoing"` or `"Finished"` (PascalCase, unlike the rest of the payload) |
| `shortDescription` | string | |
| `description` | object | Language code → text |
| `startDate`, `endDate` | string | `YYYY-MM-DD` |
| `url` | object \| array \| null | Raw stored value — either a structured reference object or a legacy string array; not normalized |
| `secondaryUrl` | object \| null | Secondary reference (new-format files only) |
| `howToCite` | string | |
| `accessRights` | object | Access-rights type and optional embargo date |
| `legalInfo` | array | License, copyright holder, and authorship entries |
| `dataManagementPlan` | string \| null | |
| `dataPublicationYear` | string \| null | |
| `typeOfData` | array\<string\> \| null | |
| `dataLanguage` | array\<string\> \| null | |
| `clusters` | array\<string\> \| null | Cluster ids |
| `collections` | array\<string\> \| null | Collection ids |
| `records` | array\<string\> \| null | Record ids |
| `keywords` | array\<object\> | Language maps |
| `disciplines` | array | |
| `temporalCoverage` | array | |
| `spatialCoverage` | array | Authority-file references |
| `attributions` | array | |
| `abstract` | object \| null | Language map (the field is named `abstract`, not `abstractText`) |
| `contactPoint` | array\<string\> \| null | |
| `publications` | array \| null | |
| `funding` | object | |
| `alternativeNames` | array\<object\> \| null | |
| `documentationMaterial` | array\<string\> \| null | |
| `provenance` | string \| null | |
| `additionalMaterial` | array\<string\> \| null | |

## Notes and limitations

- **Read-only.** GET only; there are no create/update/delete operations.
- **No pagination or filtering** on the list endpoint — it always returns the full collection.
- **Cached at startup.** Data is read from the configured data directory when the server starts; changes to project files require a restart to surface.
- **`url` is a raw passthrough.** Unlike the typed project model used to render pages, the JSON output emits the stored `url` value verbatim (structured object or legacy array), including any placeholder values.
