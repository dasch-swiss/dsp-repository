# Connecting — servers, environments, and authentication

This topic covers how `dsp-cli` reaches a server and how it authenticates.
Flag-level detail stays in `--help` (e.g. `dsp auth login --help`).

## There is no default server

Every command must be told which server to talk to — there is no implicit default
(principle 5: explicit beats implicit). Provide it with `-s` / `--server`, or set
it once via the `DSP_SERVER` environment variable or a `.env` file.

A server is either a full URL (`https://api.example.org`) or a built-in shortcut:

| shortcut | server |
|---|---|
| `prod` | `https://api.dasch.swiss` |
| `stage` | `https://api.stage.dasch.swiss` |
| `dev` | `https://api.dev.dasch.swiss` |
| `demo` | `https://api.demo.dasch.swiss` |

(Other shortcuts exist — `rdu`, `ls-prod`, `ls-test`, `local`. Shortcuts are
case-insensitive; anything unrecognised is treated as a literal URL.) Not every
environment is always live — probe `<server>/health` if unsure.

## `.env` files

On startup `dsp-cli` loads a `.env` file from the current working directory, if
present. This lets you set `DSP_SERVER` (and more, below) once per project
directory instead of repeating flags. `.env` loading is visible filesystem state,
not hidden session state — the one carve-out to "no implicit state".

## Authentication

DSP-API authentication is a simple exchange: a user identifier (email, username,
or user IRI) plus a password, returning a JWT. There is no OAuth or device-code
flow.

```
dsp auth login -s prod -u you@example.org     # prompts for the password
dsp auth status -s prod                        # who am I on this server?
dsp auth logout -s prod                        # clear the cached token
```

The token is cached per-server in `~/.config/dsp-cli/auth.toml` (mode `0600`), so
you log in once and subsequent commands reuse it. Most v1 commands are reads
against public endpoints and need no login; `dsp vre project dump` does
(it requires project-admin rights).

### Auth-state disclosure

Every command tells you who you are and where: prose output ends with a footer
like `[authenticated as you@example.org on https://api.dasch.swiss]`, and
structured formats carry the same in `_meta.auth`. You never have to guess whether
a command ran anonymously.

### Environment-variable credentials

- `DSP_TOKEN` — a bearer token used directly, overriding the cache for that
  invocation (it is not persisted).
- `DSP_USER` / `DSP_PASSWORD` — non-interactive login credentials, convenient in a
  `.env` for local/dev use. **Never put a real production password in `.env`.**

### Reading the token back out: `dsp auth token`

`dsp auth token -s <server>` prints the resolved bearer token (env `DSP_TOKEN` or
the cache, same precedence as everywhere else) verbatim to stdout — the read-out
counterpart to `set-token`. It makes no server round-trip; it exits `3` if no
token is cached for the server, or if the token is locally detected as expired.

```bash
export DSP_TOKEN=$(dsp auth token -s dev)
curl -H "Authorization: Bearer $(dsp auth token -s dev)" https://api.dev.dasch.swiss/...
```

Four things to keep in mind:

1. **It prints a bearer credential to stdout.** Treat the output exactly like a
   password — anything that can read stdout can use your session.
2. **Exit `0` only means "the token looks unexpired locally," not "the server
   will accept it."** There is no live probe (the JWT signature is not verified
   either) — this is an advisory staleness check, not access control.
3. **Avoid `set -x` / `bash -x` / CI "print each command" verbose modes** around
   the `$(…)` idiom above. Shell xtrace prints the command *after* substitution,
   so the resolved token lands in the trace or CI log in plaintext.
4. **Running it bare at a TTY** (no `$(…)`, no redirection) prints the raw token
   to the terminal, which session-logging tools (`script`, tmux `capture-pane`,
   iTerm2 logging) may persist to disk.

## Harvesting a token from the browser (agent flow)

DSP-API has no OAuth flow, so an agent can't drive an interactive login that
involves SSO or 2FA. The robust path is to reuse the token the DSP web app already
holds:

1. Open the DSP app in a browser and let the user log in normally (handling any
   SSO / 2FA).
2. Read the JWT from the browser: `localStorage.getItem("ACCESS_TOKEN")`.
3. Hand it to `dsp-cli`, either as the `DSP_TOKEN` environment variable (one-shot)
   or persisted: `echo "<jwt>" | dsp auth set-token -s prod` (verifies the token
   against the server and caches it like a login would).

`dsp-cli` never opens a browser itself — the harvested JWT is the same credential
`dsp auth login` would obtain.

See also: `dsp docs errors`, `dsp docs identifiers`.
