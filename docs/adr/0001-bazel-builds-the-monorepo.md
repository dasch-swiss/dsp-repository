---
status: accepted
date: 2026-09-16
---

# Bazel builds the monorepo

The repository is built with Bazel instead of a plain Cargo workspace, so that the boundaries between the areas (ADR-0002) and the capabilities inside them (ADR-0003), and the rule that the shared `platform-*` / `mosaic-*` crates import no service code, are enforced by per-target `visibility` rather than by convention: a crate can depend only on what its target declares and is allowed to see, and a forbidden edge fails the build instead of waiting for a reviewer. Bazel also gives hermetic, cached, incremental builds, and one dependency graph for the non-Rust steps — the standalone Tailwind CLI, the container images, and the C++ that Vitrinli (sipi under its new name, maintained separately today) brings when it moves into this monorepo. Bazel was chosen over Cargo for these reasons, and the same ruleset as DaSCH's other Rust codebases was chosen so that Vitrinli can arrive without a second build system (confirmed 2026-09-16).

The ruleset the migration introduces — none of these files exists yet; until they do, the Cargo workspace described in `docs/src/repo_structure.md` is what builds:

- `rules_rust` with `crate_universe` in `from_specs` mode: third-party crates are declared in `MODULE.bazel`, the root `Cargo.toml` and `Cargo.lock` are removed, and Bazel does all building. Load-bearing dependency notes (today the `serde_json` `preserve_order` requirement and the `rusqlite` 0.38 pin) move beside the crate specs.
- A hermetic LLVM CC toolchain, never the host's autodetected C compiler: the production target is Linux, developers build on macOS, and an autodetected compiler cannot resolve Apple-only link flags (`-liconv`) inside the sandbox; the hermetic toolchain fetches the Apple SDK on macOS and ships glibc and musl sysroots for Linux, so one configuration links locally and cross-compiles to a real Linux binary. Compiled C is therefore acceptable as a dependency, because it is compiled hermetically.
- `MODULE.bazel.lock` is committed and CI runs with `--lockfile_mode=error`, so a dependency change that did not update the lock fails.
- Bazel and Rust versions are pinned in `.bazelversion` and `MODULE.bazel`; rust-analyzer is set up through `gen_rust_project` (`rust-project.json` gitignored).

## Considered Options

- **Bazel (chosen)** — the dependency graph and the visibility boundaries are enforced by the build; hermetic and cached builds; one ruleset for every DaSCH Rust codebase and for Vitrinli's C++. Cost: crates.io dependencies are declared in `MODULE.bazel` instead of `Cargo.toml`, and IDE setup needs the Bazel integration.
- **Plain Cargo workspace (today)** — zero-friction Rust tooling. Boundaries hold by convention: nothing stops `editor-*` from depending on `dpe-*`, and the one mechanical check, `.github/scripts/check-platform-paths.sh`, covers only paths from a `platform-*` crate into a service. Nothing beyond Rust is covered.
- **Bazel with `crate_universe` `from_cargo` (a root `Cargo.toml` and committed `Cargo.lock` as the dependency source of truth)** — keeps Cargo tooling working alongside Bazel, but leaves two build descriptions to keep in sync and differs from the ruleset Vitrinli arrives with. Rejected.

## Consequences

- Every crate becomes a Bazel target; cross-crate dependencies are visible and reviewable in `BUILD.bazel` files.
- Everything that today keys on the Cargo workspace or on `modules/…` paths — `just` recipes, `bacon.toml` watch lists, the Dockerfiles, the CI workflows, `check-platform-paths.sh` — is replaced or re-pointed in the migration.
- The migration lands together with, or ahead of, the directory move of ADR-0002, which relies on it for enforcement.
- Nothing in this repository's build may assume every target is Rust.

Enforced by: none until the migration lands (docs-only); afterwards CI building with Bazel under `--lockfile_mode=error` (static-analysis).
