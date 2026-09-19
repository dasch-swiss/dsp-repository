# Authentication and environment selection

`dsp-cli` targets multiple DSP server environments and supports per-server cached authentication tokens.
The configuration surface is layered (flag → env var → `.env` file in CWD → cached token), with no implicit hard-coded default server.
Every command discloses the selected server and auth state in its output so the caller knows which slice of the world they're seeing.

## Environment selection

- **`--server <value>`** (short: `-s`) flag: accepts either a built-in shortcut name or a literal URL.
  Shortcut matching is case-insensitive (`PROD` == `prod`); literal URLs pass through with their original casing.
- Built-in shortcuts (reachability last verified 2026-05-27 via `GET /health`):
  - `prod`     → `https://api.dasch.swiss`
  - `stage`    → `https://api.stage.dasch.swiss`
  - `dev`      → `https://api.dev.dasch.swiss`
  - `demo`     → `https://api.demo.dasch.swiss`
  - `rdu`      → `https://api.rdu.dasch.swiss`
  - `ls-prod`  → `https://api.ls-prod-server.dasch.swiss`
  - `ls-test`  → `https://api.ls-test-server.dasch.swiss`
  - `local`    → `http://0.0.0.0:3333`
  - Any other value is treated as a literal URL (covers project-specific servers and ad-hoc instances).
  - (`test` → `api.test.dasch.swiss` was removed in PR #6: the environment is decommissioned. Inherited originally from dsp-tools, which still carries it.)
- **`DSP_SERVER`** env var: sets a default. `--server` overrides it.
- **No hard-coded default.** If neither `--server` nor `DSP_SERVER` is set, the command fails with a clear "specify a server" error.
  This protects both humans and agents from accidental-prod operations.
- User-defined shortcuts in a config file are a future addition; not v1.

## `.env` loading from CWD

On startup, `dsp-cli` loads a `.env` file from the current working directory (using a `dotenvy`-style mechanism).
Variables from `.env` populate the env-var space, so a user can pin server + token per project directory:

```
# .env in ~/work/incunabula-project/
DSP_SERVER=test
DSP_TOKEN=<jwt>
```

This is *technically* implicit state but is the "good kind": filesystem-anchored, visible to the user, freshly discovered per invocation.
Same flavour as `git` discovering `.git`, `cargo` discovering `Cargo.toml`, `direnv` walking up the tree.
It's a carve-out from dsp-cli/ADR-0003's "no implicit session state" rule because it answers "where am I working" (visible-filesystem state),
not "what context did the prior command leave me in" (invisible in-process state).

Security caveat: `.env` files commonly leak secrets when committed. The CLI ships a `.env.example` template and the docs explicitly instruct gitignore.

A separate non-secret `dsp.toml` (server-selection only, safe to commit) is a possible future addition but not v1.

## Authentication

```
dsp auth login --server <s> --user <email|username|iri>   # prompts for password interactively (stdin if not TTY)
dsp auth status --server <s>                  # shows auth state for that server
dsp auth logout --server <s>                  # clears cached token for that server
```

- **Password is never a CLI flag.** Resolution order: `DSP_PASSWORD` env var (see caveat below) → interactive prompt →
  stdin when stdin is not a TTY (so CI / agents can pipe it in).
- **`DSP_USER`** env var (or `.env`) supplies the `--user` value for `dsp auth login`; the value may be an email address, a username, or a user IRI —
  the CLI auto-detects the identifier type from the value.
- **`--user` identifier auto-detection**: an `http(s)://` prefix is treated as a user IRI, a value containing `@` as an email address, and any other value as a username;
  the matching DSP-API JSON key (`iri`, `email`, or `username`) is sent in the request body.
- **`DSP_PASSWORD`** env var (or `.env`) supplies the password non-interactively. **Local / dev / test setups only — never a production password.**
  It is a durable plaintext master credential on disk; for non-interactive use against real environments, prefer the scoped, expiring `DSP_TOKEN` over `DSP_PASSWORD`.
  This is a deliberate carve-out from the "password never leaves interactive/stdin" stance,
  justified only by the personal-workstation threat model and the don't-use-with-prod rule.
- **Token cache:** `~/.config/dsp-cli/auth.toml`, file mode `0600`, keyed by server URL. Tokens for different servers coexist.
- **`DSP_TOKEN`** env var overrides the cached token (for CI and pre-provisioned agent use).

v2 schema (PR #6) adds optional `user`, `acquired_at`, `expires_at` fields to `ServerEntry`. Legacy token-only entries continue to parse.
The bump is additive and backward-compatible — no new ADR is warranted.
- **OS keyring integration** (macOS Keychain, Linux Secret Service, Windows Credential Manager) is a v2 polish — not blocking.
- This is **identity state**, distinct from the "session state" forbidden by dsp-cli/ADR-0003.
  Identity answers "who am I"; session would answer "what context did the prior command leave me in".

## Behaviour without auth

Commands attempt unauthenticated requests when no token is available. Public DSP-API endpoints respond normally. Private resources behave one of two ways depending on the endpoint:

1. The endpoint returns a 401/403 — `dsp-cli` produces a clear error: "this resource requires authentication; run `dsp auth login --server <s>` first".
2. The endpoint **silently filters** the response to the subset the caller is allowed to see (true for **instance-side endpoints**: resource listings, search, etc.).
   Project metadata and data models are always fully public; the silent-filter behaviour does **not** affect workflow 1 (schema-side only).

The mitigation for case 2 is the disclosure pattern below.
v1 is schema-side only and therefore not exposed to silent filtering,
but the disclosure pattern is implemented from day one so v2 instance-side commands inherit it without retrofit.

## Auth-state disclosure in command output

Every command's output discloses the server and auth state, terse, in a position appropriate for each format:

- **`prose`** — single footer line after the data:
  ```
  [anonymous on prod]
  [authenticated as you@dasch.swiss on test]
  ```
  For commands that *can* be silently filtered (instance-side, v2+), the line appends `— results may be filtered; login to see private resources`.

- **`json`** — wrapped object: `{ "_meta": { "auth": "...", "server": "...", "note": "..." }, "data": [ ... ] }`. The `note` field appears only when silent filtering is possible.

- **`lines` / `csv` / `tsv`** — data on stdout,
  meta as a single line on stderr (so pipes don't see it, humans do).

## Consequences

- Every command in `dsp-cli` interacts with the four-layer config stack (flag → env var → `.env` → cached token).
  The CLI must resolve the effective server and token at startup.
- "Specify a server" is the most common first-time error users will hit. Help text and error messages should mention `--server prod`, `DSP_SERVER=prod`, and `.env` together.
- `.env` files committed to git is the most likely real-world security failure. Documentation must be loud about this.
- The disclosure pattern is a public CLI contract — once shipped, the output footer line / meta field becomes something users and agents may parse and depend on.
- Configurable shortcuts, OS keyring storage, and `dsp.toml` non-secret project config are all open for v2 — explicitly deferred, not rejected.

## Amendment (2026-06-16, plan 022)

### Authenticated-state filter-disclosure wording

The disclosure pattern described above (case 2 silent filtering) applies to instance-side commands beginning with `dsp vre resource list`.
The prose footer and JSON `note` field carry one of two messages depending on auth state:

- **Anonymous caller** — `results may be filtered; login to see private resources`
- **Authenticated caller** (any token origin) — `results limited to your permissions`

The original ADR text gave only the anonymous phrasing. Both forms are now the stable public contract.
The `lines`/`csv`/`tsv` stderr disclosure line appends the same message after the `[<auth-state> on <server>]` prefix.

### Instance-side reads arrived in the 0.1.0 line

The original ADR framed instance-side commands as "v2+". `dsp vre resource list` (plan 022, Phase 8a) ships in the 0.1.0 release line.
The silent-filter disclosure pattern was implemented speculatively from day one precisely for this moment; no retrofit is required.
Future instance-side read commands in the 0.1.x / 0.2.x range inherit the same disclosure pattern.

## Amendment (2026-07-13, plan 026)

### Exposing the token: `dsp auth token`

`dsp auth token` is the **read-out** counterpart to `set-token`: where `set-token` reads a JWT from stdin into the cache,
`dsp auth token` resolves the effective token (env `DSP_TOKEN` > cache, unchanged precedence) and prints it verbatim to stdout —
the only way to get a cached token back out of the CLI for use in another tool
(`export DSP_TOKEN=$(dsp auth token -s dev)`, `curl -H "Authorization: Bearer $(dsp auth token -s dev)"`).

This is a **bearer-credential footgun by design**: the command's entire purpose is to put a live credential on stdout,
so every caution that applies to handling `DSP_TOKEN` (see _Environment-variable credentials_ above and `dsp docs connecting`) applies doubly here —
the token can land in shell history, `ps` output, xtrace/CI logs, or terminal scrollback depending on how the caller invokes it.

**Divergence from the "the probe is the trust boundary" stance:** `set-token` and `login` both treat a live server round-trip as the point where a token is validated —
no local check substitutes for asking the server.
`dsp auth token` makes no server round-trip at all (D2, plan 026 — an explicit non-goal, to keep the command network-free and fast for piping), so it has no probe to lean on.
Instead it does a **local, advisory check** of the JWT `exp` claim and refuses (exit `3`, `AuthRequired`) when the token is locally detected as expired or absent.
This is deliberately **not** a security control — the JWT signature is not verified (`insecure_decode`), so a forged `exp` could lie,
and the token is the caller's own, so there is no trust boundary to defend.
It exists purely to save the caller a failed downstream request: "your cached token looks stale, re-login" is UX, not authorization.
See `src/client/jwt.rs`'s module doc for the equivalent framing at the code level.

## Amendment (2026-08-07, plan 035)

### No auth-state disclosure carve-out: `dsp docs`, `dsp auth token`, `dsp vre sparql query`

This ADR's auth-state disclosure requirement (a stderr line, a prose footer, or a JSON `_meta.auth`
key) presumes a `Renderer` to carry it. Three commands have none: `dsp docs` (empty `_meta`), `dsp
auth token` (no `_meta` at all — the bare token is the whole output), and now `dsp vre sparql query`
([dsp-cli/ADR-0016](0016-sparql-passthrough.md)). Naming the carve-out explicitly here, rather than leaving
three commands as undocumented exceptions to "every command discloses":

> A no-`Renderer` command discloses nothing — there is no `_meta`, no prose footer, and no stderr
> line to carry it.

For `dsp vre sparql query` specifically: its stdout is **unsanitised store bytes by contract**
(dsp-cli/ADR-0016) — an accepted risk, not an oversight — and the sanitisation duty moves to the stderr prose
path instead, where a store rejection's text is reported (control-character-stripped and capped) on a
non-`2xx` relay.

## Amendment (2026-09-19) — no `.env.example` template

The CLI no longer ships a `.env.example` template (it was not carried over in the migration to
`dsp-repository`, dsp-cli/ADR-0014). The `.env` guidance — which variables to set and the
gitignore caveat — now lives in [Testing Strategy](../../../docs/src/dsp-cli/testing-strategy.md#general-configuration-variables)'s
"General configuration variables" section.

## Amendment (2026-09-20) — plain `http://` is refused for a non-local server

"Any other value is treated as a literal URL" above is no longer unconditional. A literal URL whose
scheme is `http` and whose host is not local is now **refused** with a usage diagnostic (exit `2`),
because an authenticated command sends a bearer token and plain HTTP puts it on the wire in
cleartext. `https://` is always accepted, and a value that does not parse as an absolute URL is
still passed through unchanged — there is no cleartext risk to assess on it.

**Local means loopback or unspecified**, not loopback alone: the `local` shortcut above expands to
`http://0.0.0.0:3333`, and `0.0.0.0` is the unspecified address, not a loopback one. The accepted
set is therefore the domain `localhost`, any IPv4 or IPv6 loopback address (`127.0.0.0/8`, `::1`),
and the unspecified addresses (`0.0.0.0`, `::`). Every built-in shortcut still resolves.

The refusal is overridable by `--allow-insecure-server` or `DSP_ALLOW_INSECURE_SERVER=1`, flag
before env, the precedence this record already sets for every other setting. The override is a
global clap flag, so it appears in every subcommand's help. A server value containing a control
character is refused outright, on any scheme, rather than sanitized: sanitizing the stored value
would change the string used as the auth-cache key and sent in outgoing requests, so refusal is
safer. The server value is still stripped of control characters (`sanitize_for_diagnostic`) when it
appears inside a refusal message itself, so no diagnostic — this one included — can carry a raw
byte back out.
