# AI Agent Guide for `deposit-server`

The Deposit Area's composition root (`areas/deposit/ADR-0004`), binary `deposit-server`; the editor's guidance is in `areas/deposit/editor/CLAUDE.md` and identity's in `areas/deposit/identity/CLAUDE.md`.

## What this crate is

`main`, CLI dispatch (`serve`, `healthcheck`, `vre-bridge-refresh`), configuration loading and validation (`EditorConfig`, `Secret`, the VRE bridge's `EDITOR_VRE_BRIDGE_*` settings, read as text and checked by the bridge), OTel and Pyroscope init, the security-header, CSRF and OTel layers, the untraced routes, the page-URL normalizer built from both web crates' route paths, and the fallback 404. `serve.rs` opens both stores, constructs the adapters (`LiveAuthenticator`, `LiveUserDirectory`, `LiveOpenWork`), starts `Identity` and then `Editor`, merges their routers, and builds and starts the VRE bridge (`VreBridge`, `areas/deposit/vre-bridge/CLAUDE.md`), which never stops the process. It holds no business logic: each capability receives the part of the configuration it declares (`identity_settings`, `editor_settings`). The tests here are the ones that need both capabilities (`cross_capability`, `identity`, `removed_account`) or the assembled app (`router`).

## Router assembly: the traced/untraced order matters

`router.rs` is kept apart from `serve()` so the assembly is unit-testable. `build_router` merges the editor's and identity's routers, puts `ServeDir` with the app's 404 as the fallback, then applies the OTel layers. `build_app` then adds `/healthz` and `POST /telemetry/collect` *after* those `.layer()` calls, so they stay untraced, and wraps everything in `csrf::require_same_origin`, so the CSRF check covers every route including the beacon. Only the security-header layers (anti-framing, `nosniff`, and `Cache-Control: private, no-store` on everything but a static file) sit outside it, so CSRF refusals carry those headers too. Axum wraps only routes declared before a `.layer()`: do not reorder without checking `router.rs`'s tests.

## Dependency boundary

`BUILD.bazel` depends on the two web crates (`identity-web`, `editor-web`), the two store crates, `vre-bridge-server` (granted to this package alone), `deposit-shell`, `shared-rate-limit`, `shared-telemetry` and infra crates (`editor-ports` only in the test target, for the port types its tests hold), and never on a `core` crate, its test target included: what the root needs from identity's rules (`CODE_TTL`, `is_plausible_address`) it takes from `identity-web`, and its tests seed and read through the stores' `test-support` helpers and the web crates' test support. Bazel `visibility` enforces this: no `core` or `api-*` target grants a composition root, and a capability's `web` crate grants it only where the capability has no `server` crate, as here, so such an edge fails analysis (ADR-0003).

## The VRE bridge

`serve()` installs rustls' ring provider first, before any client is built, then builds the bridge after `init_otel`, whose meter provider its instruments need. `vre-bridge-refresh <SHORTCODE>` is the operator's refresh, run with `docker exec` in the deployment's container: the same configuration, its own tokio runtime, a plain JSON `tracing_subscriber` (`init_plain_logging`), no `init_otel`, and the bridge built on a meter provider with no reader, so it exports no metrics; its outcome line is the record. The panic hook asks `vre_bridge_server::panic_payload_withheld` and logs no payload of the bridge's panics. The image sets `EDITOR_VRE_BRIDGE_INPUTS_DIR` and never an output location: the Cloud Run previews run the image publicly, and without one the bridge does nothing. `tests/vre_bridge.rs` runs the binary itself. Operations: `docs/src/vre-bridge/operations.md`.

## Telemetry identity

The OTel tracer name and the Pyroscope application name are pinned to the literal `"editor-server"` (`observability.rs::TELEMETRY_NAME`), the Pyroscope `service.namespace` tag is `"editor"`, the telemetry collector's scope is `"editor"` (`collect_route` in `router.rs`) and the image is `daschswiss/metadata-editor`. These are dashboard and deployment identities, not derived from `CARGO_PKG_NAME`: renaming this crate must never change what a dashboard groups traces and profiles under. `service.name` is not among them: with `OTEL_SERVICE_NAME` unset it is the SDK's `unknown_service`, named after neither the crate nor the binary.
