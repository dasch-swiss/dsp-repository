---
status: accepted
date: 2026-09-16
---

# The repository root is shaped by the three areas of the Trusted Repository

`dsp-repository` is DaSCH's research data repository: a trusted repository in the sense of the OAIS reference model, whose job is the long-term preservation and trustworthy dissemination of humanities research data. OAIS separates the producer who submits data, the archive that preserves it, and the consumer who reads it. The platform follows that separation with three **areas**, and the producer side is deliberately brought under the repository roof rather than left to external tools: everything the designated community needs, including creating and curating the data, is part of the repository. The areas are the platform's bounded contexts, and the boundary that matters most — the sealed archive against everything else — should be the first thing the tree shows. A flat `modules/` hides which context a service belongs to, and Bazel visibility (ADR-0001) keys on directories, so the directory is the boundary (rationale confirmed 2026-09-16).

Every crate therefore lives under exactly one of seven root directories, and the `modules/` directory goes away:

- `deposit-area/` — the producer side: where a depositing project team creates and edits its data and metadata and RDU reviews it before anything is submitted to the archive. Today the metadata editor. The name is "Deposit", not "Ingest", because "Deposit" is the producer-side vocabulary the platform already uses (depositor, Deposition, DepositAgreement), while "Ingest" is an OAIS functional entity that lives inside the archive, so naming the producer side after it puts the wrong context's word on the door.
- `archive-area/` — the OAIS archive, working name **Spycherli**: ingest, archival storage and the supporting functional entities; the sealed heart of the platform. No code yet.
- `access-area/` — the consumer side, OAIS Access: the services that produce dissemination packages for consumers. Today DPE, the Discovery and Presentation Environment; later CPE, the Configurable Presentation Environment, and the other read-side services.
- `platform/` — `platform-{role}` crates that exist only to be depended on by more than one service (`platform-metadata`, the research-metadata contract; `platform-telemetry`, the browser-beacon collector).
- `mosaic/` — the design system, a component library plus its playground, used by every hypermedia server here.
- `vitrinli/` — the media engine (IIIF Image API rendering, range-served downloads, derivation of Service Files), sipi under its new name, once it moves in. It is a library, not a service and not a capability: a `media` capability in the Deposit Area and one in the Access Area depend on it and give it its area's rules through the traits it accepts (ADR-0003) — the successor of the Lua scripts and configuration that shape sipi today. In the Deposit Area, `media` takes the Originals a depositor uploads in preparation for archiving and, for images, has Vitrinli derive the Service Files the deposit frontends display; in the Access Area, `media` hands Vitrinli the Service Files the archive produced, and nothing else. 
- `chischtli/` — the in-house triplestore engine (an embedded RDF store with graph-granular write dispatch, exact update deltas, full-text search and optional SHACL on update), the replacement for Fuseki, once it moves in. Like Vitrinli it is a library, not a service and not a capability: it stores named graphs for whichever capability owns them and holds no opinion about which graphs exist or who may read them (ADR-0003). In the Access Area it holds the projection of the archive's data products that DPE, CPE and the SPARQL endpoint read, written by the `sync` capability alone; in the Deposit Area it is the working store of data creation, long-term the successor of the active-research platform's triplestore.

Mosaic, Vitrinli and Chischtli are root peers rather than area members because each is used by more than one area: Mosaic by every hypermedia server, Vitrinli by both `media` capabilities, Chischtli by the Deposit and the Access Area moduliths alike (confirmed 2026-09-16).

Within an area, each service is a directory named by its crate prefix, and the `{service}-{role}` crate convention of `docs/src/repo_structure.md` is unchanged: `access-area/dpe/core` holds `dpe-core`, `deposit-area/editor/server` holds `editor-server`. The editor keeps the name "editor" (confirmed 2026-09-16).

Between areas, the rules are:

- **No crate under one area depends on a crate under another area.** Areas integrate over the wire, never by Rust import — today the editor reads DPE's published project files from a directory handed to it as `EDITOR_DATA_DIR`, and approved records are meant to return as a pull request against this repository (documented in `docs/src/editor/architecture.md`; the collection step is not built yet, records wait in `approved_records`); in the target design, the Deposit Area submits packages to Spycherli over an intent protocol (register an intent with a declared manifest, upload to a quarantine bucket, complete, validate), and Spycherli feeds every Access-Area service with pointer messages on a message bus plus immutable objects in object storage.
- **`platform-*`, `mosaic-*`, Vitrinli and Chischtli depend on no area crate**; an area depends on them freely. The dependency arrow is `areas → platform, mosaic, vitrinli, chischtli`.
- **Each service owns its durable state and is its sole reader and writer** — the editor's SQLite database, DPE's data directory and in-process caches, Spycherli's sealed store. A cross-area reference is an opaque identifier (a shortcode, an ARK), resolved through the owning area's published surface. Inside an area, ADR-0003 applies the same rule per capability.
- **A concept shared by two areas lives in `platform/`** (`platform-metadata` is the research-metadata contract both speak); a shape one area happens to need from another does not. Share concepts, never shapes.
- **Preservation storage is exclusive to the Archive Area.** No other area reaches into the sealed store, its log, or the preserved bytes; they receive data products the archive publishes.

## Considered Options

- **Areas at the root (chosen).**
- **Keep the flat `modules/<service>/` layout (today)** — the area a service belongs to is invisible in the tree, and the seam that matters most has no directory of its own.
- **One modulith binary for the whole platform** — rejected. DPE and the editor are separate deployables on separate origins on purpose: on a shared origin an XSS in the public, unauthenticated DPE could drive authenticated editor mutations past the `Sec-Fetch-Site` CSRF control (`docs/src/editor/architecture.md`, "Relationship to DPE"). The modulith is adopted one level down instead — one per area (ADR-0003).
- **`mosaic/`, `vitrinli/` and `chischtli/` under `platform/`** — not chosen. All three are substantial systems with their own vocabulary and test infrastructure (and, for Vitrinli, C++ still being rewritten), not single shared library crates; `platform/` stays the home of `platform-{role}` crates that exist only to be depended on.
- **Chischtli as a shared service (one triplestore process per area, reached over a socket)** — not chosen for the layout: whether the engine runs in-process or as a supervised sidecar of the area's binary is an operational choice inside the area's modulith and does not change who owns which graph or who may depend on the crate.
- **Vitrinli inside `access-area/`** — rejected: the Deposit Area's `media` capability depends on it as well, so an area directory would misstate who may depend on it.
- **Vitrinli as a service on its own origin** — rejected: each area needs it against different inputs (Originals in the Deposit Area, archive-made Service Files in the Access Area) and behind its own authentication, which a capability inside each modulith gives for free and a shared service would have to re-invent.
- **Vitrinli mounted directly as a capability in both moduliths** — rejected: it would own tables and routes in two databases and two routers, and it would have to consume area data through ports whose direction is easy to get backwards; a library with an explicit trait interface plus a per-area `media` capability keeps the arrow `areas → vitrinli` unambiguous.
- **Vitrinli configured per area by files, as sipi is by Lua scripts today** — rejected: untyped per-deployment configuration is what no test covers and no agent can trace to an owner; the `media` capability is typed Rust that fails to compile when wired wrongly.
- **`deposit-area/metadata-editor/`** — rejected: the crates would have to follow (`metadata-editor-core`), touching every commit scope and every runbook for no gain in meaning.

## Consequences

- Everything keyed on `modules/…` moves with the crates in the same change: the build targets, the CI path filters in `.github/workflows/*.yml`, the `bacon.toml` watch lists, the `justfile` recipes, the Dockerfiles, `.github/scripts/check-platform-paths.sh`, `docs/src/repo_structure.md`, the per-module `CLAUDE.md` and `CONTEXT.md` files, and the concrete paths in this ADR and in `ARCH-MAP.md`.
- Commit scopes stay crate names (`CONVENTIONS.md`); an area is not a scope.
- "Deposit Area" is the canonical name from here on; earlier design material that says "Ingest Area" means this area, and the root `CONTEXT.md` records the rename under Flagged ambiguities.

Enforced by: Bazel `visibility` per area once ADR-0001 lands (structure). Until then `.github/scripts/check-platform-paths.sh` covers the platform half (static-analysis), and a service-to-service import is caught only in review (review).
