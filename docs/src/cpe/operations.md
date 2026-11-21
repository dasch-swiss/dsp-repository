# CPE Operations

CPE has no image and no process of its own. The Access Area's binary, `access-server`, serves it beside DPE,
and the image is the one [DPE Operations](../dpe/operations.md) describes. This page holds what is specific
to CPE.

## What is served

- **Where**: every path under `/cpe` (`CPE_BASE`). A project is at `/cpe/<project id>/`; today there is one,
  Incunabula, at `/cpe/incunabula/`. `/cpe` and `/cpe/` are CPE's overview: the projects it serves that are not
  hidden, each with a link to its start page. Any other path that names no project is CPE's own not-found page.
- **From what**: the project folders in `CPE_PROJECTS_DIR` (`SHORTCODE`, assets), `sync`'s snapshots in
  `SYNC_DATA_DIR` and `media`'s deposited files in `MEDIA_DATA_DIR` (configuration, curation, narrative), each
  read from the generation the project's Ready token names in that directory. The
  project folders are in the image; the other two are the locations the VRE bridge writes, which a deployment
  mounts, and without them CPE serves no project. Each project's store is built in memory from that data at every start; nothing is
  written to disk, and a restart leaves nothing behind.
- **Refresh**: by a project's Ready token, with no restart (ADR-0012). `sync` polls its tokens every
  `SYNC_POLL_SECS` and announces a project whose token appeared, changed or was removed. On an announcement for
  a shortcode a project folder declares, CPE re-activates: it loads every project again, and serves the result
  only if every project it served before loads again, or is no longer in `sync` (its token removed, after
  which it answers `404`; a project served before is then logged at warning level, `a served project is no
  longer in the archive projection; its prefix answers 404`, since a partial or stale mount looks the same). Each
  project's provenance is read from both ports before and after its store is built, and all four reads must be
  equal. Otherwise CPE keeps serving what it served, logs at error level which projects held the re-activation
  back (`CPE's re-activation is held back`), and tries once more after `RETRY_AFTER_POLLS` poll intervals (`access-server`);
  beyond that, the next announcement or a restart refreshes it. A project whose token is gone while
  `SYNC_DATA_DIR` is missing, unreadable or holds no project directory at all, as when a mount loses its target,
  holds the re-activation back rather than being taken for removed; so does removing the last project, which
  then takes a restart. A project whose `media` token was replaced and whose `sync` token was not yet is held
  back until `sync`'s announces. An announcement for a shortcode without a
  folder is ignored. A file edited in place without a new token is not seen until a restart. While a
  re-activation builds, the stores it replaces are still in memory, so it needs about twice their memory.
  Not yet in code: `media`'s
  port announces nothing (DEV-7938), and CPE re-activates every project instead of rebuilding the one named
  (DEV-7989).
- **Static files**: CPE's stylesheets, scripts and logos are compiled into the binary and served at
  `/cpe/<project id>/_static/…`, and at `/cpe/_static/…` for the overview and CPE's not-found page; a
  project's own files at `/cpe/<project id>/_assets/…`. `cpe/cpe.js` is answered with
  `Cache-Control: no-cache` and an `ETag` from its bytes, so a browser revalidates it on every page load
  (a `304` while it is unchanged) and never pairs a deployment's HTML with another deployment's script. The
  other static files carry no `Cache-Control` and no validator; the browser's heuristics decide.

## Configuration

The five variables (`CPE_BASE`, `CPE_PROJECTS_DIR`, `CPE_PUBLIC_ORIGIN`, `SYNC_DATA_DIR`, `MEDIA_DATA_DIR`) are
in the [environment table](../dpe/operations.md#environment-variables) of DPE Operations. The image sets `CPE_BASE`
and `CPE_PROJECTS_DIR`; the other three are the deployment's to set. The three `CPE_*` values can also come from a `cpe.toml` in the working directory; the
environment wins. A sixth, `ACCESS_HIDDEN_PROJECTS`, is in the same table: `access-server` reads it and hands
it to CPE, which honours it but never reads it, and it cannot come from `cpe.toml`. A seventh, `SYNC_POLL_SECS`,
how often `sync` polls its Ready tokens, is there too; the image does not set it.

### Hiding and un-hiding a project

A project whose shortcode is in `ACCESS_HIDDEN_PROJECTS` is a Hidden project: every response under its prefix
carries `X-Robots-Tag: noindex, nofollow`, its pages and its not-found carry
`<meta name="robots" content="noindex, nofollow">`, and the overview neither lists nor counts it. It is
otherwise served as before, to anyone with the link. The start log has three lines for it: the shortcodes
read (`hidden projects: none`, or `hidden projects: <list>`), the hidden project ids (`hidden project ids`),
and each listed shortcode that no project folder has (`a hidden shortcode names no project folder`), so a typo
shows there. A hidden project with no Ready token is not among the hidden project ids and carries no
`X-Robots-Tag`: it answers `404` as any project with no Ready token does.

### Releasing a project on a deployment

A project is released on a deployment by adding its shortcode to that deployment's VRE bridge allowlist,
`EDITOR_VRE_BRIDGE_ALLOWLIST`, in ops-deploy. The bridge then writes the project's data and Ready tokens to
`SYNC_DATA_DIR` and `MEDIA_DATA_DIR`, `sync` announces the project, and CPE serves it without a restart. On a
deployment that does not release it, a project whose folder is in the image has no Ready token: its paths
answer CPE's not-found, `404`, as a path that names no project, the overview does not list it, and the log has
`project is not in the archive projection; its prefix answers 404` at info level. Nothing in the
project's files marks it released, and merging a project folder does not by itself serve it anywhere.

The variable is read at start: hiding or un-hiding takes a restart or a redeploy. Search engines notice that a
project was un-hidden only when they next crawl it, and an HTTP cache keeps a `noindex` response until it
expires.

## Checking the projects: `access-server validate`

`access-server validate <dpe_data_dir>` checks DPE's data, then loads every CPE project once, as `serve` would,
and serves nothing. It reads the same variables as `serve` (`CPE_PROJECTS_DIR`, `SYNC_DATA_DIR`,
`MEDIA_DATA_DIR`, and their defaults, which fit a run from the repository root) and checks the projects whether
or not `CPE_BASE` is set. It prints one paragraph per project folder:

| Outcome | Printed | Fails the run |
|---------|---------|---------------|
| The project loads | `served`, and each validation warning with its place in `config.kdl` | no |
| The project has no Ready token in `SYNC_DATA_DIR` (not released where the check runs) | `warning: unknown here, so not checked`, with the reason | no |
| The project does not load, for any reason in the table below that answers `503` | `FAILED`, with the report CPE logs | yes |
| The folder's `SHORTCODE` is malformed, unreadable or a symlink, or its name is not a project id | `FAILED`, with the reason | yes |
| The folder has no `SHORTCODE` | nothing: it is no project | no |
| CPE's configuration is refused, or no folder has a `SHORTCODE` | `CPE: not checked`, with the reason | yes |

Every project's paragraph is printed before the verb exits, so one run shows every failure. Folder names and
report lines are printed with their control characters escaped. `serve` is not strict: there a project that
fails takes only itself offline.

`--inputs <dir>` names a directory laid out as the VRE bridge ships a project's inputs
(`<shortcode>/config.kdl`, `<shortcode>/curation.csv`, `<shortcode>/narrative/<name>`). For each project folder
it adds a data-free check of what is shipped for its shortcode: the configuration parses, its `remodel` block
loads and declares the folder's shortcode, its `site` block passes every check that needs neither archive data
nor assets (the languages and `default-language` among them, with the diagnostics the full load gives), and each
narrative file it names has its base file or its variant in the site's default language (`default-language`,
else the first language). The check follows no symbolic link inside the inputs directory: a `narrative` that is
not a real directory fails it, and a linked file counts as missing. A narrative file name the deposited files
refuse at activation (characters outside `A-Za-z0-9._-`, a leading `.`, a path separator) fails it too. Whether
the cover image is in the project's assets, the nav and `not-found` routes, pages and components are checked by
the full load only: they need the project's assets or archive data.

The data-free check's outcome is a line below the project's: `inputs: checked`, `inputs: none shipped`, or
`inputs: FAILED` with the report, which fails the run. A project with neither stand-in data (unknown) nor shipped
inputs passes with the note `neither stand-in data nor shipped inputs`. A directory given that is not there fails
the run.

`--must-serve <shortcode>` (repeatable, or comma-separated) fails the run when that project is unknown, failed or
declared by no folder, so a stand-in that went missing cannot pass as an unknown project. When the check fails as
a whole (`CPE: not checked`), that error is the one cause printed.

`just validate-cpe` runs the verb from the repository root, as `just validate-data` does, against the committed
stand-ins under `areas/access/{sync,media}/data` (`serve`'s defaults), with
`--inputs areas/deposit/vre-bridge/inputs --must-serve 0803`. `test.yml`'s required job runs it after
`just test`. A project with a committed stand-in gets the full check, and one with shipped inputs alone, as a new
project has before its data is deposited, gets the data-free one. Run it after changing a project folder, its
shipped inputs or its stand-in.

The image carries CPE's project folders but no data of `sync`'s or `media`'s, so `validate` run in a container
without `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` pointing at mounted data fails every project, as `serve` would
answer `503` for each.

## Start-up, and what CPE answers when something is missing

The port is bound first, and CPE's projects are loaded afterwards, in the background. Until they are, every
path under `/cpe` answers `503` with `Retry-After: 30` and `Cache-Control: no-store`. That `503`, and the one
a refused configuration answers, carries no `X-Robots-Tag`, since no project id is known yet. CPE's
not-found, `404`, also carries `Cache-Control: no-store`, since the next re-activation may serve what it
answers for. DPE and `/healthz` answer throughout. Nothing of CPE's stops the binary or delays its start.

| Situation | Under `/cpe` | Logged | DPE |
|-----------|--------------|--------|-----|
| Projects are still loading | `503`, every path | `CPE is mounted and activates in the background` (info) | serves |
| A project is loaded | its pages, and its entry in the overview, unless it is a Hidden project | `project loaded`, then `CPE's projects are loaded` with the counts (info) | serves |
| A project's Ready token in `SYNC_DATA_DIR` is missing while its directory, or another project's, is there: the project is not released on this deployment | CPE's not-found, `404`, on that project's paths, as for a path that names no project; the overview does not list it | `project is not in the archive projection; its prefix answers 404`, with the shortcode (info) | serves |
| A re-activation finds a project it served with no Ready token in `SYNC_DATA_DIR` while its directory, or another project's, is there | as for a project with no Ready token: CPE's not-found, `404` | `a served project is no longer in the archive projection; its prefix answers 404`, with the project id (warning), then `CPE re-activated` | serves |
| A re-activation is held back | what was served before | `CPE's re-activation is held back`, with the project ids (error), after each project's own line | serves |
| `SYNC_DATA_DIR` is missing or holds no project directory; the project's Ready token is unreadable or a symlink; the generation the token names or its snapshot is missing, unreadable or a symlink; or the token names an interim format version `sync` does not read | `503` on that project's paths; the overview does not list it | `project does not load; its prefix answers 503`, with the reason (error); for a missing directory also `SYNC_DATA_DIR names no directory …` at start (warning) | serves |
| `MEDIA_DATA_DIR`, a project's Ready token in it, the generation the token names, or a file the project needs from it is missing, unreadable or not UTF-8 | `503` on that project's paths; the overview does not list it | `project does not load; its prefix answers 503`, with the reason (error); for a missing directory also `MEDIA_DATA_DIR names no directory …` at start (warning) | serves |
| A project folder has no `SHORTCODE` | the folder is no project: CPE's not-found, `404`, on its paths | nothing | serves |
| A project folder's `SHORTCODE` is malformed or a symlink | the project is not served: CPE's not-found, `404`, on its paths | `a project folder is not served`, with the folder and the reason (warning) | serves |
| Two project folders declare the same `SHORTCODE` | `503` on both projects' paths | `project does not load; its prefix answers 503`, naming the other folder (error) | serves |
| `CPE_PROJECTS_DIR` does not exist | the overview with no project, `200`, at `/cpe` and `/cpe/`; CPE's not-found, `404`, on every other path | `CPE's projects are loaded` with `served=0` (info) and nothing else | serves |
| An entry of `ACCESS_HIDDEN_PROJECTS` is not a shortcode | as without that entry; the other entries hide their projects | `ACCESS_HIDDEN_PROJECTS: an entry is not a shortcode and is ignored`, with the entry (warning) | serves |
| `ACCESS_HIDDEN_PROJECTS` is not UTF-8 | as without the variable: nothing is hidden | `ACCESS_HIDDEN_PROJECTS is not UTF-8 and is ignored: no project is hidden` (warning) | serves |
| `CPE_PUBLIC_ORIGIN` is refused (a path, a trailing slash) | `503`, every path | `CPE's configuration is refused …`, with the reason (error) | serves |
| `CPE_BASE` is unset or empty | not mounted: DPE's `404` | `CPE_BASE is not set: CPE is not mounted …` (warning) | serves alone |
| `CPE_BASE` is refused (no leading `/`, a trailing one) | not mounted: DPE's `404` | `CPE's configuration is refused …`, with the reason (error) | serves alone |
| `CPE_BASE` is a path the binary or DPE has a route for (`/healthz`, `/dpe/projects`) | mounted: the route keeps its answer, CPE answers the rest | as for a mounted CPE | serves its routes; its static files under the base are CPE's `404` |
| The router cannot be built with CPE at `CPE_BASE` (no value is known to do this) | not mounted: DPE's answer | the panic's own line, then `the router's build with CPE at CPE_BASE panicked …`, with the base and the reason (error) | serves alone |

`/healthz` says nothing about CPE. Whether Incunabula is served is read from the log line
`CPE's projects are loaded` (`served`, `unavailable`, `unknown`) or from the status of `/cpe/incunabula/`.

A CPE request is traced by the binary's layers, under DPE's telemetry identity, with a span named by CPE's
route (`GET /cpe/incunabula/books/{id}`).

## Measurements

Measured on 2026-10-12. Two set-ups, both with the image's environment and the committed data (Incunabula: 4,198 resources,
20.3 MB of snapshot and curation files, on that date both in `SYNC_DATA_DIR`; the curation file has since moved
to `MEDIA_DATA_DIR`):

- **The image**, built from a staged context as CI builds it, but for `linux/arm64` (CI builds `linux/amd64`):
  a static musl binary, run with `docker run --cpus=1 --memory=512m`, the limits of a PR preview, under Docker
  Desktop on an Apple M5 Max.
- **The host**: the release binary run on macOS on the same machine, from a directory laid out as the image.

Note (2026-11-20): these measurements were taken with an image that carried the 0803 data. The current image
carries none, so they cannot be reproduced with it without mounting the data.

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

CPE's data, counted on 2026-10-21 as the sum of the files' sizes: `sync/data` 20,264,543 bytes (3 files) and
`media/data` 90,734 bytes (19 files), which a deployment mounts and the image does not carry, and
`cpe/projects` 122,868 bytes (7 files), which the image carries.

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

DPE's Playwright suite starts `access-server` without `CPE_BASE`, so CPE is not mounted in it. CPE's suite
(`areas/access/cpe/web-e2e-tests`, `just test-e2e-cpe`) sets `CPE_BASE=/cpe` and `DPE_SITE_ADDR=127.0.0.1:4102`
and nothing else, so the binary serves the committed data of project 0803 from the repository root; it waits for
`/cpe/incunabula/` to answer, not for `/healthz`. `just run` and `just dev` set `CPE_BASE=/cpe`.
