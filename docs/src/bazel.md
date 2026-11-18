# Building with Bazel

Bazel builds and tests the crates that have a `BUILD.bazel` file, beside the Cargo workspace (ADR-0001). Until Cargo is removed, both builds are gated on every pull request: `check` runs `just bazel-clippy` after `just check`, and `test / ubuntu / stable` runs `just bazel-test` after the Cargo tests, then `just bazel-image-smoke`.

The pins are those of Sipi: Bazel in `.bazelversion`, the rulesets and the Rust version in `MODULE.bazel`. The C and C++ toolchain is hermetic (`llvm`, zero-sysroot): it fetches the macOS SDK itself and builds the Linux libc runtimes from source, so no Xcode and no host compiler is used for compiling.

## Every pull request keeps both builds green

While both exist, a change that touches dependencies or crates is made on both sides:

- A dependency added, removed or changed in a `Cargo.toml` is also changed in the `crate.spec` lines of `MODULE.bazel` and in the `deps` of the crate's `BUILD.bazel`, followed by `just repin`.
- A crate that has a `BUILD.bazel` file gets new source files without a BUILD edit (`srcs` is a glob), but a new file it reads with `include_str!` or at run time goes into `compile_data` or `data`.
- A Dependabot security update bumps `Cargo.lock` alone. Add the matching `just repin <crate>` to its branch, or close it and make the bump by hand.

## Commands

| Recipe | What it runs |
|---|---|
| `just bazel-build` | `bazel build //...` |
| `just bazel-test` | `bazel test //...` |
| `just bazel-clippy` | clippy over every target, tests included, with `-Dwarnings` |
| `just repin [crate]` | re-resolves the crate universe and refreshes `Cargo.Bazel.lock` and `MODULE.bazel.lock` |
| `just audit` | `cargo-audit` and `osv-scanner` over `Cargo.Bazel.lock` |
| `just rust-project` | writes `rust-project.json` for rust-analyzer |
| `just snap-accept <target>` | runs one test target from the checkout with `INSTA_UPDATE=always`, so `insta` rewrites the snapshots that changed |
| `just bazel-dsp-cli-test-live` | the `dsp-cli` live tests against a DSP stack, with the `DSP_TEST_*` variables and `DSP_TOKEN` passed through |
| `just canonicalize-projects` | runs the editor's canonical round-trip test from the checkout with `CANONICALIZE_PROJECT_FILES=1`, so it rewrites the committed project files |
| `just bazel-docker-build-dpe`, `-editor`, `-mosaic` | builds an image and loads it into Docker as `local-scan/<name>:latest` |
| `just bazel-image-smoke` | loads the three images, starts each, and expects 200 from `/healthz` (the playground: `/`) and exit 0 from the `healthcheck` subcommand |
| `just bazel-build-fuzz` | the libFuzzer binaries of the DPE fuzz targets, `--config=fuzz` |

Each `bazel-*` recipe passes extra arguments to Bazel, for example `just bazel-test --test_output=all` or a single package: `bazel test //shared/telemetry/...`.

`bazel` is the Bazelisk command (`brew install bazelisk`, or the Nix dev shell). A first build fetches the toolchains and compiles the third-party crates, and takes several minutes; later builds are incremental.

## Adding a crate or a dependency

Third-party crates are declared once, in `MODULE.bazel`, as `crate.spec` lines with the union of the features every first-party crate needs. Targets name them as `@crates//:<name>`. To add one:

1. Add the `crate.spec` (and the dependency in the crate's `Cargo.toml`, while Cargo exists).
2. Run `just repin`. It resolves the new spec and leaves every other locked version where it is. `just repin <crate>` moves one crate, which is how a transitive crate gets a security bump.
3. Add `@crates//:<name>` to the `deps` of the targets that use it.

A first-party crate's `BUILD.bazel` is hand-written: `rust_library`, `rust_test` and, for integration tests, `rust_test_suite`, with `crate_name` always Cargo's name. A target is visible only to the packages listed in its `visibility`; a new dependency edge between first-party crates is one line there, in the provider's BUILD file. No first-party target is public.

Never run `bazel mod tidy`, also when Bazel's output suggests it: it removes `use_repo` lines the `llvm` extension needs. `MODULE.bazel.lock` and `Cargo.Bazel.lock` are regenerated (`just repin`), never merged by hand; after a rebase conflict, take `main`'s side and repin.

CI runs with `--config=ci`, which sets `--lockfile_mode=error`: a `MODULE.bazel` change without the matching `MODULE.bazel.lock` fails.

## Tests and the files they read

A test runs from its runfiles tree, where every file it declares in `data` is a symbolic link into the checkout. A target whose sources use `env!("CARGO_MANIFEST_DIR")` sets `rustc_env = {"CARGO_MANIFEST_DIR": "<package path>"}`: the relative path resolves from the runfiles root. Two consequences:

- A test that refuses symbolic links, as CPE's project discovery does, reads a `copy_to_directory` of its files with `root_paths = []`, which keeps their repository paths, and is tagged `no-sandbox`, because the local sandbox links each copied file again. `CARGO_MANIFEST_DIR` then points into the copy (`cpe-server`, `cpe-web`, `access-server`).
- `figment::Jail` moves the whole process into a temporary directory while a config test runs, and a relative path read by a test on another thread fails. A test target whose crate uses `Jail` sets `env = {"RUST_TEST_THREADS": "1"}`.

Snapshots are `data` of their test target. `just snap-accept <target>` accepts changed ones; review them with `git diff`.

`rust_test_suite` names each test crate `<suite>_<file>_test`, and `insta` names a snapshot after the crate, so integration tests with snapshots are one `rust_test` per file with `crate_name` set to the file stem instead (`dsp-cli`). `rules_rust` does not set `CARGO_BIN_EXE_<name>`: a test that runs a binary of its package sets it with `rustc_env = {"CARGO_BIN_EXE_dsp": "$(rootpath :dsp)"}` and the binary in `data`, and makes the runfiles-relative path absolute before use.

Bazel runs no doctests: `rust_doc_test` cannot link against the hermetic toolchain. A doctest is an `ignore` block, and a unit test carries its assertion.

A crate with the `test-support` feature (the Deposit Area's `ports`, `store` and `web` crates) has a second library, `<name>_test_support`, built with the feature and marked `testonly`, so its fixtures cannot reach a production binary. Inside a test's graph every crate that has such a variant uses it, or two builds of one crate meet in a compile; a crate whose tests take a sibling's fixtures therefore runs them on its own variant (`rust_test(crate = ":editor_web_test_support")`).

## Stylesheets

Each application's stylesheet is a build action: `//areas/access/dpe:css`, `//areas/deposit/editor:css` and `//modules/mosaic/playground:css`, a `tailwind_css` rule (`tools/tailwind/defs.bzl`) running the Tailwind standalone CLI. The DPE and editor stylesheets are directories holding `app.<hash8>.css`, as `just css-release` and `just css-editor-release` write them; the playground's is `app.css`. Their output is byte-identical to the `just css*` recipes' at the same commit.

The entry files import `tailwindcss` with `source(none)`, so Tailwind reads only the files the target lists, and a class used in a file it does not list is silently missing from the stylesheet. Each scanned crate exports its `.rs` files as `tailwind_sources`, a glob over the whole package, tests included; a nested package exports its own. An `@source` line added to an entry file needs its files added to `srcs` of `:css` in the same edit: the dev loop's Tailwind watches the checkout and still shows the classes, the image does not. The entry files say so at their `@source` lines.

The CLI is pinned twice until the `just css*` recipes build through Bazel: in `tailwind.pins`, which the recipes and the playground's `Dockerfile` read, and in the four `http_file`s in `MODULE.bazel`. A Tailwind bump edits both, with the same version and hashes.

## Images

`//areas/access/server:image`, `//areas/deposit/server:image` and `//modules/mosaic/playground:image` are the DPE, editor and Mosaic playground images: a static musl binary, built for `//platforms:linux_x86_64_musl` through a transition, on distroless `static-debian12:nonroot`, pinned by digest in `MODULE.bazel`. Layers are built by `tar.bzl` (`tools/image/defs.bzl`): the binary, the public directory with the stylesheet, and the data directories, each entry owned by root with a fixed modification time. Environment, entrypoint, command, port and working directory are those of the matching `Dockerfile`; `HEALTHCHECK` has no equivalent in an OCI image, and both servers keep their `healthcheck` subcommand.

Each image has `:image_load`, which loads it into Docker as the tag the Scout workflow scans, and `:image_push`, which pushes to Docker Hub with the tags passed after `--` (`bazel run --config=release --stamp //areas/access/server:image_push -- --tag <tag>`). Both are `manual`, so `//...` does not build images. The image recipes add `--config=release`, the profile the published binaries are built with, and `--stamp` for the editor's footer version. Nothing publishes the Bazel images yet: the deploy workflows still build from the `Dockerfile`s.

The images are `linux/amd64` only; on an arm64 Mac, Docker runs them emulated.

## Fuzz targets

`//areas/access/dpe/server/fuzz` holds the five DPE fuzz targets as `cc_fuzz_test` (`rules_fuzzing`), after Sipi's: `fuzz_target.cc` is libFuzzer's entry point and calls `dpe_fuzz_entry`, which each target's `rust_static_library` shim in `shims/` defines. Without `--config=fuzz` the engine is `replay`, which needs no libFuzzer runtime, so `bazel test //...` builds and runs the five targets on every platform; with no corpus checked in, that run checks that they build and link. `--config=fuzz` (`.bazelrc`) switches to libFuzzer, builds with `-c opt` and instruments the Rust crates with SanitizerCoverage on stable Rust, with debug assertions and overflow checks on.

`just bazel-build-fuzz` builds `<target>_bin` for each and prints the binaries' paths under `bazel-out/` (not `bazel-bin/`, which follows whichever configuration was built last). Run a binary directly with a corpus directory, not through `bazel run`, whose sandbox discards the corpus the run grows:

```bash
just bazel-build-fuzz
<printed path of tab_validation_bin> areas/access/dpe/server/fuzz/corpus/tab_validation \
  -artifact_prefix=areas/access/dpe/server/fuzz/artifacts/ -max_total_time=60 -max_len=4096
```

`-artifact_prefix` keeps a crashing input out of the repository root: libFuzzer writes it as `crash-<sha1>` to that directory, which must exist. `corpus/` and `artifacts/` are git-ignored. The nightly `fuzz.yml` still runs the `cargo-fuzz` targets in `fuzz_targets/`, whose bodies the shims copy: a change to one is made in both until Cargo is removed.

## Version stamping

`deposit-shell`'s footer shows `DEPOSIT_VERSION`. A build with `--stamp` takes it from `tools/workspace_status.sh`, which reads the `DEPOSIT_VERSION` environment variable (`DEPOSIT_VERSION=v1.2.3 bazel build --stamp //areas/deposit/server:deposit_server`) and emits `dev` when it is unset. A build without `--stamp`, every test build included, shows `dev`, so a new commit relinks no test.

## Remote cache and execution

In CI, the `bazel-setup` action adds the remote cache and executor flags for DaSCH's NativeLink endpoint. Without the secrets (a pull request from a fork, Dependabot) the flags are empty and the build runs cold and local. So does every pull request while the endpoint is down. Measured on `ubuntu-latest` with an empty repository cache, while Bazel built the shared crates and `mosaic-tiles` only: `just bazel-clippy` took 3.5 minutes and `just bazel-test` 3 minutes, against `timeout-minutes: 60` on the `check` and `test / ubuntu / stable` jobs. The time grows as more crates move to Bazel.

## IDE setup

rust-analyzer reads `rust-project.json`, which `just rust-project` generates from the Bazel graph (git-ignored, it holds absolute paths). Re-run it after adding or moving a target. While the Cargo workspace exists, rust-analyzer finds `Cargo.toml` first and uses it unless `rust-analyzer.linkedProjects` points at `rust-project.json`.

## Personal settings

`user.bazelrc` at the repository root is read if it exists and is git-ignored, for flags such as `--disk_cache=~/.cache/bazel-disk`.
