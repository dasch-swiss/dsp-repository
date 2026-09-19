# Update check and self-update

`dsp-cli` is published to crates.io (0.1.0, 2026-07-17). A user who ran
`cargo install dsp-cli` months ago has no signal that a newer version exists.
This ADR records how `dsp` tells the user a newer version is available — and,
deliberately, how far it goes (advise, not self-replace) and why.

Scope note: this ADR covers the **advise-only update check shipping in 0.1.3**
and draws the boundary to the two heavier self-update mechanisms it does *not*
adopt. The heavier mechanism (binary self-replace) is deferred to a
post-migration follow-on (PROJECT_PLAN Phase 11) and gets its own decision record
when it is actually built.

## Context

- **Agent-first (ADR-0001, `idea.md`).** The primary consumer is an LLM agent.
  stdout is data (ADR-0012); an update notice is neither data nor an error, so it
  has no slot in the output — least of all the JSON `_meta`/error envelope
  (ADR-0003, a stable contract). A background network call on every invocation
  also adds latency and is telemetry-adjacent (it phones home).
- **Distribution is crates.io-only.** There are no prebuilt binaries on GitHub
  Releases. Every install is a source build via `cargo install dsp-cli` (README,
  ADR-0011). This is the load-bearing fact for the mechanism choice below.
- **On-disk state precedent.** `~/.config/dsp-cli/auth.toml` (ADR-0007) already
  establishes the config directory and an atomic-write pattern
  (`src/config/auth_cache.rs`).
- **TTY precedent.** `dsp auth login` already gates on `std::io::IsTerminal`
  (`src/actions/auth/login.rs`).

## Decision

### Mechanism: advise-only

Three self-update mechanisms were weighed. `dsp-cli` adopts **mechanism 1
(advise)** for 0.1.3.

1. **Advise — chosen.** Detect a newer version, print the upgrade command to
   stderr. Zero new infrastructure; honest for a crates.io-only tool (the command
   we print is exactly how the user got the binary). The reminder itself is the
   whole feature.
2. **Shell out to `cargo install dsp-cli` — considered, declined for 0.1.3.**
   `dsp` could run the upgrade for the user. It respects crates.io provenance, but
   requires `cargo` + a toolchain on `PATH`, rebuilds from source (slow, surprising
   for a CLI to invoke `cargo`), and is only marginally more convenient than
   printing the one-line command. Not worth its weight now; reopenable if demand
   appears.
3. **Binary self-replace (`self_update` / `self_replace`) — deferred.** The
   `self_update` crate has **no crates.io backend**; it downloads prebuilt
   per-target archives from GitHub/GitLab/S3. `dsp-cli` publishes none. Self-
   replacing a `cargo install`ed binary with a downloaded GitHub binary also
   silently switches install provenance (breaks `cargo install --list` /
   `cargo uninstall` bookkeeping, risks target/glibc mismatch) and obligates a
   cross-platform release-artifact pipeline that does not exist. Bundling it purely
   for self-replace on a crates.io-first tool is over-engineering. **Deferred** —
   see "Deferred: binary self-replace" below.

### Where it runs and what gates it

The check runs **once, from `src/main.rs`**, after the command's action returns
and after its result has been handled — following the `init_tracing`-in-`main`
precedent, **not** threaded through `lib.rs::run()`. The advisory is the last
thing written to stderr. It fires **only** when **all** of:

- the effective output format is **prose** (the default) — suppressed for
  `-j`/`json`/`lines`/`csv`/`tsv`, and for `auth token` (a raw-credential pipe
  command that must never chatter);
- **stderr is an interactive TTY** (`std::io::stderr().is_terminal()`) — no human
  reads a redirected/piped stderr, and an agent's stderr is captured, not a TTY;
- the opt-out env var is **not** set (below);
- a newer stable version is known (freshly fetched, or remembered from the cache).

The format+TTY+opt-out gate is evaluated **before any I/O**, so the common
non-interactive / agent path costs nothing.

### Transport, frequency, politeness

- **crates.io sparse index**, not the REST API:
  `https://index.crates.io/ds/p-/dsp-cli` (the `{ab}/{cd}/{name}` path scheme).
  Newline-delimited JSON, one object per published version; the latest **non-yanked,
  non-prerelease** `vers` is "latest". The index is built for exactly this
  lightweight lookup (ETag-cacheable, no rate-limit policy text). A
  `User-Agent` (`dsp-cli/<version>`) is sent as good citizenship even though the
  index does not mandate it. _(Amended 2026-07-24, plan 033: the update-check
  client now shares the single `crate::util::USER_AGENT` constant used by every
  dsp-cli HTTP client; the former repo-suffixed form
  `dsp-cli/<version> (github.com/dasch-swiss/dsp-incubator)` was simplified to
  this plain form so dsp-cli emits one consistent `User-Agent` everywhere.)_
- **A dedicated `reqwest::blocking` GET**, not the `DspClient` trait — that trait
  is DSP-API-coupled and extending it breaks every mock (CLAUDE.md trait-touch-
  point warning). A short timeout (≈2s) bounds the worst-case latency of the one
  interactive run per day that actually fetches.
- **At most one network fetch per 24h.** A `last_checked` timestamp (and the
  `latest_seen` version) is cached at `~/.config/dsp-cli/update_check.toml`. Within
  the window the reminder still shows every interactive run, from the cached
  `latest_seen` — so once a new version is known the user keeps being reminded
  until they upgrade, without re-hitting the network. `last_checked` is stamped on
  every *attempt* (success or failure), so a persistent network failure retries at
  most once per 24h rather than every command.
- **`semver`** compares the built-in `CARGO_PKG_VERSION` against the fetched
  version; notify iff `latest > current`.

### Opt-out

A standalone env var **`DSP_NO_UPDATE_CHECK`** (set to any non-empty value)
disables the check entirely. It is **not** a per-command flag — an update check is
a cross-cutting, set-once concern; a flag would clutter every command and be
tedious to repeat (owner's call, 2026-07-21). It is read directly via `std::env`
in the update module (after `dotenvy::dotenv()` in `main`, so a `.env` entry is
honored) — **not** through `Config::resolve` (which is server-only). It does **not**
depend on the parked non-secret `dsp.toml` (ADR-0007); when/if `dsp.toml` lands, an
`update_check = false` key there is a natural additional source, but the env var
ships standalone today.

### Non-fatal, out of the output contract

The check **never** changes the command's exit code and **never** blocks fatally.
Any error (network, timeout, parse, cache I/O, unresolvable home dir) is logged at
`tracing::debug` and swallowed. The advisory goes to **stderr in every format**,
never to stdout and never into the JSON `_meta`/error envelope (ADR-0012 /
ADR-0003 stay unamended — the notice is deliberately kept out of the contract).

### Deferred: binary self-replace (post-migration)

Mechanism 3 (true self-replace) is deferred and **gated on first standing up a
prebuilt cross-platform binary pipeline** (per-target GitHub-Release archives on
every tag). That pipeline is a distribution-model addition best done in the
dsp-repository release tooling — hence **post-migration** (PROJECT_PLAN Phase 11
follow-on, owner's steer 2026-07-21). Once prebuilt binaries exist, adding
`self_update`/`self_replace` against the GitHub backend extends this advise-only
check into active self-replace. That work gets its own decision record (and likely
a distribution ADR) when scheduled; this ADR only records that the door is left
open and why it is not walked through in 0.1.3.

## Consequences

- Interactive human users learn about new releases and get the exact upgrade
  command; agents and scripts see nothing (gated off) and pay no latency.
- One new dependency (`semver`, tiny); `reqwest` is already present. A second
  small on-disk cache file joins `auth.toml` in `~/.config/dsp-cli/`.
- The check's correctness-sensitive logic (index parse, staleness, version
  compare, gating) is pure and unit-tested; the network path is wiremock-tested;
  the real crates.io fetch sits behind `--features live` (ADR-0009). The advisory
  never appears in `insta` snapshots because those runs are non-TTY.
- Self-replace remains available as a future extension without re-litigating the
  advise-only decision — it is a strictly additive, infrastructure-gated follow-on.

## Considered alternatives

- **Notice in the JSON `_meta` envelope.** Rejected — `_meta` is a stable public
  contract and an update notice is not command data; keeping it stderr-only (all
  formats) avoids polluting the contract (ADR-0003).
- **Default-on for all runs / no TTY gate.** Rejected — pollutes agent output and
  adds per-invocation latency, contradicting the agent-first premise.
- **Per-command `--check-update` flag instead of an env var.** Rejected — a
  cross-cutting set-once concern; a flag on every command is clutter and bad for
  agent ergonomics.
- **crates.io REST API (`/api/v1/crates/dsp-cli`).** Rejected as the default
  transport — heavier payload and a stricter crawler/User-Agent policy; the sparse
  index is the blessed lightweight path. (The REST API stays a fallback option if
  richer metadata is ever needed.)
- **Bundle `self_update` now.** Rejected for 0.1.3 — no crates.io backend; needs a
  prebuilt-binary pipeline that does not exist; over-engineering for a crates.io-
  first tool. Deferred (above), not abandoned.
