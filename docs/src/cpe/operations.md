# CPE Operations

CPE has no image and no process of its own. The Access Area's binary, `access-server`, serves it beside DPE,
and the image is the one [DPE Operations](../dpe/operations.md) describes. This page holds what is specific
to CPE.

## What is served

- **Where**: every path under `/cpe` (`CPE_BASE`). A project is at `/cpe/<project id>/`; today there is one,
  Incunabula, at `/cpe/incunabula/`. `/cpe` and `/cpe/` are CPE's own not-found page: there is no index of
  projects.
- **From what**: the project folders in `CPE_PROJECTS_DIR` (configuration, narrative, assets) and `sync`'s
  data in `SYNC_DATA_DIR`. Both are in the image. Each project's store is built in memory from that data at
  every start; nothing is written to disk, and a restart leaves nothing behind.
- **Static files**: CPE's stylesheets, scripts and logos are compiled into the binary and served at
  `/cpe/<project id>/_static/…`; a project's own files at `/cpe/<project id>/_assets/…`.

## Configuration

The four variables (`CPE_BASE`, `CPE_PROJECTS_DIR`, `CPE_PUBLIC_ORIGIN`, `SYNC_DATA_DIR`) are in the
[environment table](../dpe/operations.md#environment-variables) of DPE Operations. The image sets all but
`CPE_PUBLIC_ORIGIN`. The three `CPE_*` values can also come from a `cpe.toml` in the working directory; the
environment wins.

## Start-up, and what CPE answers when something is missing

The port is bound first, and CPE's projects are loaded afterwards, in the background. Until they are, every
path under `/cpe` answers `503` with `Retry-After: 30` and `Cache-Control: no-store`. DPE and `/healthz`
answer throughout. Nothing of CPE's stops the binary or delays its start.

| Situation | Under `/cpe` | Logged | DPE |
|-----------|--------------|--------|-----|
| Projects are still loading | `503`, every path | `CPE is mounted and activates in the background` (info) | serves |
| A project is loaded | its pages | `project loaded`, then `CPE's projects are loaded` with the counts (info) | serves |
| `SYNC_DATA_DIR` or a project's file in it is missing | `503` on that project's paths | `project does not load; its prefix answers 503`, with the reason (error); for a missing directory also `SYNC_DATA_DIR names no directory …` at start (warning) | serves |
| `CPE_PROJECTS_DIR` does not exist | CPE's not-found, `404`, every path | `CPE's projects are loaded` with `served=0` (info) and nothing else | serves |
| `CPE_PUBLIC_ORIGIN` is refused (a path, a trailing slash) | `503`, every path | `CPE's configuration is refused …`, with the reason (error) | serves |
| `CPE_BASE` is unset or empty | not mounted: DPE's `404` | `CPE_BASE is not set: CPE is not mounted …` (warning) | serves alone |
| `CPE_BASE` is refused (no leading `/`, a trailing one) | not mounted: DPE's `404` | `CPE's configuration is refused …`, with the reason (error) | serves alone |
| `CPE_BASE` is a path the binary or DPE has a route for (`/healthz`, `/dpe/projects`) | mounted: the route keeps its answer, CPE answers the rest | as for a mounted CPE | serves its routes; its static files under the base are CPE's `404` |
| The router cannot be built with CPE at `CPE_BASE` (no value is known to do this) | not mounted: DPE's answer | the panic's own line, then `the router's build with CPE at CPE_BASE panicked …`, with the base and the reason (error) | serves alone |

`/healthz` says nothing about CPE. Whether Incunabula is served is read from the log line
`CPE's projects are loaded` (`served`, `unavailable`) or from the status of `/cpe/incunabula/`.

A CPE request is traced by the binary's layers, under DPE's telemetry identity, with a span named by CPE's
route (`GET /cpe/incunabula/books/{id}`).

## Measurements

Measured on 2026-10-12. Two set-ups, both with the image's environment and the committed data (Incunabula: 4,198 resources,
20.3 MB in `SYNC_DATA_DIR`):

- **The image**, built from a staged context as CI builds it, but for `linux/arm64` (CI builds `linux/amd64`):
  a static musl binary, run with `docker run --cpus=1 --memory=512m`, the limits of a PR preview, under Docker
  Desktop on an Apple M5 Max.
- **The host**: the release binary run on macOS on the same machine, from a directory laid out as the image.

| | The image, 1 CPU, 512 MiB | The image, 2 CPUs, 1 GiB | The host |
|---|---:|---:|---:|
| Port bound, after process start | 0.06 to 0.07 s | 0.08 s | 0.21 s |
| Incunabula loaded, after process start | 1.12 to 1.14 s | 0.86 s | 0.63 to 0.64 s |
| of which loading the project | 1.05 to 1.08 s | 0.79 s | 0.42 to 0.43 s |
| Peak resident memory, start-up | 213 to 214 MiB | 213 MiB | 265 to 269 MiB |
| Resident memory once loaded | 136 MiB | 136 MiB | not read |
| Peak resident memory with `CPE_BASE` empty (DPE alone) | 154 MiB | not run | 173 to 194 MiB |
| Peak resident memory after 61,064 requests | 212 MiB | not run | 272 MiB |

`/healthz` answered `200` before the log line `CPE's projects are loaded` in every run, and the first request
to `/cpe/incunabula/` answered `503`. The memory figures are of the whole process, DPE included: the peak is
reached while the project is loaded, and the snapshot, the built rows and the SQL text are freed afterwards.
The 61,064 requests are a crawl of every URL Incunabula's pages link to, 48 at a time
against the image and 8 against the host; after it Docker reported the container healthy, with
no restart and no out-of-memory kill.

A PR preview on Cloud Run is deployed with `--no-cpu-throttling`. With throttling, an instance gets CPU only
while it handles a request, and `/cpe/incunabula/` answered `503` after a cold start until the sixth request.
The load time on Cloud Run without throttling is not measured.

The image's data grew by 195,708 bytes (23 files, `cpe/projects`): the staged
directory without the binary was 69,819,957 bytes before and is 70,015,665 bytes with it. `sync/data`
(20,281,022 bytes) was already staged.

## Response times: the baseline for store reads on the request path

A page's store reads run on the async worker threads the binary shares with DPE, and one project's requests
wait on one lock (`ARCH-MAP.md`, `areas/access/cpe`). These figures are what a change to that is compared
with. Times are to the last byte.

Every URL Incunabula's pages link to (61,064) was asked once by one client, on the host: median
1.0 ms, 95th percentile 1.9 ms, 99th 2.1 ms, slowest 9.5 ms.
The slowest pages of that pass and five ordinary ones (18 pages) were then asked thirty times each by one
client, and by eight clients at once, each its own process, 25 rounds over the list:

| 18 of Incunabula's pages | The image, 1 CPU, 512 MiB | The host |
|---|---:|---:|
| One client: median | 3.9 ms | 3.8 ms |
| One client: slowest | 10.7 ms | 11.2 ms |
| Eight clients: median | 32.6 ms | 22.9 ms |
| Eight clients: 95th percentile | 46.5 ms | 66.1 ms |
| Eight clients: slowest | 84.0 ms | 153.1 ms |

The five slowest responses, in the image:

| URL | Bytes | One client, median | Eight clients, median | 95th percentile | Slowest |
|-----|------:|------:|------:|------:|------:|
| `/cpe/incunabula/woodcuts?lang=de` | 138,864 | 8.9 ms | 36.5 ms | 56.3 ms | 84.0 ms |
| `/cpe/incunabula/woodcuts` | 138,844 | 8.8 ms | 34.2 ms | 48.9 ms | 60.6 ms |
| `/cpe/incunabula/books/narrenschiff-dt?page=narrenschiff-dt-u1r&lang=de` | 109,758 | 5.2 ms | 36.9 ms | 51.4 ms | 68.0 ms |
| `/cpe/incunabula/books/narrenschiff-dt/_compound?lang=en&page=narrenschiff-dt-u1r` | 103,269 | 5.2 ms | 35.0 ms | 55.4 ms | 77.6 ms |
| `/cpe/incunabula/books/bereitung/_compound?page=bereitung-d2r-2&lang=de` | 130,005 | 4.0 ms | 34.5 ms | 48.9 ms | 60.1 ms |

And what the same eight clients did to an answer that reads no store, in the image at one CPU:

| URL | One client, median | Eight clients on CPE's pages, median | 95th percentile | Slowest |
|-----|------:|------:|------:|------:|
| `/dpe/projects` | 0.4 ms | 27.0 ms | 42.9 ms | 61.2 ms |
| `/cpe/incunabula/_static/cpe/shell.css` | 0.1 ms | 24.9 ms | 42.5 ms | 58.0 ms |

On the host, with more cores than clients, `/dpe/projects` stayed at a median of
0.3 ms under the same load. So on one CPU eight clients on
CPE's pages slow every answer of the binary, DPE's included; CPE has no rate limit.

Incunabula has no search bar. A project with one has a request these figures do not cover.

## Tests that start the binary

DPE's Playwright suites (end-to-end and accessibility) start `access-server` without `CPE_BASE`, so CPE is
not mounted in them. `just run` and `just dev` set `CPE_BASE=/cpe`.
