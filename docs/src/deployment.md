# Release, Deployment and Versioning

## CI/CD Pipelines

All CI/CD workflows are defined as GitHub Actions in `.github/workflows/`.

### Checks and Tests

Every push and pull request runs:

- **check.yml** — `just check` with Bazel: the gate scripts, vendored-JS checksums among them (see [Security](./security.md)), formatting (`maudfmt` for `html!` macros and the pinned nightly `rustfmt`), clippy over every Bazel target; a dsp-cli packaging dry run and test run against its own manifest (`cargo publish --dry-run`, `cargo test`); the E2E suites' Biome lint; the mdBook build
- **test.yml** — `just test` (every Bazel test) and `just image-smoke` (the three images built, started and answering), and the tests of the `.github/scripts` gates (`just test-scripts`)
- **scout-dpe.yml** / **scout-editor.yml** / **scout-mosaic-playground.yml** — Docker image vulnerability scanning of the Bazel-built images (see [Security](./security.md))
- **dsp-cli-drift.yml** — its `changes` job (deciding whether anything under `dsp-cli/**` changed) runs on every PR; see [dsp-cli drift detection](#dsp-cli-drift-detection) below

### Accessibility Testing

Defined in `a11y-dpe.yml`.

Runs on PRs and pushes to `main` that touch `areas/access/dpe/**`, `areas/access/server/**`, `areas/access/cpe/**`, `areas/access/sync/**`, `areas/access/media/**`, `shared/**` or `modules/mosaic/tiles/**` (everything compiled into the binary the suite starts), or the Bazel build's own files. The suite tests DPE's pages. `just test-a11y-dpe` builds the release server and stylesheet with Bazel, then runs Playwright accessibility tests with axe-core against WCAG 2.1 AA. `a11y-editor.yml` does the same for the editor, with its whole suite (`just test-e2e-editor`).

### Fuzz Testing

Defined in `fuzz.yml`.

Runs nightly at 02:00 UTC (and on manual dispatch). Builds the libFuzzer binaries with Bazel (`just build-fuzz`) and fuzzes the `tab_validation` and `query_params` targets for 10 minutes each. Corpus is cached between runs. On crash, the job uploads the crash input as a `fuzz-crashes-{target}` artifact (90-day retention) and fails; GitHub Issues are disabled on this repository, so no issue is filed automatically — a person files a Linear issue from the artifact. This is the same signal model the drift job's nightly `latest` run uses (see below).

### dsp-cli Drift Detection

Defined in [`dsp-cli-drift.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/workflows/dsp-cli-drift.yml).

Runs dsp-cli's live test suite (dsp-cli/ADR-0009 layer 5) against a pinned, containerized dsp-api
stack. An OpenAPI diff of DSP-API's endpoint surface cannot see a renamed or re-shaped key inside a
JSON-LD response body, but dsp-cli deserializes exactly those bodies — running the real client
against a real server is what catches that class of drift.

Four jobs:

- **`changes`** — `pull_request` only. Decides whether anything under `dsp-cli/**` changed.
- **`pinned`** — runs the live suite against the pinned stack when `changes` says something did.
- **`gate`** — an always-runs aggregator, green when `pinned` either passed or was legitimately
  skipped. **`gate`, not `pinned`, is the required check**: GitHub treats a job that never ran as
  pending rather than passing, so a required check that is conditionally skipped would block every
  PR that doesn't touch `dsp-cli/**`. (Configuring `gate` as a required check in repository settings
  is a follow-up action, not yet done.)
- **`latest`** — nightly and `workflow_dispatch`, same steps against the `latest` images instead of
  the pin. `workflow_dispatch` exists because scheduled runs are delayed under load and GitHub
  disables a schedule after 60 days of repository inactivity, so a manual trigger is the escape
  hatch. It still loads the fixtures and ontologies checked out at the **pinned** `API` tag from
  `stack.env` — `latest` has no corresponding dsp-api ref to check out. So a red nightly can mean
  either real dsp-api drift, or that the pinned fixtures no longer load into a newer API. A failure
  in the "Load fixtures" step is the latter; a failure in the test step is the former.

#### The stack

Lives in [`dsp-cli/ci/stack/`](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/ci/stack/): a `docker-compose.yml` with two services, `db` (Apache Jena Fuseki) and `api`
(knora-api) — no sipi, no ingest — plus `load-fixtures.sh`, `token.sh`, `wait-for-api.sh`, and a
`README.md`. `stack.env` holds two independent pins, `API` and `DB` — and **both are this
repository's own choice, not a dsp-api release signal**; `API` sets the `knora-api` image tag and
also the `dasch-swiss/dsp-api` tag `load-fixtures.sh` checks the fixtures and ontologies out from,
while `DB` sets only the Fuseki image. Bumping either means editing `stack.env`, re-running the
suite locally, and committing the result.

Run the stack locally with `just dsp-cli-stack-up`, `just dsp-cli-stack-fixtures`, and
`just dsp-cli-stack-down`.

#### Signal model

Same as fuzz testing above: no issue is filed automatically (GitHub Issues are disabled on this
repository). When the nightly `latest` run goes red, a person reads the job summary — it carries the
failing test names and the dsp-api image digest — and files a Linear issue from it.

### Reusable Actions

Common CI steps are extracted into composite actions in `.github/actions/`:

| Action | Purpose |
|--------|---------|
| `bazel-setup` | Bazelisk, `just`, the Bazel repository cache and the remote cache and executor flags (`bazel-rbe`); see [Building with Bazel](./bazel.md#remote-cache-and-execution) |
| `build-dpe` | Build the DPE image with Bazel and load it into Docker as `local-scan/dpe:latest`, for the Scout scan |
| `build-editor` | Build the editor image with Bazel, stamped with the caller's `DEPOSIT_VERSION`, and load it into Docker as `local-scan/editor:latest` |
| `docker-publish` | Build an image with Bazel and push it with its `:image_push` target under the caller's tag, to Docker Hub or the repository the caller names; takes no result from the remote cache unless the caller is a pull request's own preview |
| `docker-scout` | Run Docker Scout CVE scan and upload SARIF results |
| `commit-lint` | Run the commit gate — commitlint-rs (type allowlist and mandatory scope) plus the 100-commit limit and no merge commits |
| `dsp-cli-stack-test` | Start the Fuseki/knora-api stack, load fixtures, mint tokens, and run dsp-cli's live tests against it |

Every image is a Bazel target (`//areas/access/server:image`, `//areas/deposit/server:image`, `//modules/mosaic/playground:image`; see [Building with Bazel](./bazel.md#images)): a static musl binary, the public directory with the stylesheet, and the data directories on distroless `static`. The preview, Scout and Mosaic publish workflows also run when the Bazel build changes: `MODULE.bazel`, `MODULE.bazel.lock`, `Cargo.Bazel.lock`, `.bazelrc`, `.bazelversion`, `platforms/`, `bazel/`, `tools/` and the composite actions they use.

### Mosaic Playground

The Mosaic component library playground has two deployment paths:

#### PR Preview (Cloud Run)

Defined in `cloud-run-mosaic-pull-request.yml`.

When a pull request modifies files under `modules/mosaic/`, a preview of the Mosaic playground is automatically deployed to Google Cloud Run. The preview URL is posted as a comment on the PR and updated on each push.

- **Trigger:** PRs that touch `modules/mosaic/**` or the Bazel build (same-repo only, not forks)
- **Service:** Ephemeral Cloud Run service per PR
- **Cleanup:** The Cloud Run service and container image are deleted when the PR is closed or merged

Authentication uses Workload Identity Federation (keyless, OIDC-based).

#### Production (Docker Hub + Jenkins)

Defined in `mosaic-docker-publish.yml`.

When changes to `modules/mosaic/` or the Bazel build are merged to `main`, Bazel builds the playground image and pushes it to Docker Hub, and a Jenkins webhook triggers the production deployment.

### DPE

#### PR Preview (Cloud Run)

Defined in `cloud-run-dpe-pull-request.yml`.

When a pull request modifies files under `areas/access/dpe/`, `areas/access/server/`, `areas/access/cpe/`, `areas/access/sync/`, `areas/access/media/`, `shared/`, `modules/mosaic/tiles/` or the Bazel build, a preview of the DPE is automatically deployed to Google Cloud Run. Works the same way as the Mosaic preview: ephemeral service per PR, cleaned up on close/merge.

The preview is the whole image, so it serves CPE under `/cpe` too, from the data the image carries; a change to a presentation's code or files gets a preview of its own. It runs without CPU throttling, because CPE loads its projects after the port is bound.

#### Continuous Deployment (Docker Hub + Jenkins)

Defined in `dpe-docker-publish.yml`.

On every push to `main`:
1. Builds the image with Bazel (`//areas/access/server:image_push`), taking no result from the remote cache
2. Pushes it to Docker Hub (`daschswiss/dpe:{tag}`)
3. Triggers a Jenkins webhook for STAGE deployment

#### Release Publishing

Defined in `dpe-release-publish.yml`.

When a GitHub Release is published (tag starting with `v`), builds and pushes a release-tagged Docker image and triggers the PROD deployment.

### Editor

#### PR Preview (Cloud Run)

Defined in `cloud-run-editor-pull-request.yml`.

When a pull request modifies files under `areas/deposit/`, `shared/`, `modules/mosaic/tiles/`, DPE's data (`areas/access/dpe/server/data/`, which the image carries) or the Bazel build, a preview of the metadata editor is deployed to Google Cloud Run. Works the same way as the DPE and Mosaic previews: ephemeral service per PR, cleaned up on close/merge.

#### Continuous Deployment (Docker Hub + Jenkins)

Defined in `editor-docker-publish.yml`.

On every push to `main`:
1. Builds the image with Bazel (`//areas/deposit/server:image_push`), stamped with the tag as the footer version, taking no result from the remote cache
2. Pushes it to Docker Hub (`daschswiss/metadata-editor:{tag}`)
3. Triggers a Jenkins webhook (`Service=deposit`) for STAGE deployment, `deposit.stage.dasch.swiss`

The editor deploys as its own `deposit` stack in ops-deploy. That stack has no dev host; as with DPE, stage follows `main`.

#### Release Publishing

Defined in `editor-release-publish.yml`.

When a GitHub Release is published (tag starting with `v`), builds and pushes a release-tagged Docker image and triggers the PROD deployment, `deposit.dasch.swiss`. The version is the repository-wide release-please version, so every `v*` release ships DPE and the editor to prod together.

### Release Please

Defined in `release-please.yml`.

On every push to `main`, [Release Please](https://github.com/googleapis/release-please) reads conventional commit messages and creates or updates a release PR with auto-generated changelog. Merging the release PR creates a GitHub Release.

Which prefix produces which changelog section and version bump is defined in [Git Conventions](./git-conventions.md#commit-message-schema) — that page is the human source for the type vocabulary, and this config is the machine source.

Configuration lives in [`.github/release-please/config.json`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/release-please/config.json) and [`.github/release-please/manifest.json`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/release-please/manifest.json). The root package is of release type `simple`, which writes its version to `version.txt` at the repository root; the dev recipes read it as the services' `service.version`.

#### dsp-cli

`dsp-cli` is a second release-please package, configured alongside the root `"."` package in the same `config.json`. Top-level `separate-pull-requests: true` keeps the two release PRs apart, so merging a root release never ships `dsp-cli`, and merging the `dsp-cli` release never ships the root package.

Tag format differs by package: `dsp-cli` releases are tagged `dsp-cli-v<version>` (the package sets `include-component-in-tag: true`), while the root keeps the plain `v<version>` tag. The `dsp-cli` release branch is named `release-please--branches--main--components--dsp-cli`.

**Release attribution is by path, not by commit scope.** release-please assigns a commit to a package when any file it touches sits under that package's path; the root `"."` package receives every commit unless its `exclude-paths` excludes it, and a commit is excluded from a package only when *all* of its files fall under an excluded path. The root's `exclude-paths` is `[".github", "dsp-cli"]`. The consequence — accepted and documented, not worked around — is the **double-bump rule**: a `dsp-cli` commit that also touches a root file (for example, a `MODULE.bazel` dependency bump) lands in both packages' release PRs. Both packages are pre-1.0 and release often, so a double bump is cheap. See [Git Conventions](./git-conventions.md#scopes) for keeping `dsp-cli` commits scoped to `dsp-cli/**` where possible.

A `Release-As: <version>` commit footer forces a deliberate version choice for a package, overriding the version release-please would otherwise compute. The first `dsp-cli` release from its new workspace home carries `Release-As: 0.3.0`: with `bump-minor-pre-major`, a `feat` alone would only produce `0.2.2`, which would not signal the move.

Publishing `dsp-cli` to crates.io is handled by [`publish-dsp-cli.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/workflows/publish-dsp-cli.yml), triggered on `release: published` and guarded by `startsWith(github.event.release.tag_name, 'dsp-cli-v')` so it never fires for a root release. It publishes via **trusted publishing** (`rust-lang/crates-io-auth-action@v1` with `permissions: id-token: write`) rather than a long-lived `CARGO_REGISTRY_TOKEN` secret, and runs in the `crates-io` GitHub environment as defence in depth. Trusted publishing requires a one-time configuration on crates.io, done by an existing crate owner, naming this repository, the workflow file, and the `crates-io` environment; authentication fails until that configuration exists.

### Documentation (GitHub Pages)

Defined in `gh-pages.yml`. The mdBook documentation is built and deployed to GitHub Pages on pushes to `main`.

### Claude Code

Defined in `claude.yml`.

Responds to `@claude` mentions in PR comments and issue comments. Supports code review (`@claude review`) and general assistance. Runs with limited permissions (contents: read, pull-requests: write).
