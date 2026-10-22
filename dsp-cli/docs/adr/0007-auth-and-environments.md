# Servers are chosen explicitly, tokens are cached per server, and every response discloses the auth state

`dsp-cli` targets multiple DSP server environments and supports per-server cached authentication tokens.
The configuration surface is layered (flag → env var → `.env` file in CWD → cached token), with no implicit hard-coded default server.
Every command that has a `Renderer` discloses the selected server and auth state in its output so the caller knows which slice of the world they're seeing.

## Environment selection

- **`--server <value>`** (short: `-s`) flag: accepts either a built-in shortcut name or a literal URL.
  Shortcut matching is case-insensitive (`PROD` == `prod`); literal URLs pass through with their original casing.
- Built-in shortcuts:
  - `prod`     → `https://api.dasch.swiss`
  - `stage`    → `https://api.stage.dasch.swiss`
  - `dev`      → `https://api.dev.dasch.swiss`
  - `demo`     → `https://api.demo.dasch.swiss`
  - `rdu`      → `https://api.rdu.dasch.swiss`
  - `ls-prod`  → `https://api.ls-prod-server.dasch.swiss`
  - `ls-test`  → `https://api.ls-test-server.dasch.swiss`
  - `local`    → `http://0.0.0.0:3333`
  - Any other value is treated as a literal URL (covers project-specific servers and ad-hoc instances).
  - There is no `test` shortcut: that environment is decommissioned.
- **`DSP_SERVER`** env var: sets a default. `--server` overrides it.
- **No hard-coded default.** If neither `--server` nor `DSP_SERVER` is set, the command fails with a usage error ("specify a server").
  This protects both humans and agents from accidental-prod operations. The error mentions `--server`, `DSP_SERVER` and `.env` together.
- **Plain `http://` is refused for a non-local server** (usage diagnostic, exit `2`), because an authenticated command sends a bearer token and plain HTTP puts it
  on the wire in cleartext. `https://` is always accepted, and a value that does not parse as an absolute URL passes through unchanged (no cleartext risk to assess).
  *Local* means loopback or unspecified, not loopback alone: `localhost`, any IPv4 or IPv6 loopback address (`127.0.0.0/8`, `::1`) and the unspecified
  addresses (`0.0.0.0`, `::`), so every built-in shortcut resolves (`local` is `0.0.0.0`).
  `--allow-insecure-server` or `DSP_ALLOW_INSECURE_SERVER=1` overrides the refusal, flag before env, the precedence that holds for every other setting;
  it is a global clap flag and appears in every subcommand's help.
- **A server value containing a control character is refused outright**, on any scheme, rather than sanitised: sanitising would change the string used as the
  auth-cache key and sent in outgoing requests. The value is stripped of control characters (`sanitize_for_diagnostic`) when it appears inside a refusal message,
  so no diagnostic carries a raw byte back out.

## `.env` loading from CWD

On startup, `dsp-cli` loads a `.env` file from the current working directory (`dotenvy`, which never overrides an already-set variable).
Variables from `.env` populate the env-var space, so a user can pin server + token per project directory:

```
# .env in ~/work/incunabula-project/
DSP_SERVER=dev
DSP_TOKEN=<jwt>
```

This is *technically* implicit state but is the "good kind": filesystem-anchored, visible to the user, freshly discovered per invocation.
Same flavour as `git` discovering `.git`, `cargo` discovering `Cargo.toml`, `direnv` walking up the tree.
It's a carve-out from dsp-cli/ADR-0003's "no implicit session state" rule because it answers "where am I working" (visible-filesystem state),
not "what context did the prior command leave me in" (invisible in-process state).

Security caveat: `.env` files commonly leak secrets when committed; the docs instruct gitignoring them. The CLI ships no `.env.example`;
the variables and the caveat are in [Testing Strategy](../../../docs/src/dsp-cli/testing-strategy.md#general-configuration-variables).

Cross-context warning: when `DSP_TOKEN` comes from the caller's shell but `DSP_SERVER` is newly introduced by a `.env` file in the current directory,
the CLI warns once, before any authenticated request, that a token acquired for one server may be sent to another (`cross_context_token_risk` in `main.rs`).

## Authentication

```
dsp auth login --server <s> --user <email|username|iri>   # prompts for password interactively (stdin if not TTY)
dsp auth status --server <s>                  # shows auth state for that server
dsp auth logout --server <s>                  # clears cached token for that server
dsp auth set-token --server <s>               # reads a JWT from stdin, verifies it with a live probe, caches it only if the probe succeeds
dsp auth token --server <s>                   # prints the resolved bearer token to stdout
```

- **Password is never a CLI flag.** Resolution order: `DSP_PASSWORD` env var (see caveat below) → interactive prompt →
  stdin when stdin is not a TTY (so CI / agents can pipe it in).
- **`DSP_USER`** env var (or `.env`) supplies the `--user` value (`-u`) for `dsp auth login`; the value may be an email address, a username, or a user IRI.
  The CLI detects the identifier type from the value: an `http(s)://` prefix is a user IRI, a value containing `@` an email address, anything else a username;
  the matching DSP-API JSON key (`iri`, `email`, or `username`) is sent in the request body.
- **`DSP_PASSWORD`** env var (or `.env`) supplies the password non-interactively. **Local / dev / test setups only — never a production password.**
  It is a durable plaintext master credential on disk; for non-interactive use against real environments, prefer the scoped, expiring `DSP_TOKEN`.
  This is a deliberate carve-out from the "password never leaves interactive/stdin" stance,
  justified only by the personal-workstation threat model and the don't-use-with-prod rule.
- **Token cache:** `~/.config/dsp-cli/auth.toml`, file mode `0600`, keyed by server URL. Tokens for different servers coexist.
  An entry carries the token and optional `user`, `acquired_at` and `expires_at`; token-only entries still parse.
- **`DSP_TOKEN`** env var overrides the cached token (for CI and pre-provisioned agent use); a blank value falls through to the cache.
- The token cache is **identity state**, distinct from the "session state" forbidden by dsp-cli/ADR-0003.
  Identity answers "who am I"; session would answer "what context did the prior command leave me in".
- **`dsp auth token`** resolves the effective token (`DSP_TOKEN` > cache) and prints it verbatim: the only way to get a cached token back out of the CLI
  (`export DSP_TOKEN=$(dsp auth token -s dev)`). It is a bearer-credential footgun by design: the token can land in shell history, `ps` output, xtrace/CI logs or
  scrollback depending on how it is invoked, so every caution about `DSP_TOKEN` applies doubly.
  It makes no server round-trip (network-free, fast for piping), so it does a **local, advisory** check of the JWT `exp` claim and refuses (exit `3`, `AuthRequired`)
  when the token is locally detected as expired or absent. This is UX, not a security control: the signature is not verified (`insecure_decode`)
  and the token is the caller's own. `login` and `set-token` validate against the server instead.

## Behaviour without auth

Commands attempt unauthenticated requests when no token is available. Public DSP-API endpoints respond normally. Private resources behave one of two ways depending on the endpoint:

1. The endpoint returns a 401/403 — `dsp-cli` produces a clear error: "this resource requires authentication; run `dsp auth login --server <s>` first".
2. The endpoint **silently filters** the response to the subset the caller is allowed to see (true for **instance-side endpoints**: resource listings, search, etc.).
   Project metadata and data models are always fully public and unaffected.

The mitigation for case 2 is the disclosure below, which every instance-side read command carries.

## Auth-state disclosure in command output

Every command with a `Renderer` discloses the server and auth state, terse, in a position appropriate for each format:

- **`prose`** — single footer line after the data, naming the resolved server URL:
  ```
  [anonymous on https://api.dasch.swiss]
  [authenticated as you@dasch.swiss on https://api.dev.dasch.swiss]
  ```
- **`json`** — wrapped object: `{ "_meta": { "auth": "...", "server": "...", "note": "..." }, "data": [ ... ] }`. The `note` field appears only when silent filtering is possible.
- **`lines` / `csv` / `tsv`** — data on stdout, meta as a single line on stderr (so pipes don't see it, humans do).

For commands that can be silently filtered (instance-side), the line or `note` appends a message depending on auth state, and both wordings are the stable public contract:

- **Anonymous caller** — `results may be filtered; login to see private resources`
- **Authenticated caller** (any token origin) — `results limited to your permissions`

A command without a `Renderer` (`dsp docs`, `dsp auth token`, `dsp vre sparql query`; see dsp-cli/ADR-0003) discloses nothing: there is no `_meta`, footer or stderr line to carry it.
The sanitisation consequences for `sparql query` are in [dsp-cli/ADR-0016](0016-sparql-passthrough.md).

## Considered Options

- **A default server** (e.g. `prod`). Rejected: accidental-prod operations.
- **Password as a CLI flag.** Rejected: it lands in shell history and `ps`.
- **Sanitising a server value containing control characters.** Rejected: it would change the auth-cache key and the string sent in requests; refusal is safer.
- **A config file for user-defined server shortcuts and non-secret server selection** (a safe-to-commit `dsp.toml`). *Deferred*, not rejected. It may be the
  `dsp config` meta-group of [dsp-cli/ADR-0006](0006-top-level-shape.md).
- **OS keyring storage** (macOS Keychain, Linux Secret Service, Windows Credential Manager) instead of `auth.toml`. *Deferred*, not rejected; not blocking.

## Consequences

- Every command interacts with the four-layer config stack (flag → env var → `.env` → cached token) and resolves the effective server and token at startup.
- "Specify a server" is the most common first-time error. `.env` files committed to git are the most likely real-world security failure; documentation must be loud about it.
- The disclosure pattern is a public CLI contract: users and agents may parse the footer line or `_meta` field.

Enforced by: the no-server, insecure-`http://` and control-character refusal tests in `dsp-cli/tests/cli.rs`, the shortcut-resolution unit tests in
`dsp-cli/src/config/mod.rs`, the `0600` test in `dsp-cli/src/config/auth_cache.rs`, unit tests on token precedence and the cache format, wiremock tests for
login and set-token, and the disclosure footer and `_meta.auth` pinned in `dsp-cli/tests/*_snapshots.rs` (**static-analysis**); the rest by **review**.
