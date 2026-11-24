# AI Agent Guide for `access-server`

The Access Area's composition root (ADR-0003), binary `access-server`; DPE-specific guidance is
in `areas/access/dpe/CLAUDE.md`.

## What this crate is

`main`, CLI dispatch (`serve`, `validate`, `healthcheck`), OTel/Pyroscope init and router
assembly — nothing else. Per ADR-0003, a composition root "holds no adapter logic": it constructs
each capability and injects it, and mounts routes, but does not implement business logic of its
own. It wires two capabilities, DPE and CPE (ADR-0007), without either depending on the other, and
constructs `sync`'s and `media`'s adapters of CPE's two ports to hand them to CPE.

## Router assembly: the traced/untraced order matters

`serve.rs`'s `app()` merges DPE's router and nests CPE's, applies the OTel layers, then mounts
`/healthz` and `POST /telemetry/collect` *after* `.layer()`. Axum does not wrap routes declared
after a `.layer()` call, so this order is what keeps liveness probes and the telemetry beacon
itself untraced. A test pins it — do not reorder without checking `serve.rs`'s tests.

## CPE: nested at its base, activated in the background

- **The base has one source.** `mount_cpe` nests `Cpe::router()` at `Cpe::base()` (`CPE_BASE`), and CPE
  writes the same value in front of every link. This crate writes no CPE path of its own; a test fails
  on a `"/cpe` literal in a file of `src/` outside a test module.
- **Nested, never merged.** CPE's router ends in its own fallback, so every path under the base is
  CPE's (its 404 and its 503 included) and every other path stays DPE's, `ServeDir` fallback included.
  Axum panics on merging two routers that each have a fallback. `nest` does not match `<base>/`, so
  that one path has its own route. `<base>` and `<base>/` are both the root of CPE's router, which is
  CPE's overview of its projects; neither redirects to the other.
- **No base, no mount.** With `CPE_BASE` unset or refused, `Cpe::base()` is empty: `nest` panics on an
  empty path and CPE at the root would answer DPE's paths, so the binary serves DPE alone and logs why.
  `just run`, `just dev` and the Docker image set `CPE_BASE=/cpe`. The image also sets `CPE_PROJECTS_DIR` to
  the copy it carries; `CPE_PUBLIC_ORIGIN`, `SYNC_DATA_DIR` and `MEDIA_DATA_DIR` are the deployment's to set,
  the last two to the locations the VRE bridge writes, mounted read-only.
- **CPE never stops or delays DPE.** `CpeConfig::load` and `Cpe::new` cannot fail and read nothing.
  `activate_in_background` calls `Cpe::follow` in `spawn_blocking` after the listener is bound, over `sync-store`'s
  `LiveArchiveProjection` on `SYNC_DATA_DIR` (default `areas/access/sync/data`) and `media-store`'s
  `LiveDepositedFiles` on `MEDIA_DATA_DIR` (default `areas/access/media/data`); `data_dir` takes an empty value
  for unset. Until the first activation finishes, and for a project that failed, CPE answers 503; a missing data directory is
  such a failure, not a start-up error, and a missing `MEDIA_DATA_DIR` is logged as a warning. Do not `.await` the activation and do not `expect` anything of CPE's in `serve()`.
  `serve()` passes `SYNC_POLL_SECS` (`sync_poll_interval`) to `LiveArchiveProjection::with_poll_interval`, and
  `RETRY_AFTER_POLLS` poll intervals to `follow` as the wait before it retries a held-back re-activation; CPE re-activates on
  `sync`'s announcements on a thread of its own (`docs/src/cpe/operations.md`, *Refresh*). The Ready-token poll
  belongs to `sync` and `media`, so add no reload endpoint or signal for CPE here (ADR-0012). Not yet in code:
  `media`'s port announces nothing (DEV-7938), and CPE re-activates every project instead of rebuilding the
  one named (DEV-7989).
  `app` builds the router with CPE inside `catch_unwind`: if that build panics, it logs an error naming
  `CPE_BASE` and builds DPE alone. No base does that today: a route of DPE's or the binary's under the base
  (`/healthz`, `/dpe/projects`) keeps answering, and only a second any-method route for `<base>/` panics.
- **Hidden projects are the root's to read.** `hidden_projects_from_env` reads `ACCESS_HIDDEN_PROJECTS` once
  (`parse_hidden_projects`: trimmed, upper-cased, checked with `cpe_ports::is_shortcode`; a bad entry is a
  warning, never a stop) and hands the set to each capability that presents projects as a value in its
  config, today `CpeConfig::hidden_shortcodes`; no capability reads the variable itself. The image does not
  set it: it is the deployment's to set, as `CPE_PUBLIC_ORIGIN` is (`areas/access/CONTEXT.md`, Hidden project).
- **The span of a CPE request.** The OTel layer sees CPE as one service, so `name_span_by_cpe_route`
  names the span from the `cpe_server::MatchedRoute` CPE puts on its responses:
  `GET /cpe/<project id>/<route template>`, and `GET /cpe/` for the overview. A path that matches no
  route keeps the method alone.

## The image: the places that have to agree

The published image is `:image` in `BUILD.bazel`. `image_data` maps each data directory to its place in the
image, and the `env` of `app_image` names where each copy is. Today that is `server/data` (`DPE_DATA_DIR`) and
`cpe/projects` (`CPE_PROJECTS_DIR`), each under `/app/`. Data for a capability is added in both, as a
filegroup the data's package grants to this one, and in the environment table of `docs/src/dpe/operations.md`.
Nothing checks that they agree: a directory the layer places where no `env` points, or an `env` that names the
wrong place, builds, and the container is healthy with CPE answering 503, or an overview with no project and
its not-found. `just image-smoke` starts the image and expects `/healthz` to answer, which a misplaced data
directory still passes.

The image carries no data of `sync`'s or `media`'s, and sets neither `SYNC_DATA_DIR` nor `MEDIA_DATA_DIR`:
a deployment mounts the VRE bridge's locations (`areas/deposit/ADR-0005`). Their committed `data` filegroups
are `testonly`, so `image_data`, which is not, fails analysis if it lists one; keep them `testonly` and do
not add a non-`testonly` copy. With neither variable set, the image serves DPE and no CPE project.

A new capability's paths also go into each path filter that should see it (`a11y-dpe.yml`,
`cloud-run-dpe-pull-request.yml`, `scout-dpe.yml`); `docs/src/deployment.md` says what each runs on. All three
have `areas/access/cpe/**`, `areas/access/sync/**` and `areas/access/media/**` beside DPE's and this crate's
paths, so a change to a presentation's files is previewed, scanned and checked for accessibility.
`bazel query 'deps(//areas/access/server:image_musl)' --output=package` lists every package the image is
built from.

## Dependency boundary

`BUILD.bazel` depends on `dpe-server`, `cpe-server`, `cpe-ports`, `sync-store`, `media-store`,
`shared-rate-limit`, `shared-telemetry` and infra crates only — never on a capability's `core`, `web` or `api-*` crate, and never on
`cpe-store`. This crate imports only `dpe-server`'s public surface (`DpeConfig`, `Dpe`, `validate`,
`normalize_page_url`), `cpe-server`'s (`CpeConfig`, `Cpe`,
`MatchedRoute`), the ports' traits and `is_shortcode` from `cpe-ports`, `sync-store`'s `LiveArchiveProjection` and
`DEFAULT_POLL_INTERVAL`, and `media-store`'s `LiveDepositedFiles`;
reaching past them into a capability's internals defeats the boundary the capability crate exists
to hold. Bazel `visibility` enforces this: no `core`, `web` or `api-*` target and not `cpe-store` grants this
package, so such an edge fails analysis (ADR-0003, ADR-0007).

## Telemetry identity is DPE's, not this crate's

The OTel tracer name and the Pyroscope application name are pinned to the literal `"dpe-server"`
(`observability.rs::TELEMETRY_NAME`), and the telemetry collector's OTel scope and the Pyroscope
`service.namespace` tag are `"dpe"`. These are Grafana dashboard identities, not derived from
`CARGO_PKG_NAME`: renaming this crate must never change what a dashboard groups traces and
profiles under. CPE's requests are traced by the same layers and so appear under this identity
too, told apart by their span names; CPE sends no browser beacons. If CPE gets a telemetry identity
of its own, it keeps it the same way — this constant does not become a general one.
