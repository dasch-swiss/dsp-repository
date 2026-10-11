# AI Agent Guide for `access-server`

The Access Area's composition root (ADR-0003), binary `access-server`; DPE-specific guidance is
in `areas/access/dpe/CLAUDE.md`.

## What this crate is

`main`, CLI dispatch (`serve`, `validate`, `healthcheck`), OTel/Pyroscope init and router
assembly — nothing else. Per ADR-0003, a composition root "holds no adapter logic": it constructs
each capability and injects it, and mounts routes, but does not implement business logic of its
own. It wires two capabilities, DPE and CPE (ADR-0007), without either depending on the other, and
constructs `sync`'s adapter of CPE's port to hand it to CPE.

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
  that one path has its own route.
- **No base, no mount.** With `CPE_BASE` unset or refused, `Cpe::base()` is empty: `nest` panics on an
  empty path and CPE at the root would answer DPE's paths, so the binary serves DPE alone and logs why.
  `just run`, `just dev` and the Docker image set `CPE_BASE=/cpe`. The image also sets `SYNC_DATA_DIR` and
  `CPE_PROJECTS_DIR` to the copies it carries; `CPE_PUBLIC_ORIGIN` is the deployment's to set.
- **CPE never stops or delays DPE.** `CpeConfig::load` and `Cpe::new` cannot fail and read nothing.
  `Cpe::activate` runs in `spawn_blocking` after the listener is bound, over `sync-store`'s
  `LiveArchiveProjection` on `SYNC_DATA_DIR` (default `areas/access/sync/data`). Until it finishes,
  and for a project that failed, CPE answers 503; a missing data directory is such a failure, not a
  start-up error. Do not `.await` the activation and do not `expect` anything of CPE's in `serve()`.
  `app` builds the router with CPE inside `catch_unwind`: if that build panics, it logs an error naming
  `CPE_BASE` and builds DPE alone. No base does that today: a route of DPE's or the binary's under the base
  (`/healthz`, `/dpe/projects`) keeps answering, and only a second any-method route for `<base>/` panics.
- **The span of a CPE request.** The OTel layer sees CPE as one service, so `name_span_by_cpe_route`
  names the span from the `cpe_server::MatchedRoute` CPE puts on its responses:
  `GET /cpe/<project id>/<route template>`. A path that matches no route keeps the method alone.

## The image: three places that have to agree

`.github/actions/build-dpe/action.yml` fills the `staging/` directory, `Dockerfile` copies from it, and the `ENV`
lines name where each copy is. Data for a capability is added in all three, and in the environment table of
`docs/src/dpe/operations.md`. Nothing checks that they agree: a path that is in `staging/` but not copied, or an
`ENV` that names the wrong place, builds, and the container is healthy with CPE answering 503 or its not-found.

A new capability's paths also go into each path filter that should see it (`a11y-dpe.yml`,
`cloud-run-dpe-pull-request.yml`, `scout-dpe.yml`); `docs/src/deployment.md` says what each runs on.

## Dependency boundary

`Cargo.toml` depends on `dpe-server`, `cpe-server`, `cpe-ports`, `sync-store`, `shared-telemetry`
and infra crates only — never on a capability's `core`, `web` or `api-*` crate, and never on
`cpe-store`. This crate imports only `dpe-server`'s public surface (`DpeConfig`, `Dpe`, `validate`,
`normalize_page_url`, `RightmostXffKeyExtractor`), `cpe-server`'s (`CpeConfig`, `Cpe`,
`MatchedRoute`), the port's trait from `cpe-ports` and `sync-store`'s `LiveArchiveProjection`;
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
