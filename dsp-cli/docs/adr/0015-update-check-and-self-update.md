# `dsp` advises on a newer release and does not update itself

`dsp-cli` is published to crates.io, and a user who ran `cargo install dsp-cli` months ago has no signal that a newer version exists. `dsp` therefore tells an interactive user when a newer stable version is available. It goes no further than that: it advises, and does not replace itself.

## Context

- **Agent-first (dsp-cli/ADR-0001).** The primary consumer is an LLM agent. stdout is data (dsp-cli/ADR-0012); an update notice is neither data nor an error, so it has no slot in the output, least of all in the JSON `_meta` and error envelope (dsp-cli/ADR-0003), which is a stable contract. A background network call on every invocation also adds latency and is telemetry-adjacent: it phones home.
- **Distribution is crates.io-only.** There are no prebuilt binaries on GitHub Releases. Every install is a source build via `cargo install dsp-cli` (dsp-cli/ADR-0011). This is the load-bearing fact for the choice of mechanism.

## Decision

### Mechanism: advise only

`dsp` detects a newer version and prints the upgrade command to stderr. This needs no new infrastructure and is honest for a crates.io-only tool: the command it prints is exactly how the user got the binary. The reminder is the whole feature.

### Where it runs and what gates it

The check runs once, from `src/main.rs`, after the command's action returns and its result has been handled, not threaded through `lib.rs::run()`. The advisory is the last thing written to stderr. It fires only when all of these hold:

- the effective output format is **prose**, the default. It is suppressed for `-j`, `json`, `lines`, `csv` and `tsv`, and for `auth token`, a raw-credential pipe command that must never chatter;
- **stderr is an interactive TTY** (`std::io::stderr().is_terminal()`): no human reads a redirected or piped stderr, and an agent's stderr is captured, not a TTY;
- the opt-out environment variable is not set;
- a newer stable version is known, freshly fetched or remembered from the cache.

The format, TTY and opt-out gate is evaluated before any I/O, so the common non-interactive and agent path costs nothing.

### Transport, frequency, politeness

- **The crates.io sparse index**, not the REST API: `https://index.crates.io/ds/p-/dsp-cli`. It returns newline-delimited JSON, one object per published version; the latest non-yanked, non-prerelease `vers` is "latest". The index is built for exactly this lightweight lookup (ETag-cacheable, no rate-limit policy text). The request carries the shared `crate::util::USER_AGENT` (`dsp-cli/<version>`), the one `User-Agent` every dsp-cli HTTP client sends.
- **A dedicated `reqwest::blocking` GET**, not the `DspClient` trait: that trait is DSP-API-coupled, and extending it breaks every mock. A short timeout (about 2s) bounds the worst-case latency of the one interactive run per day that actually fetches.
- **At most one network fetch per 24 hours.** A `last_checked` timestamp and the `latest_seen` version are cached at `~/.config/dsp-cli/update_check.toml`. Within the window the reminder still shows on every interactive run, from the cached `latest_seen`, so a user who knows of a new version keeps being reminded until they upgrade, without re-hitting the network. `last_checked` is stamped on every attempt, success or failure, so a persistent network failure retries at most once per 24 hours rather than on every command.
- **`semver`** compares the built-in `CARGO_PKG_VERSION` with the fetched version; the notice is shown if and only if `latest > current`.

### Opt-out

The standalone environment variable **`DSP_NO_UPDATE_CHECK`**, set to any non-empty value, disables the check entirely. It is not a per-command flag: an update check is a cross-cutting, set-once concern, and a flag would clutter every command and be tedious to repeat. The variable is read directly through `std::env` in the update module, after `dotenvy::dotenv()` in `main` so a `.env` entry is honoured, and not through `Config::resolve`, which is server-only. It does not depend on a non-secret config file (`dsp.toml`, dsp-cli/ADR-0007). If one is added, an `update_check = false` key there is a natural additional source.

### Non-fatal, out of the output contract

The check never changes the command's exit code and never blocks fatally. Any error (network, timeout, parse, cache I/O, unresolvable home directory) is logged at `tracing::debug` and swallowed. The advisory goes to **stderr in every format**, never to stdout and never into the JSON `_meta` and error envelope: the notice is deliberately kept out of the output contract of dsp-cli/ADR-0003 and dsp-cli/ADR-0012.

## Consequences

- Interactive human users learn about new releases and get the exact upgrade command; agents and scripts see nothing and pay no latency.
- One small dependency (`semver`) is added, and a second on-disk cache file joins `auth.toml` in `~/.config/dsp-cli/`.
- The correctness-sensitive logic (index parse, staleness, version comparison, gating) is pure and unit-tested; the network path is wiremock-tested; the real crates.io fetch sits behind the `live` feature (dsp-cli/ADR-0009).
- Binary self-replace stays available as a future extension without reopening the advise-only decision. It is strictly additive and gated on infrastructure that does not exist yet.

## Considered Options

- **Binary self-replace** (`self_update` / `self_replace`): deferred, not rejected. The `self_update` crate has no crates.io backend; it downloads prebuilt per-target archives from GitHub, GitLab or S3, and dsp-cli publishes none. Replacing a `cargo install`ed binary with a downloaded one would also silently switch install provenance (breaking `cargo install --list` and `cargo uninstall` bookkeeping, and risking a target or glibc mismatch), and it obliges a cross-platform release-artifact pipeline. It comes back once a prebuilt cross-platform binary pipeline exists, per-target archives on every release, built in the repository's release tooling. It then gets its own decision record, most likely together with a distribution decision.
- **Shell out to `cargo install dsp-cli`**: deferred, not rejected. It respects crates.io provenance, but it needs `cargo` and a toolchain on `PATH`, rebuilds from source (slow, and surprising for a CLI to invoke `cargo`), and is only marginally more convenient than printing the one-line command. It is reopened if demand appears.
- **A notice in the JSON `_meta` envelope**: rejected. `_meta` is a stable public contract and an update notice is not command data.
- **Default-on for all runs, no TTY gate**: rejected. It pollutes agent output and adds per-invocation latency, contradicting the agent-first premise.
- **A per-command `--check-update` flag instead of an environment variable**: rejected. It is a cross-cutting set-once concern, and a flag on every command is clutter and bad for agent ergonomics.
- **The crates.io REST API** (`/api/v1/crates/dsp-cli`): rejected as the default transport, for a heavier payload and a stricter crawler and `User-Agent` policy. It stays a fallback if richer metadata is ever needed.

Enforced by: the unit tests in `dsp-cli/src/update/mod.rs` (the format, TTY and opt-out gate, index parsing, the cached version and the version
comparison) and `dsp-cli/tests/update_check.rs` (the sparse-index fetch, the plain `User-Agent`, failures returning no version), run by `just test`
(**static-analysis**); the stderr-only, never-in-the-envelope placement, the 24-hour fetch window and the exit code staying untouched, by **review**.
