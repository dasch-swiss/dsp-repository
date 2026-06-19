# DPE -- Discovery and Presentation Environment

Server-side rendered web application built with [Maud](https://maud.lambda.xyz/) and [Axum](https://github.com/tokio-rs/axum). Pages render to HTML on the server; interactive behavior (tab switching, live search) is driven by [Datastar](https://data-star.dev/) SSE fragments -- no client-side WASM.

## Project Structure

```
dpe/
├── core/             # Pure domain types and data loading (crate: dpe-core)
├── api-oai/          # OAI-PMH 2.0 API (crate: dpe-api-oai)
├── web/              # Maud pages and components, `fn -> Markup` (crate: dpe-web)
├── server/           # Server binary, routing, head, fragment handlers (crate: dpe-server)
├── telemetry/        # Browser telemetry types and validation (crate: dpe-telemetry)
├── web-e2e-tests/    # Playwright E2E tests
├── public/           # Static assets (served by ServeDir, includes compiled app.<hash>.css)
└── style/            # Tailwind v4 entry (main.css)
```

## Prerequisites

- Rust toolchain (managed via `rustup`)
- [just](https://github.com/casey/just) command runner
- [bacon](https://dystroy.org/bacon/) for the dev loop (`cargo install bacon`)

The Tailwind CLI is fetched automatically by the `just css*` recipes; CSS needs no Node or pnpm.

## Running the Development Server

```bash
just dev
```

Runs Tailwind in `--watch` mode alongside `bacon serve`, which rebuilds and restarts `dpe-server` on change. The server listens at `http://127.0.0.1:4000`.

## Building for Production

```bash
cargo build --release --bin dpe-server   # static binary
just css-release                          # content-hashed app.<hash>.css into public/assets/
```

Output:
- Server binary: `target/release/dpe-server`
- Compiled stylesheet: `modules/dpe/public/assets/app.<hash>.css`

## Testing

### Unit and Integration Tests

```bash
just test
```

### E2E Tests

E2E tests use [Playwright](https://playwright.dev) and live in `web-e2e-tests/`.

```bash
cd modules/dpe/web-e2e-tests
pnpm install
npx playwright test
```

A web-based test report is available via `npx playwright show-report` from the `web-e2e-tests/` directory.

## Docker Deployment

### Building the Docker Image

```bash
docker build -t dpe .
```

### Running with Docker

```bash
docker run -p 8080:8080 dpe
```

The application will be available at `http://localhost:8080`.

### Environment Variables

See `docs/src/dpe/operations.md` for the full list of environment variables, CLI commands, and operations details.

## Remote Deployment Without Toolchain

After building the binary and stylesheet (see [Building for Production](#building-for-production)), copy:

1. The server binary from `target/release/dpe-server`
2. The `public/` directory from `modules/dpe/public/` (static assets + the content-hashed `app.<hash>.css`)
3. The data directory from `modules/dpe/server/data/`

Set the environment variables documented in `docs/src/dpe/operations.md` (notably `DPE_PUBLIC_DIR` and `DPE_DATA_DIR`) and run the binary.
