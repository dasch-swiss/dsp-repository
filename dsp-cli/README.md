# dsp-cli

An AI-agent-friendly command-line interface for the [DaSCH Service Platform (DSP)](https://www.dasch.swiss/).

`dsp-cli` talks to a DSP server using the vocabulary researchers use — **data-models**,
**resource-types**, and **fields** — instead of DSP-API's raw RDF/JSON-LD surface. It is
built for exploration: list projects, inspect data models, drill into resource-types and
their fields, and read instance data. Every command speaks both to humans (readable prose)
and to machines (`-j` for a structured JSON envelope), so it fits AI agents and shell
scripts as well as interactive use.

The published crate is **`dsp-cli`**; the installed binary is **`dsp`**.

> **Status:** pre-1.0 and read-focused (no write operations yet).
> The command surface may change in any `0.x` release.

## Install

`dsp-cli` is a Rust program installed with `cargo`. If you don't have a Rust toolchain yet,
install one with [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then install `dsp-cli` from crates.io:

```bash
cargo install dsp-cli
```

This places the `dsp` binary in `~/.cargo/bin` (make sure that directory is on your `PATH`).

## Quickstart

Most project and schema metadata is public, so you can explore without logging in. Point
`dsp` at a server with `--server` (a full URL, or a built-in shortcut like `stage`), then read:

```bash
# List the projects on the DaSCH staging server
dsp vre project list --server https://api.stage.dasch.swiss

# List a project's data-models (0801 = "Bernoulli-Euler Online")
dsp vre data-model list -p 0801 --server stage

# Show the fields of one resource-type
dsp vre resource-type describe -p 0801 --data-model beol --resource-type letter --server stage
```

Add `-j` to any command for JSON output. Set `DSP_SERVER` (or a `.env` file in the current
directory) to avoid repeating `--server`.

Private projects and instance data (resources) need authentication. Log in once — the token
is cached, so later commands reuse it automatically:

```bash
dsp auth login --server stage --user you@example.org   # prompts for your password

# instance reads now run as your authenticated user (private resources included):
dsp vre resource list -p 0801 --resource-type letter --server stage
```

`dsp auth token` prints the cached token to stdout, which makes it easy to hand to anything
else that needs a DSP bearer token — for example a raw `curl` against DSP-API:

```bash
# call a DSP-API endpoint that requires authentication:
curl -H "Authorization: Bearer $(dsp auth token --server stage)" https://api.stage.dasch.swiss/admin/users

# or lift the token into the environment for a series of calls:
export DSP_TOKEN=$(dsp auth token --server stage)
```

Keep the token inside `$(…)` or a pipe — calling `dsp auth token` on its own prints the raw
credential to your terminal and shell history. See `dsp docs connecting` for the full auth story.

## What you can do

```text
dsp auth              log in and manage tokens (login, token, status, logout, set-token)
dsp vre project       list / describe projects; dump a full project archive
dsp vre data-model    list / describe a project's data-models; show their structure
dsp vre resource-type list / describe resource-types and their fields
dsp vre resource      list / describe instance data (add --values for field contents)
dsp vre vocabulary    list / describe a project's controlled vocabularies
dsp vre sparql        raw SPARQL passthrough to the server's triplestore (SystemAdmin)
dsp docs              read the built-in documentation
```

## Documentation

The full user documentation is built into the binary:

```bash
dsp docs                 # list the available topics
dsp docs connecting      # servers, environments, and authentication
dsp docs workflows       # chaining commands into real tasks
```

## License

[Apache-2.0](./LICENSE).
