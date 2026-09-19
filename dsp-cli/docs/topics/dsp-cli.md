# dsp-cli — what this tool is

`dsp-cli` is a command-line interface for the DaSCH Service Platform (DSP),
designed first for AI agents and second for humans. The binary is named `dsp`.
It sits between DSP-APP (a human web app) and DSP-API (a verbose RDF HTTP API),
giving you compact, high-level commands that speak the language researchers use.

If you're new to the DSP itself, read `dsp docs dsp` first. For the vocabulary,
read `dsp docs concepts`. For how to chain commands into real tasks, read
`dsp docs workflows`.

## Who it's for

The primary audience is an AI agent operating the CLI on a researcher's behalf.
Every design choice is judged against "does this make sense to an agent reading it
cold?" — and the same choices happen to serve humans well too.

## Design principles

1. **Agent-first ergonomics.** Vocabulary, output shape, command structure, and
   errors are all tuned for a reader who arrives with no prior context.
2. **Prose by default, structured on demand.** Default output is readable prose with
   next-step hints; `-j`, `--csv`, `--tsv`, `-l` switch to machine formats. See
   `dsp docs output`.
3. **Agent-mediated chaining, no implicit state.** Each command stands alone. There
   is no session, no piping primitive — you read one command's output, lift an
   identifier, and build the next command. See `dsp docs workflows`.
4. **Domain vocabulary, not RDF jargon.** `data-model`, `resource-type`, `field`,
   `value-type` — not `ontology`/`class`/`property`. The translation happens once,
   at the API boundary. See `dsp docs concepts`.
5. **Explicit beats implicit.** No default server; every command targets one
   explicitly. No silent fallbacks. Errors say what's missing and how to supply it.
   See `dsp docs connecting`.
6. **Documentation ships with the binary.** These topics are embedded at compile
   time, so they're always in sync with the version you're running.

## What it is — and is not

`dsp-cli` is **not** `dsp-tools`. `dsp-tools` does file-driven, declarative, bulk
operations (creating projects, ingesting data). `dsp-cli` does per-command,
interactive reads and discovery. See `dsp docs dsp-tools`.

## v1 scope and limitations

v1 is intentionally narrow: **schema-side, read-only discovery in the VRE**, plus
two deliberate non-schema carve-outs. Concretely:

- **Read-only.** v1 lists and describes projects, data-models, resource-types, and
  resource instances. It does not create or modify data.
- **Instance reads included.** `dsp vre resource list` and `dsp vre resource describe`
  read actual resource records (instance data), not just schema definitions. These
  are v1 — write operations remain out of scope. Pass `--values` to `resource describe`
  to include the resource's field values in the output: one value per field line in
  prose, a `"values"` array in json, and one row per value in tabular formats
  (`csv`/`tsv`/`lines`), with resource `label`/`iri` available via `--columns`.
- **One dump at a time, server-wide.** The DSP-API holds a single dump across the
  whole instance — not one per project. `dsp vre project dump` refuses to touch a
  dump held by another project.
- **`data-model structure` skips reused sibling links.** A link field *defined in*
  one data-model and only *reused* in another is omitted from the structure view
  (it needs a second fetch dsp-cli doesn't make in v1). Links defined where they're
  used are fully covered.

## Staying up to date

`dsp` checks for a newer published version at most once per day, but only on
interactive, prose-format runs — it is off for `-j`/`-l`/other non-prose formats
and for any non-interactive (piped) invocation, so it is completely invisible to
agents and scripts. When a newer version is found, it prints a short advisory to
stderr recommending `cargo install dsp-cli`; it never touches stdout, the JSON
envelope, or the exit code. Set `DSP_NO_UPDATE_CHECK=1` (or any non-empty value)
to disable it entirely.

See also: `dsp docs dsp`, `dsp docs concepts`, `dsp docs workflows`.
