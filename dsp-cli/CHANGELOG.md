# Changelog

All notable changes to `dsp-cli` are documented in this file.

## [0.2.1] - 2026-08-07

### Added

- New `dsp vre sparql query` command — a raw SPARQL 1.1 passthrough to the server's underlying
  triplestore (`POST /admin/sparql/query`, dsp-api `e4ef2e348`, DEV-6734 milestone 1). Requires a
  `SystemAdmin` token; off by default per deployment. The query text comes via `--query <text>`,
  `--query-file <path>`, or stdin; `--accept` selects the response media type (an alias table —
  `json` default, `xml`, `csv`, `tsv`, `turtle`, `ntriples`, `jsonld` — or any raw media type
  containing `/`); `--timeout <seconds>` bounds the client-side request timeout down from its
  one-hour default. This is the third command with no `Renderer`/`--format` (after `dsp docs` and
  `dsp auth token`): the response body is relayed byte-verbatim to stdout in whatever media type the
  store negotiated. A non-2xx relay (the store's own rejection, or a dsp-api-side failure) reports on
  stderr instead, exit `1` (or `2`/`3` for a subset of dsp-api's own typed failures) — stdout stays
  empty on failure. New `dsp docs sparql` topic. See `docs/adr/0016-sparql-passthrough.md` and
  amendments to dsp-cli/ADR-0001, dsp-cli/ADR-0003, dsp-cli/ADR-0007, dsp-cli/ADR-0008, dsp-cli/ADR-0009, dsp-cli/ADR-0010, dsp-cli/ADR-0012.

### Changed

- **Control-character sanitisation now covers the C1 range (`U+0080`–`U+009F`) in every command's
  output**, not just the C0 range and DEL. This affects all commands, not only the new SPARQL one:
  prose values (`strip_control_chars`) drop C1 characters, and tabular cells
  (`replace_control_chars`, used by `lines`/`csv`/`tsv`) replace them with a space. The reason is
  `U+009B`, the single-character form of `ESC [` (CSI) — an ASCII-only filter left a working terminal
  control introducer in server-supplied text, so dsp-cli/ADR-0007's sanitisation guarantee was narrower than
  documented. Server data containing C1 characters (vanishingly rare in practice, and not valid in
  XML 1.0 content) will now render with those characters removed or spaced rather than passed
  through. Surfaced by the plan-035 review.

## [0.2.0] - 2026-07-30

### Added

- New `dsp vre vocabulary { list | describe }` noun-group: `list` shows a project's controlled
  vocabularies (DSP-API "lists"), with `--filter` (case-insensitive substring over `name` and every
  label in every language) and an opt-in `--count` (per-vocabulary tree-size disclosure — one extra
  HTTP call per vocabulary, degrading gracefully to a per-row `None` on partial failure, with the cost
  disclosed up front). `describe` resolves a root **or** node IRI — a node IRI (e.g. pasted from
  `resource describe --values`) resolves upward to its vocabulary and marks the addressed node in the
  output, per the new `dsp docs concepts` cross-reference — and renders the **whole** tree, with
  absolute 1-based dotted `number`s (e.g. `1.2.1`) and a per-segment language-fallback `path`
  breadcrumb. `--subtree` narrows rendering to one branch with no extra HTTP calls. All five output
  formats. See the `docs/adr/0001-vocabulary-divergence.md` amendment (new `vocabulary` term).

### Changed

- The `list-item` value-type token — surfaced by `resource describe --values` and `resource-type
  describe`, in the `value_type` field/column across json, csv, tsv, and lines output — is now
  `vocabulary-item`, so the new `vocabulary describe` command and the existing value-rendering
  commands use one consistent word for the same concept. This is a breaking change to already-shipped
  output: any consumer matching on the literal string `list-item` must update to `vocabulary-item`.
  Lands as `0.2.0` per the project's pre-1.0 versioning convention (a breaking change bumps the second
  digit). See the `docs/adr/0013-instance-reads-and-value-rendering.md` amendment.

### Removed

- The agent-facing Claude Code skill is no longer shipped in this repository. It is now
  maintained as the `dsp-cli` skill in
  [`dasch-claude-plugins`](https://github.com/dasch-swiss/dasch-claude-plugins)
  (`misc/skills/dsp-cli/`), distributed via the plugin marketplace. Consequently:
  `skill/SKILL.md` was deleted; the `just install` recipe was removed (it only installed
  the binary and symlinked the skill); the crate's `include` allowlist no longer packages
  the skill; and the `skill/SKILL.md` ↔ clap drift-guard test was removed. dsp-cli/ADR-0011
  amended. No CLI-surface change.

## [0.1.6] - 2026-07-24

### Added

- Every outgoing HTTP request now sends a `User-Agent: dsp-cli/<version>` header so DSP
  server operators can identify dsp-cli traffic (both DSP-API calls and the crates.io
  update check). The update check previously sent a longer, differently-formatted
  `User-Agent`; it now uses the same plain `dsp-cli/<version>` form.

## [0.1.5] - 2026-07-22

### Changed

- Reworked `README.md` into a crates.io landing page aimed at new users rather than contributors: a from-nothing install path (rustup → `cargo install dsp-cli`), a task-oriented quickstart (public reads, then `dsp auth login` and reusing `dsp auth token` in a raw `curl` / `$DSP_TOKEN` call), and a compact command overview; the repo-internal navigation (ADRs, project plan, contributor docs) was dropped. Also fixed the stale "not yet published to crates.io" install note in `skill/SKILL.md`. Documentation only — no functional or CLI-surface changes.

## [0.1.4] - 2026-07-21

### Fixed

- `-j` now emits the structured JSON error envelope (`{"_meta": …, "error": {"kind": …}}`) to stdout for top-level errors (previously a prose `Error: …` line to stderr, even under `-j`); every other format is unchanged (`Error: …` to stderr). `_meta` includes the resolved server when it could be determined from `--server`/`-s` or `DSP_SERVER`, and always includes `exit_code`. See dsp-cli/ADR-0012 and dsp-cli/ADR-0003 amendments.

## [0.1.3] - 2026-07-21

### Added

- Interactive update check: on prose-format, interactive-TTY runs (unless opted out), `dsp` checks the crates.io sparse index for a newer published version and, if one exists, prints a two-line advisory to **stderr** recommending `cargo install dsp-cli` (also mentions `cargo install-update -a` for `cargo-update` users). At most one network fetch per 24h, cached at `~/.config/dsp-cli/update_check.toml`; within the window the reminder still shows every interactive run from the cache. The check never touches stdout, the JSON `_meta`/error envelope, or the exit code, and is entirely invisible to agents and scripts — it is gated on effective format `Prose` **and** an interactive stderr TTY, so `-j`/`-l`/`--format` non-prose runs and any piped/non-interactive invocation never trigger it. Any check failure is swallowed (logged at `tracing::debug` only). Set `DSP_NO_UPDATE_CHECK` to any non-empty value to disable it entirely. See dsp-cli/ADR-0015.

## [0.1.2] - 2026-07-20

### Added

- `--count` flag (default off) on `dsp vre resource-type list` and `dsp vre resource-type describe`, adding per-resource-type instance counts via one extra call to the v3 `GET /v3/projects/{projectIri}/resourcesPerOntology` route. `list --count` adds a `count` column (all five output formats — prose gets a right-aligned COUNT column, json a `count` key, csv/tsv auto-show the column, lines opts in via `--columns count`); `describe --count` adds an `Instances:` line (prose + json only — tabular formats already omit every resource-type-level scalar, so `count` follows the same precedent). Counts are non-deleted but **not** permission-filtered (unlike `resource list`, which is) — a disclosure note is emitted whenever `--count` is used (prose footer, `_meta.note` in json, a stderr line for lines/csv/tsv), via a new `MetaContext.count_caveat` field distinct from the existing dsp-cli/ADR-0007 `filter_warning`. Supersedes the previously-backlogged standalone `dsp vre resource count` idea (see `docs/BACKLOG.md`); the v3-route decision and unfiltered-count semantics are recorded in dsp-cli/ADR-0013.

## [0.1.1] - 2026-07-17

### Fixed

- Reads refused for authentication reasons now return a clear `auth_required` error
  (exit `3`) with a "run `dsp auth login` to (re)authenticate" hint, instead of a bare
  `unexpected status 401` runtime error (exit `1`). The ontology reads (`data-model
  describe`, `data-model structure`, `resource-type list`/`describe`) and other
  endpoints that routed unexpected statuses through the shared error mapper now share
  the `401`/`403` → `auth_required` handling the dump/auth endpoints already had. The
  common trigger is an **expired cached token** (surfaced by `dsp auth status`), which
  is now actionable rather than cryptic. Exit-code change on 401/403 for the affected
  reads: `1` → `3`.

## [0.1.0] - 2026-07-17

First public release. The full v1 command surface — `dsp auth`, `dsp vre project`,
`dsp vre data-model`, `dsp vre resource-type`, `dsp vre resource`, and `dsp docs` — is
implemented, with five output formats and embedded end-user documentation. Everything
below accumulated during pre-release development.

### Added

- crates.io publish metadata in `Cargo.toml`: `homepage`, `keywords`, `categories`, `rust-version` (MSRV 1.92), and a package
  `include` allowlist (`src/**/*`, `docs/topics/*.md`, `skill/SKILL.md`, `README.md`, `LICENSE`, `CHANGELOG.md`) so the packaged
  crate ships only what it needs to compile and describe itself; `publish = false` removed. Verified with `cargo package --list`
  and a clean `cargo publish --dry-run`. dsp-cli/ADR-0011 amended: crates.io publishing is now a recorded decision — owner is the
  `dasch-swiss` GitHub team (added as a crate owner post-publish, not a personal account), publishing starts at 0.1.0.
- Install docs (`README.md`, `skill/SKILL.md`) now prefer `cargo install dsp-cli` as the primary install path, framed as
  available once 0.1.0 is published to crates.io; `cargo install --git ...` remains documented as the pre-publish / from-source
  fallback.

### Security

- Control-character hardening across every human-viewable output format (closes terminal-escape-sequence-injection vectors from malicious or malformed server-controlled text):
  - **prose** — `dsp vre resource describe` and `dsp vre resource list` now strip ASCII control characters (C0 + DEL,
    keeping `\n`/`\t`) from the resource **label** (the `Resource: <label>` header line and the prose label column)
    — the user-entered free-text scalar that 8c had not yet covered (value scalars, field labels, filenames, and list-node labels were already sanitised then).
    Schema-side prose identifiers (data-model / resource-type names) remain out of scope: they are NCName-constrained ontology names, not free text (the Phase 7 descope).
  - **csv / tsv** — all tabular formats now replace every ASCII control character (C0 incl. tab/newline, plus DEL) with a space, joining `lines` (which already did).
    Previously `csv` quoted per RFC-4180 but passed control characters through,
    and `tsv` was identity — so an embedded ESC reached the terminal raw and (for `tsv`) an embedded tab/newline silently corrupted columns.
    This revises the earlier "csv/tsv preserve fidelity" stance (dsp-cli/ADR-0003 amended,
    Phase 8.5); the fidelity cost is negligible (tabular columns are single-line by DSP convention) and `-j` (json) remains for full fidelity.
    `json` is unchanged — `serde_json` escapes all C0 (incl. ESC) as `\uXXXX`, so a raw escape never reaches the terminal.
  - Internal: the shared tabular control-char helper `lines_field` was renamed `replace_control_chars` and relocated to `src/util/text.rs` beside its prose sibling
    `strip_control_chars`.
- Bumped `jsonwebtoken` 9.3.1 → 10.3.0 (Dependabot security update).
  Absorbed the v10 API change: `extract_exp` now uses the crypto-free `jsonwebtoken::dangerous::insecure_decode` in place of the deprecated
  `Validation::insecure_disable_signature_validation` (which also drops the manual algorithm allow-list),
  and v10's now-required crypto backend is pinned to the pure-Rust `rust_crypto` feature.
  `dsp auth status` behaviour is unchanged.

### Fixed

- `dsp vre project dump --help` no longer calls the non-asset portion of a dump "metadata". `--skip-assets` is now described as downloading "only the structured RDF data" — at
  DaSCH "metadata" means the project-level descriptive layer published by the Repository/DPE,
  not the project's RDF research data.
  The distinction is now codified in `CONTEXT.md` (new _Structured data (RDF data)_ and _Binary asset_ glossary entries plus a _Flagged ambiguities_ note reserving "metadata").
- `dsp vre project dump` no longer downloads (or, with `--replace`/`--delete`,
  destroys) another project's dump when the DSP-API's single server-wide dump slot is occupied by a different project.
  The command now detects the owning project from the conflict and refuses, or — with `--replace --discard-other-project` — explicitly discards it.
  See plan 008 and dsp-cli/ADR-0001 (vocabulary), dsp-cli/ADR-0012 (diagnostics).
- `dsp auth status` no longer fails with a cache-load error when `DSP_TOKEN` is set but `auth.toml` is corrupt or unreadable.
  dsp-cli/ADR-0007 says the env token wins regardless of cache state, so a broken cache now falls through to env-auth status.
- `AuthCache::load_from` rejects oversized cache files (> 1 MiB) before reading them into memory.
  Protects against a misconfigured symlink at `~/.config/dsp-cli/auth.toml` pointing at a huge file.
- `AuthCache::save` now best-effort removes the `auth.toml.<pid>` temp sibling when the atomic-rename step fails,
  so failed writes do not leave temp-file residue in `~/.config/dsp-cli/`.
- Error message for an unparseable dump-conflict (409) response no longer leaks DSP-API "export" vocabulary (dsp-cli/ADR-0001).
- `skill/SKILL.md` (the agent-facing skill shipped alongside the binary,
  dsp-cli/ADR-0011) verified against the actual command surface and corrected: documents the `--discard-other-project` dump flag and the one-slot cross-project guard (were undocumented);
  fixes the `data-model list` column list (`last-modified` → `last_modified`,
  adds the missing `is_builtin` — a copied `--columns last-modified` would have failed); documents the global `-v`/`--verbose` flag; and removes DSP-API vocabulary leaks
  ("export", "ontology/ontologies") from agent-facing prose,
  leaving the term only in the deliberate `dsp-cli`↔DSP-API mapping table (dsp-cli/ADR-0001).

### Changed

- `_meta.auth` JSON field now uses unified dsp-cli/ADR-0007 vocabulary across all commands: `anonymous` (no token), `authenticated via DSP_TOKEN` (env token),
  `authenticated as <user>` (cached token with user), or `authenticated` (cached token, no user).
  Previously `dsp auth status`, `dsp auth login`, `dsp auth set-token`, `dsp auth logout`,
  and `dsp vre project dump` emitted the older `logged_in…` / `not_logged_in` family of strings.
  This is a breaking change to the JSON output contract for those five commands; the read commands (`project list/describe`, `data-model *`,
  `resource-type *`) already used the dsp-cli/ADR-0007 vocabulary and are unchanged.
  Expiry semantics are unchanged: `_meta.auth` reflects token presence/origin,
  not validity — an expired cached token still reports `authenticated as <user>` (the richer expiry detail remains in `auth status`'s data output).
- `dsp vre` commands' `--project` help text now lists all three project identifiers (shortcode, shortname, IRI), matching `dsp vre project dump`.
- `dsp auth login --user` now accepts a username or user IRI in addition to an email address.
  The identifier type is auto-detected from the value: an `http(s)://` prefix is treated as a user IRI, a value containing `@` as an email address,
  and any other value as a username; the matching DSP-API JSON key (`iri`, `email`, or `username`) is sent in the authentication request body.
- The JSON error `kind` enum gained two new stable values: `conflict` (a server-side resource is busy or already exists — e.g. a dump already in progress) and `io` (a local
  filesystem operation the user requested failed — e.g. writing the dump file).
  Both map to exit code `1`; the four exit codes are unchanged.
  This is an additive change — existing consumers that ignore unknown kinds are unaffected. dsp-cli/ADR-0012 amended.
- JSON output uses a uniform envelope: `{"_meta": {…}, "data": …}` on success and `{"_meta": {…}, "error": {…}}` on failure,
  with `_meta` always first and deterministic key order.
  The payload is nested under `data` (an object for single results, an array for lists) so the shape generalises to list commands; `server` is no longer duplicated outside `_meta`.
  See dsp-cli/ADR-0003.
- Server-shortcut matching is now case-insensitive (`PROD` resolves like `prod`); literal URLs still pass through with original casing.
- Server-shortcut table updated: dropped the decommissioned `test`, added live `dev` (`https://api.dev.dasch.swiss`) and `demo` (`https://api.demo.dasch.swiss`).
  Reachability verified via `GET /health`.
- `lines` format now replaces every ASCII control character (`char::is_ascii_control()` — the C0 range including tab, newline, CR, NUL, ESC, … plus DEL `\x7f`)
  with a single space.
  Previously `lines` applied no transformation to control characters.
  This hardens one-record-per-line consumers against embedded control characters in server-controlled text (labels, longnames),
  especially when column projection makes those fields first-class `lines` output.

### Added

- `dsp vre resource describe --values` now renders field values in tabular formats (`lines`/`csv`/`tsv`) as **one row per value** — previously these formats showed the metadata
  row only, plus a stderr note pointing to `prose`/`json`.
  Full column set: `label, iri, field, field_label, value_type, value`; the compact default (no `--columns`) is `field, field_label, value_type, value`
  — the resource's `label`/`iri` are constant across every row of a single-resource describe, so they are opt-in via `--columns` rather than repeated by default.
  `prose`/`json` `--values` behaviour is unchanged (they add a values section on top of the metadata envelope; tabular `--values` shows values only). See dsp-cli/ADR-0013.

- `dsp vre resource describe --values` now surfaces per-value comments (`knora-api:valueHasComment` — DSP-API's optional free-text annotation on any value,
  e.g. "reading uncertain"). `prose` always renders a value's comment on an indented line under the value when present (sanitised via the same
  control-character stripping as every other prose scalar). `json` always carries a `comment` key on a value object, but only when the value has one —
  omitted, never `null`, otherwise, so every existing json snapshot is unaffected. Tabular (`csv`/`tsv`/`lines`) adds `comment` to the full column set
  (`--columns label,iri,field,field_label,value_type,value,comment`) but not the compact default — opt-in, since comments are empirically rare and a
  default trailing column would usually be empty. No new flag: comment surfacing rides on the existing `--values`/`--columns` mechanisms. See dsp-cli/ADR-0013.

- `dsp auth token` — print the resolved bearer token to stdout for piping (exit 3 if none is cached or it is locally detected as expired).

- `dsp vre resource list --order-by <field>` — sort results server-side in ascending order by a project-defined field. `<field>` is a field name (e.g. `title`; resolved to its
  field IRI by scanning the resource-type's schema) or a full field IRI (contains `://`; used directly without resolution).
  Ordering is ascending-only (the server sorts ascending; no `--desc` flag).
  Resources with no value for the field sort first.
  Maps to the `orderByProperty` query parameter on the `GET /v2/resources` endpoint (the wire mapping stays at the client boundary — see `dsp docs concepts`).

- `dsp vre resource describe --values` — extend `resource describe` with full field-value rendering behind an explicit flag (default off; metadata envelope only when omitted).
  With `--values`, every field defined on the resource is shown with its human label (when resolvable), field name, and rendered value(s).
  Rendering is available in every format: `prose` and `json` add a values section to the metadata envelope, and the tabular formats (`csv`, `tsv`,
  `lines`) render one row per value (see the tabular `--values` entry above).
  Resolving field and list-item labels triggers additional server requests (one per project ontology,
  one per distinct list node) that are deduplicated and graceful: any label fetch that fails degrades to the local field name or node IRI rather than aborting the describe.
  Prose output runs every server-supplied scalar through control-character sanitisation before printing; json keeps raw server values verbatim (dsp-cli/ADR-0003 fidelity).
  See dsp-cli/ADR-0013 (instance reads and value rendering).

- `dsp vre resource describe --resource <internal-iri> [-p/--project <guard>] [--format <f> | -j | -l] --server <s>` — fetch the metadata envelope of a single resource by its
  internal IRI (ARK addressing is not supported in v1).
  Returns label, resource-type, IRI, ARK URL, creation date, last-modification date, attached project, owner,
  and two translated permission facets: `visibility` (who can see the resource: `public` / `public (restricted view)` / `logged-in users` / `project members only`) and
  `your_access` (what the caller can do: `restricted view` / `view` / `edit` / `delete` / `manage`).
  Both facets are derived at the client boundary from the server's permission data; raw ACL strings and `knora-admin:` group names never appear in output. `-p`/`--project` is an
  optional cross-project guard: when supplied,
  the command fails if the resource's attached project does not match.
  Filter-disclosure note in output (dsp-cli/ADR-0007): anonymous callers see "results may be filtered; login to see private resources"; authenticated callers see "results limited to your
  permissions".
  Five output formats; tabular columns: `label`, `iri`, `resource_type`, `ark_url`, `creation_date`, `last_modified`, `attached_project`, `owner`, `visibility`, `your_access`.
  Uses `GET /v2/resources/<iri>?schema=complex` (complex schema for all resource reads, per plan 023 D4).

- `dsp vre resource list --project <p> --resource-type <name-or-IRI> [--data-model <m>] [--page N | --all] [--filter <text>] [--format <f> | -j | -l] --server <s>`
  — list resource instances of a given resource-type within a project.
  The first instance-side read command. `--resource-type` accepts a local name (resolved by scanning the project's data-models) or a full IRI (used directly,
  skipping the scan). `--data-model` narrows the scan to a single data-model and resolves cross-data-model name ambiguity.
  Pagination: `--page N` fetches one page (default page 0); `--all` fetches all pages and accumulates the results. Five output formats.
  Filter-disclosure note in output: anonymous callers see "results may be filtered; login to see private resources"; authenticated callers see "results limited to your
  permissions" (dsp-cli/ADR-0007).
  JSON `_meta` pagination keys: `page` + `may_have_more_results` (single-page mode) or `pages_fetched` + `may_have_more_results` (--all mode) — see dsp-cli/ADR-0003 amendment.
  The command uses the full resource representation,
  so output includes `creation_date` and `last_modified` columns; `last_modified` is absent for resources that have never been modified (server-side optional).

- `--help` for `dsp vre project describe`, `dsp vre data-model list`, `dsp vre data-model describe`, `dsp vre data-model structure`,
  and `dsp vre resource-type list` now includes a "See also: `dsp docs concepts`" pointer to the centralised dsp-cli↔DSP-API vocabulary mapping (`data-model`↔`ontology`,
  `resource-type`↔`class`, `field`↔`property`).
  The other schema-noun commands (`project list`, `resource-type describe`) already carried this pointer; every schema-noun command now links to it.

- `--columns=X,Y,Z` — select and reorder output columns for `csv`, `tsv`, and `lines` output (Phase 6.6). User order wins; duplicates are rejected as a usage error.
  On `lines`, selects from the full column set (overrides the lean default subset). Not supported for `json` or `prose` — exits with a usage error and a hint.
  Valid column names per command are listed in `--help` under `Columns (--columns):`.
  See dsp-cli/ADR-0003 amendment for the rename rationale (`--fields` → `--columns`) and the full contract.

- `--no-header` and `--header-only` — header-control flags for `csv` and `tsv` output (Phase 6.6). `--no-header` suppresses the header row,
  enabling row-level concatenation across invocations (`--format csv --no-header >> all.csv`). `--header-only` emits only the header row and exits 0; note the server fetch still
  happens in v1.
  The two flags are mutually exclusive; using either with `prose`, `lines`, or `json` exits with a usage error. Compose with `--columns` to emit or suppress a projected header.

- `dsp docs -j` — machine-readable JSON topic index (Phase 6.6).
  Emits the standard envelope `{"_meta": {}, "data": [{"name": …, "summary": …}, …]}` with one entry per topic (name + summary only; bodies remain
  raw markdown via `dsp docs <topic>`).
  `_meta` is the empty object `{}` — no server or auth context applies to embedded documentation; this is the one command with an empty `_meta` (see dsp-cli/ADR-0003 amendment).
  `-j` conflicts with the `<topic>` positional and `--pager` (clap-level, exit 2). Uses the same compact `serde_json::to_string` style as all other `-j` output.
  Implemented via derived `Serialize` structs (`DocsJsonEnvelope` + `TopicIndexEntry`) in `src/actions/docs.rs`; the no-`Renderer` design (plan 018 D1) is unchanged.

- `dsp docs [topic] [--pager]` — embedded end-user documentation (Phase 6). `dsp docs` lists the available topics with one-line descriptions; `dsp docs <topic>` prints
  the topic's raw markdown to stdout; `--pager` pages it through `$PAGER` (default `less`).
  Topic resolution is strict exact-match — an unknown name exits `1` (not found) with a "did you mean …" suggestion for close typos and a pointer to `dsp docs`.
  Nine topics ship, embedded at compile time via `include_str!`: `dsp-cli`, `dsp`, `concepts`, `identifiers`, `connecting`, `output`, `workflows`, `errors`, `dsp-tools`.
  This expands dsp-cli/ADR-0010's original six-topic catalog (amended in the same change) with three operational topics — `workflows`, `identifiers`,
  `errors` — that teach an agent how to *operate* the CLI, not just describe it.
  `dsp docs` is the one command with neither a network client nor a `--format` flag (its output is format-agnostic markdown).
  Note: the `errors`/`output` topics document current behaviour — errors print as prose to stderr in every format; the structured JSON error envelope (dsp-cli/ADR-0012) is implemented in
  the renderer but not yet wired into the binary.

- `dsp vre data-model structure --project <p> --data-model <dm> [--include-builtins] [--format …] --server <s>` — edge-centric relations overview for a data-model: emits a flat
  list of `link` relations (link fields between resource-types, labelled with the field name; cross-model targets tagged `[to <dm>]`) and `inherits` relations (superclass edges).
  Combines both relation kinds in a single `/v2/ontologies/allentities` fetch — no extra API call.
  Five output formats: `prose`, `json`, `lines`, `csv`,
  `tsv`. `--include-builtins` reveals system superclasses and built-in link fields that are hidden by default. **v1 limitation:** link fields that are *defined in a sibling
  data-model and only reused* here are omitted (no sibling-ontology fetch); the common case — a field defined in this data-model pointing to a resource-type in a sibling — is
  fully covered via the `[to <dm>]` cross-model tag.

- `dsp auth status` now emits the auth-state disclosure line to stderr for the `lines`/`csv`/`tsv` formats (uses the unified dsp-cli/ADR-0007 disclosure vocabulary,
  e.g. `[authenticated via DSP_TOKEN on <server>]`, aligned with `dsp vre project list` by the `_meta.auth` harmonization below).

- `dsp vre resource-type describe --project <p> --data-model <m> --resource-type <name|IRI> [--include-builtins] --server <s>` — the v1 leaf read command: shows a resource-type's
  full field list (name,
  value-type, cardinality, label), its representation kind for asset types, and its project superclass (`Extends:`).
  `--include-builtins` also lists the inherited platform fields (arkUrl, permissions, timestamps, …); cross-data-model fields are resolved and tagged `[from <dm>]`.
  Adds a `-p` shortcut for `--project`.

- `dsp vre resource-type list [--include-builtins] [--filter <text>] --project <shortcode|shortname|IRI> --data-model <name-or-IRI> --server <s>`
  — list all resource-types defined in one data model of a project. `--project` / `-p` accepts a shortcode,
  shortname, or IRI; `--data-model` accepts the data-model's short name (e.g. `beol`,
  case-insensitive) or its full IRI. `--filter` matches case-insensitively client-side against name and label.
  By default only resource-types defined in the project's own data model are shown; `--include-builtins` appends the four user-instantiable platform built-ins (`Region`,
  `AudioSegment`, `VideoSegment`, `LinkObj`) to the output.
  Note: the deprecated `knora-base:Annotation` built-in is intentionally excluded even with `--include-builtins` — in current DaSCH tooling,
  annotation-style resources are expressed as `Region`, `AudioSegment`, or `VideoSegment`.
  Five output formats: `prose` (default), `json`, `lines`, `tsv`, `csv`. Columns: `name`, `iri`, `label`, `is_builtin`. Prose appends a `(built-in)` marker on built-in rows.
  Reuses the `/v2/ontologies/allentities` read established by `data-model describe`. Authentication is optional — public project schemas are returned without a token.
  Does **not** return the field list for each resource-type — use the future `resource-type describe` for that.
  Establishes the `ResourceType` list-projection model (name/iri/label/is_builtin) alongside the existing `ResourceTypeSummary` (the describe-child); first command to surface a
  top-level resource-type list and the `builtin_resource_types()` helper for the 4 user-instantiable built-ins.

- `dsp vre data-model describe --project <shortcode|shortname|IRI> --data-model <name-or-IRI> --server <s>` — describe a single data model and summarise its resource-types.
  `--project` / `-p` accepts a shortcode, shortname, or IRI; `--data-model` accepts the data-model's short name (e.g. `beol`, case-insensitive) or its full IRI.
  The command resolves the project, lists its data-models, matches `--data-model`,
  then fetches the full schema from the DSP-API `/v2/ontologies/allentities` JSON-LD endpoint and filters it to resource-types (`knora-api:isResourceClass`).
  Five output formats: `prose` (default), `json`, `lines`, `tsv`, `csv`.
  Prose shows a `Data-model: <name>` header, a label/value block (label, IRI, last-modified as a date),
  and the resource-types as an aligned `name  label` list under a `Resource-types (N):` header (the names are the identifiers to pass to the future `resource-type describe`); JSON
  is the dsp-cli/ADR-0003 single-object envelope with a `resource_types` array carrying each resource-type's `name`,
  `iri` (CURIE-expanded to a full IRI via the response `@context`),
  and `label`; the tabular formats carry the data-model's scalar fields with `resource_types` as a count (a nested list can't fit a cell), mirroring `project describe`.
  An unknown `--data-model` yields a `not_found` error with a "run `data-model list`" hint.
  Authentication is optional per dsp-cli/ADR-0007 (the endpoint is public); the auth-state disclosure (prose footer / `_meta.auth` / stderr line) mirrors the other read commands.
  Establishes the `DataModelDetail` + `ResourceTypeSummary` models and the `Renderer::data_model_describe` method; first command to read `/v2/ontologies/allentities` and to
  surface resource-types. (Inter-resource-type relations — the link graph — are deferred to a separate planned view; see `docs/PROJECT_PLAN.md`.)

- `dsp vre data-model list [--include-builtins] [--filter <text>] --project <shortcode|shortname|IRI> --server <s>` — list all data models for a project. `--project` / `-p`
  accepts a shortcode,
  shortname, or IRI (same three identifiers as all `vre` commands). `--filter` matches case-insensitively client-side against name and label.
  By default only project-defined data models are shown; `--include-builtins` adds the three platform built-ins (`knora-api`, `standoff`, `salsah-gui`) to the output.
  Five output formats: `prose` (default), `json`, `lines`, `tsv`, `csv`.
  Columns: `name`, `iri`, `label` (the human-readable English label from the ontology JSON-LD metadata), and `last-modified` (ISO-8601 timestamp, date-only in `prose`).
  dsp-cli/ADR-0007 auth-state disclosure: prose footer, `_meta.auth` in JSON, stderr disclosure line for `lines`/`tsv`/`csv`.
  First command to expose the built-in vs project-defined distinction; establishes the `DataModelListView` view-struct and `Renderer::data_models` method,
  and the `/v2/ontologies/metadata` JSON-LD read pattern.

- `dsp vre project describe --project <shortcode|shortname|IRI> --server <s>` — fetch the full detail of a single DSP project. `--project` / `-p` accepts a shortcode, shortname,
  or IRI (the same three identifiers that `dump` uses); the server resolves the project by whichever form is supplied.
  Five output formats are supported: `prose` (default), `json`, `lines`, `tsv`, `csv`.
  Prose shows a compact `Project: <shortname> (<shortcode>)` title line (short identifiers in the header; the longname appears as a `Name:` labeled field, omitted when absent)
  followed by a label/value block: IRI, status (active/inactive), keywords (comma-joined), a data-models summary (`Data-models (N): name1, name2, …` with names sorted
  alphabetically and derived from each data-model's IRI), and the description rendered as plain text — HTML converted (links shown as `text (url)`, tags stripped, control chars
  and ANSI escapes removed) rather than raw HTML.
  JSON exposes the same fields with `data_models` as an array of objects carrying both the data-model `name` and `iri` (enabling chaining into the future `data-model describe`
  command), and the description value is kept raw (lossless).
  The tabular formats are scalar projections consistent with `project list`: `csv` and `tsv` emit shortcode, shortname, longname, status, data-model count,
  and IRI; `lines` emits shortcode, shortname, and longname (tab-separated, no header).
  Authentication is optional per dsp-cli/ADR-0007: a cached or env token is sent when present; anonymous callers see the same public metadata.
  The auth-state disclosure (prose footer / `_meta.auth` / stderr line) mirrors `project list`.
  First describe (single-object) command; the `data` JSON payload is a single object (not an array), per dsp-cli/ADR-0003.

- `dsp vre project list [--filter <text>] --server <s>` — list all projects on a DSP server. `--filter` (short: none; argument is a plain substring) matches case-insensitively
  against shortcode, shortname, and longname; filtering is client-side.
  Five output formats are supported: `prose` (default), `json`, `lines`, `tsv`, `csv`.
  Auth-state disclosure per dsp-cli/ADR-0007: the prose footer and the `_meta.auth` JSON field use the `anonymous` / `authenticated as <user>` vocabulary from `read_auth_state`; the
  `lines`/`tsv`/`csv` formats emit a `[<auth-state> on <server>]` line to stderr (data stays on stdout).
  The first read command; establishes the per-noun `Renderer::projects` + `ProjectListView` view-struct pattern.

- `--discard-other-project` flag for `dsp vre project dump`: when used with `--replace`,
  discards a dump that belongs to a different project to free the single server-wide slot. `--replace` refuses cross-project without it.
  Requires `--replace`; conflicts with `--delete`. See plan 008 and dsp-cli/ADR-0001.
- `dsp auth set-token --server <s>` — cache a pre-issued bearer JWT (read from stdin) after verifying it against the server; fills the gap where `DSP_TOKEN` overrides but never
  persists a token.
- `dsp vre project dump` — trigger, poll, and download a project Dump (a server-produced bagit-zip archive containing all project data and, by default, its binary assets).
  Flags: `--project` / `-p` (required; accepts shortcode, shortname, or IRI), `--server` / `-s`, `--skip-assets` (maps to `skipAssets=true`),
  `--output` / `-o` (default `./<shortcode>-<UTC>.zip`), `--force` (allow overwriting an existing output file),
  `--cleanup` (delete the server-side dump after a successful download; errors are non-fatal), `--timeout` (max polling seconds; default 3600),
  `--replace` (discard an existing dump and create a fresh one; force a new version), `--delete` (remove the project's existing dump without downloading).
  `--replace` and `--delete` are mutually exclusive.
  Uses `Authorization: Bearer` — the first command to send a real auth header; requires a system-administrator token. **Existing-dump handling (idempotent adopt by default):**
  re-running `dsp vre project dump` when a dump already exists adopts the existing dump — downloading it if complete or polling it if still in progress.
  The existing dump's id and owning project are discovered from the 409 conflict response body (the API has no list endpoint); the existing dump's id and owning project are
  extracted from `errors[0].details.id` and `projectIri` in the response. `--replace` discards a completed or failed dump and creates a fresh one; `--delete` removes an existing
  dump without downloading.
  Progress events (triggered, polling, downloading, done, adopting, deleting) go to stderr; the final written-path line goes to stdout only.
  All five output formats supported (`prose`, `json`, `lines`, `csv`, `tsv`); in `json` mode, stderr progress is NDJSON.
  Downloads are written atomically (temp file renamed on success; no partial file on failure).
  Output for all five formats covered by `insta` snapshots; HTTP paths covered by `wiremock` tests; one `--features live` test (skipped without a configured admin token + test
  project).
  See dsp-cli/ADR-0007 (auth/bearer token resolution), dsp-cli/ADR-0008 (architecture), dsp-cli/ADR-0012 (new `conflict` and `io` diagnostic kinds used by this command).
- `DSP_TOKEN` env var now overrides the cached token (bearer auth) — login is skipped when it is set.
  `dsp auth status` reports when `DSP_TOKEN` is in effect (`_meta.auth: "authenticated via DSP_TOKEN"` in JSON, prose equivalent in default format) and shows the token's
  expiry when the JWT `exp` claim is readable.
- `-s` short alias for `--server` (all commands) and `-u` for `--user` (`auth login`).
- `DSP_USER` env var (and `.env`) supplies `--user`; `DSP_PASSWORD` env var (and `.env`) supplies the login password non-interactively
  (resolution order: `DSP_PASSWORD` → prompt → stdin).
  `DSP_PASSWORD` is documented as local/dev/test only — never a production password; prefer `DSP_TOKEN` for real environments.
- `dsp auth login`, `dsp auth status`, and `dsp auth logout` with TTY/non-TTY
  password prompt, reqwest-blocking HTTP client (first real `DspClient` impl),
  and `wiremock` test pattern. The `auth.toml` cache schema gained optional
  `user`, `acquired_at`, and `expires_at` fields (legacy token-only entries
  still parse). The `Renderer::diagnostic` method signature changed to
  `Result<(), Diagnostic>` (resolves the `BACKLOG.md` entry from PR #3).
- `AuthCache` module at `src/config/auth_cache.rs` — load/save helpers for the
  on-disk token cache at `~/.config/dsp-cli/auth.toml` (file mode `0600` on Unix,
  server-URL-keyed TOML). Now consumed by the `dsp auth` commands listed above. See
  [dsp-cli/ADR-0007](docs/adr/0007-auth-and-environments.md).
- `--format` / `-j` / `-l` flags on the six `dsp vre` data commands (`project {list,describe}`, `data-model {list,describe}`, `resource-type {list,describe}`).
  Flags parse and dispatch to one of five renderer stubs (`prose` default, plus `json`, `lines`, `csv`, `tsv`) — no rendering behaviour yet; that lands with real data in Phase 3.
  See [dsp-cli/ADR-0003](docs/adr/0003-chaining-and-output.md) and [dsp-cli/ADR-0008](docs/adr/0008-internal-architecture.md).
- `Config` resolution layer: resolves the active DSP server from `--server` flag, `DSP_SERVER` environment variable,
  or a `.env` file in the current working directory (loaded via `dotenvy`).
  Supports the built-in shortcuts (`prod`, `stage`, `dev`, `demo`, `rdu`, `ls-prod`, `ls-test`, `local`, case-insensitive); any other value is treated as a literal URL.
  Module only — wiring into action signatures lands incrementally as each command is implemented. See [dsp-cli/ADR-0007](docs/adr/0007-auth-and-environments.md).
- Nested clap subcommand structure for `auth` (`login`, `status`, `logout`), `vre` (`project`, `data-model`, `resource-type` — each with `list` and `describe`), and `docs`.
  The `auth` subcommands now dispatch to real implementations (see the top entry); `vre` and `docs` remain help-text-only and return `Diagnostic::not_implemented`.
- Design phase: 12 ADRs covering vocabulary divergence, command shape, chaining and output, dsp-tools boundary, language choice (Rust), top-level shape, auth/environments,
  internal architecture, testing strategy, embedded documentation, distribution, and diagnostics.
- `CONTEXT.md` glossary (project, data-model, resource-type, resource, field, value, value-type, built-in) with relationships and flagged ambiguities.
- `docs/dev/` contributor docs: domain-language practices, coding conventions, testing strategy, review guidelines, git workflow, AI agent setup.
- `docs/PROJECT_PLAN.md` with per-command implementation order.
- `docs/BACKLOG.md` holding pen for surfaced ideas.
- `skill/SKILL.md` Claude Code skill (not yet wired up to `just install`).
