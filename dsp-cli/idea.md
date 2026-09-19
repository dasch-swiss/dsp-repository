# dsp-cli

A simple, AI-agent-friendly CLI for interacting with the DaSCH Service Platform (DSP).

## Context

DaSCH (Swiss National Data and Service Center for the Humanities) provides digital infrastructure for creating, archiving,
and publishing qualitative research data in the humanities, named the DaSCH Service Platform (DSP).

The DSP consists of two main areas: the **DSP VRE** (Virtual Research Environment; for data creation) and the **DSP Repository** (for long-term archival and data publication).
The repository is currently being built; the VRE already exists and presently serves as both the archive and the presentation environment until the repository takes that role over.

For that reason, the initial scope of `dsp-cli` focuses exclusively on the VRE.

Existing ways to interact with the DSP:

- **DSP-APP** — an Angular web application for viewing and editing data on the DSP.
  The main client for humans, not suitable for AI agents.
- **DSP-TOOLS** — a Python CLI (and library) for initialising projects, creating data models from declarative files, and bulk-ingesting data.
  File-driven; serves specialised use cases.
- **DSP-API** — the public HTTP API both clients use. Partially RESTful; many endpoints operate on RDF formats.
  Exposes much of the system's internal complexity and produces verbose responses that burn AI-agent context.

`dsp-cli` is a third, complementary interaction surface — built for the gap between DSP-APP (human-only) and DSP-API (verbose and complex).
It is not a replacement for either; it sits next to `dsp-tools` with a sharp scope boundary (see ADR-0004).

## Vision

`dsp-cli` provides access to the DSP using **high-level verbs that abstract away DSP-API's internal complexity**.
It uses terminology that aligns with the language used by domain experts and end users (researchers), and produces output that is compact and both human- and AI-readable.
It prefers prose responses by default but offers structured formats (JSON, CSV, TSV, line-based) on demand.
It is structured around a nested hierarchy of noun-groups with meaningful help text at each level, enabling progressive discovery without context bloat.

### Design principles

1. **Agent-first ergonomics.** Every choice (vocabulary, output shape, command structure, error handling) is evaluated against
   "does this make sense to an LLM agent reading it cold?" Humans are a secondary audience, served well by the same choices.
2. **Prose by default, structured on demand.** Default output is rich, contextual prose with next-step hints.
   Structured formats (`-j`, `-l`, `--csv`, `--tsv`) are one flag away for piping into standard Unix tooling.
3. **Agent-mediated chaining, no implicit session state.** Each command stands alone; the agent reads output, extracts identifiers, constructs the next command.
   The CLI itself has no piping primitives and no session-state between invocations.
4. **Domain-expert vocabulary, not RDF jargon.** We say `data-model`, `resource-type`, `field`, `value`, `value-type` — terms that map to how researchers describe their work,
   not RDF/OWL terminology smuggled through from DSP-API. The translation happens once, at the API boundary.
5. **Explicit beats implicit.** No default server; every command must target one. No silent fallbacks. Errors say what's missing and how to provide it.
6. **Documentation lives with the code.** Domain vocabulary, design decisions, and end-user docs are all in this repository.
   The CLI embeds its end-user docs in the binary (`dsp docs <topic>`), so version sync is automatic.

## Shape

```
dsp auth   ...     # authentication (login / logout / status / set-token / token)
dsp vre    ...     # VRE operations (this is the v1 scope)
dsp docs   ...     # embedded end-user documentation
dsp repo   ...     # Repository operations — reserved, not v1
```

The VRE noun-groups in v1:

```
dsp vre project        list | describe
dsp vre data-model     list | describe | structure   --project <p>
dsp vre resource-type  list | describe               --project <p> --data-model <m>
dsp vre resource       list | describe               --project <p>
```

`describe` on a resource-type returns the full field list (name, value-type, cardinality, label) — the v1 leaf, designed so workflow 1 ("what does this data model look like?")
is answered in one call. Beyond the original four columns, `describe` also surfaces the resource-type's **representation kind** (e.g. `still-image`) for asset types,
its **project superclass** (`Extends:` — the cheap inheritance hint), and a **cross-data-model source tag** (`[from <dm>]`) for fields reused from a sibling data-model
in the same project.
Pass `--include-builtins` to also show the inherited platform fields (arkUrl, permissions, timestamps, …) that every resource-type carries.

`dsp vre data-model structure` answers the complementary question: **how do the resource-types in a data-model relate to one another?**
It emits a flat, edge-centric list of `link` relations (link fields between resource-types, labelled with the field name) and `inherits` relations (superclass edges).
Cross-model link targets are tagged `[to <dm>]`. Pass `--include-builtins` to also surface system superclasses and built-in link fields. All five output formats are supported.
v1 limitation: link fields defined in a sibling data-model and only reused here are omitted (no sibling-ontology fetch).

Commands take identifiers as flags, not as positional path elements (the `--project`/`--data-model` pattern).
This keeps every command self-contained, IRI-friendly, and reproducible — at the cost of some flag repetition that `.env` files can mitigate per-directory.

## Scope

The feature scope of `dsp-cli` is not defined up front and grows over time as concrete workflows demand new verbs. v1 is intentionally narrow:

- **v1: schema-side read-only discovery in the VRE, plus two deliberate carve-outs.** The read-only surface covers listing and describing projects, data models,
  and resource-types with their fields. The two carve-outs are: (a) the project dump (`dsp vre project dump`) — a non-read addition that exercises bearer auth,
  multi-step orchestration, and binary streaming; and (b) instance-side reads (`dsp vre resource list` and `dsp vre resource describe`) — the first commands to read actual
  resource records rather than schema definitions, established as v1 after the schema-side suite proved stable. Write operations and bulk file-roundtripping remain out of scope.
- **v2 and beyond:** instance-side write operations (create/update/delete resources and values), search, then repository operations as the DSP Repository comes online.

Boundary with `dsp-tools`: `dsp-tools` owns file-roundtripping declarative bulk operations (project JSON, XML data, Excel inputs).
`dsp-cli` owns per-command agent-interactive operations.
The two tools touch some of the same data but never duplicate each other's interaction modes. See ADR-0004 for the explicit rule.

## How decisions are recorded

This repository is set up so that anyone — human or AI agent — entering at this file can reach every relevant artefact within one hop.

- **[`CONTEXT.md`](./CONTEXT.md)** — the glossary. The vocabulary the codebase commits to, with the synonyms we avoid and the ambiguities we've flagged. Read this first.
- **[`docs/adr/`](./docs/adr/)** — Architecture Decision Records. Each one captures a load-bearing decision with rationale and the alternatives we rejected.
  The ADRs are not user documentation; they're the answer to "why is the code shaped this way?".
- **[`docs/dev/`](./docs/dev/)** — contributor-facing documentation, including the practices for maintaining the domain language
  ([`domain-language.md`](./docs/dev/domain-language.md)).
- **[`docs/topics/`](./docs/topics/)** — end-user documentation, embedded into the binary and surfaced via `dsp docs <topic>`.
  Conceptual, crosscutting; the agent's encyclopaedia for the CLI.
  v1 ships nine topics (`dsp-cli`, `dsp`, `concepts`, `identifiers`, `connecting`, `output`, `workflows`, `errors`, `dsp-tools`); see ADR-0010.
- **Agent skill** — the Claude Code skill that announces `dsp-cli` to AI agents is no longer in this repo; it now lives in
  [`dasch-claude-plugins`](https://github.com/dasch-swiss/dasch-claude-plugins) (`misc:dsp-cli`), distributed via the plugin marketplace (see ADR-0011).

## Status

Personal exploratory project. If it proves valuable in practice, it will migrate to the `dasch-swiss` GitHub organisation —
most likely as a crate inside the `dsp-repository` Cargo workspace.
Implementation language is Rust (ADR-0005); distribution during the personal phase is `cargo install --path .` (the agent skill lives in `dasch-claude-plugins`, not this repo — ADR-0011).
