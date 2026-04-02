# DPE -- Discovery and Presentation Environment

Server-side rendered web application built with [Leptos](https://github.com/leptos-rs/leptos) and [Axum](https://github.com/tokio-rs/axum). Interactive behavior (tab switching, live search) is driven by [Datastar](https://data-star.dev/) SSE fragments -- no client-side WASM.

## Project Structure

```
dpe/
├── core/             # Pure domain types and data loading (crate: dpe-core)
├── api-oai/          # OAI-PMH 2.0 API (crate: dpe-api-oai)
├── web/              # Leptos components and pages (crate: dpe-web)
├── server/           # Server binary and fragment handlers (crate: dpe-server)
├── web-e2e-tests/    # Playwright E2E tests
├── public/           # Static assets
└── style/            # CSS / Tailwind configuration
```

## Prerequisites

- Rust toolchain (managed via `rustup`)
- `cargo-leptos` (`cargo install cargo-leptos --locked`)
- Node.js + pnpm (for Tailwind CSS / DaisyUI)
- [just](https://github.com/casey/just) command runner

## Running the Development Server

```bash
just watch-dpe
```

Starts the server with hot reload at `http://127.0.0.1:4000`.

## Building for Production

```bash
cargo leptos build --project dpe --release
```

Output:
- Server binary: `target/release/dpe-server`
- Site assets: `modules/dpe/target/site/`

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

After `cargo leptos build --release`, copy:

1. The server binary from `target/release/dpe-server`
2. The `site/` directory from `modules/dpe/target/site/`
3. The data directory from `modules/dpe/server/data/`

Set the environment variables listed above and run the binary.
