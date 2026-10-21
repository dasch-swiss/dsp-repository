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
  `just run`, `just dev` and the Docker image set `CPE_BASE=/cpe`. The image also sets `SYNC_DATA_DIR`,
  `MEDIA_DATA_DIR` and `CPE_PROJECTS_DIR` to the copies it carries; `CPE_PUBLIC_ORIGIN` is the deployment's to
  set.
- **CPE never stops or delays DPE.** `CpeConfig::load` and `Cpe::new` cannot fail and read nothing.
  `Cpe::activate` runs in `spawn_blocking` after the listener is bound, over `sync-store`'s
  `LiveArchiveProjection` on `SYNC_DATA_DIR` (default `areas/access/sync/data`) and `media-store`'s
  `LiveDepositedFiles` on `MEDIA_DATA_DIR` (default `areas/access/media/data`); `data_dir` takes an empty value
  for unset. Until it finishes, and for a project that failed, CPE answers 503; a missing data directory is
  such a failure, not a start-up error, and a missing `MEDIA_DATA_DIR` is logged as a warning. Do not `.await` the activation and do not `expect` anything of CPE's in `serve()`.
  Activation runs once per start, so for now an edit under either data directory needs a restart. Not yet in
  code: the ports announce change and CPE rebuilds the one project named; the operator lever belongs to `sync`
  and `media`, so add no reload endpoint or signal for CPE here (ADR-0012, DEV-7622).
  `app` builds the router with CPE inside `catch_unwind`: if that build panics, it logs an error naming
  `CPE_BASE` and builds DPE alone. No base does that today: a route of DPE's or the binary's under the base
  (`/healthz`, `/dpe/projects`) keeps answering, and only a second any-method route for `<base>/` panics.
- **The span of a CPE request.** The OTel layer sees CPE as one service, so `name_span_by_cpe_route`
  names the span from the `cpe_server::MatchedRoute` CPE puts on its responses:
  `GET /cpe/<project id>/<route template>`, and `GET /cpe/` for the overview. A path that matches no
  route keeps the method alone.

## The image: the places that have to agree

`.github/actions/build-dpe/action.yml` fills the `staging/` directory, `Dockerfile` copies from it, and the `ENV`
lines name where each copy is. Today that is `sync/data` (`SYNC_DATA_DIR`), `cpe/projects` (`CPE_PROJECTS_DIR`)
and `media/data` (`MEDIA_DATA_DIR`), each under `/app/`. `just build-docker-dpe` stages the same directories
for a local image, so it is a fourth place. Data for a capability is added in all of them, and in the
environment table of `docs/src/dpe/operations.md`. Nothing checks that they agree: a path that is in `staging/` but not copied, or an
`ENV` that names the wrong place, builds, and the container is healthy with CPE answering 503, or an overview
with no project and its not-found.

A new capability's paths also go into each path filter that should see it (`a11y-dpe.yml`,
`cloud-run-dpe-pull-request.yml`, `scout-dpe.yml`); `docs/src/deployment.md` says what each runs on. All three
have `areas/access/cpe/**`, `areas/access/sync/**` and `areas/access/media/**` beside DPE's and this crate's
paths, so a change to a presentation's files is previewed, scanned and checked for accessibility.

## Dependency boundary

`Cargo.toml` depends on `dpe-server`, `cpe-server`, `cpe-ports`, `sync-store`, `media-store`,
`shared-telemetry` and infra crates only — never on a capability's `core`, `web` or `api-*` crate, and never on
`cpe-store`. This crate imports only `dpe-server`'s public surface (`DpeConfig`, `Dpe`, `validate`,
`normalize_page_url`, `RightmostXffKeyExtractor`), `cpe-server`'s (`CpeConfig`, `Cpe`,
`MatchedRoute`), the ports' traits from `cpe-ports`, `sync-store`'s `LiveArchiveProjection` and
`media-store`'s `LiveDepositedFiles`;
reaching past them into a capability's internals defeats the boundary the capability crate exists
to hold. `.github/scripts/check-composition-root-deps.sh` and `check-cpe-seam.sh` enforce this in CI.

## Telemetry identity is DPE's, not this crate's

The OTel tracer name and the Pyroscope application name are pinned to the literal `"dpe-server"`
(`observability.rs::TELEMETRY_NAME`), and the telemetry collector's OTel scope and the Pyroscope
`service.namespace` tag are `"dpe"`. These are Grafana dashboard identities, not derived from
`CARGO_PKG_NAME`: renaming this crate must never change what a dashboard groups traces and
profiles under. CPE's requests are traced by the same layers and so appear under this identity
too, told apart by their span names; CPE sends no browser beacons. If CPE gets a telemetry identity
of its own, it keeps it the same way — this constant does not become a general one.
