# Security

## Why Security Scanning Matters

Software depends on a deep stack of third-party components: base OS images, system
libraries, language runtimes, and application dependencies. Vulnerabilities are
regularly discovered in these components — the [CVE database](https://www.cve.org/)
publishes thousands each year. A single unpatched dependency in a Docker image can
become an entry point for attackers in production.

Manual tracking of vulnerabilities across all dependencies is not practical.
Automated scanning integrates into the development workflow so that new
vulnerabilities are surfaced early — ideally before code reaches production.

## Container Image Scanning with Docker Scout

We use [Docker Scout](https://docs.docker.com/scout/) to scan our Docker images for
known vulnerabilities (CVEs). Scout analyzes the Software Bill of Materials (SBOM) of
each image — the full inventory of OS packages, libraries, and application
dependencies — and matches them against vulnerability databases.

### What Gets Scanned

| Image | Workflow | Trigger |
|-------|----------|---------|
| DPE (`daschswiss/dpe`) | `scout-dpe.yml` | PRs touching `modules/dpe/**` or `Cargo.lock` |
| Mosaic Playground (`daschswiss/mosaic-playground`) | `scout-mosaic-playground.yml` | PRs touching `modules/mosaic/**` or `Cargo.lock` |

### How It Works

Each Scout workflow:

1. **Builds the Docker image locally** — the image is loaded into the runner's Docker
   daemon (`load: true`) but never pushed to a registry. This means Scout scans
   exactly what would be deployed, without exposing unreviewed images.

2. **Runs a CVE analysis** — Docker Scout compares the image's SBOM against known
   vulnerability databases, filtering for **critical** and **high** severity issues.

3. **Posts a PR comment** — a summary of findings is posted directly on the pull
   request, giving developers immediate visibility without leaving their review
   workflow.

4. **Uploads a SARIF report** — results are uploaded to the
   [GitHub Security tab](https://docs.github.com/en/code-security/code-scanning/integrating-with-code-scanning/sarif-support-for-code-scanning)
   in SARIF format (Static Analysis Results Interchange Format), the industry
   standard for security tool output. This integrates with GitHub's code scanning
   alerts.

### What To Do With Results

Scout results are currently **informational** — they do not block merging. When a
scan reports vulnerabilities:

- **Critical/High in base image** — check if a newer base image version is available
  that patches the issue. For DPE (distroless), these are rare. For Mosaic
  (Debian-based), update the base image tag.
- **Critical/High in dependencies** — check if a dependency update resolves the issue.
  Run `cargo update` and re-test.
- **False positives** — some CVEs may not be exploitable in our context. Document the
  rationale if choosing to accept the risk.

### Prerequisites

- Docker Scout is enabled for the `daschswiss` Docker Hub organization
- Repository secrets `DOCKER_USER` and `DOCKER_HUB_TOKEN` (shared with publish
  workflows)
- GitHub Advanced Security or a public repository (for SARIF upload)

### Future Enhancements

- **Production comparison** — using Docker Scout's `compare` command to show only
  *new* vulnerabilities introduced by a PR (requires configuring Docker Scout
  environments on Docker Hub)
- **Main-branch scanning** — continuous monitoring of production images
- **Blocking on critical CVEs** — failing the PR check when critical vulnerabilities
  are detected
