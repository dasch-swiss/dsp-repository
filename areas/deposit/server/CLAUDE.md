# AI Agent Guide for `deposit-server`

The Deposit Area's composition root (`areas/deposit/ADR-0004`), binary `deposit-server`; the editor's guidance is in `areas/deposit/editor/CLAUDE.md` and identity's in `areas/deposit/identity/CLAUDE.md`.

## What this crate is

`main`, CLI dispatch (`serve`, `healthcheck`), configuration loading and validation (`EditorConfig`, `Secret`), OTel and Pyroscope init, the CSRF and OTel layers, the untraced routes, the page-URL normalizer built from both web crates' route paths, and the fallback 404. `serve.rs` opens both stores, constructs the adapters (`LiveAuthenticator`, `LiveUserDirectory`, `LiveAuthoredWork`), starts `Identity` and then `Editor`, and merges their routers. It holds no business logic: each capability receives the part of the configuration it declares (`identity_settings`, `editor_settings`). The tests here are the ones that need both capabilities (`cross_capability`, `identity`, `removed_account`) or the assembled app (`router`).

## Router assembly: the traced/untraced order matters

`router.rs` is kept apart from `serve()` so the assembly is unit-testable. `build_router` merges the editor's and identity's routers, puts `ServeDir` with the app's 404 as the fallback, then applies the OTel layers. `build_app` then adds `/healthz` and `POST /telemetry/collect` *after* those `.layer()` calls, so they stay untraced, and wraps everything in `csrf::require_same_origin` last, so the CSRF check is outermost and covers every route including the beacon. Axum wraps only routes declared before a `.layer()`: do not reorder without checking `router.rs`'s tests.

## Dependency boundary

`Cargo.toml` depends on the two web crates (`identity-web`, `editor-web`), the two store crates, `deposit-shell`, `shared-telemetry` and infra crates (`editor-ports` only as a dev-dependency, for the port types its tests hold), and never on a `core` crate, dev-dependencies included: what the root needs from identity's rules (`CODE_TTL`, `is_plausible_address`) it takes from `identity-web`, and its tests seed and read through the stores' `test-support` helpers and the web crates' test support. `.github/scripts/check-composition-root-deps.sh` enforces this in CI: a root never depends on a capability's `core` or `api-*` crate, and on a capability's `web` crate only where the capability has no `server` crate, as here.

## Telemetry identity

The OTel tracer name and the Pyroscope application name are pinned to the literal `"editor-server"` (`observability.rs::TELEMETRY_NAME`), the Pyroscope `service.namespace` tag is `"editor"`, the telemetry collector's scope is `"editor"` (`collect_route` in `router.rs`) and the image is `daschswiss/metadata-editor`. These are dashboard and deployment identities, not derived from `CARGO_PKG_NAME`: renaming this crate must never change what a dashboard groups traces and profiles under. `service.name` is not among them: with `OTEL_SERVICE_NAME` unset it is the SDK's `unknown_service`, named after neither the crate nor the binary.
