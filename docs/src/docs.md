# About this Documentation

This documentation is built using [mdBook](https://github.com/rust-lang/mdBook).

## Pre-requisites

Before contributing, please ensure you have the following installed:

- [The Rust toolchain](https://www.rust-lang.org/tools/install).
- [Just](https://just.systems/man/en/).

Any further dependencies can be installed using `just` commands:

```bash
just install-requirements
```

## Building and Serving the Documentation

To run the documentation locally, use:

```bash
just docs-serve
```

## Contributing to the Documentation

mdBook uses Markdown for documentation.

The documentation is organized into chapters and sections, which are defined in the `SUMMARY.md` file.
Each section corresponds to a Markdown file in the `src` directory.

To configure the documentation (e.g. adding plugins), modify the `book.toml` file.

## Deployment

This documentation is not yet deployed. The deployment process will be defined in the future.
