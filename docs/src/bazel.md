# Building with Bazel

Bazel builds and tests the crates that have a `BUILD.bazel` file, beside the Cargo workspace (ADR-0001). Until Cargo is removed, both builds are gated on every pull request: `check` runs `just bazel-clippy` after `just check`, and `test / ubuntu / stable` runs `just bazel-test` after the Cargo tests.

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

## Remote cache and execution

In CI, the `bazel-setup` action adds the remote cache and executor flags for DaSCH's NativeLink endpoint. Without the secrets (a pull request from a fork, Dependabot) the flags are empty and the build runs cold and local. So does every pull request while the endpoint is down. Measured on `ubuntu-latest` with an empty repository cache, while Bazel built the shared crates and `mosaic-tiles` only: `just bazel-clippy` took 3.5 minutes and `just bazel-test` 3 minutes, against `timeout-minutes: 60` on the `check` and `test / ubuntu / stable` jobs. The time grows as more crates move to Bazel.

## IDE setup

rust-analyzer reads `rust-project.json`, which `just rust-project` generates from the Bazel graph (git-ignored, it holds absolute paths). Re-run it after adding or moving a target. While the Cargo workspace exists, rust-analyzer finds `Cargo.toml` first and uses it unless `rust-analyzer.linkedProjects` points at `rust-project.json`.

## Personal settings

`user.bazelrc` at the repository root is read if it exists and is git-ignored, for flags such as `--disk_cache=~/.cache/bazel-disk`.
