# dsp-cli Usage

`dsp-cli` is an AI-agent-friendly command-line interface for the DaSCH Service Platform (DSP). The
published crate is `dsp-cli`; the installed binary is `dsp`. Full end-user documentation, quickstart
examples, and the command surface live in
[`dsp-cli/README.md`](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/README.md) —
this page only orients; it does not duplicate it.

## Install

```bash
cargo install dsp-cli
```

Places the `dsp` binary in `~/.cargo/bin`. See the README for prerequisites (a Rust toolchain via
[rustup](https://rustup.rs/)).

## Built-in documentation: `dsp docs`

End-user documentation ships embedded in the binary, version-synced with the release:

```bash
dsp docs                 # list the available topics
dsp docs connecting      # servers, environments, and authentication
dsp docs workflows       # chaining commands into real tasks
```

`dsp docs <topic> --pager` pages through `$PAGER`; `dsp docs -j` emits a machine-readable JSON
topic index.

## Server shortcuts

Every command is pointed at a server with `--server`/`-s` — a full URL or a built-in shortcut
(`prod`, `stage`, `dev`, `demo`, `rdu`, `local`, and a couple of load-specific ones). There is no
hard-coded default; a command with neither `--server` nor `DSP_SERVER` set fails fast with a clear
error. See `dsp docs connecting` for the full list and rationale.

## Auth

Most project and schema metadata is public and needs no login. Instance data (resources) and
private projects do:

```bash
dsp auth login --server stage --user you@example.org   # prompts for your password; caches the token
dsp auth status --server stage                          # shows auth state for that server
dsp auth token --server stage                            # prints the cached bearer token, for piping
dsp auth logout --server stage                           # clears the cached token
```

See the README's Quickstart for the full login/token walkthrough, and
[Testing Strategy](./testing-strategy.md#general-configuration-variables) for the environment
variables (`DSP_SERVER`, `DSP_USER`, `DSP_PASSWORD`, `DSP_TOKEN`) that let a script or agent skip
the interactive prompts entirely.
