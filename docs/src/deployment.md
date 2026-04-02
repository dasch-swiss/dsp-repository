# Release, Deployment and Versioning

## CI/CD Pipelines

All CI/CD workflows are defined as GitHub Actions in `.github/workflows/`.

### Checks and Tests

Every push and pull request runs:

- **check.yml** — Formatting (`rustfmt`, `leptosfmt`) and linting (`clippy`)
- **test.yml** — Runs the full test suite

### Accessibility Testing

Defined in `a11y-dpe.yml`.

Runs on PRs and pushes to `main` that touch DPE UI code (`modules/dpe/web/`, `modules/dpe/style/`, `modules/dpe/public/`). Builds the DPE, then runs Playwright accessibility tests with axe-core against WCAG 2.1 AA.

### Fuzz Testing

Defined in `fuzz.yml`.

Runs nightly at 02:00 UTC (and on manual dispatch). Fuzzes `tab_validation` and `query_params` targets for 10 minutes each using `cargo-fuzz` on nightly Rust. Corpus is cached between runs. On crash, automatically creates a GitHub issue with reproduction instructions.

### Mosaic Playground

The Mosaic component library playground has two deployment paths:

#### PR Preview (Cloud Run)

Defined in `cloud-run-pull-request.yml`.

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

When a pull request modifies files under `modules/dpe/`, a preview of the DPE is automatically deployed to Google Cloud Run. Works the same way as the Mosaic preview: ephemeral service per PR, cleaned up on close/merge.

#### Continuous Deployment (Docker Hub + Jenkins)

Defined in `dpe-docker-publish.yml`.

On every push to `main`:
1. Builds site assets with `cargo-leptos`
2. Builds a static musl-linked binary
3. Pushes the Docker image to Docker Hub (`daschswiss/dpe:{tag}`)
4. Triggers a Jenkins webhook for DEV deployment

#### Release Publishing

Defined in `dpe-release-publish.yml`.

When a GitHub Release is published (tag starting with `v`), builds and pushes a release-tagged Docker image.

### Release Please

Defined in `release-please.yml`.

On every push to `main`, [Release Please](https://github.com/googleapis/release-please) reads conventional commit messages and creates or updates a release PR with auto-generated changelog. Merging the release PR creates a GitHub Release.

Configuration lives in `.github/release-please/config.json` and `.github/release-please/manifest.json`.

### Documentation (GitHub Pages)

Defined in `gh-pages.yml`. The mdBook documentation is built and deployed to GitHub Pages on pushes to `main`.

### Claude Code

Defined in `claude.yml`.

Responds to `@claude` mentions in PR comments and issue comments. Supports code review (`@claude review`) and general assistance. Runs with limited permissions (contents: read, pull-requests: write).
