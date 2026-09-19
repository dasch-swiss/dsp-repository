# Workflows and Conventions

## Entry Points

The first entry point of this repository is the README file, which should give anyone an indication of where to find any information they need.

For any interaction or coding-related workflow, the justfile is the primary source of truth. Run `just` without arguments to see all available commands with descriptions.

## Key Development Commands

| Command | Description |
|---------|-------------|
| `just check` | Run formatting, linting and checksum checks |
| `just build` | Build all targets |
| `just test` | Run all tests |
| `just fmt` | Format all code (`maudfmt` for `maud::html!` macro contents, then `cargo +nightly fmt`) |
| `just run` | Run server (release mode) |
| `just watch` | Watch for changes and run tests |
| `just dev` | Run DPE with hot reload |
| `just dev-editor` | Run the metadata editor with hot reload (binds 4100, so it can run alongside `just dev`) |
| `just watch-mosaic-playground` | Run Mosaic playground with hot reload |
| `just install-requirements` | Install all development dependencies |
| `just install-e2e-requirements` | Install Playwright browsers for E2E tests |
| `just docs-serve` | Serve documentation locally at localhost:3000 |
| `just validate-data` | Validate all data files in the default data directory |
| `just verify-checksums` | Check vendored JS against each `vendor/README.md` table and `tailwind.pins` for completeness (also run by `just check`) |
| `just check-shared-paths` | Fail the build if a crate under `shared/` hardcodes a path into a service module (also run by `just check`) |
| `just check-adr-refs` | Fail the build if a decision-record reference does not resolve to a record (also run by `just check`) |
| `just check-live-tests-ignored` | Fail the build if a dsp-cli live test is not `#[ignore]`d (also run by `just check`) |
| `just tailwind-pins-refresh <version>` | Re-pin the Tailwind standalone CLI digests for a new version |
| `just dsp-cli-run <args>` | Run the `dsp` command-line client with the given arguments |
| `just dsp-cli-test-live` | Run dsp-cli's live tests against a reachable DSP stack (never part of `just test`) |
| `just dsp-cli-stack-up` | Start the containerized DSP stack used by dsp-cli's live tests |
| `just dsp-cli-stack-fixtures` | Load fixture data into the dsp-cli stack |
| `just dsp-cli-stack-down` | Tear down the dsp-cli stack |
| `just dsp-cli-snap-review` | Review pending insta snapshots for dsp-cli |

## Git, Commits, and Pull Requests

All git handling — the rebase-merge workflow, commit message schema, commit organization, and PR conventions — lives in a single file: [Git Conventions](./git-conventions.md).

## Release Workflow

Releases are automated via [Release Please](https://github.com/googleapis/release-please). On every push to `main`, Release Please reads conventional commit messages and either creates or updates a release PR. Merging the release PR creates a GitHub Release with auto-generated release notes.

## Code Review

See [Review Guidelines](./fundamentals/review-guidelines.md) for the review checklist.

## CI/CD

GitHub Actions workflows run automatically on pushes and pull requests. See [Release, Deployment and Versioning](./deployment.md) for details on the CI/CD pipelines.
