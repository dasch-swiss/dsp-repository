# Release, Deployment and Versioning

## CI/CD Pipelines

All CI/CD workflows are defined as GitHub Actions in `.github/workflows/`.

### Checks and Tests

Every push and pull request runs:

- **check.yml** — Formatting (`maudfmt` for `html!` macros + `cargo +nightly fmt`), linting (`clippy`), third-party artifact checksums (see [Security](./security.md)), and a dsp-cli packaging dry run (`cargo publish -p dsp-cli --dry-run`)
- **test.yml** — Runs the full test suite
- **scout-dpe.yml** / **scout-mosaic-playground.yml** — Docker image vulnerability scanning (see [Security](./security.md))
- **dsp-cli-drift.yml** — its `changes` job (deciding whether anything under `dsp-cli/**` changed) runs on every PR; see [dsp-cli drift detection](#dsp-cli-drift-detection) below

### Accessibility Testing

Defined in `a11y-dpe.yml`.

Runs on PRs and pushes to `main` that touch `modules/dpe/**` or `shared/**` — any DPE crate, style or asset, plus the shared crates DPE renders through. Builds the DPE, then runs Playwright accessibility tests with axe-core against WCAG 2.1 AA.

### Fuzz Testing

Defined in `fuzz.yml`.

Runs nightly at 02:00 UTC (and on manual dispatch). Fuzzes `tab_validation` and `query_params` targets for 10 minutes each using `cargo-fuzz` on nightly Rust. Corpus is cached between runs. On crash, the job uploads the crash input as a `fuzz-crashes-{target}` artifact (90-day retention) and fails; GitHub Issues are disabled on this repository, so no issue is filed automatically — a person files a Linear issue from the artifact. This is the same signal model the drift job's nightly `latest` run uses (see below).

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
| `build-dpe` | Compile DPE (static musl `dpe-server` binary + content-hashed Tailwind `app.css` via `just css-release`) and stage artifacts |
| `build-editor` | Compile the metadata editor (static musl `editor-server` binary + content-hashed Tailwind `app.css` via `just css-editor-release`) and stage artifacts |
| `docker-publish` | Set up Buildx, log in to Docker Hub, build and push an image |
| `docker-scout` | Run Docker Scout CVE scan and upload SARIF results |
| `commit-lint` | Run the commit gate — commitlint-rs (type allowlist and mandatory scope) plus the one-commit-per-PR cap |
| `dsp-cli-stack-test` | Start the Fuseki/knora-api stack, load fixtures, mint tokens, and run dsp-cli's live tests against it |

### Mosaic Playground

The Mosaic component library playground has two deployment paths:

#### PR Preview (Cloud Run)

Defined in `cloud-run-mosaic-pull-request.yml`.

When a pull request modifies files under `modules/mosaic/`, a preview of the Mosaic playground is automatically deployed to Google Cloud Run. The preview URL is posted as a comment on the PR and updated on each push.

- **Trigger:** PRs that touch `modules/mosaic/**` (same-repo only, not forks)
- **Service:** Ephemeral Cloud Run service per PR
- **Cleanup:** The Cloud Run service and container image are deleted when the PR is closed or merged

Authentication uses Workload Identity Federation (keyless, OIDC-based).

#### Production (Docker Hub + Jenkins)

Defined in `mosaic-docker-publish.yml`.

When changes to `modules/mosaic/` are merged to `main`, the playground image is built, pushed to Docker Hub, and a Jenkins webhook triggers the production deployment.

### DPE

#### PR Preview (Cloud Run)

Defined in `cloud-run-dpe-pull-request.yml`.

When a pull request modifies files under `modules/dpe/` or `shared/`, a preview of the DPE is automatically deployed to Google Cloud Run. Works the same way as the Mosaic preview: ephemeral service per PR, cleaned up on close/merge.

#### Continuous Deployment (Docker Hub + Jenkins)

Defined in `dpe-docker-publish.yml`.

On every push to `main`:
1. Builds the content-hashed Tailwind stylesheet (`just css-release`)
2. Builds a static musl-linked `dpe-server` binary
3. Pushes the Docker image to Docker Hub (`daschswiss/dpe:{tag}`)
4. Triggers a Jenkins webhook for DEV deployment

#### Release Publishing

Defined in `dpe-release-publish.yml`.

When a GitHub Release is published (tag starting with `v`), builds and pushes a release-tagged Docker image.

### Editor

#### PR Preview (Cloud Run)

Defined in `cloud-run-editor-pull-request.yml`.

When a pull request modifies files under `areas/deposit/editor/` or `shared/`, a preview of the metadata editor is deployed to Google Cloud Run. Works the same way as the DPE and Mosaic previews: ephemeral service per PR, cleaned up on close/merge.

#### Continuous Deployment (Docker Hub + Jenkins)

Defined in `editor-docker-publish.yml`.

On every push to `main`:
1. Builds the content-hashed Tailwind stylesheet (`just css-editor-release`)
2. Builds a static musl-linked `editor-server` binary
3. Pushes the Docker image to Docker Hub (`daschswiss/metadata-editor:{tag}`)
4. Attempts a Jenkins webhook for DEV deployment

Step 4 is marked `continue-on-error` and only warns. The editor is not yet registered as a deployable service in Jenkins — that lands with its inventory host and playbook — so the webhook rejects `Service=editor`. The image is already published by the time the trigger runs, so a rejection must not fail the workflow on `main`. Remove the guard once the Jenkins job exists, so a genuinely broken webhook is loud again.

### Release Please

Defined in `release-please.yml`.

On every push to `main`, [Release Please](https://github.com/googleapis/release-please) reads conventional commit messages and creates or updates a release PR with auto-generated changelog. Merging the release PR creates a GitHub Release.

Which prefix produces which changelog section and version bump is defined in [Git Conventions](./git-conventions.md#commit-message-schema) — that page is the human source for the type vocabulary, and this config is the machine source.

Configuration lives in [`.github/release-please/config.json`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/release-please/config.json) and [`.github/release-please/manifest.json`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/release-please/manifest.json).

#### dsp-cli

`dsp-cli` is a second release-please package, configured alongside the root `"."` package in the same `config.json`. Top-level `separate-pull-requests: true` keeps the two release PRs apart, so merging a workspace release never ships `dsp-cli`, and merging the `dsp-cli` release never ships the workspace.

Tag format differs by package: `dsp-cli` releases are tagged `dsp-cli-v<version>` (the package sets `include-component-in-tag: true`), while the root keeps the plain `v<version>` tag. The `dsp-cli` release branch is named `release-please--branches--main--components--dsp-cli`.

**Release attribution is by path, not by commit scope.** release-please assigns a commit to a package when any file it touches sits under that package's path; the root `"."` package receives every commit unless its `exclude-paths` excludes it, and a commit is excluded from a package only when *all* of its files fall under an excluded path. The root's `exclude-paths` is `[".github", "dsp-cli"]`. The consequence — accepted and documented, not worked around — is the **double-bump rule**: a `dsp-cli` commit that also touches a root file (for example, a `Cargo.lock` dependency bump) lands in both packages' release PRs. Both packages are pre-1.0 and release often, so a double bump is cheap. See [Git Conventions](./git-conventions.md#scopes) for keeping `dsp-cli` commits scoped to `dsp-cli/**` where possible.

A `Release-As: <version>` commit footer forces a deliberate version choice for a package, overriding the version release-please would otherwise compute. The first `dsp-cli` release from its new workspace home carries `Release-As: 0.3.0`: with `bump-minor-pre-major`, a `feat` alone would only produce `0.2.2`, which would not signal the move.

Publishing `dsp-cli` to crates.io is handled by [`publish-dsp-cli.yml`](https://github.com/dasch-swiss/dsp-repository/blob/main/.github/workflows/publish-dsp-cli.yml), triggered on `release: published` and guarded by `startsWith(github.event.release.tag_name, 'dsp-cli-v')` so it never fires for a root release. It publishes via **trusted publishing** (`rust-lang/crates-io-auth-action@v1` with `permissions: id-token: write`) rather than a long-lived `CARGO_REGISTRY_TOKEN` secret, and runs in the `crates-io` GitHub environment as defence in depth. Trusted publishing requires a one-time configuration on crates.io, done by an existing crate owner, naming this repository, the workflow file, and the `crates-io` environment; authentication fails until that configuration exists.

### Documentation (GitHub Pages)

Defined in `gh-pages.yml`. The mdBook documentation is built and deployed to GitHub Pages on pushes to `main`.

### Claude Code

Defined in `claude.yml`.

Responds to `@claude` mentions in PR comments and issue comments. Supports code review (`@claude review`) and general assistance. Runs with limited permissions (contents: read, pull-requests: write).
