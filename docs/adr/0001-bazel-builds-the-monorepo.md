# Bazel builds the monorepo

The repository is built with Bazel instead of a plain Cargo workspace. The reason is enforcement: the
boundaries between the areas (ADR-0002) and the capabilities inside them (ADR-0003), and the rule that the
`shared-*` and `mosaic-*` crates import no area code, are held by per-target `visibility` rather than by
convention. A crate can depend only on what its target declares and is allowed to see, and a forbidden edge
fails the build instead of waiting for a reviewer. Bazel also gives hermetic, cached, incremental builds and one
dependency graph for the steps that are not Rust: the standalone Tailwind CLI, the container images, and the
C++ that Vitrinli, sipi under its new name, brings into this monorepo (ADR-0002). The ruleset is the same as in
DaSCH's other Rust codebases, so that Vitrinli arrives without a second build system.

The ruleset:

- `rules_rust` with `crate_universe` in `from_specs` mode. Third-party crates are declared in `MODULE.bazel`,
  there is no root `Cargo.toml` or `Cargo.lock`, and Bazel does all building. Load-bearing dependency notes (the
  `serde_json` features `preserve_order` and `float_roundtrip`, the `rusqlite` 0.38 pin) sit beside the crate
  specs.
- A hermetic LLVM CC toolchain, never the host's autodetected C compiler. The production target is Linux and
  developers build on macOS; an autodetected compiler cannot resolve Apple-only link flags (`-liconv`) inside
  the sandbox. The hermetic toolchain fetches the Apple SDK on macOS and ships glibc and musl sysroots for
  Linux, so one configuration links locally and cross-compiles to a real Linux binary. Compiled C is therefore
  acceptable as a dependency, because it is compiled hermetically.
- `MODULE.bazel.lock` is committed, and CI runs with `--lockfile_mode=error`, so a dependency change that did not
  update the lock fails.
- Bazel and Rust versions are pinned in `.bazelversion` and `MODULE.bazel`; rust-analyzer is set up through
  `gen_rust_project` (`rust-project.json` gitignored).

## Considered Options

- **Bazel (chosen)** — the dependency graph and the visibility boundaries are enforced by the build; one ruleset
  for every DaSCH Rust codebase and for Vitrinli's C++. Cost: crates.io dependencies are declared in
  `MODULE.bazel` instead of `Cargo.toml`, and IDE setup needs the Bazel integration.
- **Plain Cargo workspace** — zero-friction Rust tooling. Boundaries hold by convention and by scripts over
  `cargo metadata` and paths (`check-shared-paths.sh`, `check-capability-deps.sh`,
  `check-composition-root-deps.sh`, `check-cpe-seam.sh`), each written and maintained separately, and nothing
  beyond Rust is covered.
- **Bazel with `crate_universe` `from_cargo`** (a root `Cargo.toml` and committed `Cargo.lock` as the dependency
  source of truth) — keeps Cargo tooling working alongside Bazel, but leaves two build descriptions to keep in
  sync and differs from the ruleset Vitrinli uses. Rejected.

## Consequences

- Every crate is a Bazel target; cross-crate dependencies are visible and reviewable in `BUILD.bazel` files.
- The `just` recipes, the `bacon.toml` watch lists, the Dockerfiles, the CI workflows and the gate scripts key
  on Bazel targets, not on the Cargo workspace.
- Nothing in this repository's build may assume every target is Rust.
- dsp-cli is the one crate here that publishes to crates.io, so `dsp-cli/Cargo.toml` and a `dsp-cli/Cargo.lock`
  stay as its publishing manifest (ADR-0002).
- dsp-cli's live tests, which need a running DSP stack, are a `manual`, `no-remote-exec` Bazel test target and
  never part of the default `bazel test //...`. The drift stack (a pinned dsp-api and Fuseki running those tests
  in CI) stays a local-execution job for the same reason: Bazel's remote execution provides no live network
  stack.

Not yet in code: there is no `MODULE.bazel`, `.bazelversion` or `BUILD.bazel`; the root `Cargo.toml` and
`Cargo.lock` build the workspace, and no boundary is held by Bazel `visibility` (not scheduled).

Enforced by: CI building with Bazel under `--lockfile_mode=error` (static-analysis); `visibility` for the
boundaries of ADR-0002 and ADR-0003 (structure).
