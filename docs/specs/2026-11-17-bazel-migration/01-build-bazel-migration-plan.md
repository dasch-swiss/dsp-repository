---
title: "build: build the monorepo with Bazel"
type: build
date: 2026-11-17
author: "Marcin Procyk"
status: reviewed
repository: dasch-swiss/dsp-repository
linear: DEV-7699
---

# build: build the monorepo with Bazel

## Overview

Put ADR-0001 into code: Bazel builds, tests and packages every crate, the Tailwind CSS and the three container images, and `visibility` holds the boundaries of ADR-0002 and ADR-0003. Cargo and Bazel coexist on `main` from Phase 2 until Phase 10, with both builds gated on every pull request until Phase 9. The work ships as one stack of ten single-commit pull requests (`docs/src/git-conventions.md`, *One PR, several commits, or a stack*), one phase per pull request, each stacked on the one before.

| Phase | Pull request | Cargo on `main` afterwards |
|---|---|---|
| 1 | One TLS provider, `aws-lc-sys` out of the graph | authoritative |
| 2 | Bazel foundation: toolchains, crate universe, the shared crates and `mosaic-tiles`, Bazel steps in the two required jobs | authoritative, Bazel gated beside it |
| 3 | Access Area crates and `access-server` | same |
| 4 | Deposit Area crates, `deposit-server`, `editor-pull-requests` | same |
| 5 | `dsp-cli` and `mosaic-playground` | same |
| 6 | Tailwind CSS and the three images, built and verified, not yet published | same |
| 7 | Fuzz targets | same |
| 8 | Deploy path builds with Bazel (images, previews, Scout) | gated, but builds nothing that ships |
| 9 | Pull-request gates and developer tooling on Bazel, boundaries held by structure, Dockerfiles gone | present, not built by CI |
| 10 | Remove Cargo, documentation | gone, except `dsp-cli`'s publishing manifest |

The model is the Sipi migration (DEV-6341), with the same pins as Sipi runs today (DEV-7575). Where Sipi's code differs from what DEV-7699 and ADR-0001 assumed, this plan follows the code and says so under *Corrected premises*.

## Problem Statement / Motivation

The boundaries between areas and capabilities are held by five scripts over `cargo metadata`, manifest text and paths, each written and maintained separately (`check-capability-deps.sh`, `check-composition-root-deps.sh`, `check-cpe-seam.sh`, `check-shared-paths.sh`, `check-views-no-axum.sh`). Two of them say in their header that they are "a grep until Bazel `visibility` lands". Nothing covers the steps that are not Rust: the Tailwind CLI is pinned and downloaded in three places (`justfile`, the Mosaic Dockerfile, CI through `just`), and each image is described in four places that nothing keeps in step (`areas/access/server/CLAUDE.md`). Vitrinli is decided to move into this repository (ADR-0002) and already builds with Bazel, so without this work the monorepo would carry two build systems.

## Starting point, measured

Measured on `origin/main` at `bc6d73b9`, 2026-11-17. Each figure names the command that reproduces it. Re-run them before Phase 1.

| Fact | Value | Command |
|---|---|---|
| Workspace members | 34, plus the out-of-workspace fuzz crate | `cargo metadata --no-deps --format-version=1 \| jq '.workspace_members \| length'` |
| Locked packages | 595 | `grep -c '^\[\[package\]\]' Cargo.lock` |
| Direct third-party crates | 64 (9 of them dev-only) | `cargo metadata` filtered to members' registry dependencies |
| Third-party build scripts | 90; `aws-lc-sys`, `ring` and `libsqlite3-sys` are the ones that compile C | `cargo metadata`, targets of kind `custom-build` and the `links` key |
| `aws-lc-sys` in the Linux build | yes, in `access-server`, `deposit-server`, `dsp-cli`, `editor-pull-requests`, only through `reqwest` 0.13's `rustls` feature (directly, and through `pyroscope`'s default `rustls-tls`) | `cargo tree --locked -i aws-lc-rs --target x86_64-unknown-linux-musl -e features,normal` |
| Lines using `env!("CARGO_MANIFEST_DIR")` | 66, of which 10 feed `include_str!`/`include_bytes!` | `git grep -n 'env!("CARGO_MANIFEST_DIR")' -- '*.rs'` |
| First-party `build.rs`, proc-macro crates | none | `git ls-files '**/build.rs'`, `git grep 'proc-macro = true'` |
| Required checks on `main` | `check`, `check / docs / stable`, `check / hack / ubuntu / stable / features`, `test / ubuntu / stable`, `test / ubuntu / beta`, `gate`, strict | `gh api repos/dasch-swiss/dsp-repository/rulesets/5843207` |
| Remote-build credentials | `REMOTEBUILD_CA_CERT`, `REMOTEBUILD_CLIENT_CERT`, `REMOTEBUILD_CLIENT_KEY`, variable `REMOTEBUILD_RUNNER_ENDPOINT`, all visible to this repository | `gh api repos/dasch-swiss/dsp-repository/actions/organization-secrets` and `…/organization-variables` |
| Image `HEALTHCHECK` consumers | none: ops-deploy defines its own for both stacks, and the Mosaic image has none | `git -C ops-deploy grep -n deploy_healthcheck origin/main` |
| `cargo deny` in CI or `just` | not run anywhere | `git grep -n 'cargo deny' -- .github justfile` |

### Corrected premises

| DEV-7699 or ADR-0001 says | What holds |
|---|---|
| CI runs `--lockfile_mode=error`, as in Sipi | Sipi sets no lockfile mode. This plan introduces it. |
| The hermetic toolchain ships glibc and musl sysroots | The `llvm` module is zero-sysroot and builds libc runtimes from source. It supports musl (static only), but Sipi builds glibc only, and `rules_rust` has no libc constraint, so a musl Rust target needs its own `rust.repository_set`. One set is not enough: `rules_rust`'s default toolchains come first in the hub and match the musl platform too (`rust/platform/triple_mappings.bzl:374-377`, the musl constraint is commented out; issues #2544, #3831). `rules_rust`'s `examples/cross_compile_musl` shows the shape: the default `rust_linux_x86_64` set is redeclared with the libc constraint's non-musl value and the musl set with the musl value, once per exec triple, and `target_compatible_with` replaces the triple's whole constraint list. With this toolchain the two values are `@llvm//constraints/libc:unconstrained` and `:musl` (inferred from `rules_rs`, unverified: confirm during implementation). The musl platform also carries `@llvm//constraints/pie:off` (hermetic-llvm README: rustc and the toolchain disagree on `-static-pie`). `rules_rust` 0.74.0 has no `@rules_rust//rust/platform:x86_64-unknown-linux-musl` target for `crate_universe`'s selects to name (whether listing the triple in `supported_platform_triples` fails is unverified: confirm during implementation). Neither project tests `rules_rust` with this toolchain on musl, and hermetic-llvm's own `rules_rust` test does not run on macOS. Settled in Phase 2. |
| Boundaries by `visibility` / `package_group` | Sipi rejected `package_group`. Plain `__pkg__` label lists are shorter and are used here. |
| Reuse Rust 1.98 from Sipi | This repository is on 1.93.0. Bazel pins the version `rust-toolchain.toml` holds when Phase 2 starts, so the compiler does not change in the same window as the build system. |
| `rusqlite` and `aws-lc-sys` are compiled by the hermetic toolchain | `libsqlite3-sys` and `ring` go through `cc-rs`, which works in Sipi (`ring`). `aws-lc-sys` panics in a Bazel action in Sipi, which removed it instead of fixing it. Both causes are reported fixed since (`rules_rust` #3260 and #3370, `aws-lc-sys` 0.39.1), but no build of it with `rules_rust` and this toolchain is known. Phase 1 removes it here. |
| ADR-0012's data directories are build-time inputs | ADR-0012 is about the two data channels of a reading capability. Its stand-in directories still need `data` declarations, which is what the issue meant. |
| Dependabot keeps crates current | Dependabot's `bazel` ecosystem covers `bazel_dep` only. Sipi runs with Dependabot off. |

## Proposed Solution

**Ruleset and pins, the same as Sipi's `MODULE.bazel` today:** Bazel 9.3.0, `rules_rust` 0.74.0, `llvm` 0.8.25 with LLVM 23.1.2, `rules_oci` 2.3.0, `rules_distroless` 0.8.0, `tar.bzl` 0.10.4, `aspect_bazel_lib` 2.22.5, `platforms` 1.1.0, `rules_fuzzing` 0.8.0, plus Sipi's two `llvm` patches (`hermetic_llvm_headers_glob.patch` for remote execution, `hermetic_llvm_darwin_rtlib.patch`). Sipi's `MODULE.bazel` also carries `rules_cc` 0.2.25 (any BUILD file that loads a `cc_*` rule needs it under Bazel 9) and a `single_version_override` of `rules_android` to 0.7.2 with `common --repo_env=ANDROID_HOME=` in `.bazelrc` (protobuf in the module graph pulls a `rules_android` whose `CcInfo` load breaks every build). Each is copied if `bazel mod graph` shows the same module here. Newer releases exist. Taking Sipi's pins means every problem its journals record is already solved for this combination, and both repositories upgrade together.

**Decisions this plan takes.** Each is confirmed or changed in H1 before execution starts.

- **D1. `crate.from_specs` from the first Bazel commit, not `from_cargo` first.** The lock Bazel manages (`Cargo.Bazel.lock`) is seeded with a copy of `Cargo.lock`, so both builds resolve the same third-party versions on day one. Dependabot's `cargo` version updates are paused for the dual period, because under either mode a `Cargo.lock` bump leaves the Bazel side stale until someone repins. Security updates cannot be paused that way and are enabled on this repository: such a pull request bumps `Cargo.lock` alone, so it gets the matching `just repin` added, or is closed and redone by hand.
- **D2. Linux target: static musl, with glibc as the stated fallback.** Musl keeps what ships today unchanged (a static binary in `distroless/static`). If `@crates//...` cannot be built for the musl platform without patching `rules_rust` or a crate, the target becomes glibc 2.28 and the image base `distroless/cc-debian12`, which the Mosaic image already runs on.
- **D3. One TLS provider, `ring`.** Lands under Cargo first, in Phase 1. Both servers already link `ring` for their other TLS clients (OTLP export, mail, `ureq`); `dsp-cli` and `editor-pull-requests` gain it.
- **D4. Both builds are required from Phase 2 on.** The Bazel steps run inside the existing required jobs `check` and `test / ubuntu / stable`, so no ruleset edit is needed until Phase 9.
- **D5. What is retired.** Without replacement: the beta leg, `cargo hack`, the nightly rustdoc job, `cargo-watch`, `just test-changed` (Bazel's cache does it), and `cargo deny` together with `deny.toml`'s licence allow-list and source policy, none of which ever ran in CI or `just`, and native `arm64` images on a developer's machine (`just build-docker-*` builds `linux/amd64` on every host, so Apple silicon runs it emulated). With a follow-up issue (H10): the coverage upload, `cargo machete`'s unused-dependency check, and automated updates of crates, `bazel_dep`s and the base-image digest.
- **D6. New: an advisory check outside the required jobs.** `just audit` runs `cargo-audit` and `osv-scanner` over `Cargo.Bazel.lock`, as in Sipi, from a scheduled workflow and on pull requests that change the lock. It replaces the Dependabot security alerts that stop when the root `Cargo.lock` goes. It is not in `check`: an advisory published overnight must not fail every unrelated pull request.
- **D7. Formatting stays file-based.** Nightly `rustfmt` and `maudfmt` run on files, outside the Bazel graph, from the one pin in `.rustfmt-nightly`.
- **D8. Public `just` recipe names survive.** `bazel-*` twins exist during the dual period. In Phase 9 the canonical names (`just check`, `just test`, `just css`, `just dev`, …) are re-keyed on Bazel and the twins are deleted, so `eng.yaml`, the `CLAUDE.md` files and most documentation keep naming the right command.
- **D9. Servers run from the checkout, not from runfiles.** The dev loop, `just run`, `just validate-data` and the Playwright suites start the Bazel-built binary with the repository root as working directory, and the CSS recipes copy their Bazel output into `public/assets/` (gitignored, as today). The configuration defaults (`areas/access/dpe/server/data`, …) are relative to the repository root and stay valid, and edits to data and static files are seen without a rebuild.

## Alternative Approaches Considered

| Alternative | Why not |
|---|---|
| `from_cargo` during the dual period, `from_specs` at cutover | The cutover commit would rewrite every `deps` list and change dependency resolution in the same commit that deletes Cargo, the one moment with no reference build left. It also does not remove the repin step after a `Cargo.lock` bump. |
| Make `aws-lc-sys` build under Bazel with `crate.annotation` | Sipi planned exactly this and never got it working. The two failures it hit are reported fixed since: build scripts seeing `CARGO_PKG_VERSION` 0.0.0, which broke the symbol prefixes (`rules_rust` #3250, fixed by #3260 and #3370), and the sandbox symlink panic (`aws-lc-sys` 0.39.1). No `rules_rust` build of it with this toolchain is known, though, and a musl link failure is reported (nativelink #2734). D3 needs no annotation, removes the cmake prerequisite, and is checkable under Cargo. |
| Bump Rust to 1.98 as part of this stack | Changes the compiler and the build system in one window. A clippy or codegen difference would have two possible causes. The bump is an ordinary change afterwards (H10). |
| glibc from the start, as Sipi | Cheapest to wire, but changes the production base image and links against libc dynamically for no need of the migration itself. Kept as the D2 fallback. |
| A separate, non-required Bazel workflow during the dual period | A red optional check is ignored, and the stack would be rebased over breakage introduced by unrelated pull requests. |
| One cutover pull request for deploy path and gates | A failed deploy would have two candidate causes, which is the concern DEV-7699 raises about windows. Phases 8 and 9 separate "what ships" from "what gates". |
| Run `rustfmt` through `rustfmt_aspect` with a nightly toolchain registered in `MODULE.bazel` | A second copy of the nightly pin that `MODULE.bazel` cannot read from `.rustfmt-nightly`, and `maudfmt` has no rule anyway, so the format gate would be half in Bazel. |
| Keep `cargo-fuzz` by keeping three manifests | Violates ADR-0001 for `dpe-core`, `shared-metadata` and `shared-telemetry`. Sipi's `rules_fuzzing` setup runs on stable Rust. |
| A committed gate comparing `Cargo.lock` with `Cargo.Bazel.lock` | A recipe, a CI step and a `.test.sh` that live for the dual period only. Two runs of the script under *Verification commands* (Phase 2 and Phase 10) catch the same drift. |
| Dependabot's `bazel` ecosystem for `bazel_dep`s | A bump of `rules_rust` or `llvm` is a toolchain upgrade that Sipi plans and journals, not a weekly pull request. Left to H10 together with crate and base-image updates. |
| A check that production targets name no dev-only crate (`insta`, `wiremock`, …) | Cargo held this through `[dev-dependencies]`; `@crates` targets are public. Enforcing it needs a script that parses which target a `deps` list belongs to. The cost of a miss is binary size, and the line is visible in the BUILD diff. Given up. |
| Run servers with plain `bazel run` | The working directory is the runfiles tree: the Tailwind watch writes `app.css` into the checkout where the server does not look, and data edits need a rebuild. D9. |

## Technical Considerations

### Toolchains and platforms

- Registration order is load-bearing: the `llvm` toolchain blocks come before the `rust` extension in `MODULE.bazel`, or build scripts silently use the host `cc` (Sipi, `MODULE.bazel:487-492`).
- Exec platforms declared: `aarch64` macOS (developers) and `x86_64` Linux (CI runner and the remote worker). Sipi's `linux_aarch64` platform exists for its arm64 CI leg, which this repository does not have.
- `platforms/BUILD.bazel` uses Sipi's names so that the copied `bazel-rbe` action works unchanged: `linux_x86_64` carries `exec_properties = {"cpu_arch": "x86_64"}`. The image target platform adds the libc constraint (`@llvm//constraints/libc:musl`) and is the one new piece of wiring. The image recipes select it with their own `--platforms` flag after the action's flags, not through the action's `target-platform` input: for any value other than `//platforms:linux_x86_64` the action adds that platform as an execution platform and emits `--config=rbe-cross` (Sipi, `.github/actions/bazel-rbe/action.yml:91-101`).
- Cross-compiling from macOS needs the Linux target triples on the Rust toolchain (`extra_target_triples`, or the `repository_set`s of the musl wiring). Without them, Sipi's toolchain-upgrade journal records, `--platforms=//platforms:linux_x86_64` "finds no Rust toolchain for a darwin exec platform".
- macOS needs no Xcode for compile actions: the toolchain fetches the SDK. Sipi's `build:macos` block (deployment target 14.0, because `cc-rs` objects of `ring` are built for 14.0) is copied without its `--repo_env=PATH` line, which serves Sipi's Kakadu fetch. One repository rule does need the Command Line Tools inside the Nix dev shell: `rules_fuzzing` pulls a `rules_python` sdist build that calls `xcrun`, which the shell's `apple-sdk` stub cannot answer, so Sipi's `build --repo_env=DEVELOPER_DIR=/Library/Developer/CommandLineTools` is copied too (Sipi, `.bazelrc:20-31`). `.cargo/config.toml`'s macOS `split-debuginfo=off` needs no equivalent: `rules_rust` emits no debug info in the default `fastbuild` mode.
- `.bazelrc` takes from Sipi: `--incompatible_strict_action_env`, `BAZEL_DO_NOT_DETECT_CPP_TOOLCHAIN=1`, `--enable_platform_specific_config`, the remote tuning flags, `try-import %workspace%/user.bazelrc`. Two differences from Sipi's file: the detection switch is set with `common --repo_env`, because Bazel 9 no longer passes `--action_env` to repository rules, which is where the detection runs; and `build --action_env=PATH` is not copied, because it puts `/nix/store` paths into every action key and nothing here needs it (`xmllint` is in `/usr/bin`). New here: `common:ci --lockfile_mode=error`. Use `extra_rustc_flag` (singular, accumulates), never `extra_rustc_flags` (plural, replaces the list).
- Never run `bazel mod tidy`, also when Bazel's output suggests it: it strips `use_repo` lines the `llvm` extension needs (Sipi, toolchain-upgrade plan). The lock is refreshed with `bazel mod deps --lockfile_mode=update`.
- `.bazelignore` lists what `//...` must not walk: `target`, the three `node_modules` directories, `.claude` (worktrees are whole checkouts with their own `MODULE.bazel`), `docs/book`, `.direnv`.
- `[profile.release]` maps to a `build:release` config: `-c opt`, `--@rules_rust//rust/settings:lto=fat`, `--@rules_rust//rust/settings:codegen_units=1`, `-Copt-level=z`, and `-Cdebuginfo=line-tables-only` with `-Cstrip=none`, because `rules_rust` strips debug info in `opt` and both servers' panic hooks need line tables. `dsp-cli`'s own `[profile.release]` only ever applied to `cargo install` and stays in its manifest.
- Every `just` recipe needs `cargo` on the path until Phase 9, because the `justfile` evaluates `CARGO_VERSION` with `cargo metadata` when it loads.

### Crate universe

```starlark
# MODULE.bazel (sketch; the llvm blocks above this are Sipi's, verbatim)
bazel_dep(name = "rules_rust", version = "0.74.0")

rust = use_extension("@rules_rust//rust:extensions.bzl", "rust")
rust.toolchain(edition = "2021", versions = ["1.93.0"])
use_repo(rust, "rust_toolchains")
register_toolchains("@rust_toolchains//:all")

crate = use_extension("@rules_rust//crate_universe:extensions.bzl", "crate")
# `preserve_order`: the canonical project writer emits keys in declaration order.
# `float_roundtrip`: cpe-store-build writes annotation geometry as parsed.
crate.spec(package = "serde_json", version = "1", features = ["preserve_order", "float_roundtrip"])
# Held at 0.38: deadpool-sqlite 0.13 requires rusqlite ^0.38 (`links = "sqlite3"`).
crate.spec(package = "rusqlite", version = "0.38", features = ["bundled", "chrono"])
crate.spec(package = "matchit", version = "=0.8.4")
# … one spec per direct third-party crate …
crate.from_specs(
    cargo_lockfile = "//:Cargo.Bazel.lock",
    # The musl triple is listed only if Phase 2 finds a way: see Corrected premises.
    supported_platform_triples = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-unknown-linux-musl"],
)
use_repo(crate, "crates")
```

- One universe means one feature set per crate. A feature that Cargo enables only for tests (figment's `test`) is compiled into production builds of that crate. Accepted: it adds code, not behaviour. A missing feature fails to compile or is caught by the test that depends on it (`canonical_round_trip` for `preserve_order`).
- Crates are referenced as `@crates//:<name>`, hand-written in each `deps`, as in Sipi. `rules_rust` 0.71 added per-crate subpackages (`@crates//<name>`) and plans to stop generating the root aliases (`incompatible_no_root_alias_targets`, off by default in 0.74). The rewrite is mechanical and happens with the `rules_rust` bump that removes them.
- Repin is `just repin`: `CARGO_BAZEL_REPIN=1 bazel fetch @crates//:all`, then a plain `bazel mod deps --lockfile_mode=update` without the variable, because the repin variable is recorded in `MODULE.bazel.lock` and `--lockfile_mode=error` rejects that state. `bazel sync`, which the `rules_rust` documentation still prints, was removed in Bazel 9. No upstream document describes this sequence for Bazel 9, so Phase 2 proves it.
- `CARGO_BAZEL_REPIN=1` resolves new and changed specs and leaves every other locked version where it is. `just repin <crate>` passes `CARGO_BAZEL_REPIN=<crate>` and moves that one crate, which is how a transitive crate gets a security bump. A floor on a transitive crate is a `crate.spec` of its own with a comment saying so, never pruned as unreferenced (Sipi, `MODULE.bazel:601-605`).
- A `rustls` spec sets `default_features = False`: its default features bring `aws-lc-sys` back (Sipi, `MODULE.bazel:615-618`).
- On its first run `cargo-bazel` rewrites the seeded lock: the workspace's own packages go and a synthetic root appears. Parity with `Cargo.lock` is therefore by package and version, which is what the script under *Verification commands* compares.
- The workspace uses Cargo's MSRV-aware resolver (`resolver = "3"`); `crate_universe` splices with resolver 2 by default (`crate.splicing_config(resolver_version = …)` exists, untested here). A repin can select a crate version that needs a newer compiler than the pinned one. The seeded lock prevents that on day one, and a repin is followed by the full test run, where it fails at compile and is fixed with an upper bound on that crate's spec, as `kdl` already has.
- The comments that carry dependency decisions in the root `Cargo.toml` move beside their `crate.spec`.

### Shape of a crate's BUILD file

Hand-written `rust_library`, `rust_binary`, `rust_test` and `rust_test_suite`, no wrapper macro: the dependency list of every crate stays readable in its own directory.

```starlark
# areas/deposit/identity/store/BUILD.bazel (sketch)
load("@rules_rust//rust:defs.bzl", "rust_library", "rust_test")

_SRCS = glob(["src/**/*.rs"])
_COMPILE_DATA = ["src/migrations/0001_initial.sql"]

rust_library(
    name = "identity_store",
    srcs = _SRCS,
    compile_data = _COMPILE_DATA,
    crate_name = "identity_store",
    visibility = [
        "//areas/deposit/identity/web:__pkg__",
        "//areas/deposit/server:__pkg__",
    ],
    deps = [
        "//areas/deposit/editor/ports:editor_ports",
        "//areas/deposit/identity/core:identity_core",
        "@crates//:rusqlite",
    ],
)

# The `test-support` feature, for other crates' tests only.
rust_library(
    name = "identity_store_test_support",
    testonly = True,
    srcs = _SRCS,
    compile_data = _COMPILE_DATA,
    crate_features = ["test-support"],
    crate_name = "identity_store",
    visibility = [
        "//areas/deposit/identity/web:__pkg__",
        "//areas/deposit/server:__pkg__",
    ],
    deps = [
        "//areas/deposit/editor/ports:editor_ports_test_support",
        "//areas/deposit/identity/core:identity_core",
        "@crates//:rusqlite",
    ],
)

rust_test(
    name = "identity_store_test",
    crate = ":identity_store",
)
```

- `crate_name` is always Cargo's, because `insta` snapshot file names start with it.
- `srcs` is a glob, so adding a source file needs no BUILD edit during the dual period. Adding a dependency needs the manifest and the BUILD file.
- Integration tests under `tests/` are a `rust_test_suite`, one test per file, except in `dsp-cli`. The macro names each test's crate `<suite>_<file>_test` and accepts no `crate_name` (`rust/private/rust.bzl:1794-1805`), and `insta` prefixes a snapshot file with the crate name, which under Cargo is the test file's stem (`cli__…`). `dsp-cli`'s integration tests are therefore one `rust_test` per file, written as a list comprehension over the file stems with `crate_name = <stem>` and the helper modules (`tests/common`, `tests/support`) in `srcs`, so its 333 snapshots keep their names. `cpe-store-build/tests/it/` is a single multi-file test crate and is one `rust_test` with `crate_root = "tests/it/main.rs"`.
- `rust_test(crate = …)` inherits the library's `srcs`, `deps`, `compile_data`, `rustc_env` and `rustc_env_files`, but not `crate_features` or `rustc_flags` (`rust/private/rust.bzl:474-518`, `rust/private/rustc.bzl:1296-1297`): a test of a variant target repeats them.
- `cpe-image-viewer`'s manifest forbids unsafe code for every target (`[lints.rust] unsafe_code = "forbid"`). It is the only lint a manifest sets, and it moves to `rustc_flags` on that crate's targets.

### Cargo features become targets

| Feature | Crates | Bazel shape |
|---|---|---|
| `test-support` | `identity-{ports,store,web}`, `editor-{ports,store,web}` | A second `testonly` library `<name>_test_support` per crate. Inside that graph every dependency that has a variant uses the variant, or two builds of one crate meet in a single compile and their types do not unify. A crate whose Cargo dev-dependencies enable the feature on a sibling tests through its own variant (`rust_test(crate = ":identity_web_test_support")`). `testonly` makes "fixtures never reach a production binary" a build error, which Cargo did not check. |
| `dev` | `dpe-server`, `mosaic-playground`, `access-server` | Variant targets: the library `dpe_server_dev` and the binary `mosaic_playground_dev` add `tower-livereload` and `notify`; the binary `access_server_dev` adds nothing of its own and depends on `dpe_server_dev`. The dev loop runs the `_dev` binary. Building them in `//...` replaces the feature powerset check, and `dpe_server_dev` and `mosaic_playground_dev` each get a test target with `crate_features = ["dev"]`, because `just test` runs the dev-only tests today (`justfile:197`). |
| `live` | `dsp-cli` | The `rust_test` targets of `tests/live_*.rs` except `live_update_check.rs`, with `crate_features = ["live"]`, tagged `manual`, `no-remote-exec`, `requires-network`, run with `--ignored`, `DSP_LIVE_STRICT=1`, and every `DSP_TEST_*` variable and `DSP_TOKEN` passed by name with `--test_env`, since the strict action environment drops what is not named. `live_update_check.rs`, which calls crates.io, is its own `manual` target, as the nextest filter excludes it today. The tests keep `#[ignore]`, so `check-live-tests-ignored.sh` stays valid. |

### Files a crate reads

Verified in `rules_rust` 0.74.0 (`rust/private/rustc.bzl:1115-1119`, `:1443-1450`, `:1455-1464`; `rust/private/rust.bzl:620-625`): the compile-time `CARGO_MANIFEST_DIR` is `${pwd}/<package>`, an absolute sandbox path that is invalid when the test runs; a target's own `rustc_env` is applied after it; `CARGO_BIN_EXE_<name>` is set at compile time for a binary listed in `data`, to a path relative to the runfiles root, not an absolute one as under Cargo; a `rust_test` runs from the runfiles root with the run-time `CARGO_MANIFEST_DIR` set to the workspace-relative package path.

- **Rule:** every target whose sources use `env!("CARGO_MANIFEST_DIR")` sets `rustc_env = {"CARGO_MANIFEST_DIR": "<package path>"}`. The path is then relative, resolves from the runfiles root or the repository root, and needs no source change for the 56 lines that join or `concat!` it at run time. It also keeps an absolute path out of the binary.
- **The exception:** `include_str!`/`include_bytes!` resolve a relative path against the including file, so the 10 lines that build an include path from `CARGO_MANIFEST_DIR` (all in `areas/access/cpe/store-build`) are rewritten to file-relative literals. That form works under Cargo too.
- `include_str!`/`include_bytes!` inputs go in `compile_data` (`data` does not reach `rustc`: `rust/private/rustc.bzl:776-777`). That includes files inside the crate's own directory: `dsp-cli/docs/topics/*.md`, `cpe-image-viewer/assets/**`, `editor-core/data/*.json`, the `schema.sql` and migration files, `cpe-components/testdata`. Files read at run time go in `data`.
- A file in another package is exported by its owner as a `filegroup` with an explicit `visibility`, which turns every cross-crate file read into a reviewed grant. The table below lists the reads that leave the reading crate's directory. Each phase lists a crate's reads with `git grep -nE 'include_(str|bytes)!|CARGO_MANIFEST_DIR' -- <crate path>` before writing `compile_data` and `data`.
- A `filegroup` over a `public/` directory excludes `assets/app.css` and `assets/app.*.css`. Those are gitignored outputs of the CSS recipes; on a developer's machine a glob would otherwise carry a stale stylesheet into tests and into the image, where the server picks the stylesheet by scanning the directory.

| Reader | Reads, outside its own directory | When |
|---|---|---|
| `dpe-core`, `dpe-web`, `dpe-api-oai` | `areas/access/dpe/server/data/**` | test run |
| `dpe-api-oai` | `shared/metadata/testdata/0803-records.json`, `shared/fair/testdata/schemas/datacite-4.3-schema.json` | compile |
| `dpe-server` | `areas/access/dpe/public/**` | test run |
| `access-server` | the corpus, `areas/access/dpe/public`, `areas/access/cpe/projects/**` | test run |
| `cpe-server` | `cpe/components/style/*.css`, `areas/access/cpe/public/**` | compile, in production code; its tests also list those directories at run time |
| `cpe-server` | `areas/access/cpe/projects`, `areas/access/sync/data`, `areas/access/media/data` | test run |
| `cpe-web` | `cpe/components/style/*.css`, `areas/access/cpe/public/vendor/datastar.js` | compile, in tests |
| `cpe-store-build` | `cpe/store/schema.sql` | compile, in production code |
| `cpe-store-build` | `cpe/core/src/validate.rs`, `areas/access/media/data/0803/**`, `areas/access/sync/data`, `areas/access/cpe/projects`, `docs/src/cpe/config-language.md` | compile and test run, in tests |
| `cpe-core` | `cpe/server/tests/fixtures/activation/things/config.kdl` | compile, in tests |
| `sync-store` | `areas/access/sync/data` | test run |
| `editor-core`, `editor-store`, `editor-web`, `editor-pull-requests` | the corpus, through `editor-core`'s `checkout_dpe_data_dir()` | test run; `editor-pull-requests` also at run time from the repository root |
| `editor-web` | `areas/deposit/editor/style/main.css` | compile, in tests |
| `deposit-server` | `areas/deposit/editor/public` | test run |

- The published corpus (`areas/access/dpe/server/data`) is one `filegroup` in `//areas/access/dpe/server`. Its grants to the Deposit Area are the one cross-area path ADR-0002 names: the tests of the four crates in the table above, and the editor image, which carries the corpus as its data directory.
- Tests that read their own crate's sources at run time (`dpe-server`, `access-server`, `cpe-store-build`) list `glob(["src/**/*.rs"])` as `data`. `sync-store`'s integration test includes `src/vocab.rs` with `include_str!`, so there it is `compile_data` on the test. `cpe-server` includes files that are already in its `srcs`.
- Runfiles are symlinks into the source tree. A test that exercises a path-traversal guard built on `canonicalize` sees the real path and rejects it. Where that happens, the data dependency is a `copy_to_directory` with `root_paths = ["."]`, which keeps the repository-relative layout the `..` joins rely on (Sipi, root `BUILD.bazel:25-53`).
- `dsp-cli`'s tests find the binary through `assert_cmd::Command::cargo_bin("dsp")`, which looks in `target/`. They move to `env!("CARGO_BIN_EXE_dsp")`, which Cargo sets natively and `tests/stdout_broken_pipe.rs` already uses. Because Bazel's value is relative, the helper that builds the command makes it absolute against the test's working directory before any `current_dir` call; under Cargo that is a no-op.
- `option_env!("DEPOSIT_VERSION")` in `deposit-shell` is fed through `rustc_env_files` under `--stamp`. Two things `rules_rust` does not do by itself: a `rust_library` has `stamp = 0` and ignores `--stamp` unless it sets `stamp = -1` (`rust/private/rust.bzl:900-902`), and in an unstamped build the placeholder stays in the value as literal text, so the footer would show `{STABLE_DEPOSIT_VERSION}` instead of `dev`. The env file is therefore attached only when stamping (a `select` on a `config_setting` for `--stamp`), and the status script emits `dev` when the variable is unset. `tools/workspace_status.sh` emits `STABLE_DEPOSIT_VERSION` from the `DEPOSIT_VERSION` environment variable, which the three editor workflows already compute (`git describe` on `main`, the tag on a release, `pr-<N>-<sha7>` on a preview). Only the image builds pass `--stamp`; test builds do not, so a commit does not relink every test.

### Tests

- `.bazelrc` sets `test --test_env=INSTA_WORKSPACE_ROOT=.` and `test --test_env=INSTA_UPDATE=no`. Snapshot directories are `data` of their test target.
- One mechanism runs a test against the real checkout instead of the runfiles: `bazel run --run_under="cd $PWD && " <target>`. `just snap-accept <target>` uses it with `INSTA_UPDATE=always`, so `insta` itself writes the snapshot (the repository's learning: never accept by renaming `.snap.new`), and the review is `git diff`. `just canonicalize-projects` uses it with `CANONICALIZE_PROJECT_FILES=1`.
- OAI tests shell out to `xmllint`, which the remote worker does not have. Those targets are tagged `no-remote-exec` and run on the CI runner, where the workflow installs `libxml2-utils`.
- The three crates with compiling doctests (`mosaic-tiles`, `cpe-image-viewer`, `dsp-cli`) get a `rust_doc_test`. `rules_rust` has open defects there (`data` unsupported, `include_bytes!` in doctests), so a doctest that cannot run moves into a unit test with the same assertion.
- The clippy aspect also lints `rust_test` targets, which `just check` does not today (`cargo clippy --all-features`, without `--all-targets`). Findings in test code are fixed in the phase that adds the crate's BUILD file, as source edits that pass under Cargo too. `-Dwarnings` is passed explicitly: the aspect applies it by default only while no other clippy flag is set.

### Boundaries: what becomes structure

Every first-party target is private by default and lists its consumers one by one. A legal new edge is one line in the provider's BUILD file, reviewed in the provider's directory.

| Rule (source) | Today | After |
|---|---|---|
| No crate of one area depends on a crate of another (ADR-0002) | `check-capability-deps.sh` | **structure**: no grant crosses `areas/access` and `areas/deposit`, except the corpus `filegroup` |
| `shared-*` and `mosaic-*` depend on no area crate (ADR-0002) | `check-shared-paths.sh` for paths | **structure** for crate edges. The path grep **stays**: a path string in a source file is not a dependency edge. |
| Capability role edges: `ports` on `shared-*` only, `api-*` on `core` and `shared-*`, `web`/`server` may use the shell and `mosaic-tiles` (ADR-0003) | `check-capability-deps.sh` | **structure**, through each crate's consumer list |
| A provider's `store` on a consumer's `ports`, nothing else across capabilities (ADR-0003) | `check-capability-deps.sh` | **structure**: `ports` is visible area-wide, everything else to its own capability and the composition root |
| `sync-store` and `media-store` as dev-dependencies only (ADR-0003, ADR-0008, ADR-0012) | `check-capability-deps.sh`, `DEV_ONLY` | **structure**: other capabilities see only a `testonly` alias of the store |
| Composition root never depends on `core`, `api-*`, or a `web` beside a `server` (ADR-0003) | `check-composition-root-deps.sh` | **structure**. Script retired. |
| CPE seam: outside CPE only `cpe-ports` and `cpe-server` are reachable, no CPE crate uses Mosaic, `cpe-core` is a leaf, fixed edges per crate (ADR-0007) | `check-cpe-seam.sh` | **structure**. Script retired. |
| `dsp-cli` depends on no area and no `shared-*` crate (ADR-0002) | `cargo publish --dry-run` | **structure** (nothing grants it visibility), and the dry run stays for the manifest |
| Third-party bans per role: `ports` none, `core` no web framework, Maud or database crate, `shell` and view-only `web` no web framework (ADR-0003) | `check-capability-deps.sh` | **stays a script**, re-keyed from `cargo metadata` to the `deps` of each role's `BUILD.bazel`. `@crates` targets are public, so `visibility` cannot ban them per consumer. Direct edges only, as today. |
| No first-party target is public | none needed | **new in the same script**: no `//visibility:public` in a `BUILD.bazel` under `areas/`, `shared/`, `modules/` or `dsp-cli/`. Widening a target to public is the quickest way to silence a visibility error, and it would undo every row above without touching a rule. |
| View files of `editor-web` and `identity-web` use no `axum::` path | `check-views-no-axum.sh` | **stays**: a per-file rule inside one target |
| Datastar attribute grammar, ADR references, live tests ignored, commit count, vendored checksums | their scripts | **stay**, none reads a manifest. `verify-checksums.sh` loses its Tailwind half. |

Each boundary gets a one-off negative check when its grants are written: add the forbidden dependency, run `bazel build --nobuild` on the consumer, see the visibility error, remove it. No negative test is committed: Bazel's analysis is the enforcement, and the grant line is the review point.

### CSS

- The Tailwind standalone CLI is four `http_file`s with `sha256` in `MODULE.bazel`, selected by exec platform. `tailwind.pins`, `_tailwind-bin` and `tailwind-pins-refresh` go in Phase 9, and the pin then exists once. From Phase 6 until then a Tailwind bump edits both pins; `docs/src/bazel.md` says so.
- Each stylesheet (`dpe`, `editor`, `mosaic-playground`) is a build action. Its inputs are the entry file, the token and component CSS it imports, and every `.rs` file its `@source` globs name. The entries use `source(none)`, so an undeclared source file is not an error: its classes silently disappear from the output. The check is byte equality with the output of today's `just css*` recipe at the same commit, both produced on the same platform.
- An `@source` line names a whole crate directory, tests and nested packages included (`areas/access/dpe/server` reaches `fuzz/fuzz_targets/`). The scan inputs are therefore a `glob(["**/*.rs"])` exported by each scanned crate, not that crate's `srcs`, and a nested package exports its own files.
- The byte comparison has a reference only until Phase 9 re-keys the recipes. After that, an `@source` line added without its input is visible in the dev loop, whose watch scans the checkout, and missing from the image. Each entry file carries a comment at its `@source` lines naming the BUILD file that lists the inputs.
- Tailwind 4.2.4 has reports about `@source` globs and symlinked directories (#16765; a fix, #20203, was merged after 4.2.4), and a sandbox presents inputs as symlinks. If the comparison fails for that reason, the inputs are materialised with `copy_to_directory`. Tailwind is not bumped inside this stack: it would change the reference.
- The release name `app.<hash8>.css` depends on the content, so the `dpe` and `editor` rules declare a directory output. The servers already find the name by scanning the asset directory at startup. The playground's stylesheet is a plain `app.css`.
- If the CLI cannot run on the remote worker, the CSS actions are tagged `no-remote-exec`.

### Developer loop and E2E (D9)

- `just css*` builds the stylesheet with Bazel and copies it to the path it has today (`<app>/public/assets/app.css`, or `app.<hash8>.css` for the release recipes, replacing older hashed files). The Tailwind `--watch` of the `dev` recipes runs the Bazel-provided CLI and writes to the same path.
- `just run`, `just dev*`, `just validate-data` and the `bacon` jobs start the binary with `bazel run --run_under="cd $PWD && "`, so the working directory is the repository root. `bacon` restarts a server by signalling the process it started, which under `cargo run` is the server itself. Here it is a shell, so the prefix may need to be `cd $PWD && exec ` (unverified: confirm during implementation: the port is free after a restart).
- Both servers pick their stylesheet by compilation mode: with debug assertions they link `/assets/app.css`, without them they scan for `app.<hash8>.css`. The `_dev` binaries and the dev recipes are therefore `fastbuild` with `just css*`, and `just run`, the Playwright suites and the images are `--config=release` with the release stylesheet, as `cargo run --release` is today. Any other pairing links a file that is not there, without an error.
- The playground has no `bacon` job: `cargo watch` is its only restarter today. `bacon.toml` gains one, and `just watch-mosaic-playground` runs it.
- The Playwright configs take the server path from an environment variable that the recipes running them (`test-a11y-dpe`, `test-e2e-editor`, `test-a11y-editor`) set after building the server with `--config=release` for the host platform (`bazel cquery --output=files`), instead of `target/release`. `bazel-bin` is not a stable address: it points at the output tree of the last build's configuration, so a `fastbuild` run in between leaves it naming another binary. Their working directory is already the repository root.
- rust-analyzer reads `rust-project.json` from `just rust-project` (`bazel run @rules_rust//tools/rust_analyzer:gen_rust_project`), re-run after adding or moving a target.

### Images

- Three `oci_image` targets on a digest-pinned `distroless/static-debian12:nonroot` (D2 fallback: `cc-debian12`), layers through `tar.bzl` with fixed mtimes: binary, public directory with the hashed CSS, data directories. Environment, entrypoint, command, port and working directory copy the Dockerfiles. The user comes from the base.
- The base is the one the Dockerfiles use, pinned to the digest the tag resolves to when Phase 6 is written, so that Phase 6 compares like with like. distroless lists only `-debian13` images today and no longer updates `-debian12` tags, so the move to `-debian13` is a change of its own (H10). `oci.pull` needs `platforms` for a multi-architecture base.
- `tar` puts a binary's runfiles into the archive by default (`include_runfiles = True`) and `mutate()` cannot set a file mode: the binary layer turns runfiles off and sets its mode in an mtree line.
- The Mosaic image stops compiling inside a Dockerfile (cargo-chef, glibc, `distroless/cc`) and becomes the same shape as the other two.
- `HEALTHCHECK` has no `oci_image` attribute (the OCI image specification only reserves the field) and is dropped. ops-deploy does not depend on it (see the measured table), and both servers keep their `healthcheck` subcommand.
- One `oci_push` and one `oci_load` per image. `oci_load` tags the image with the reference the Scout workflows scan today (`local-scan/<name>:latest`). CI keeps computing publish tags as today and passes them with `-- --tag`, and the Cloud Run previews pass `--repository`. Tags therefore need no stamping.
- Single architecture, `linux/amd64`, as today.

### CI

- `.github/actions/bazel-rbe` is copied from Sipi unchanged. It emits remote cache and remote execution flags for the NativeLink endpoint, and nothing when the secrets are absent, in which case the build runs cold and local. Dependabot pull requests receive Dependabot secrets, not Actions secrets, so they build cold until H2 is done.
- One composite action, `.github/actions/bazel-setup`, holds what every Bazel job needs: Bazelisk, the repository cache restored with `actions/cache/restore` keyed on `MODULE.bazel.lock`, and `bazel-rbe`, and it hands the calling recipe `--config=ci` together with the remote flags. Each calling job saves the cache with a guarded `actions/cache/save`. No disk cache. This follows Sipi's current code (`ci-setup`), not the older learning, with one exception: Sipi installs Bazelisk with `bazelbuild/setup-bazelisk`, which is archived ("superseded by setup-bazel"). Here it is `bazel-contrib/setup-bazel` with its own caches off.
- Phases 2 to 8 add Bazel steps to the existing jobs `check` and `test / ubuntu / stable`, whose timeout rises from 20 to 60 minutes for the dual period, because each then runs two builds. Phase 9 removes the Cargo steps and keeps the job names. The two required contexts that disappear (`check / hack / ubuntu / stable / features`, `test / ubuntu / beta`) are removed from the ruleset by a person when that pull request merges (H8).
- Local workflow files and local composite actions (`./.github/actions/…`) are taken from the pull request's own checkout, so a pull request tests its own edits to them. `push`- and `release`-triggered workflows are only exercised after merge, which is why Phase 8 routes them through the same composite actions the pull-request workflows use. What remains untested until a real release is the release workflows' own tag expression and their two Jenkins triggers, which Phase 8 does not edit. Accepted, and H9 keeps Cargo in the tree until that release has deployed.
- After Phase 9 no CI job builds or tests the Cargo workspace. The manifests stay until Phase 10 as the way back, and they are one only while they still build: a dependency change in that window still edits the manifests (the rule in `docs/src/bazel.md`), and nothing enforces it.
- The way back, by phase. Phases 2 to 7 add files and steps, so each is reverted alone. For a bad deploy after Phase 8, the previous image tag is redeployed first, then Phase 8 is fixed forward or reverted, after Phase 9 if that has merged. A revert of Phase 9 restores the Cargo gates and puts the two required contexts of H8 back into the ruleset by hand.
- Rebasing the stack. `MODULE.bazel.lock` and `Cargo.Bazel.lock` are never merged by hand: a conflict takes `main`'s side and is regenerated (`just repin`, then `bazel mod deps --lockfile_mode=update`). After each rebase a phase re-runs its own parity commands (test list, lock, image listing), because `main` has moved under BUILD files that were written against an older tree. A manifest that `main` changed and Phase 10 deletes stays deleted.

### `dsp-cli`

- Built and tested by Bazel like every other crate, edition 2024, with a `version` attribute that release-please updates from Phase 5 on, so `CARGO_PKG_VERSION` matches the manifest.
- In Phase 10 `dsp-cli/Cargo.toml` becomes a standalone manifest with its own `dsp-cli/Cargo.lock`. Each `workspace = true` dependency is written out with the root entry's version and features, `serde_json`'s `preserve_order` in particular: the `_meta`-first envelope depends on it. `dsp-cli/Cargo.lock` starts as a copy of the root lock that Cargo trims to the crate's own graph, not as a fresh resolution, so the published crate starts on the versions that were tested. The lock-parity script confirms it against `Cargo.Bazel.lock`.
- The dependency list then exists twice, in `MODULE.bazel` and in the manifest. Two things keep the published crate honest: `cargo publish --dry-run --locked` builds the packaged crate from the manifest alone, and `cargo test --locked --manifest-path dsp-cli/Cargo.toml` runs in the same job, because a missing feature such as `preserve_order` compiles and only the snapshot tests notice. Version drift between the two locks within semver ranges is not checked: any `cargo install` without `--locked` already resolves differently from what was tested.
- `rust-toolchain.toml` stays for that Cargo use and for the tools installed with `cargo binstall`. It may differ from Bazel's pin, and nothing built for deployment reads it.

### What stays outside Bazel

Playwright suites, the mdBook build, release-please, Jenkins triggers and Cloud Run deploy steps, the readiness and commit-hygiene workflows, the F-UJI check, the Python data scripts, `rustfmt` and `maudfmt`, the `dsp-cli` drift stack, and the tools CI installs with `cargo install` (`commitlint-rs`, the mdBook plugins).

### Verification commands

Lock parity, used in Phases 2, 8 and 10. Phase 8 is the one that matters for what ships: from there the deployed versions are Bazel's, and a `Cargo.lock` change that landed on `main` during the dual period without a repin would otherwise surface as a version jump in production. Prints, for packages present in both locks, the versions only one of them holds; no output means parity. Tested against `Cargo.lock` and a copy with one version changed.

```bash
#!/usr/bin/env bash
# usage: lock-parity.sh Cargo.lock Cargo.Bazel.lock
set -euo pipefail
export LC_ALL=C
pairs() { awk '/^name = /{n=$3} /^version = / && n {print n, $3}' "$1" | tr -d '"' | sort -u; }
a=$(mktemp); b=$(mktemp); both=$(mktemp); trap 'rm -f "$a" "$b" "$both"' EXIT
pairs "$1" > "$a"; pairs "$2" > "$b"
join <(cut -d" " -f1 "$a" | uniq) <(cut -d" " -f1 "$b" | uniq) > "$both"
diff <(join "$both" "$a") <(join "$both" "$b")
```

Test parity, used in Phases 2 to 5 for the crates each phase adds, and once more in Phase 9 for the whole workspace, before the Cargo gate goes: a test or crate added on `main` during the dual period without a BUILD entry passes both gates until then. The baseline is what CI runs today (`--all-features`). Every test function Cargo lists must be listed by a Bazel test target; Bazel may list more, because a variant target repeats its crate's tests. The Bazel half is written for the phase and has not been run.

```bash
cargo test --locked --all-features --tests -p <crates> -- --list --format terse 2>/dev/null | grep ': test$' | sort > cargo-tests.txt
bazel test --test_arg=--list --test_arg=--format=terse --test_output=all --cache_test_results=no <targets> 2>/dev/null | grep ': test$' | sort > bazel-tests.txt
comm -23 cargo-tests.txt bazel-tests.txt   # prints nothing
```

Image parity, used in Phase 6, against the image the matching `just build-docker-*` recipe builds from the Dockerfile:

```bash
docker inspect --format '{{json .Config}}' <image> | jq '{Env, Entrypoint, Cmd, ExposedPorts, User, WorkingDir}'
docker export "$(docker create <image>)" | tar -t | sort
```

### Accessibility

Not a UI plan: no control or view changes. The Playwright accessibility suites run unchanged against the Bazel-built binaries from Phase 9 on.

### Review

Phase reviews use `eng:review:devops-reviewer` (BUILD files, workflows, `justfile`, images), `eng:review:security-reviewer` (TLS provider, secrets in workflows, image contents), `eng:review:rust-reviewer` (source edits), `eng:review:nix-reviewer` (`flake.nix`), `eng:review:dune-reviewer` (visibility against ARCH-MAP and the ADRs), `eng:review:consistency-reviewer` and `eng:review:code-simplicity-reviewer`.

## Implementation Phases

Every phase is one pull request with one commit, stacked on the previous phase's branch with `gh stack`. Until Phase 9, `just check` and `just test` are the Cargo gates and must stay green. The `just bazel-*` recipes are added from Phase 2 on. An item marked *(CI)* is verified on the pull request's own workflow run after pushing.

#### Phase 1: One TLS provider

Cargo only. No Bazel file exists after this phase.

- [ ] `reqwest` 0.13 uses `rustls-no-provider` in `dsp-cli`
- [ ] `reqwest` 0.13 uses `rustls-no-provider` in `editor-pull-requests`
- [ ] `pyroscope` no longer enables `reqwest`'s `rustls` feature in `access-server` and `deposit-server`, and still reaches an HTTPS endpoint through the `ring` provider
- [ ] `rustls` has the `ring` feature and not `aws_lc_rs` in all four binaries, as a direct dependency with `default-features = false` where no other crate enables it (`dsp-cli` and `editor-pull-requests` have no `ring` in their graph today)
- [ ] Each of the four binaries installs `ring` as the process default (`rustls::crypto::ring::default_provider().install_default()`) on the code path that builds its HTTPS client: with `rustls-no-provider`, reqwest panics at `Client` construction when no provider is installed, and the `ring` feature alone installs none
- [ ] Test per binary that builds the HTTPS client through that same code path (DSP-API client, GitHub client, profiler agent), without installing a provider itself: it catches a binary that never installs one, which panics at client construction and not at compile time
- [ ] `cargo tree --locked -i aws-lc-sys --target x86_64-unknown-linux-musl` reports that no such package is in the graph
- [ ] cmake removed from `just install-requirements` and `flake.nix`, with the `justfile` comment that explains it
- [ ] Run `just check && just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 2: Bazel foundation

Lands the toolchains, the whole third-party universe and the six leaf crates, and gates Bazel on every pull request from here on.

- [ ] `.bazelversion` at 9.3.0
- [ ] `.bazelignore` with the entries under *Toolchains and platforms*
- [ ] `.bazelrc` with the settings under *Toolchains and platforms*
- [ ] `MODULE.bazel` with the pins under *Proposed Solution*, the `llvm` toolchain blocks before the `rust` extension, and Sipi's two `llvm` patches under `bazel/patches/`
- [ ] `platforms/BUILD.bazel` with `linux_x86_64`, the Linux image target platform and `darwin_aarch64`
- [ ] One `crate.spec` per direct third-party crate, with the union of the features all manifests request and the dependency comments from the root `Cargo.toml`
- [ ] `Cargo.Bazel.lock` seeded from `Cargo.lock`, and `MODULE.bazel.lock`, both committed
- [ ] The lock-parity script under *Verification commands* prints nothing for `Cargo.lock` and `Cargo.Bazel.lock`
- [ ] `bazel build @crates//...` passes on macOS
- [ ] *(CI)* `bazel build @crates//...` passes on Linux through the `bazel-rbe` action (a step in a throwaway commit, dropped before review: the six leaf crates alone do not pull the whole universe in)
- [ ] Linux target settled (D2), smallest step first, each with the wiring under *Corrected premises*: a `rust_binary` with no dependencies builds for the musl platform from macOS and `--toolchain_resolution_debug=@rules_rust//rust:toolchain` shows the musl toolchain selected; `file` calls its output statically linked; `@crates//:ring` and `@crates//:libsqlite3-sys` build for it; `bazel build --platforms=<musl platform> @crates//...` passes; *(CI)* the same passes with the Linux remote executor, where the `gnu` exec toolchain must not win (#3831). The first step that cannot pass without patching `rules_rust`, the `llvm` module or a crate ends the attempt: the target is glibc 2.28 and the image base `distroless/cc-debian12`
- [ ] `shared/datastar/BUILD.bazel`: library and tests, private by default
- [ ] `shared/fair/BUILD.bazel`: library, tests, and its `testdata/schemas/datacite-4.3-schema.json` exported to `dpe-api-oai`
- [ ] `shared/metadata/BUILD.bazel`: library, tests, and its `testdata` exported to `dpe-api-oai`
- [ ] `shared/rate-limit/BUILD.bazel`: library and tests
- [ ] `shared/telemetry/BUILD.bazel`: library and tests
- [ ] `modules/mosaic/tiles/BUILD.bazel`: library and tests
- [ ] `justfile` recipes `bazel-build`, `bazel-test` and `bazel-clippy` (aspect with `-Dwarnings`)
- [ ] `justfile` recipe `repin`, with an optional crate name
- [ ] `just repin` on macOS leaves `MODULE.bazel.lock` in a state that `bazel build --config=ci //...` accepts there and *(CI)* on Linux
- [ ] A `crate.spec` version edited without `just repin` fails `bazel build --config=ci //...`
- [ ] `justfile` recipe `audit` over `Cargo.Bazel.lock`
- [ ] Advisories `just audit` reports on the seeded lock are each fixed by a bump or ignored with a written reason, beside the `--ignore` flag in the recipe and in `osv-scanner.toml`
- [ ] `justfile` recipe `rust-project`
- [ ] `.github/actions/bazel-rbe` copied from Sipi
- [ ] `.github/actions/bazel-setup` per *CI*
- [ ] `test.yml`: the stable leg of the `required` job runs `bazel-setup`, `just bazel-test` and the cache save
- [ ] `check.yml`: the `check` job runs `bazel-setup` and `just bazel-clippy`
- [ ] `timeout-minutes: 60` on those two jobs
- [ ] `.github/workflows/audit.yml`: daily schedule, `workflow_dispatch`, and pull requests that change `Cargo.Bazel.lock`; installs `cargo-audit` and `osv-scanner` at pinned versions; runs `just audit`
- [ ] `bazelisk`, `cargo-audit` and `osv-scanner` in `flake.nix` (as packages, with a `bazel` shim for `bazelisk` as in Sipi's `flake.nix`: nixpkgs installs no `bazel` command) and in `just install-requirements`
- [ ] Dependabot's `cargo` ecosystem paused with `open-pull-requests-limit: 0` in `.github/dependabot.yml`
- [ ] `docs/src/bazel.md`, listed in `docs/src/SUMMARY.md`: build, test, add a crate, repin, IDE setup, and the rule that every pull request keeps both builds green until Cargo is removed
- [ ] `rust-project.json`, `bazel-*` and `user.bazelrc` in `.gitignore`
- [ ] ADR-0001 rewritten in place where the code differs: zero-sysroot toolchain, the settled Linux target, images as `rules_oci` targets instead of Dockerfiles; its `Not yet in code` line, false once this phase merges, says what is still open (the root `Cargo.toml` and `Cargo.lock` exist beside the Bazel build) and names DEV-7699; the commit body says what the record said before
- [ ] The test-parity command under *Verification commands* prints nothing for the six crates
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] *(CI)* `check` and `test / ubuntu / stable` pass with their Bazel steps
- [ ] *(CI)* One run of both jobs without the remote flags (a throwaway commit that skips `bazel-rbe`, dropped before review) is timed, and the duration is written into `docs/src/bazel.md`: it is what a fork pull request, which receives no secrets, and every pull request during an endpoint outage will take
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 3: Access Area

Every BUILD file lists its consumers per ADR-0003 and ADR-0007, and sets `compile_data`, `data` and `rustc_env` per *Files a crate reads*.

- [ ] `areas/access/dpe/core/BUILD.bazel`
- [ ] `areas/access/dpe/api-oai/BUILD.bazel`, with the test targets that call `xmllint` tagged `no-remote-exec`
- [ ] `areas/access/dpe/web/BUILD.bazel`
- [ ] `areas/access/dpe/server/BUILD.bazel`, with `dpe_server_dev`, its test target and the corpus `filegroup`
- [ ] `areas/access/cpe/core/BUILD.bazel`, with `src/validate.rs` exported to `cpe-store-build`'s tests
- [ ] `areas/access/cpe/ports/BUILD.bazel`
- [ ] `areas/access/cpe/image-viewer/BUILD.bazel`, with the manifest's `unsafe_code = "forbid"` as `rustc_flags` on its targets
- [ ] `areas/access/cpe/components/BUILD.bazel`, with its `style/` exported to `cpe-server` and `cpe-web`
- [ ] `areas/access/cpe/store/BUILD.bazel`, with `schema.sql` exported to `cpe-store-build`
- [ ] `areas/access/cpe/store-build/BUILD.bazel`
- [ ] `areas/access/cpe/web/BUILD.bazel`
- [ ] `areas/access/cpe/server/BUILD.bazel`, with `tests/fixtures/activation/things/config.kdl` exported to `cpe-core`'s tests
- [ ] `areas/access/sync/store/BUILD.bazel`, with a `testonly` alias as the only target another capability can see
- [ ] `areas/access/media/store/BUILD.bazel`, with a `testonly` alias as the only target another capability can see
- [ ] `areas/access/server/BUILD.bazel`: `access_server` and `access_server_dev`
- [ ] `filegroup`s for `areas/access/dpe/public` and `areas/access/cpe/public`, without generated stylesheets
- [ ] `filegroup`s for `areas/access/sync/data`, `areas/access/media/data` and `areas/access/cpe/projects`
- [ ] `docs/src/cpe/config-language.md` exported to `cpe-store-build`'s tests
- [ ] The 10 `include_str!`/`include_bytes!` lines in `cpe-store-build` that go through `CARGO_MANIFEST_DIR` use file-relative paths
- [ ] `.bazelrc`: the two `insta` settings under *Tests*
- [ ] `justfile` recipe `snap-accept`, documented in `docs/src/bazel.md`
- [ ] `just snap-accept //areas/access/dpe/server:dpe_server_test` leaves `git status` clean
- [ ] Negative check: a `dpe-*` crate cannot depend on a `cpe-*` crate
- [ ] Negative check: `access-server` cannot depend on `dpe-core`
- [ ] Negative check: `cpe-server`'s library cannot depend on `sync-store`
- [ ] Negative check: a crate outside CPE cannot depend on `cpe-core`
- [ ] The test-parity command under *Verification commands* prints nothing for the Access Area crates
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 4: Deposit Area

Every BUILD file lists its consumers per ADR-0003 and `areas/deposit/ADR-0004`. A crate with the `test-support` feature gets its `_test_support` variant in the same file, wired per *Cargo features become targets*.

- [ ] `areas/deposit/shell/BUILD.bazel`, with `DEPOSIT_VERSION` from `rustc_env_files`
- [ ] `areas/deposit/identity/core/BUILD.bazel`
- [ ] `areas/deposit/identity/ports/BUILD.bazel`, with its variant
- [ ] `areas/deposit/identity/store/BUILD.bazel`, with its variant
- [ ] `areas/deposit/identity/web/BUILD.bazel`, with its variant
- [ ] `areas/deposit/editor/core/BUILD.bazel`
- [ ] `areas/deposit/editor/ports/BUILD.bazel`, with its variant
- [ ] `areas/deposit/editor/store/BUILD.bazel`, with its variant
- [ ] `areas/deposit/editor/web/BUILD.bazel`, with its variant
- [ ] `areas/deposit/editor/pull-requests/BUILD.bazel`
- [ ] `areas/deposit/server/BUILD.bazel`
- [ ] `filegroup` for `areas/deposit/editor/public`, without generated stylesheets
- [ ] `areas/deposit/editor/style` exported to `editor-web`'s tests
- [ ] The corpus `filegroup` grants the tests of `editor-core`, `editor-store`, `editor-web` and `editor-pull-requests`, and no other target under `areas/access` is visible from `areas/deposit`
- [ ] `tools/workspace_status.sh`, wired in `.bazelrc`, emitting `STABLE_DEPOSIT_VERSION` from the `DEPOSIT_VERSION` environment variable (`dev` when unset) and no volatile key
- [ ] An unstamped build renders the footer as an unset `DEPOSIT_VERSION` does today, and `DEPOSIT_VERSION=x bazel build --stamp` renders `x`
- [ ] `justfile` recipe `canonicalize-projects`, documented in `docs/src/bazel.md`
- [ ] `just canonicalize-projects` leaves `git status` clean
- [ ] Negative check: `deposit-server` cannot depend on `editor-core`
- [ ] Negative check: an `editor-*` crate cannot depend on a `dpe-*` crate
- [ ] Negative check: `deposit-server`'s binary cannot depend on a `_test_support` target
- [ ] The test-parity command under *Verification commands* prints nothing for the Deposit Area crates
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 5: dsp-cli and the Mosaic playground

- [ ] `dsp-cli/BUILD.bazel`: library, the `dsp` binary, edition 2024, `docs/topics/*.md` as `compile_data`, `version` equal to `dsp-cli/Cargo.toml`'s
- [ ] `.github/release-please/config.json`: the `dsp-cli` package also updates the `version` in `dsp-cli/BUILD.bazel` (an `extra-files` entry of type `generic`, with an `x-release-please-version` comment on that line)
- [ ] `dsp-cli`'s `cli.rs` and `docs.rs` tests find the binary through `CARGO_BIN_EXE_dsp`, made absolute in the helper that builds the command
- [ ] `dsp-cli/ADR-0011`'s `Enforced by:` line names `CARGO_BIN_EXE_dsp` instead of `Command::cargo_bin("dsp")`
- [ ] Integration test targets for `dsp-cli`, one `rust_test` per file with `crate_name` equal to the file stem, `tests/snapshots/` and `tests/fixtures/` as inputs and `tests/common`, `tests/support` in `srcs`
- [ ] Live test targets for `dsp-cli` per *Cargo features become targets*
- [ ] `justfile` recipe `bazel-dsp-cli-test-live`
- [ ] `bazel build` of the live test targets passes
- [ ] `modules/mosaic/playground/BUILD.bazel`: `mosaic_playground`, `mosaic_playground_dev` with its test target, and its `public` `filegroup` without generated stylesheets
- [ ] `rust_doc_test` for `mosaic-tiles`
- [ ] `rust_doc_test` for `cpe-image-viewer`
- [ ] `rust_doc_test` for `dsp-cli`
- [ ] A doctest that `rules_rust` cannot run is a unit test with the same assertion
- [ ] `bazel build //...` passes, including every `_dev` and `_test_support` target
- [ ] Every package name `cargo metadata --no-deps` lists is the `crate_name` of a `rust_library` or `rust_binary` in `bazel query --output=build //...`
- [ ] The test-parity command under *Verification commands* prints nothing for `dsp-cli` and `mosaic-playground`
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 6: CSS and images

Built, verified and gated. Nothing is published from them yet.

- [ ] The Tailwind CLI as `http_file`s for four platforms and one exec-selected alias
- [ ] CSS target for `dpe`, with every `@source` input declared
- [ ] CSS target for `editor`, with every `@source` input declared
- [ ] CSS target for `mosaic-playground`, with every `@source` input declared
- [ ] Each CSS target's output is byte-identical to the output of `just css-release`, `just css-editor-release` and `just css-mosaic` at the same commit
- [ ] `build:release` in `.bazelrc` per *Toolchains and platforms*
- [ ] The release `access-server` and `deposit-server` binaries contain a `.debug_line` section, and are statically linked if Phase 2 settled on musl
- [ ] `oci.pull` of the base image by digest
- [ ] `oci_image`, `oci_load` and `oci_push` for `daschswiss/dpe`
- [ ] `oci_image`, `oci_load` and `oci_push` for `daschswiss/metadata-editor`, with the corpus `filegroup` granted to it as its data layer
- [ ] `oci_image`, `oci_load` and `oci_push` for `daschswiss/mosaic-playground`
- [ ] The image-parity commands under *Verification commands* show equal configuration for each image, and file listings that differ only by the CSS file name
- [ ] The same listing is produced from a checkout that holds a locally built `app.css` and an old `app.<hash>.css`
- [ ] `justfile` recipes `bazel-docker-build-dpe`, `bazel-docker-build-editor`, `bazel-docker-build-mosaic`
- [ ] `justfile` recipe `bazel-image-smoke`: loads each image, starts it, expects 200 from `/healthz` (the playground: `/`) and exit 0 from the `healthcheck` subcommand
- [ ] `test.yml`: the stable leg runs `just bazel-image-smoke`
- [ ] Images, CSS and the two Tailwind pins in `docs/src/bazel.md`
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] *(CI)* The image smoke step passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 7: Fuzz targets

The Bazel harness lives beside the Cargo one in `areas/access/dpe/server/fuzz/`, in new files. The corpus stays at `fuzz/corpus/<target>`, which is untracked and cached by the workflow.

- [ ] The five fuzz targets as `cc_fuzz_test` over a `testonly` `rust_static_library` shim, after Sipi's `src/iiifparser/fuzz/BUILD.bazel`, with the target bodies copied from `fuzz_targets/*.rs`
- [ ] `build:fuzz` in `.bazelrc`, from Sipi, with `extra_rustc_flag` in the singular
- [ ] In the default configuration the five targets build and run as part of `just bazel-test`
- [ ] `justfile` recipe `bazel-build-fuzz`
- [ ] The `tab_validation` binary from `just bazel-build-fuzz` runs with `-max_total_time=60 -max_len=4096` and exits 0
- [ ] The `dpe` stylesheet is still byte-identical to `just css-release`: the new package under `fuzz/` takes its files out of the server package's glob, and `main.css` scans that directory
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 8: Deploy path builds with Bazel

From here what is published and deployed comes from Bazel. Cargo is still built and tested on every pull request.

- [ ] The lock-parity script under *Verification commands* prints nothing, or `Cargo.Bazel.lock` is repinned to `Cargo.lock`'s versions in this commit
- [ ] The base-image digest in `oci.pull` equals what the Dockerfiles' tag resolves to in this commit
- [ ] `.github/actions/build-dpe` loads the image with Bazel instead of staging a Cargo build
- [ ] `.github/actions/build-editor` loads the image with Bazel, built with `--stamp` and the caller's `DEPOSIT_VERSION`
- [ ] `.github/actions/docker-publish` pushes with the image's `oci_push` target and the caller's tag
- [ ] `dpe-docker-publish.yml` and `dpe-release-publish.yml` publish through those actions, tags and Jenkins steps unchanged
- [ ] `editor-docker-publish.yml` and `editor-release-publish.yml` publish through those actions, tags and Jenkins steps unchanged
- [ ] `mosaic-docker-publish.yml` publishes the Bazel image, tag and Jenkins step unchanged
- [ ] `scout-dpe.yml`, `scout-editor.yml` and `scout-mosaic-playground.yml` scan the image `oci_load` tags
- [ ] `cloud-run-dpe-pull-request.yml` pushes with `--repository` and `--tag`
- [ ] `cloud-run-editor-pull-request.yml` pushes with `--repository` and `--tag`, and the preview shows `pr-<N>-<sha>` in its footer
- [ ] `cloud-run-mosaic-pull-request.yml` pushes with `--repository` and `--tag`
- [ ] Path filters: the three Scout workflows name `MODULE.bazel`, `MODULE.bazel.lock`, `Cargo.Bazel.lock`, `.bazelrc`, `.bazelversion`, `platforms/`, `bazel/` and the composite actions in place of `Cargo.lock`; `mosaic-docker-publish.yml` and the three preview workflows, which name no lock today, gain the same list (the two other publish workflows have no path filter)
- [ ] `docs/src/deployment.md` and the "places that have to agree" section of `areas/access/server/CLAUDE.md` describe the Bazel image
- [ ] The compressed size of each image is in the pull request description: before from the published tag (`docker manifest inspect`), after from the layer blobs of the `oci_image` output
- [ ] Run `just bazel-test`; it passes
- [ ] Run `just check && just test`; it passes
- [ ] *(CI)* The three Scout jobs pass
- [ ] *(CI)* The three preview deployments answer 200
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 9: Pull-request gates and developer tooling on Bazel

After this phase nothing runs Cargo except `dsp-cli`'s publish and dry run.

- [ ] Before any recipe is re-keyed: the test-parity command under *Verification commands* prints nothing for the whole workspace, and every package name `cargo metadata --no-deps` lists is still the `crate_name` of a Bazel target (the Phase 5 check, repeated against today's `main`)
- [ ] Before `just dev` is re-keyed: the time from saving a one-line change in `areas/access/dpe/web/src` to the dev server answering again, three runs under Cargo; after it, three runs under Bazel on the same machine; both in the pull request description
- [ ] `just check` re-keyed: the gate scripts that remain, `just --check --fmt --unstable`, the `maudfmt` check, the `rustfmt` check, the clippy aspect
- [ ] `just fmt` and the `rustfmt` check call the pinned nightly `rustfmt` on files: edition 2021 for the tree, edition 2024 for `dsp-cli`. The binary is `$RUSTFMT` when set (the Nix shell exports it), else `rustup which --toolchain "$(cat .rustfmt-nightly)" rustfmt`: once `cargo +nightly fmt` is gone, a bare `rustfmt` is the stable one
- [ ] `just test` re-keyed: `bazel test //...` and `just test-scripts`
- [ ] `just build`, `just run`, `just validate-data` and `just clean` re-keyed per *Developer loop and E2E*
- [ ] `just css`, `just css-release`, `just css-editor`, `just css-editor-release` and `just css-mosaic` re-keyed per *Developer loop and E2E*
- [ ] `just dev`, `just dev-otel`, `just dev-editor`, `just dev-editor-otel` and `just watch-mosaic-playground` re-keyed, with the Tailwind watch running the Bazel-provided CLI
- [ ] `just build-docker-*` and `just run-docker-*` re-keyed on the image targets
- [ ] `just dsp-cli-run` and `just dsp-cli-test-live` re-keyed, and `just dsp-cli-snap-review` replaced by `just snap-accept`
- [ ] The twins `bazel-build`, `bazel-test`, `bazel-clippy`, `bazel-docker-build-*`, `bazel-dsp-cli-test-live`, `bazel-build-fuzz` and `bazel-image-smoke` deleted or renamed to the canonical recipe (`image-smoke` for the last)
- [ ] `bacon.toml`: `check` runs `bazel build //...`, `serve` runs `access_server_dev`, `serve-editor` runs `deposit_server` (it has no `dev` feature), and a new playground job runs `mosaic_playground_dev`, all per *Developer loop and E2E*
- [ ] Removed from the `justfile`: `CARGO_VERSION`, `IMAGE_TAG`, `DOCKER_IMAGE`, `DOCKER_REPO`, `COMMIT_HASH`, `GIT_TAG`, `docker-build-amd64`, `docker-build-arm64`, `docker-push-amd64`, `docker-push-arm64`, `docker-publish-manifest`, `docker-image-tag`, `watch`, `test-changed`, `_tailwind-bin`, `tailwind-pins-refresh`
- [ ] `version.txt` at the root with the value `.github/release-please/manifest.json` holds for `.`, read by the dev recipes' `service.version`
- [ ] `.github/release-please/config.json`: the root package loses its `Cargo.toml` entry under `extra-files`; `version.txt` is the file its `simple` release type updates by default
- [ ] `test.yml`: job `test / ubuntu / stable` runs `just test` and `just image-smoke`; the beta leg, nextest, the doc-test step and the coverage job are removed
- [ ] `test.yml`: the `scripts` job no longer installs a Rust toolchain
- [ ] `check.yml`: job `check` runs `just check` with `bazel-setup` and the pinned nightly `rustfmt`; the `hack` and `rustdoc` jobs are removed
- [ ] Timeouts of `check` and `test / ubuntu / stable` set to twice the slowest warm run of their Bazel steps in Phases 2 to 8
- [ ] `a11y-dpe.yml` builds the server with Bazel through `bazel-setup`, and its path filter gains the list Phase 8 gave the Scout workflows
- [ ] `a11y-editor.yml` builds the server with Bazel through `bazel-setup`, and its path filter gains the same list
- [ ] Both Playwright configs that start a server take its path from the environment variable their recipes set, instead of `target/release`, and their error message names the recipe
- [ ] `fuzz.yml` builds with `just bazel-build-fuzz` through `bazel-setup` and runs the binaries with the cached corpus
- [ ] `open-editor-pull-requests.yml` builds `editor-pull-requests` with Bazel and runs the binary from the repository root
- [ ] `.github/actions/dsp-cli-stack-test` runs the live suite with Bazel through `bazel-setup`, and its failure summary reads Bazel's test log instead of nextest's `FAIL` lines
- [ ] `claude.yml`: the review job's `Cargo.lock` wording names the Bazel locks (its tool grant holds no cargo entry)
- [ ] `check-capability-deps.sh` reduced to the third-party bans per role, re-keyed on `BUILD.bazel` `deps`, with the public-visibility rule, and its `.test.sh` on fixture BUILD files
- [ ] `check-composition-root-deps.sh`, `check-cpe-seam.sh`, `test-changed.sh` and their `.test.sh` files deleted, and their recipes removed from `just check`
- [ ] `verify-checksums.sh` without its Tailwind half, and `tailwind.pins` deleted
- [ ] The three Dockerfiles and `.dockerignore` deleted: after this phase no recipe or workflow builds them, and the Mosaic one reads `tailwind.pins`
- [ ] `.github/dependabot.yml`: the `cargo` and `docker` ecosystems removed
- [ ] `eng.yaml`: `test_targeted` is `just test`, and the two Dockerfile overrides name the BUILD files that define the images
- [ ] `flake.nix` and `just install-requirements` without cargo-nextest, cargo-deny, cargo-machete, cargo-watch and the `cargoWrapper` shim; `rustStable`, `rustNightly` and `RUSTFMT` stay
- [ ] `deny.toml` deleted
- [ ] Every ADR that names a script this phase deletes or re-keys, in its `Enforced by:` line or in its text, says what holds the rule now, `visibility` (structure) where the table under *Boundaries* says so: ADR-0002, ADR-0003, ADR-0007, ADR-0008, ADR-0012, ADR-0013 and `areas/deposit/ADR-0004`; the commit body records what each line named before
- [ ] `ARCH-MAP.md` refreshed with `dune:dune-map`, so each boundary rule carries its new enforcement level
- [ ] `git grep -nE 'test-changed|check-cpe-seam|check-composition-root-deps|tailwind\.pins|tailwind-pins-refresh|deny\.toml|cargo[ -](nextest|deny|machete|watch|hack|llvm-cov)|Dockerfile' -- . ':!docs/specs' ':!docs/learnings' ':!CHANGELOG.md' ':!dsp-cli/CHANGELOG.md' ':!.claude/settings.json'` prints nothing: what this phase deletes is named nowhere, the `CLAUDE.md`, `CONTEXT.md` and `CONVENTIONS.md` files included
- [ ] Run `just check`; it passes
- [ ] Run `just test`; it passes
- [ ] *(CI)* `check`, `check / docs / stable`, `test / ubuntu / stable` and `gate` pass
- [ ] *(CI)* Both accessibility workflows pass
- [ ] *(CI)* `dsp-cli-drift / gate` passes with the live suite run by Bazel
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

#### Phase 10: Remove Cargo

- [ ] The lock-parity script under *Verification commands* prints nothing, before the root lock is deleted
- [ ] Root `Cargo.toml`, root `Cargo.lock` and `.cargo/config.toml` deleted
- [ ] Every member manifest except `dsp-cli/Cargo.toml` deleted
- [ ] `areas/access/dpe/server/fuzz/Cargo.toml`, its `Cargo.lock` and `fuzz_targets/` deleted
- [ ] `dsp-cli/Cargo.toml` standalone per *`dsp-cli`*, with `dsp-cli/Cargo.lock` committed, seeded from the root lock
- [ ] The lock-parity script prints nothing for `dsp-cli/Cargo.lock` and `Cargo.Bazel.lock`
- [ ] `check.yml`: `publish-dry-run` runs `cargo publish --dry-run --locked --manifest-path dsp-cli/Cargo.toml` and `cargo test --locked --manifest-path dsp-cli/Cargo.toml`
- [ ] `publish-dsp-cli.yml` publishes with `--manifest-path dsp-cli/Cargo.toml`
- [ ] `release-please.yml`: `amend-lockfile` runs for the `dsp-cli` release branch only and amends `dsp-cli/Cargo.lock`
- [ ] `git grep -nE 'cargo [a-z+]|Cargo\.(toml|lock)|target/release' -- . ':!docs/specs' ':!docs/learnings' ':!CHANGELOG.md' ':!dsp-cli/CHANGELOG.md' ':!dsp-cli/Cargo.toml' ':!.claude/settings.json'` prints only lines about `cargo publish`, `cargo install`, `cargo binstall`, or `dsp-cli`'s manifest, lock and its `cargo test` and `cargo update`
- [ ] `CLAUDE.md`, `CONVENTIONS.md` and `REVIEW.md` name the Bazel commands
- [ ] The per-directory `CLAUDE.md` files (`areas/access/dpe`, `areas/access/server`, `areas/deposit/editor`, `areas/deposit/server`, `modules/mosaic`, `dsp-cli`) and `.claude/skills/add-mosaic-component/SKILL.md` name the Bazel commands
- [ ] `docs/src/` pages that describe the Cargo workflow rewritten: `dpe/testing-strategy.md`, `deployment.md`, `security.md`, `git-conventions.md`, `workflows.md`, `repo_structure.md`
- [ ] `docs/src/fundamentals/{tech_stack,onboarding,review-guidelines}.md`, `mosaic/component-api-conventions.md`, `editor/{operations,persistence}.md` and `dsp-cli/{testing-strategy,usage,architecture}.md` rewritten the same way
- [ ] ADR-0001: the `Not yet in code` line deleted
- [ ] ADRs that still speak Cargo say it in Bazel's terms: "dev-dependency" in ADR-0003, ADR-0008 and ADR-0012 becomes a dependency of `testonly` targets, `dsp-cli/ADR-0005`'s `Enforced by:` line no longer calls `dsp-cli` a root workspace member, and `cargo publish -p dsp-cli` in ADR-0002 and `dsp-cli/ADR-0011` takes `--manifest-path`; the commit body records what each said before
- [ ] `ARCH-MAP.md` refreshed with `dune:dune-map` for whatever still names a manifest
- [ ] One learning under `docs/learnings/build-errors/` per problem recorded in a "Challenges and Decisions" section of this stack's pull requests that `dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md` does not cover
- [ ] Run `just check && just test`; it passes
- [ ] *(CI)* `check / publish-dry-run / stable` passes
- [ ] Phase review: adversarial review of this phase's commits; verified findings fixed before the next phase starts

## Human Actions

| Id | Action | Who | When | Why not the agent |
|----|--------|-----|------|-------------------|
| H1 | Review this plan and confirm or change D1 to D9, the fallback rule in D2, and the commit scope for build-graph commits (proposed: `build(deps)` where `MODULE.bazel` specs change, `build(ci)` otherwise; `CONVENTIONS.md` asks before a new scope is invented, and `docs/src/git-conventions.md` files CI and build automation under `chore(ci)`, so the type is part of the question). Decide whether the ten pull requests reference DEV-7699 or the per-phase follow-up issues it asks for | Marcin, with the team | before start | Decisions about what production runs on and which checks the team gives up |
| H2 | Add `REMOTEBUILD_CA_CERT`, `REMOTEBUILD_CLIENT_CERT` and `REMOTEBUILD_CLIENT_KEY` as Dependabot secrets: Dependabot-triggered runs do not receive Actions secrets | Infra | before start | Organisation secret administration |
| H3 | Tell the team that from the merge of Phase 2 until the merge of Phase 9 every pull request keeps both builds green, that a dependency change edits the manifests until Phase 10 merges, and that `docs/src/bazel.md` arrives with Phase 2 | Marcin | before start | A team announcement |
| H4 | Add the Bazel commands and the new `just` recipes to the `permissions.allow` list in `.claude/settings.json` | Marcin | before start | An agent does not edit its own permissions, and without them Phases 2 to 9 stop at every `bazel` call |
| H5 | Have Docker running on the machine that executes Phase 6, and install the stack extension the repository's convention names (`gh extension install github/gh-stack`; `gh extension list` is empty on this machine) | Marcin | before start | The image comparisons need a daemon on the developer's machine, and installing tools on it is its owner's call |
| H6 | Merge Phases 1 to 7 in order as each is approved. After Phase 1 has deployed, confirm that profiles still arrive in Pyroscope from both servers: the phase's tests prove a client can be built, not that a handshake succeeds | Marcin | after ship | Merging is the owner's decision, and the check needs the deployed environments |
| H7 | Merge Phase 8, then confirm on the publishes it triggers that DPE on dev, the editor on stage and the Mosaic playground run the new images (200 from the health route, hashed CSS served, footer version shown) before merging Phase 9. A defect found here is fixed forward from `main` or by reverting Phase 8, and the open children are rebased | Marcin | after ship | Needs the deployed environments |
| H8 | Immediately before merging Phase 9, remove `check / hack / ubuntu / stable / features` and `test / ubuntu / beta` from ruleset 5843207 | Repository admin | after ship | Ruleset administration; the pull request cannot merge while they are required and never reported |
| H9 | Rebase and merge Phase 10 only after one `v*` release has deployed DPE and the editor to prod from Bazel-built images, and the release pull request has bumped `version.txt` | Marcin | after ship | Needs a production release |
| H10 | Create the follow-up issues: coverage through `bazel coverage`; automated updates of crates (Renovate's `bazel-module` manager reads `crate.spec` and `oci.pull`; adopting it needs an organisation decision), `bazel_dep`s and the base-image digest; the base images' move to `-debian13`; the Rust bump to Sipi's version; an unused-dependency check | Marcin | after ship | Scoping and prioritising other work |
| H11 | Before Phases 1, 2, 7 and 9 merge, someone with a Nix dev shell checks each out and runs `nix develop --command just check` (Phases 1 and 9) or `nix develop --command just bazel-test` (Phases 2 and 7) | A reviewer who uses Nix | after ship | The machine that executes the plan has no Nix and CI runs none, so nothing else evaluates the `flake.nix` edits |
| H12 | After Phase 10 merges, remove the Cargo entries (`cargo build`, `cargo clippy`, `cargo deny`, `cargo nextest`, `cargo test`, `just test-changed`) from `.claude/settings.json` | Marcin | after ship | An agent does not edit its own permissions; the file is excluded from the Phase 9 and Phase 10 greps for that reason |
| H13 | After 20 successful `main` runs following Phase 9, compare the CI medians per *Success Metrics* and note the result on DEV-7699 | Marcin | after ship | The runs do not exist when the stack ships |

## Acceptance Criteria

- [ ] `bazel build //...` and `bazel test //...` pass on macOS arm64 and on the Linux CI runner
- [ ] No `Cargo.toml` or `Cargo.lock` is tracked outside `dsp-cli/`
- [ ] CI runs with `--lockfile_mode=error`, and a `crate.spec` change without `just repin` fails `check`
- [ ] Adding a dependency from an Access Area crate to a Deposit Area crate, from the composition root to a `core` crate, or from a production target to a `_test_support` target fails `bazel build` with a visibility or `testonly` error
- [ ] A first-party target made `//visibility:public` fails `just check`
- [ ] The three images published from `main` are built by `rules_oci`, and DPE, the editor and the playground run from them on their environments
- [ ] Every test function `cargo test --all-features --tests` lists when Phase 9 removes the Cargo gate is listed by a Bazel test target
- [ ] `dsp-cli` publishes to crates.io from `dsp-cli/Cargo.toml`, and its tests pass against that manifest
- [ ] `docs/src/bazel.md` covers build, test, add a crate, repin, accept a snapshot, build an image, the dev loop, IDE setup
- [ ] ADR-0001 carries no `Not yet in code` line

## Dependencies & Risks

- **DEV-7618's gate is half met.** The editor's first prod deploy ran with `v0.8.11` on 2026-11-16. `open-editor-pull-requests.yml` has no run yet, so the first collection run has not happened. If that run changes the workflow or `editor-pull-requests`, Phase 4 and Phase 9 port what is on `main` then.
- **The dual period costs every contributor a second build.** It lasts from the merge of Phase 2 to the merge of Phase 9, and its length is set by H7, which waits for a `main` publish.
- **Stack depth.** Ten stacked single-commit pull requests under a strict up-to-date rule mean a rebase of every open child after each merge (GitHub rebases a stack's remaining branches when its bottom pull request merges; by hand it is `git rebase --onto origin/main <old parent tip> <child>` and a `--force-with-lease` push). A rebase is not only mechanical, see *Rebasing the stack* under CI. Phase 10 sits open until H9 and is rebased then.
- **Local edit-compile speed.** `rules_rust` 0.74 has no incremental compilation inside a crate. An edit to `dpe-web` recompiles that crate and everything above it from scratch. See Success Metrics for the measurement that decides whether this needs follow-up.
- **Remote endpoint availability.** With `--remote_local_fallback` a failing endpoint degrades to a local build, which on a hosted runner may exceed the job timeout for a cold cache.
- **macOS is not in CI**, as today. A change that breaks the macOS build is found by the next developer.
- **Updates become manual.** After Phase 9 nothing proposes crate, `bazel_dep` or base-image bumps until H10's follow-up is done. `audit.yml` and the Scout scans on pull requests report what needs one.

## Risk Analysis & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Musl Rust target does not resolve or link with the hermetic toolchain | M | M | Settled in Phase 2, smallest step first, before any first-party BUILD file depends on it, with a stop rule; the wiring both projects' sources call for is listed under *Corrected premises*; glibc fallback is stated |
| A `cc-rs` build script (`ring`, `libsqlite3-sys`) fails in the sandbox or on the remote worker | L | H | Phase 2 builds the whole universe, all 90 build scripts, on both platforms; `ring` works in Sipi under these pins |
| Removing `aws-lc` leaves a binary without an installed TLS provider | M | H | Phase 1's client-construction tests through the binary's own code path; the change ships and deploys under Cargo before Bazel builds anything that ships, and H6 checks it there |
| Feature unification changes behaviour silently | L | M | The lock is seeded from `Cargo.lock`; the tests that pin the two load-bearing `serde_json` features run under Bazel from Phase 3 |
| A Tailwind `@source` input is undeclared, or not seen through the sandbox's symlinks, and classes vanish | M | M | Byte-equality check against today's recipe in Phases 6 and 7; after Phase 9 only the comment at the `@source` lines and the pull-request previews stand guard |
| The two locks drift apart during the dual period (a contributor's `Cargo.lock` change, a Dependabot security update) | M | M | Lock parity before the deploy path switches in Phase 8; `audit.yml` scans the lock that Bazel uses |
| A crate or test added on `main` during the dual period has no BUILD entry | M | M | Whole-workspace test parity and crate inventory in Phase 9, before the Cargo gate goes |
| The Cargo manifests rot between Phase 9 and Phase 10 and are no way back | M | L | The first way back is the previous image tag; H3 tells the team the manifests are still edited; the window ends with H9 |
| The Bazel-built image differs from the Dockerfile-built one | M | H | Phase 6 compares config and file listing and gates a smoke test; Phase 8 deploys previews from the pull request; dev and stage run it before prod (H7, H9) |
| A stale local stylesheet ends up in an image or a test | M | M | Public-directory `filegroup`s exclude generated stylesheets; Phase 6 repeats the listing on a dirty checkout |
| Tests pass under Bazel because they are not run | M | H | Test-list comparison with Cargo in Phases 3 to 5; target inventory compared in Phase 5 |
| `insta` cannot find or accept snapshots in the sandbox | M | L | Proven on the first snapshot crate in Phase 3 |
| Path-traversal guards reject runfiles symlinks | M | L | `copy_to_directory` for the affected data, as in Sipi |
| Someone silences a visibility error by making the target public | M | H | The public-visibility rule in the retained script from Phase 9; until then the Cargo-keyed scripts still hold the boundaries |
| The release workflows fail on the first release after Phase 8 | L | M | They share the composite actions the `main` publish exercises; Cargo stays in the tree until that release has deployed (H9) |
| Dependabot and fork pull requests build cold and time out | H | L | H2; the repository is public but forks are rare; Phase 2 measures the cold build |
| A security fix is needed during the dual period while Dependabot is paused | M | M | `audit.yml` runs daily from Phase 2; a bump is a manifest edit plus `just repin` in one pull request |
| The published `dsp-cli` loses a feature the Bazel build has | M | M | `cargo test` on the standalone manifest in the dry-run job |
| The first release after Phase 9 does not bump `version.txt` | L | M | H9 checks the release pull request before Cargo's version field is deleted |

## Success Metrics

- **Boundaries:** the acceptance-criteria edges fail at analysis, and three scripts are gone.
- **CI wall-clock, warm cache:** median duration of `test / ubuntu / stable` and `check` over the last 20 successful `main` runs before Phase 2 (`gh run list --workflow test.yml --branch main --status success --limit 20 --json startedAt,updatedAt`, and the same for `check.yml`) compared with the same measure over the 20 runs after Phase 9. Target: not slower.
- **Local edit loop:** time from saving a one-line change in `areas/access/dpe/web/src` to the dev server answering again, measured three times with `just dev` before Phase 9 and three times after, on the same machine. If the Bazel figure is more than twice the Cargo one, that is input for H10, not a blocker.
- **Image size:** compressed size of each published image before Phase 8 and after (`docker manifest inspect`), reported in the Phase 8 pull request.
- **Places a pin lives:** Tailwind version in one file (was three consumers of `tailwind.pins`); image description in one BUILD file per image (was four places).

## References

- Decision: `docs/adr/0001-bazel-builds-the-monorepo.md`, with ADR-0002, ADR-0003, ADR-0007, ADR-0012 for the boundaries
- Sipi, read at `origin/main` `5dcdb7e2`: `MODULE.bazel`, `.bazelrc`, `platforms/BUILD.bazel`, `.github/actions/{bazel-rbe,ci-setup}/action.yml`, `src/BUILD.bazel` (images), `src/iiifparser/fuzz/BUILD.bazel`, `test/e2e/sipi_e2e_test.bzl`, `tools/workspace_status.sh`, `docs/src/development/{bazel,ci,rbe}.md`
- Sipi plans and journals: `docs/specs/2026-04-29-01-refactor-sipi-bazel-migration-plan.md`, `2026-08-30-01-refactor-bazel-visibility-tightening-plan{,-journal}.md`, `2026-10-20-01-bazel-toolchain-upgrade-plan{,-journal}.md`, `2026-11-11-02-local-rbe-from-macos-plan.md`
- Learnings: `dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md`, `dasch-specs/learnings/build-errors/tailwind-source-none-silent-class-loss.md`, `dasch-specs/learnings/best-practices/deferred-work-lost-across-phase-handoffs.md`, `docs/learnings/test-setup/insta-snapshot-accept-by-rename-keeps-assertion-line.md`, `docs/learnings/test-setup/env-gated-live-tests-pass-vacuously.md`, `docs/learnings/configuration-errors/release-please-attributes-release-as-by-path.md`, `sipi/docs/learnings/workflow/orchestrator-must-return-not-park-on-long-builds.md`
- `rules_rust` 0.74.0 source: `rust/private/rustc.bzl` (`CARGO_MANIFEST_DIR`, `rustc_env` order, `CARGO_BIN_EXE_*`), `rust/private/rust.bzl` (test run environment); documentation at <https://bazelbuild.github.io/rules_rust/crate_universe_bzlmod.html>
- `rules_rust` issues that shaped decisions: #3139 and #3250 (`aws-lc-sys` symbol prefix, fixed by #3260 and #3370; #661 is the open tracking issue), #2544 and #3831 (musl toolchain resolution), #4407 (exec root in `cc-rs` objects), discussion #3828 (repin and the lock); `examples/cross_compile_musl` in the 0.74.0 tree
- Hermetic toolchain: <https://github.com/hermeticbuild/hermetic-llvm> (README: musl, `pie:off`, use with `rules_rust`); `rules_oci`: <https://github.com/bazel-contrib/rules_oci/tree/main/docs>
- Related issues: DEV-6341 (Sipi migration), DEV-7575 (pins), DEV-7083 (visibility), DEV-7618 (editor prod deploy), DEV-7767 (dsp-app)
