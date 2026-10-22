# dsp-cli Overview

`dsp-cli` (binary `dsp`) is a command-line client for the DaSCH Service Platform (DSP), built for
AI agents first and humans second. This page states what it is for and the principles its design
decisions are measured against; the decisions themselves are in `dsp-cli/docs/adr/`.

## Context

The DSP has two parts: the **DSP VRE** (Virtual Research Environment, for creating data) and the
**DSP Repository** (for long-term archival and publication). Until the Repository takes over, the
VRE serves as archive and presentation environment as well, so `dsp-cli` covers the VRE only.

The existing ways to reach the DSP leave a gap that `dsp-cli` fills:

- **DSP-APP**, the Angular web application, is built for humans and unusable for agents.
- **DSP-TOOLS**, the Python CLI, is file-driven: it creates projects and data models from
  declarative files and bulk-ingests data.
- **DSP-API**, the HTTP API both use, is partly RESTful and partly RDF. Its responses expose the
  system's internal complexity and are verbose enough to exhaust an agent's context.

`dsp-cli` sits beside `dsp-tools`, with a sharp boundary between them (dsp-cli/ADR-0004): `dsp-tools`
owns file-roundtripping bulk operations, `dsp-cli` owns interactive, one-command-at-a-time
operations. The two touch some of the same data but never duplicate each other's interaction mode.

## Design principles

1. **Agent-first ergonomics.** Vocabulary, output shape, command structure and error handling are
   judged by whether an LLM agent reading them cold can act on them. The same choices serve
   humans well.
2. **Prose by default, structured on demand.** The default output is compact prose with next-step
   hints. JSON, CSV, TSV and line-based output are one flag away, for piping into other tools.
3. **Agent-mediated chaining, no implicit session state.** Each command stands alone: the agent
   reads the output, extracts identifiers and builds the next command. The CLI has no piping
   primitives and keeps no state between invocations.
4. **Domain-expert vocabulary, not RDF terms.** `data-model`, `resource-type`, `field`, `value`
   and `value-type` describe the data the way researchers do. Translation to DSP-API's `ontology`,
   `class` and `property` happens once, at the API boundary (dsp-cli/ADR-0001).
5. **Explicit beats implicit.** There is no default server, and no silent fallback. An error names
   what is missing and how to supply it.
6. **Documentation ships with the code.** The end-user documentation is embedded in the binary and
   read with `dsp docs <topic>` (dsp-cli/ADR-0010), so it always matches the installed version.

## Shape

```
dsp auth   ...   # login, logout, status, set-token, token
dsp vre    ...   # VRE reads: project, data-model, resource-type, resource, vocabulary, sparql
dsp docs   ...   # embedded end-user documentation
```

Commands are nested noun-groups with help text at every level, so an agent discovers the surface
one level at a time instead of loading all of it. Identifiers are passed as flags
(`--project`, `--data-model`), never as positional arguments (dsp-cli/ADR-0002): every command is
self-contained and reproducible, and a per-directory `.env` file absorbs the repetition.
`dsp --help` and [Usage](./usage.md) list the commands themselves.

## Scope

The scope grows as concrete workflows need new commands; it is not fixed up front.

- **Now:** read-only access to the VRE: schema discovery (projects, data models, resource-types
  and their fields, controlled vocabularies), instance reads (resources and their values), a raw
  SPARQL passthrough, and the project dump, which downloads a project's data.
- **Later:** writing resources and values, search, and Repository operations once the DSP
  Repository is in service.

## See also

- [Architecture](./architecture.md): layers, test seam, crate layout.
- `dsp-cli/CONTEXT.md`: the domain glossary.
- `dsp-cli/CLAUDE.md`: conventions and constraints for working on the crate.
