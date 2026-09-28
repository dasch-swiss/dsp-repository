# AI Agent Guide for `access-server`

The Access Area's composition root (ADR-0003), binary `access-server`; DPE-specific guidance is
in `areas/access/dpe/CLAUDE.md`.

## What this crate is

`main`, CLI dispatch (`serve`, `validate`, `healthcheck`), OTel/Pyroscope init and router
assembly — nothing else. Per ADR-0003, a composition root "holds no adapter logic": it constructs
each capability and injects it, and mounts routes, but does not implement business logic of its
own. Today DPE is the Access Area's only capability; when CPE joins (ADR-0007), this crate wires
both without either depending on the other.

## Router assembly: the traced/untraced order matters

`serve.rs`'s `app()` merges each capability's router, applies the OTel layers, then mounts
`/healthz` and `POST /telemetry/collect` *after* `.layer()`. Axum does not wrap routes declared
after a `.layer()` call, so this order is what keeps liveness probes and the telemetry beacon
itself untraced. A test pins it — do not reorder without checking `serve.rs`'s tests.

## Dependency boundary

`Cargo.toml` depends on `dpe-server`, `shared-telemetry` and infra crates only — never on
`dpe-core`, `dpe-web` or `dpe-api-oai`. This crate imports only `dpe-server`'s public surface
(`DpeConfig`, `Dpe`, `validate`, `normalize_page_url`, `RightmostXffKeyExtractor`); reaching past
it into a capability's internals defeats the boundary the capability crate exists to hold.
`.github/scripts/check-composition-root-deps.sh` enforces this in CI.

## Telemetry identity is DPE's, not this crate's

The OTel tracer name and the Pyroscope application name are pinned to the literal `"dpe-server"`
(`observability.rs::TELEMETRY_NAME`), and the telemetry collector's OTel scope and the Pyroscope
`service.namespace` tag are `"dpe"`. These are Grafana dashboard identities, not derived from
`CARGO_PKG_NAME`: renaming this crate must never change what a dashboard groups traces and
profiles under. When a second capability arrives, its telemetry keeps its own identity the same
way — this constant does not become a general one.
