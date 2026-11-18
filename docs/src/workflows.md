# Workflows and Conventions

## Entry Points

The first entry point of this repository is the README file, which should give anyone an indication of where to find any information they need.

For any interaction or coding-related workflow, the justfile is the primary source of truth. Run `just` without arguments to see all available commands with descriptions.

## Key Development Commands

| Command | Description |
|---------|-------------|
| `just check` | Run the gate scripts, formatting checks and clippy over every Bazel target |
| `just build` | Build every Bazel target |
| `just test` | Run every Bazel test, then the gate scripts' tests (`just test-scripts`). Bazel reruns only the tests whose inputs changed; for one package, `bazel test //<package>/...` |
| `just test-scripts` | Run the tests of every `.github/scripts` gate (`*.test.sh`); CI runs it as the `test / scripts` job |
| `just fmt` | Format all code (`maudfmt` for `maud::html!` macro contents, then the pinned nightly `rustfmt`) |
| `just run` | Run the server from the repository root, a release build with the release stylesheet |
| `just dev` | Run DPE with hot reload |
| `just dev-editor` | Run the metadata editor with hot reload (binds 4100, so it can run alongside `just dev`) |
| `just watch-mosaic-playground` | Run Mosaic playground with hot reload |
| `just snap-accept <target>` | Rewrite the changed `insta` snapshots of one test target; review with `git diff` |
| `just install-requirements` | Install all development dependencies |
| `just install-e2e-requirements` | Install Playwright browsers for E2E tests |
| `just docs-serve` | Serve documentation locally at localhost:3000 |
| `just validate-data` | Validate all data files in the default data directory |
| `just verify-checksums` | Check vendored JS against each `vendor/README.md` table (also run by `just check`) |
| `just check-shared-paths` | Fail the build if a crate under `shared/` hardcodes a path into a service module (also run by `just check`) |
| `just check-capability-deps` | Fail the build if a Deposit or Access Area role's `BUILD.bazel` uses a third-party crate its role may not, or any `BUILD.bazel` under `areas/`, `shared/`, `modules/` or `dsp-cli/` makes a target `//visibility:public`; the bans are listed at the top of `.github/scripts/check-capability-deps.sh` (ADR-0003). Edges between first-party crates are each target's `visibility` (also run by `just check`) |
| `just check-views-no-axum` | Fail the build if a file of `editor-web`'s or `identity-web`'s `src/` outside `handlers/`, `routes.rs`, `shell.rs`, `lib.rs` and `fixtures.rs` names an `axum` or `axum_*` path: views are `fn -> Markup` (also run by `just check`) |
| `just check-adr-refs` | Fail the build if a decision-record reference does not resolve to a record (also run by `just check`) |
| `just check-live-tests-ignored` | Fail the build if a dsp-cli live test is not `#[ignore]`d (also run by `just check`) |
| `just dsp-cli-run <args>` | Run the `dsp` command-line client with the given arguments |
| `just dsp-cli-test-live` | Run dsp-cli's live tests against a reachable DSP stack (never part of `just test`) |
| `just dsp-cli-stack-up` | Start the containerized DSP stack used by dsp-cli's live tests |
| `just dsp-cli-stack-fixtures` | Load fixture data into the dsp-cli stack |
| `just dsp-cli-stack-down` | Tear down the dsp-cli stack |

The Bazel recipes, with the image, fuzz and lock recipes, are in [Building with Bazel](./bazel.md#commands).

## Git, Commits, and Pull Requests

All git handling — the rebase-merge workflow, commit message schema, commit organization, and PR conventions — lives in a single file: [Git Conventions](./git-conventions.md).

## Release Workflow

Releases are automated via [Release Please](https://github.com/googleapis/release-please). On every push to `main`, Release Please reads conventional commit messages and either creates or updates a release PR. Merging the release PR creates a GitHub Release with auto-generated release notes.

## Code Review

See [Review Guidelines](./fundamentals/review-guidelines.md) for the review checklist.

## CI/CD

GitHub Actions workflows run automatically on pushes and pull requests. See [Release, Deployment and Versioning](./deployment.md) for details on the CI/CD pipelines.
