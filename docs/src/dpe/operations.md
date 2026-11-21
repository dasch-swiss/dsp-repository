# DPE Operations Guide

Operations documentation for the DPE infrastructure team.

The image and its binary are the Access Area's: beside DPE they serve CPE, under `/cpe`. What is specific to
CPE (start-up, what it answers when something is missing, measurements) is in [CPE Operations](../cpe/operations.md);
the settings of both are in the table below.

## Docker Image

- **Base**: `gcr.io/distroless/static-debian12:nonroot`
- **User**: uid **65532** — distroless `NONROOT`. Not 65534, which is `nobody`; verified in [`common/variables.bzl`](https://github.com/GoogleContainerTools/distroless/blob/main/common/variables.bzl).
- **Shell**: None (distroless — no SSH possible)
- **Binary**: Static musl-linked `access-server` (CLI with subcommands).
- **Data**: DPE's JSON corpus at `/app/server/data` (`DPE_DATA_DIR`) and CPE's project folders
  (`areas/access/cpe/projects/`: `SHORTCODE`, assets, documents on the data) at `/app/cpe/projects`
  (`CPE_PROJECTS_DIR`). CPE's stylesheets, scripts and logos are compiled into the binary. The image carries no
  data of `sync`'s or `media`'s: a deployment mounts the locations the VRE bridge writes and names them in
  `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` (see the [environment table](#environment-variables)). The committed
  copies under `areas/access/sync/data/` and `areas/access/media/data/` are test fixtures and the data of a run
  from the checkout.

CI publishes the image Bazel builds, `//areas/access/server:image` (see [Building with Bazel](../bazel.md#images));
`just build-docker-dpe` builds the same amd64 image locally and loads it as `local-scan/dpe:latest`, and
`just run-docker-dpe` runs it on port 8080 (emulated on an arm64 machine).

## CLI Commands

The `access-server` binary provides three subcommands:

| Command | Description |
|---------|-------------|
| `access-server serve` | Start the web server |
| `access-server validate <data_dir> [--inputs <dir>] [--must-serve <shortcode>...]` | Validate DPE's data files under the given directory, then every CPE project (the two flags are CPE's) |
| `access-server healthcheck [--url URL]` | Check if the server is healthy (default: `http://localhost:8080/healthz`) |

### `access-server validate`

Validates DPE's JSON data files for structural correctness and cross-reference integrity, then checks CPE's
projects strictly (`docs/src/cpe/operations.md`, *Checking the projects: `access-server validate`*).

```bash
access-server validate ./data
```

**What it checks in DPE's data:**
- JSON schema validity for all data file types (projects, persons, organizations, records, clusters, collections)
- Cross-references between projects, persons, and organizations
- Orphaned files that are not referenced by any parent entity
- Project roles misplaced in a person's `jobTitles` (e.g. "Project Leader", "Project staff", "Creator"). Such a role belongs in the project's `attributions` (`contributorType`), where the OAI-PMH creator/contributor logic can read it. The role vocabulary is `JOB_TITLE_ROLE_WORDS` in `shared-metadata`.
- Every distinct `temporalCoverage` name resolves to a structured date (ChronOntology or the offline enrichment table), or is explicitly marked `source: "unresolved"`. See `docs/src/dpe/oai-pmh.md` → *Temporal coverage*.

**Exit codes:**
- `0` — all of DPE's data files are valid and no CPE project failed
- `1` — DPE's data has errors, or a CPE project failed, or CPE's configuration is refused (details printed to
  stdout)

### `access-server healthcheck`

Lightweight probe for Docker HEALTHCHECK or monitoring:

```bash
access-server healthcheck                             # default: http://localhost:8080/healthz
access-server healthcheck --url http://localhost:9090/healthz # custom URL
```

## Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 8080 | HTTP | Application server |

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUST_LOG` | No | `info` | Log level filter (e.g., `access_server=info,dpe_server=info,tower_http=debug`) |
| `DPE_DATA_DIR` | No | `areas/access/dpe/server/data` | Path to project/record JSON data files. Legacy alias: `DATA_DIR` (checked if `DPE_DATA_DIR` is unset) |
| `DPE_FATHOM_SITE_ID` | No | *(none)* | Fathom Analytics site ID (not a secret). **Production only** — leave unset or empty everywhere else; empty is normalised to unset. |
| `DPE_SHOW_PLACEHOLDER_VALUES` | No | `false` | Show placeholder values (MISSING, CALCULATED) in the UI, styled in red. Enable on STAGE for QA visibility. |
| `DPE_OAI_BASE_URL` | No | `https://repository.dasch.swiss/dpe/oai` | Public base URL emitted as the OAI-PMH `baseURL` and echoed in `<request>` elements. Set per environment to match the public endpoint (e.g. `http://localhost:4000/dpe/oai` locally). See [OAI-PMH](./oai-pmh.md). |
| `DPE_PUBLIC_BASE_URL` | No | `https://repository.dasch.swiss` | Public origin of the site itself, used to build the landing-page and machine-readable-representation URLs carried by the embedded metadata and the `Link` header. Origin only: scheme `http` or `https`, no path, query or trailing slash. A bad value fails startup. Independent of `DPE_OAI_BASE_URL` (see the note below). |
| `DPE_ARK_RESOLVER_BASE_URL` | No | *(none)* | Origin every emitted ARK is rewritten to carry, and the origin the deployment's own `/ark:/{naan}/{shoulder}/{shortcode}` resolver answers on. **Leave unset everywhere but a PR preview.** Unset, the ARKs are the ones the corpus records, resolving through `ark.dasch.swiss`, and no resolver route is mounted. Origin only, validated exactly as `DPE_PUBLIC_BASE_URL` is. See the note below. |
| `DPE_OAI_RATE_LIMIT_PER_SECOND` | No | `1` | Per-IP rate limit, seconds per request once the burst is spent. `1` ≈ 60 requests/minute sustained. Governs `/dpe/oai` and both machine-readable representation routes, which share one bucket. See [OAI-PMH](./oai-pmh.md) and [Machine-Readable Metadata](./machine-readable-metadata.md). |
| `DPE_OAI_RATE_LIMIT_BURST` | No | `60` | Per-IP burst allowance on the same three routes: back-to-back requests before the sustained rate applies. |
| `DPE_OAI_PAGE_SIZE` | No | `100` | Items per page in `ListRecords` / `ListIdentifiers` responses before a resumption token is emitted. Non-positive or non-numeric values fall back to the default. See [OAI-PMH](./oai-pmh.md). |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | No | *(none)* | OTLP gRPC endpoint (e.g., `http://alloy:4317`). When unset, OTel falls back to no-op export. |
| `OTEL_SERVICE_NAME` | No | *(none)* | Service name for OTel resource attributes (e.g., `dpe`) |
| `OTEL_RESOURCE_ATTRIBUTES` | No | *(none)* | Comma-separated OTel resource attributes (e.g., `service.namespace=dpe,service.version=0.2.1,deployment.environment=prod`) |
| `PYROSCOPE_ENDPOINT` | No | *(none)* | Pyroscope HTTP endpoint (e.g., `http://pyroscope:4040`). When unset, profiling is disabled. |
| `DPE_SITE_ADDR` | No | `127.0.0.1:4000` | Listen address and port. The Docker image sets `0.0.0.0:8080`. |
| `DPE_PUBLIC_DIR` | No | `areas/access/dpe/public` | Directory served as static assets by `ServeDir` (favicon, logo, vendored JS, project images, and the compiled `app.<hash>.css`). |
| `DPE_ENV` | No | `DEV` | Deployment environment (`DEV` or `PROD`). Controls OTLP log export (see [Logging](#logging)). The Docker image sets `PROD`. |
| `SYNC_DATA_DIR` | In a deployment | `areas/access/sync/data` | `sync`'s stand-in subtree, read at start and on every re-activation to build each CPE project's store: one directory per shortcode, holding the project's Ready token and the generation it names (`0803/.ready`, `0803/<generation>/snapshot.nq`); a project without a token is unknown and answers `404`, as a path that names no project. Empty is treated as unset. The Docker image neither sets it nor carries such data: a deployment mounts the `sync` subtree of the location the VRE bridge writes, read-only, and names it here. Unset in the image, the default names `/app/areas/access/sync/data`, which does not exist, so CPE serves no project. If the directory is missing or holds no project directory, a project's token is unreadable or a symlink, or the generation or snapshot it names is missing, unreadable or a symlink, that project answers `503` and an error is logged; DPE is unaffected. |
| `SYNC_POLL_SECS` | No | `30` | Seconds between two polls of the Ready tokens in `SYNC_DATA_DIR`; `sync` announces each project whose token appeared, changed or was removed, and CPE re-activates on it ([CPE Operations](../cpe/operations.md), *Refresh*); CPE retries a held-back re-activation once, after `RETRY_AFTER_POLLS` intervals (`serve.rs`). Empty is treated as unset; `0` or a value that is not a whole number of seconds is logged as a warning and treated as `30`. |
| `MEDIA_DATA_DIR` | In a deployment | `areas/access/media/data` | `media`'s stand-in directory of deposited files, one subdirectory per shortcode holding the project's Ready token and the generation it names (`0803/.ready`, `0803/<generation>/config.kdl`, `…/curation.csv`, `…/narrative/…`), read at start and on every re-activation to build each CPE project's store. Empty is treated as unset. The Docker image neither sets it nor carries such files: a deployment mounts the `media` subtree of the location the VRE bridge writes, read-only, and names it here, together with `SYNC_DATA_DIR`. Unset in the image, the default names `/app/areas/access/media/data`, which does not exist. If the directory, or a file a project needs from it, is missing or unreadable, that project answers `503` and an error is logged (a missing directory also as a warning at start); DPE is unaffected. |
| `CPE_BASE` | No | *(empty)* | Path CPE is mounted at. The Docker image sets `/cpe`. A leading `/`, no trailing one, unreserved characters only. Unset, empty or refused (logged as a warning and as an error with the reason): CPE is not mounted and DPE is served alone. It must not be a path DPE serves (`/dpe`, `/assets`, …): nothing checks that, and under such a base CPE answers every path DPE has no route of its own for, its static files included. A path the binary or DPE has a route for (`/healthz`, `/telemetry/collect`, `/dpe/projects`) keeps its answer under any base. If the router cannot be built with CPE at the base, that is logged as an error naming `CPE_BASE`, and CPE is not mounted; no value is known to do that. |
| `CPE_PROJECTS_DIR` | No | `areas/access/cpe/projects` | Folder whose subfolders are CPE's projects: each declares its shortcode in a `SHORTCODE` file and holds its assets and the documents on its data; its configuration, curation and narrative are in `MEDIA_DATA_DIR`. The Docker image sets `/app/cpe/projects`. |
| `CPE_PUBLIC_ORIGIN` | No | *(none)* | Origin CPE's absolute URLs (the share links) start with, e.g. `https://repository.dasch.swiss`. An origin only: scheme `http` or `https` and a host, no path (not `…/cpe`: the base is `CPE_BASE`), no query, no trailing slash. A value of another shape is refused, and CPE then answers `503` on every path under its base. Not set in the image. Unset, CPE takes the origin of each request from `X-Forwarded-Proto` and `Host` and logs once at start that it does. Both headers are the client's unless a proxy in front overwrites them (Cloud Run does), so set the variable on every deployment that has a cache or a proxy that passes them through. |
| `ACCESS_HIDDEN_PROJECTS` | No | *(none)* | Comma-separated shortcodes of the Hidden projects (`areas/access/CONTEXT.md`), e.g. `083C,086A`: their presentations carry `X-Robots-Tag: noindex, nofollow` and the robots meta, and no overview lists them. Read once at start by `access-server`, honoured by CPE, ignored by DPE. Entries are trimmed and upper-cased; unset, empty or blank means none, and an entry that is not a shortcode is logged as a warning and ignored. Not set in the image: it is the deployment's to set. Not access control: anyone with the link reaches the project. |

> **Two base URLs, on purpose.** `DPE_PUBLIC_BASE_URL` and `DPE_OAI_BASE_URL` are set independently and neither is derived from the other. The OAI endpoint advertises its own `baseURL` in every response, and that endpoint may answer on a different host from the site. Set both per environment.
>
> **Assessing a local run from a container.** A FAIR assessor running in Docker resolves the URLs the page emits, so `DPE_PUBLIC_BASE_URL` must name a host the container can reach — `http://host.docker.internal:4000`, not `http://localhost:4000`, which inside the container is the container itself.
>
> **PR previews set all three themselves.** `cloud-run-dpe-pull-request.yml` runs `gcloud run services update` right after the deploy, setting `DPE_PUBLIC_BASE_URL`, `DPE_OAI_BASE_URL` and `DPE_ARK_RESOLVER_BASE_URL` to the preview's own Cloud Run URL. It is a second step because Cloud Run assigns that URL only once the service exists. Each one left unset makes the preview advertise production: the typed links and the `303` target, the OAI `baseURL`, and every emitted ARK respectively.
>
> **PR previews set no `CPE_*` variable, no `SYNC_DATA_DIR` and no `MEDIA_DATA_DIR`.** They run with the image's own, so CPE is mounted under `/cpe` there, but with no data of `sync`'s or `media`'s it serves no project: its overview lists none and a project's paths answer `503`. Its share links carry the preview's host, taken from each request.
>
> **A deployment without the two locations serves no CPE project.** The image has no fallback: a deployment that sets neither variable, or points them at a path the image does not have (such as `/app/sync/data` or `/app/media/data`), starts and serves DPE, and CPE answers as it does for a missing directory ([CPE Operations](../cpe/operations.md#start-up-and-what-cpe-answers-when-something-is-missing)). Dev's `access-server` is such a deployment until dev has its own locations (DEV-7992).
>
> **Why a preview rewrites its ARKs.** A preview runs code that is not merged. If it publishes the recorded ARK, a FAIR assessor harvests an identifier that resolves to production — so the deployment it assessed and the deployment it dereferenced are two different things running different code, and the assessment says nothing about either. `DPE_ARK_RESOLVER_BASE_URL` makes the preview's identifiers point at the preview, and mounts the resolver that answers them, so what is measured is one deployment throughout.
>
> **Applied once, as corpus data enters.** `dpe-core`'s caches normalise the ARK host on load, so everything downstream — the embedded JSON-LD, the Dublin Core tags, the `Link` header, the OAI payloads, the sidebar permalink a person clicks and copies, and the `pid` in `/dpe/api/v2/projects` — is correct without any of them knowing a resolver exists. Conceptually this is the `sync` capability's work and moves there when `sync` lands; `dpe_core::ark` records that.
>
> Only the host is rewritten. The ARK path — `ark:/72163/1/{shortcode}` — is the identifier and passes through untouched, and **recorded text is quoted rather than asserted**: a `howToCite` citation whose author wrote the ARK into the sentence, and a project's recorded website (083D records its own ARK there), are served as written.
>
> The resolver answers project ARKs only, with a `302` to the landing page, mirroring `ark.dasch.swiss`. A record ARK gets a `404`: DPE serves no record landing page, and `ark.dasch.swiss` resolves a record ARK to the VRE rather than to DPE.
>
> **Do not set it on STAGE or PROD.** Those deployments *are* what the recorded ARK resolves to, or are meant to become it. Setting it there would make them publish identifiers that only they answer, and mount a second ARK authority beside `ark.dasch.swiss`.

> **What the rate limit does not bound.** It bounds the request *rate*, not the
> per-request size, and the JSON-LD representation is large. Its own byte budget
> caps it at 4,000,000 bytes ([Machine-Readable
> Metadata](./machine-readable-metadata.md#a-byte-budget)), and locally the two
> largest committed projects measure 3,972,504 bytes (0868, at the budget) and
> 3,548,716 bytes (081C, under it). At the defaults (`per_second = 1`,
> `burst = 60`) one IP can therefore pull at most ~240 MB of transient allocation
> and egress in a burst window, settling to ~4 MB/s/IP afterwards. The budget is
> what makes that a *bound* rather than an observation: before it, the figure
> grew with the corpus. Lower `DPE_OAI_RATE_LIMIT_BURST` if that matters more
> than a harvester's throughput does.

> **Rate limiting and reverse proxies.** The OAI (`/dpe/oai`) and telemetry (`/telemetry/collect`) rate limits key on the client IP, taken from the **rightmost** `X-Forwarded-For` entry (the address Traefik itself appends), falling back to the connection peer address. Reading the rightmost entry — not the leftmost — is deliberate: Traefik appends the real client after any `X-Forwarded-For` value the client supplied, so the rightmost entry is proxy-authored and cannot be spoofed, while the leftmost stays attacker-controlled. This holds only while Traefik is the sole hop in front of DPE; a second proxy that appends to `X-Forwarded-For` would shift the trusted entry and require counting hops from the right.

## Crawlers

The project list's facets are a GET form, not links, so a crawler that follows links finds no URL per
facet combination. Two hints cover the filtered URLs that people share and bookmark:

- **`/robots.txt`** (`areas/access/dpe/public/robots.txt`, served by the static-file fallback) disallows
  `/dpe/projects` with any facet parameter, `search` or `dialog`, and the two fragment routes
  `/dpe/projects/results` and `/dpe/projects/search`. It does not disallow `page`: the paginated list stays
  crawlable. The `*` in its paths is a Google and Bing extension, not RFC 9309; a crawler without it
  ignores those lines. It names no `/cpe` path, so it never advertises a Hidden project
  (`ACCESS_HIDDEN_PROJECTS` above).
- **A canonical link.** A projects page with a facet, `search` or `dialog` carries
  `<link rel="canonical" href="<DPE_PUBLIC_BASE_URL>/dpe/projects">`; the list and its pages carry none. A
  crawler that obeys `robots.txt` never fetches those URLs, so the link serves crawlers that ignore
  `robots.txt` and link previews. No page carries a robots `noindex`: it would be read only on a page that
  `robots.txt` already keeps crawlers off.

Neither stops a crawler that ignores both; the reverse proxy's rate limit is the bound for that.

## Health Check

- **Endpoint**: `GET /healthz`
- **Response**: `200 OK` (no body)
- **Purpose**: Lightweight probe for Traefik/load balancers. Does not render any page.
- **CPE**: the probe answers as soon as the port is bound, before CPE's projects are loaded, and says nothing
  about them.

## Data Volume

- **Mount point**: Value of `DPE_DATA_DIR`
- **Access**: Read-only
- **Contents**: Project metadata JSON files, organized by type (`projects/`, `persons/`, `organizations/`, `clusters/`, `collections/`, `records/`)

## Resource Requirements

The DPE is lightweight — it serves static data with no database. CPE holds each project's store in memory.

- **Memory**: about 220 MiB resident at the peak of start-up, less afterwards; CPE holds each project's store in
  memory. The measured figures are in [CPE Operations](../cpe/operations.md#measurements).
- **CPU**: Minimal (SSR rendering is fast, data is cached in-memory)
- **Disk**: Data files + static assets (~70 MB without the binary)

### Landing-page metadata points at dsp-ingest

A project landing page advertises its records' files as `schema:distribution`,
each with the dsp-ingest URL that serves the bytes
([Machine-Readable Metadata](./machine-readable-metadata.md#record-files)). A
consumer that follows those URLs downloads from **`ingest.dasch.swiss`, not from
DPE**, so DPE's per-IP rate limiter does not bound the traffic.

How much there is to follow, for the two largest file-carrying projects in the
committed corpus:

| | In the embedded block | Bytes if all of those are fetched | In `/metadata.jsonld` | Bytes if all of those are fetched |
|---|---:|---:|---:|---:|
| 0868 | 100 | 90.9 MB | 6,107 of 7,716 | 945.0 MB |
| 0803 | 4 | 81.9 MB | 4,062 | 139.7 GB |

The `/metadata.jsonld` figures are now bounded by the representation's
[byte budget](./machine-readable-metadata.md#a-byte-budget), which is why 0868's
list stops short of its 7,716 files and its total fell from 2.72 GB. Both are
measured locally; a deployment on a longer host lists a shorter prefix and draws
less. 0803's whole graph fits the budget, so nothing about it changed.

A well-behaved assessor pulls far less, and the bound does not change what it
pulls. F-UJI 3.5.0 takes at most five files per MIME type and truncates each
download at 1,000,000 bytes, so a full assessment drew **26 files and 11.3 MB
for 0868 (558 s)** and **6 files and 6.0 MB for 0803 (307 s)**, measured on
2026-09-18 against the then-unbounded representation. All five of 0868's MIME
types — `application/pdf`, `application/zip`, `image/png`, `text/csv`,
`text/plain` — still appear inside the bounded prefix, so the same sampling has
the same set to draw from. Its draw scales with how many distinct file types a
project has, not with how many records it holds.

This is a change in convenience rather than in capability. The same bytes were
already reachable by enumerating the OAI-PMH set `project:{shortcode}` and
calling `/dpe/records/{shortcode}/{record_id}/file` per record, which returns
the same `downloadUrl`. What changed is that one request now returns every URL
at once.

**Retiring ingest carries a metadata obligation.** All 11,778 published
`contentUrl` values name `ingest.dasch.swiss`. When media moves to Vitrinli and
downloads are served by the Access Area's `media` capability, the host in them
changes. Metadata heals going forward on its own — the URL comes from the corpus
export, so the first export after the migration carries the new host and DPE
re-serves it without a code change — but copies already harvested by third
parties do not, and `schema:distribution` exists to be harvested. Plan the
retirement to include a corpus re-export and a re-publish, or external copies
404.

## Logging

Structured logging via `init-tracing-opentelemetry` (OTel-aware tracing subscriber). In production (`DPE_ENV=PROD`), logs are JSON-formatted to stdout only. In local development (`DPE_ENV=DEV`), logs are additionally exported via OTLP to Loki when `OTEL_EXPORTER_OTLP_ENDPOINT` is set. Configure levels with `RUST_LOG`:

```bash
# Default (info level)
RUST_LOG=info

# Debug HTTP requests
RUST_LOG=access_server=info,dpe_server=info,tower_http=debug

# Verbose debugging
RUST_LOG=debug
```

## Observability

### Fathom Analytics

Privacy-friendly, GDPR-compliant analytics. No cookies, no personal data collected.

**Configuration:** Set the `DPE_FATHOM_SITE_ID` environment variable to your Fathom site ID (not a secret). The tracking script is automatically injected into the HTML shell. It is set in `ops-deploy/group_vars/prod.yml` and nowhere else, so STAGE renders no script at all.

**What gets tracked:**
- Page views
- Tab switches (detected automatically via `history.replaceState`)

**Disable:** Omit the `DPE_FATHOM_SITE_ID` environment variable, or set it to the empty string — both are treated as unset and no tracking script is rendered.

**Second key:** where the script *is* rendered it carries `data-included-domains="repository.dasch.swiss"`, which Fathom enforces client-side against `window.location.hostname`. A deployment handed the site ID by mistake therefore still reports nothing.

### OpenTelemetry

DPE exports traces, metrics, and structured logs via OTLP gRPC. In production, the OTLP endpoint points to Grafana Alloy, which forwards to Grafana Cloud (Tempo for traces, Mimir for metrics, Loki for logs).

When `OTEL_EXPORTER_OTLP_ENDPOINT` is not set, the OTel SDK falls back to no-op export — the application runs normally without telemetry export. See `docs/src/dpe/observability.md` for the developer guide.

### Continuous Profiling (Pyroscope)

CPU profiling via Grafana Pyroscope. Samples at 100Hz and pushes profiles to the configured endpoint.

**Configuration:** Set `PYROSCOPE_ENDPOINT` to the Pyroscope HTTP endpoint. When unset, no profiling agent runs and there is zero overhead.

**What gets profiled:**
- CPU time per function (sampling-based, 100 samples/second)
- Flame graphs viewable in Grafana (Explore > Pyroscope)
