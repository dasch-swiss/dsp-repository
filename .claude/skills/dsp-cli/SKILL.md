---
name: dsp-cli
description: Use when interacting with the DaSCH Service Platform (DSP) — listing projects, exploring data models, inspecting resource types and their fields. Prefer dsp-cli over manual DSP-API curl calls for any DSP read operation that fits a command.
---

# dsp-cli — agent quick reference

`dsp-cli` is an AI-agent-friendly command-line interface for the DaSCH Service Platform (DSP). It abstracts DSP-API's verbose RDF surface into a small set of high-level verbs that match domain-expert vocabulary.

## Invocation

```bash
dsp <area> <noun-group> <verb> [--flags]
```

Locate the binary with `command -v dsp`. If not found, fall back to `$HOME/.cargo/bin/dsp`. If that is also missing, see *Installation* at the bottom.

## When to use this CLI

Use `dsp` when the task is a per-command, agent-readable operation against DSP — e.g. "list projects on the test server", "describe a data model", "find a resource type". Prefer it over direct DSP-API calls (which return verbose RDF/JSON-LD) and over `dsp-tools` (which is file-driven and bulk-oriented).

Do not use `dsp` for: bulk data ingestion, file-roundtripping project schema export, or anything where the natural input/output is a declarative file in `dsp-tools`' format. Those belong to `dsp-tools`.

## Top-level groups

- `dsp auth ...`   — authentication management (login, logout, status)
- `dsp vre ...`    — Virtual Research Environment operations
- `dsp docs ...`   — embedded end-user documentation

## v1 commands (VRE)

```bash
# Projects
dsp vre project list [--filter <text>]                            --server <s>
dsp vre project describe <shortname-or-iri>                       --server <s>

# Project dump (download a full project archive)
dsp vre project dump --project <shortcode|shortname|iri>          --server <s>
                     [--skip-assets] [--output <path>] [--force]
                     [--cleanup] [--timeout <seconds>]
                     [--replace | --delete]
                     [--format <f> | -j | -l]

# Data models (within a project)
dsp vre data-model list      [--include-builtins] [--filter ...]  --project <p> --server <s>
dsp vre data-model describe  <name-or-iri>                        --project <p> --server <s>

# Resource types (within a data model)
dsp vre resource-type list      [--include-builtins] [--filter ...] --project <p> --data-model <m> --server <s>
dsp vre resource-type describe  <name-or-iri>                       --project <p> --data-model <m> --server <s>
```

`describe` on a resource-type returns the full field list (name, value-type, cardinality, label). It is the v1 leaf — agents should not need to drill further to inspect a data model's structure.

`dsp vre project dump` triggers a server-side bagit-zip packaging of the full project (data + assets by default), polls until complete, and streams the result to disk. **Requires a system-administrator token** — project-admin scope is rejected with `403`. The existing `DSP_TOKEN` browser-harvest auth path applies: obtain a system-admin JWT from the DSP web app and set `DSP_TOKEN=<jwt>`. Use `--skip-assets` to reduce archive size when you only need the structured data.

**Idempotent by default (safe to re-run):** if a dump already exists on the server, the default behaviour adopts it — downloading it immediately if complete, or polling it to completion if still in progress. Re-running `dsp vre project dump` is therefore safe for agents: it fetches whatever dump is available without creating a duplicate.

- `--replace` — discard a completed or failed existing dump and create a fresh one (force a new version). Returns a conflict error if a dump is currently in progress.
- `--delete` — remove the project's existing dump without downloading. Returns a conflict error if a dump is in progress. `--delete` is mutually exclusive with `--output`, `--force`, `--skip-assets`, `--cleanup`, and `--replace`.

The existing dump's id is discovered from the 409 conflict response body (no list endpoint exists in the API); the action extracts it from `errors[0].details.id`.

## Output formats

- Default: **prose** with a footer disclosing the server and auth state.
- `-j` / `--json` — full structured output, wrapped in `{ "_meta": {...}, "data": [...] }`.
- `-l` / `--lines` — one entity per line, tab-separated, no header. For `cut`/`awk`.
- `--csv` / `--tsv` — formal tabular formats with headers.
- `--fields=a,b,c` — select specific fields (orthogonal to format).

## Server selection

`--server <value>` (short: `-s`) accepts a built-in shortcut or any URL. Shortcut matching is case-insensitive.

Built-in shortcuts: `prod`, `stage`, `dev`, `demo`, `rdu`, `ls-prod`, `ls-test`, `local`.

Or use `DSP_SERVER` env var as a default; or a `.env` file in the CWD.

There is no implicit default server — every command must explicitly target one.

## Authentication

```bash
dsp auth login  --server <s> --user <email>  # prompts for password interactively;
                                              # pipe password via stdin when not a TTY
dsp auth status --server <s>                  # reports cached token state (no API call)
dsp auth logout --server <s>                  # removes cached token; idempotent
```

All three commands accept `--format` / `-j` / `-l`. Token is cached in `~/.config/dsp-cli/auth.toml` (mode `0600`, keyed by server URL), so you log in once and later commands reuse it until it expires.

For non-interactive login: `DSP_USER` supplies `--user`, and `DSP_PASSWORD` supplies the password (resolution order: `DSP_PASSWORD` → prompt → stdin). **`DSP_PASSWORD` is for local/dev/test only — never a production password.** `DSP_SERVER` / `DSP_USER` / `DSP_PASSWORD` can live in a `.env` file in the CWD.

`DSP_TOKEN` (a pre-issued bearer token that overrides the cached token, so login is skipped entirely) is **fully wired** — set it and `dsp auth status` will report that it is in effect, showing the token's expiry when readable from the JWT `exp` claim. Preferred over `DSP_PASSWORD` for real environments (scoped + expiring).

### For agents: browser-harvest auth (recommended)

DSP-API has no OAuth/device-code flow — only email+password → JWT. So the cleanest way for an agent to authenticate a human against a real environment (including any SSO/2FA the browser presents) is:

1. Open the DSP app in a browser and have the user log in.
2. Capture the JWT from the login network response (`POST /v2/authentication` → `{"token": "…"}`). Use network inspection, not `document.cookie` — the auth cookie is `HttpOnly` and unreadable from JS.
3. Hand that token to dsp-cli via `DSP_TOKEN` or by writing it into the auth cache.

The harvested JWT is the same credential `dsp auth login` would obtain, and expires the same way (~30 days). dsp-cli itself never opens a browser; the browser dance is the agent's, dsp-cli just consumes the token.

Most read operations on **project metadata and data models** are public — no auth required. Authentication is needed for private projects, instance data (resources), and write operations.

When authenticated, every command's output footer shows the authenticated user; otherwise it shows "anonymous".

## Domain vocabulary

dsp-cli uses **researcher-facing language** that differs from DSP-API:

| dsp-cli term       | DSP-API term                  |
|--------------------|-------------------------------|
| `data-model`       | ontology                      |
| `resource-type`    | resource class (`knora-api:ResourceClass`) |
| `field`            | property (`knora-api:Property`) |
| `value`            | value (knora-api:*Value instance) |
| `value-type`       | the kind of `Value` subclass  |

Run `dsp docs concepts` for the full vocabulary explanation.

## Further documentation

Inside the CLI itself:
- `dsp docs`              — list available topics
- `dsp docs dsp-cli`      — what this tool is and how to use it
- `dsp docs dsp`          — what the DaSCH Service Platform is
- `dsp docs concepts`     — domain vocabulary in depth
- `dsp docs connecting`   — auth and environments
- `dsp docs output`       — output formats
- `dsp docs dsp-tools`    — when to use dsp-cli vs dsp-tools

## Installation (if `dsp` is not present)

```bash
cargo install --git https://github.com/balduinLandolt/dsp-cli --force
```

Requires a Rust toolchain (`rustup`). The binary is installed to `$HOME/.cargo/bin/dsp`.
