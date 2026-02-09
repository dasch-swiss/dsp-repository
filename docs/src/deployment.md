# Release, Deployment and Versioning

## CI/CD Pipelines

All CI/CD workflows are defined as GitHub Actions in `.github/workflows/`.

### Checks and Tests

Every push and pull request runs:

- **check.yml** — Formatting (`rustfmt`) and linting (`clippy`)
- **test.yml** — Runs the full test suite

### Mosaic Demo

The Mosaic component library demo has two deployment paths:

#### PR Preview (Cloud Run)

Defined in `cloud-run-pull-request.yml`.

When a pull request modifies files under `modules/mosaic/`, a preview of the
Mosaic demo is automatically deployed to Google Cloud Run. The preview URL is
posted as a comment on the PR and updated on each push.

- **Trigger:** PRs that touch `modules/mosaic/**` (same-repo only, not forks)
- **Service:** Ephemeral Cloud Run service per PR
- **Cleanup:** The Cloud Run service and container image are deleted when the PR
  is closed or merged

Required secrets: `GCP_SERVICE_ACCOUNT_KEY`, `GCP_PROJECT_ID`, `GCP_REGION`,
`GCP_ARTIFACT_REGISTRY`.

#### Production (Docker Hub + Jenkins)

Defined in `docker-publish.yml`.

When changes to `modules/mosaic/` are merged to `main`, the demo image is built,
pushed to Docker Hub, and a Jenkins webhook triggers the production deployment.

### Documentation (GitHub Pages)

Defined in `gh-pages.yml`. The mdBook documentation is built and deployed to
GitHub Pages on pushes to `main`.
