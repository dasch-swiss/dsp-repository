---
status: accepted
date: 2026-09-16
---

# The repository root is shaped by the three areas of the Trusted Repository

`dsp-repository` is DaSCH's research data repository: a trusted repository in the sense of the OAIS reference model, whose job is the long-term preservation and trustworthy dissemination of humanities research data. OAIS separates the producer who submits data, the archive that preserves it, and the consumer who reads it. The platform follows that separation with three **areas**, and the producer side is deliberately brought under the repository roof rather than left to external tools: everything the designated community needs, including creating and curating the data, is part of the repository. The areas are the platform's bounded contexts, and the boundary that matters most — the sealed archive against everything else — should be the first thing the tree shows. A flat `modules/` hides which context a service belongs to, and Bazel visibility (ADR-0001) keys on directories, so the directory is the boundary (rationale confirmed 2026-09-16).

Every crate therefore lives under exactly one of five root directories, and the `modules/` directory goes away. The three areas sit together under `areas/`, so the root shows the two kinds of thing the platform is made of: the bounded contexts, and the infrastructure they share (layout revised 2026-09-17):

- `areas/deposit/` — the producer side: where a depositing project team creates and edits its data and metadata and RDU reviews it before anything is submitted to the archive. Today the metadata editor. The name is "Deposit", not "Ingest", because "Deposit" is the producer-side vocabulary the platform already uses (depositor, Deposition, DepositAgreement), while "Ingest" is an OAIS functional entity that lives inside the archive, so naming the producer side after it puts the wrong context's word on the door.
- `areas/archive/` — the OAIS archive, working name **Spycherli**: ingest, archival storage and the supporting functional entities; the sealed heart of the platform. No code yet.
- `areas/access/` — the consumer side, OAIS Access: the services that produce dissemination packages for consumers. Today DPE, the Discovery and Presentation Environment; later CPE, the Configurable Presentation Environment, and the other read-side services.
- `shared/` — `shared-{role}` crates that exist only to be depended on by more than one area (`shared-metadata`, the research-metadata contract; `shared-telemetry`, the browser-beacon collector; `shared-fair`, the FAIR exposure engine of ADR-0005). In place since the first phase of DEV-7268 landed on 2026-09-17. The directory was `platform/` in the first version of this record and was renamed before it was created, because Bazel's `@platforms` repository and the conventional `//platforms` package for target and host platform definitions (ADR-0001) would sit beside it and mean something else.
- `mosaic/` — the design system, a component library plus its playground, used by every hypermedia server here.
- `vitrinli/` — the media engine (IIIF Image API rendering, range-served downloads, derivation of Service Files), sipi under its new name, once it moves in. It is a library, not a service and not a capability: a `media` capability in the Deposit Area and one in the Access Area depend on it and give it its area's rules through the traits it accepts (ADR-0003) — the successor of the Lua scripts and configuration that shape sipi today. In the Deposit Area, `media` takes the Originals a depositor uploads in preparation for archiving and, for images, has Vitrinli derive the Service Files the deposit frontends display; in the Access Area, `media` hands Vitrinli the Service Files the archive produced, and nothing else. 
- `chischtli/` — the in-house triplestore engine (an embedded RDF store with graph-granular write dispatch, exact update deltas, full-text search and optional SHACL on update), the replacement for Fuseki, once it moves in. Like Vitrinli it is a library, not a service and not a capability: it stores named graphs for whichever capability owns them and holds no opinion about which graphs exist or who may read them (ADR-0003). In the Access Area it holds the projection of the archive's data products that DPE, CPE and the SPARQL endpoint read, written by the `sync` capability alone; in the Deposit Area it is the working store of data creation, long-term the successor of the active-research platform's triplestore.

Mosaic, Vitrinli and Chischtli are root peers rather than area members because each is used by more than one area: Mosaic by every hypermedia server, Vitrinli by both `media` capabilities, Chischtli by the Deposit and the Access Area moduliths alike (confirmed 2026-09-16).

Within an area, each capability is a directory named by its crate prefix, and the `{service}-{role}` crate convention of `docs/src/repo_structure.md` is unchanged: `areas/access/dpe/core` holds `dpe-core`, `areas/deposit/editor/server` holds `editor-server`. The editor keeps the name "editor" (confirmed 2026-09-16).

The area's composition root (ADR-0003) is `areas/<area>/server`. While an area has one capability, that capability's `server` crate is the composition root and stays inside the capability (`areas/access/dpe/server` is `dpe-server`); it moves up to `areas/access/server` when the second capability arrives, and the capability keeps only what is its own. The area's container image is built beside its composition root: a Dockerfile there until ADR-0001 lands, an image target in the same directory afterwards. Each area carries its `CONTEXT.md` at `areas/<area>/CONTEXT.md`; while the area and its first capability coincide, that is one file (today `modules/dpe/CONTEXT.md` and `areas/deposit/editor/CONTEXT.md`, the latter moved 2026-09-24; the Archive Area's seed is already at `areas/archive/CONTEXT.md`).

Between areas, the rules are:

- **No crate under one area depends on a crate under another area.** Areas integrate over the wire, never by Rust import — today the editor reads DPE's published project files from a directory handed to it as `EDITOR_DATA_DIR`, and approved records are meant to return as a pull request against this repository (documented in `docs/src/editor/architecture.md`; the collection step is not built yet, records wait in `approved_records`); in the target design, the Deposit Area submits packages to Spycherli over an intent protocol (register an intent with a declared manifest, upload to a quarantine bucket, complete, validate), and Spycherli feeds every Access-Area service with pointer messages on a message bus plus immutable objects in object storage.
- **`shared-*`, `mosaic-*`, Vitrinli and Chischtli depend on no area crate**; an area depends on them freely. The dependency arrow is `areas → shared, mosaic, vitrinli, chischtli`.
- **Each service owns its durable state and is its sole reader and writer** — the editor's SQLite database, DPE's data directory and in-process caches, Spycherli's sealed store. A cross-area reference is an opaque identifier (a shortcode, an ARK), resolved through the owning area's published surface. Inside an area, ADR-0003 applies the same rule per capability.
- **A concept shared by two areas lives in `shared/`** (`shared-metadata` is the research-metadata contract both speak); a shape one area happens to need from another does not. Share concepts, never shapes.
- **Preservation storage is exclusive to the Archive Area.** No other area reaches into the sealed store, its log, or the preserved bytes; they receive data products the archive publishes.

## Considered Options

- **Areas grouped under `areas/`, shared infrastructure beside it (chosen).**
- **Seven directories flat at the root** (the first version of this record, 2026-09-16: `deposit-area/`, `archive-area/`, `access-area/`, `platform/`, `mosaic/`, `vitrinli/`, `chischtli/`) — replaced 2026-09-17: three bounded contexts and four pieces of shared infrastructure side by side did not show which were which, and each area needed an `-area` suffix to say what it was. `areas/` says it once.
- **Keep `modules/` as a container above everything** (`modules/areas/…`, `modules/shared/…`) — rejected: one more level in every path and every Bazel label, and the word "module" would survive with no meaning; the root already separates code from `docs/`, `scripts/` and `.github/` by name.
- **Keep the flat `modules/<service>/` layout (today)** — the area a service belongs to is invisible in the tree, and the seam that matters most has no directory of its own.
- **One modulith binary for the whole platform** — rejected. DPE and the editor are separate deployables on separate origins on purpose: on a shared origin an XSS in the public, unauthenticated DPE could drive authenticated editor mutations past the `Sec-Fetch-Site` CSRF control (`docs/src/editor/architecture.md`, "Relationship to DPE"). The modulith is adopted one level down instead — one per area (ADR-0003).
- **`mosaic/`, `vitrinli/` and `chischtli/` under `shared/`** — not chosen. All three are substantial systems with their own vocabulary and test infrastructure (and, for Vitrinli, C++ still being rewritten), not single shared library crates; `shared/` stays the home of `shared-{role}` crates that exist only to be depended on.
- **`platform/` as the name of the shared root** (the first version of this record) — replaced 2026-09-17: it collides with Bazel's platform vocabulary once ADR-0001 lands, and "platform" already carries a second meaning here (the DaSCH Service Platform, see the root `CONTEXT.md`). `kernel/` (the DDD "shared kernel") and `contract/` were considered; `shared/` was chosen as the plain word for what the directory is.
- **Chischtli as a shared service (one triplestore process per area, reached over a socket)** — not chosen for the layout: whether the engine runs in-process or as a supervised sidecar of the area's binary is an operational choice inside the area's modulith and does not change who owns which graph or who may depend on the crate.
- **Vitrinli inside `areas/access/`** — rejected: the Deposit Area's `media` capability depends on it as well, so an area directory would misstate who may depend on it.
- **Vitrinli as a service on its own origin** — rejected: each area needs it against different inputs (Originals in the Deposit Area, archive-made Service Files in the Access Area) and behind its own authentication, which a capability inside each modulith gives for free and a shared service would have to re-invent.
- **Vitrinli mounted directly as a capability in both moduliths** — rejected: it would own tables and routes in two databases and two routers, and it would have to consume area data through ports whose direction is easy to get backwards; a library with an explicit trait interface plus a per-area `media` capability keeps the arrow `areas → vitrinli` unambiguous.
- **Vitrinli configured per area by files, as sipi is by Lua scripts today** — rejected: untyped per-deployment configuration is what no test covers and no agent can trace to an owner; the `media` capability is typed Rust that fails to compile when wired wrongly.
- **`areas/deposit/metadata-editor/`** — rejected: the crates would have to follow (`metadata-editor-core`), touching every commit scope and every runbook for no gain in meaning.

## Consequences

- Everything keyed on `modules/…` moves with the crates in the same change: the build targets, the CI path filters in `.github/workflows/*.yml`, the `bacon.toml` watch lists, the `justfile` recipes, the Dockerfiles, `.github/scripts/check-shared-paths.sh`, `docs/src/repo_structure.md`, the per-module `CLAUDE.md` and `CONTEXT.md` files (which become `areas/<area>/CONTEXT.md`), and the concrete paths in this ADR and in `ARCH-MAP.md`.
- Commit scopes stay crate names (`CONVENTIONS.md`); an area is not a scope.
- The shared root moved first, as the first phase of DEV-7268 (2026-09-17), ahead of the areas and ahead of Bazel: the shared half of the boundary is enforced by Cargo cycles and the paths gate (`check-shared-paths.sh`), neither of which depends on the layout, so nothing there waited on ADR-0001. The two crates became `shared-metadata` and `shared-telemetry` in that change; `shared-fair` (ADR-0005) is created after it and never carried the old prefix.
- "Deposit Area" is the canonical name from here on; earlier design material that says "Ingest Area" means this area, and the root `CONTEXT.md` records the rename under Flagged ambiguities.

Enforced by: Bazel `visibility` per area once ADR-0001 lands (structure). Until then `.github/scripts/check-shared-paths.sh` covers the shared half (static-analysis), and a service-to-service import is caught only in review (review).

## Amendment (2026-09-19) — dsp-cli joins as a sixth root

`dsp-cli/` becomes the sixth root directory, alongside `areas/`, `shared/`, `mosaic/`, `vitrinli/` and `chischtli/`: the platform's command-line client, talking to the VRE over DSP-API today and, over the wire, to the Deposit, Archive and Access moduliths once they exist. Unlike Mosaic, Vitrinli and Chischtli — each a root peer because more than one area depends on it — dsp-cli is used by no area; it is a client of every area, integrating with each over its public HTTP surface exactly as areas integrate with each other. That is a different rationale from the "used by more than one area" test the other three root peers satisfy (line 20), so it earns its own clause rather than riding on theirs.

This carves dsp-cli out of the dependency-arrow sentence at line 29: an area never depends on `dsp-cli`, and `dsp-cli` depends on no area crate. A `shared-*` dependency is permitted only if that crate is itself published to crates.io — `cargo publish` rejects a path dependency on an unpublished crate, and every `shared-*` crate is `publish = false` today, so none is currently reachable from `dsp-cli` — because a future need to share a concept between an area and dsp-cli must be met by publishing the shared crate (or vendoring the concept), not by a path dependency `cargo publish -p dsp-cli --dry-run` would reject anyway.

A command-line client is not a user-facing surface in ADR-0004's sense: ADR-0004 scopes itself to "every capability with a screen in any area's modulith", and a terminal program has no screen in that sense. ADR-0004's Consequences carries a one-line note pointing back here.

dsp-cli lands at the repository root now, independent of DEV-7268: DEV-7268 is the `modules/` → `areas/` move this ADR's other sections describe, and dsp-cli's arrival at the root neither depends on nor waits for it.

Added to Considered Options:

- **`modules/dsp-cli/`** (dsp-cli/ADR-0014's original placement guess) — rejected: `modules/` is itself removed by this ADR, so the crate would move twice.
- **A new `tools/` root** — rejected: a root-level category for one member states nothing that `dsp-cli/` itself does not already state.
- **Inside an area, as a capability** — rejected: dsp-cli talks to every area over the wire, and an area's members are capabilities of its one modulith (ADR-0003); dsp-cli is neither a capability of one area nor owned by one.

Added to Consequences:

- After ADR-0001 removes the root `Cargo.toml` and `Cargo.lock` this workspace uses today, `dsp-cli/Cargo.toml` plus a `dsp-cli/Cargo.lock` remain the publishing manifest — dsp-cli is the one crate here that publishes to crates.io.
- dsp-cli's live tests (needing a running DSP stack) become a `manual`, `no-remote-exec` Bazel test target once Bazel builds this repository — never part of the default `bazel test //...`.
- The drift stack (a pinned dsp-api + fuseki stack running dsp-cli's live tests in CI) stays a local-execution job, not a remote-executed one, for the same reason: it needs a live network stack Bazel's remote execution does not provide.
- dsp-cli/ADR-0014 names this ADR in its own dated amendment ("how the migration was actually executed"); this is the other end of that link.

Enforced by: `cargo publish -p dsp-cli --dry-run` in `check.yml` for the no-unpublished-path-dependency rule (static-analysis), becoming Bazel `visibility` after ADR-0001 (structure); review for the over-the-wire-only integration rule and the placement rationale.

## Amendment (2026-09-24): the Deposit Area half is done

The metadata editor lives at `areas/deposit/editor/`, crate names unchanged (DEV-7373). Its `CLAUDE.md` and `CONTEXT.md` moved with it: while the editor is the Deposit Area's only capability, its `CONTEXT.md` is the area's. DPE's move to `areas/access/dpe/` and Mosaic's move to the root are still open.

The 2026-09-19 amendment calls DEV-7268 "the `modules/` → `areas/` move". It is not: DEV-7268 is the FAIR-metadata ticket, and only its first phase, the shared root, belongs to this migration. No single ticket tracks the area moves; DEV-7373 is the editor's.

The editor still reaches into DPE's tree for the published data set: its tests, its development recipes, the image build, and the collector, which writes that set. Each toolchain names the directory once (`editor_core::DPE_DATA_DIR` for Rust, `DPE_DATA_DIR` in the `justfile`, one constant in the editor's Playwright config, one line in `.github/actions/build-editor`), so DPE's move changes one line per toolchain.

Added to Consequences:

- The gates that glob over application directories match both roots until `modules/` is gone: `check-shared-paths.sh` forbids a shared-crate path into `areas/<area>/` as it does into `modules/<module>/`, and `check-datastar-delimiters.sh`, `verify-checksums.sh` and the `eng.yaml` overrides each carry an `areas/*/*/…` glob beside their `modules/*/…` one. A glob left on `modules/` alone silently drops an application once it moves: the gates' absence checks fire only when nothing at all matches.
