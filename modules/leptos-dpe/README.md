<picture>
    <source srcset="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_Solid_White.svg" media="(prefers-color-scheme: dark)">
    <img src="https://raw.githubusercontent.com/leptos-rs/leptos/main/docs/logos/Leptos_logo_RGB.svg" alt="Leptos Logo">
</picture>

# Leptos Axum Starter Template

This is a template for use with the [Leptos](https://github.com/leptos-rs/leptos) web framework and the [cargo-leptos](https://github.com/leptos-rs/cargo-leptos) tool using [Axum](https://github.com/tokio-rs/axum).

## Creating your template repo

If you don't have `cargo-leptos` installed you can install it with

```bash
cargo install cargo-leptos --locked
```

Then run
```bash
cargo leptos new --git https://github.com/leptos-rs/start-axum-workspace/
```

to generate a new project template.

```bash
cd leptos-dpe
```

to go to your newly created project.
Feel free to explore the project structure, but the best place to start with your application code is in `app/src/lib.rs`.
Additionally, Cargo.toml may need updating as new versions of the dependencies are released, especially if things are not working after a `cargo update`.

### Islands support

Note that for islands to work correctly, you need to have a `use app;` in your frontend `lib.rs` otherwise rustc / wasm_bindgen gets confused.
To prevent clippy from complaining, at the top of the `frontend/src/lib.rs` file place:
```rust
#[allow(clippy::single_component_path_imports)]
#[allow(unused_imports)]
use app;
```

## Running your project

```bash
cargo leptos watch
```

## Installing Additional Tools

By default, `cargo-leptos` uses `nightly` Rust, `cargo-generate`, and `sass`. If you run into any trouble, you may need to install one or more of these tools.

1. `rustup toolchain install nightly --allow-downgrade` - make sure you have Rust nightly
2. `rustup default nightly` - setup nightly as default, or you can use rust-toolchain file later on
3. `rustup target add wasm32-unknown-unknown` - add the ability to compile Rust to WebAssembly
4. `cargo install cargo-generate` - install `cargo-generate` binary (should be installed automatically in future)
5. `npm install -g sass` - install `dart-sass` (should be optional in future

## Compiling for Release
```bash
cargo leptos build --release
```

Will generate your server binary in target/server/release and your site package in target/site

## Docker Deployment

This project includes optimized Docker configuration for production deployment with the following optimizations:
- WASM-specific release profile for minimal binary size
- Cargo release profile with LTO and size optimization (opt-level='z')
- Multi-stage build for minimal final image size
- Non-root user for security
- Proper compression and optimization flags

### Building the Docker Image

```bash
docker build -t leptos-dpe .
```

### Running with Docker

```bash
docker run -p 8080:8080 leptos-dpe
```

The application will be available at `http://localhost:8080`

### Using Docker Compose

For easier management, use Docker Compose:

```bash
# Build and start
docker-compose up -d

# View logs
docker-compose logs -f

# Stop
docker-compose down
```

### Environment Variables

The following environment variables can be configured:
- `RUST_LOG` - Logging level (default: "info")
- `LEPTOS_OUTPUT_NAME` - Output file name (default: "leptos-dpe")
- `LEPTOS_SITE_ROOT` - Site root directory (default: "site")
- `LEPTOS_SITE_PKG_DIR` - Package directory (default: "pkg")
- `LEPTOS_SITE_ADDR` - Server bind address (default: "0.0.0.0:8080")
- `LEPTOS_ENV` - Environment mode (default: "PROD")

## Testing Your Project

Cargo-leptos uses [Playwright](https://playwright.dev) as the end-to-end test tool.

Prior to the first run of the end-to-end tests run Playwright must be installed.
In the project's `end2end` directory run `npm install -D playwright @playwright/test` to install playwright and browser specific APIs.

To run the tests during development in the project root run:
```bash
cargo leptos end-to-end
```

To run tests for release in the project root run:
```bash
cargo leptos end-to-end --release
```
There are some examples tests are located in `end2end/tests` directory that pass tests with the sample Leptos app.

A web-based report on tests is available by running `npx playwright show-report` in the `end2end` directory.


## Executing a Server on a Remote Machine Without the Toolchain
After running a `cargo leptos build --release` the minimum files needed are:

1. The server binary located in `target/server/release`
2. The `site` directory and all files within located in `target/site`

Copy these files to your remote server. The directory structure should be:
```text
leptos-dpe
site/
```
Set the following environment variables (updating for your project as needed):
```text
LEPTOS_OUTPUT_NAME="leptos-dpe"
LEPTOS_SITE_ROOT="site"
LEPTOS_SITE_PKG_DIR="pkg"
LEPTOS_SITE_ADDR="127.0.0.1:3000"
LEPTOS_RELOAD_PORT="3001"
```
Finally, run the server binary.

## Licensing

This template itself is released under the Unlicense. You should replace the LICENSE for your own application with an appropriate license if you plan to release it publicly.
