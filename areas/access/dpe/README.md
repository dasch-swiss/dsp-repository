# DPE -- Discovery and Presentation Environment

Server-side rendered web application built with [Maud](https://maud.lambda.xyz/) and [Axum](https://github.com/tokio-rs/axum). Pages render to HTML on the server; interactive behavior (tab switching, live search) is driven by [Datastar](https://data-star.dev/) SSE fragments -- no client-side WASM.

## Project Structure

```
dpe/
├── core/             # DPE's view model, caches and repositories (crate: dpe-core)
├── api-oai/          # OAI-PMH 2.0 API (crate: dpe-api-oai)
├── web/              # Maud pages and components, `fn -> Markup` (crate: dpe-web)
├── server/           # DPE's router, config and validate logic, a library (crate: dpe-server)
├── web-e2e-tests/    # Playwright E2E tests
├── public/           # Static assets (served by ServeDir, includes compiled app.<hash>.css)
└── style/            # Tailwind v4 entry (main.css)
```

The binary, `access-server`, and its image live outside this directory, at `areas/access/server/`
(the Access Area's composition root; see its `CLAUDE.md`).

## Prerequisites

- [Bazelisk](https://github.com/bazelbuild/bazelisk), as `bazel`: it builds the server, the stylesheet and the image (`docs/src/bazel.md`)
- [just](https://github.com/casey/just) command runner
- [bacon](https://dystroy.org/bacon/) for the dev loop

`just install-requirements` or `nix develop` provides them. Bazel fetches the pinned Tailwind CLI; CSS needs no Node or pnpm.

## Running the Development Server

```bash
just dev
```

Runs Tailwind in `--watch` mode alongside `bacon serve`, which rebuilds and restarts `access-server` on change. The server listens at `http://127.0.0.1:4000`.

## Building for Production

```bash
just build-docker-dpe   # the published image, loaded into Docker as local-scan/dpe:latest
just run                # a release build with the release stylesheet, run from the repository root
just css-release        # content-hashed app.<hash>.css into public/assets/
```

The image is what is deployed (`docs/src/dpe/operations.md`).

## Testing

### Unit and Integration Tests

```bash
just test
```

### E2E Tests

E2E tests use [Playwright](https://playwright.dev) and live in `web-e2e-tests/`.

```bash
just install-e2e-requirements   # once
just test-a11y-dpe              # builds the release server and stylesheet, runs the accessibility spec
```

The Playwright config reads the server's path from `ACCESS_SERVER_BIN`, which the recipe sets.

A web-based test report is available via `npx playwright show-report` from the `web-e2e-tests/` directory.

## Docker Deployment

### Building the Docker Image

```bash
just build-docker-dpe
```

### Running with Docker

```bash
just run-docker-dpe
```

The application will be available at `http://localhost:8080`.

### Environment Variables

See `docs/src/dpe/operations.md` for the full list of environment variables, CLI commands, and operations details.

## Remote Deployment Without Toolchain

Deploy the image (`just build-docker-dpe`, or the tag CI publishes): it holds the binary, the `public/` directory with the content-hashed stylesheet and the data directories, and sets the environment variables `docs/src/dpe/operations.md` lists.
