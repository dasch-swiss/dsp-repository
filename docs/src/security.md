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
| DPE (`daschswiss/dpe`) | `scout-dpe.yml` | PRs touching `modules/dpe/**`, `modules/platform/**` or `Cargo.lock` |
| Mosaic Playground (`daschswiss/mosaic-playground`) | `scout-mosaic-playground.yml` | PRs touching `modules/mosaic/**` or `Cargo.lock` |
| Editor (`daschswiss/metadata-editor`) | `scout-editor.yml` | PRs touching `modules/editor/**`, `modules/platform/**` or `Cargo.lock` |

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

## Third-Party Artifact Integrity

Scanning answers "does this dependency carry a known CVE". It does not answer "are these the bytes we reviewed". Two kinds of third-party artifact in this repo are shipped or executed without passing through Cargo, so nothing else checks them. `just check` enforces both, which is what the CI `check` job runs.

### Vendored JavaScript

`modules/dpe/public/vendor/` and `modules/editor/public/vendor/` hold third-party JavaScript served straight to browsers. Each directory's `README.md` records a SHA-256 per file. Those digests are now recomputed rather than merely asserted, so the table is a check instead of a claim.

Verification fails when:

- a vendored file's bytes no longer match the digest its table records;
- a table row names a file that is not in the directory;
- the directory commits a file that no table row names;
- a row's digest is not 64 lowercase hex characters. A truncated or mistyped digest is reported, never skipped: a silently skipped row is an unverified file, which is the failure the gate exists to prevent.

Untracked files in a vendor directory are ignored, so local scratch does not fail anyone's build.

To change a vendored file, follow the update process in that directory's own `README.md`: replace the file, then put `shasum -a 256 <file>` into the table.

### The Tailwind standalone CLI

The CSS build runs a Tailwind v4 standalone binary fetched from GitHub Releases, from two places: `_tailwind-bin` in the `justfile`, which sits behind every `just css*` recipe, the `build-dpe` and `build-editor` actions and the `a11y-dpe` workflow; and `modules/mosaic/playground/Dockerfile`. Unverified, a replaced release asset would run arbitrary code in every developer checkout and every CI job.

`tailwind.pins` records the version and the expected SHA-256 of each release asset. Both download sites verify against it and fail closed. The justfile checks on every resolve rather than only after a download, so a binary cached by an earlier build is verified too; hashing it costs about 0.05s under coreutils and 0.3s under perl `shasum`.

Bump the pinned version with:

```sh
just tailwind-pins-refresh 4.1.19
```

This takes the digests from the `sha256sums.txt` published alongside that release and rewrites `tailwind.pins`. That file is read once, here, and never at build time: whoever can replace a release asset can replace its checksum file too, so verifying one against the other would establish nothing.

Be precise about what the pin buys, because it is easy to overstate. Nobody can authenticate a 64-character digest by reading it, so review of a bump commit checks the version and the asset names, not the bytes. What the pin gives is a fixed reference point and an audit trail: every later download has to match what was pinned, so tampering after the bump (a replaced asset, a poisoned CDN cache) fails closed, and an unexpected `tailwind.pins` change shows up in a diff that should not have contained one. A release already compromised at the moment of the bump would be pinned as it stands. Upstream ships no signature, provenance or SBOM that would close that last gap, so this is the strongest check available here.

`tailwind.pins` is also the only place the Tailwind version is written. The justfile and the mosaic Dockerfile both read it, so neither can drift onto a version the other has not seen.
