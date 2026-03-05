# Workflows and Conventions

## Entry Points

The first entry point of this repository is the README file,
which should give anyone an indication of where to find any information they need.

For any interaction or coding-related workflow, the justfile is the primary source of truth.
The justfile contains all the commands and workflows that are used in this repository, along with their descriptions.

## Key Development Commands

The justfile provides self-documenting commands. Key workflows include:

- `just dsp-dpe check` - Run formatting and linting checks
- `just dsp-dpe build` - Build all targets
- `just dsp-dpe test` - Run all tests
- `just dsp-dpe watch-mosaic-demo` - Run Mosaic demo with hot reload
- `just dsp-dpe watch-leptos-dpe` - Run Leptos DPE with hot reload

Run `just` without arguments to see all available commands with descriptions.

Any further information should be located in the documentation.

## CI/CD

GitHub Actions workflows run automatically on pushes and pull requests.
See [Release, Deployment and Versioning](./deployment.md) for details on the
CI/CD pipelines, including PR preview deployments and production releases.

## Git Workflow

For this repository, we use a rebase workflow.
This means that all changes should be made on a branch,
and then rebased onto the main branch before being merged.

This allows us to keep a clean commit history and avoid merge commits.
