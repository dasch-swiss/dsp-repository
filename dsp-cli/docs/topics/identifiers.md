# Identifiers — how to name things

`dsp-cli` takes identifiers as flags, never as positional path elements. This
topic explains what each flag accepts, so you can lift an identifier from one
command's output and feed it to the next (see `dsp docs workflows`).

## Project — three interchangeable identifiers

The `-p` / `--project` flag accepts **any** of three forms, never a subset:

| form | example | notes |
|---|---|---|
| shortcode | `0803` | a 4-character hex code |
| shortname | `incunabula` | a short human label |
| IRI | `http://rdfh.ch/projects/0803` | the full project IRI |

`dsp-cli` figures out which form you gave (4 hex digits → shortcode, IRI-shaped →
IRI, otherwise shortname) and resolves it. Any of the three works everywhere a
project is referenced.

## Data-model — name or IRI

The `--data-model` flag accepts a short **name** (e.g. `beol`) or the full
ontology **IRI** (e.g. `http://api.dasch.swiss/ontology/0801/beol/v2`). There is no
shortcode for a data-model — only projects have those.

## Resource-type — name or IRI

The `--resource-type` flag accepts a short **name** (e.g. `letter`) or the full
**IRI** (e.g. `…/ontology/0801/beol/v2#letter`).

## CURIEs

DSP-API IRIs are often written as **CURIEs** — `prefix:localname`, e.g.
`beol:letter`. The prefix names the data-model the thing belongs to. `dsp-cli`
expands CURIEs to full IRIs at the API boundary; in output you'll usually see the
short name (`letter`) or the full IRI, with cross-data-model references tagged
like `[from beol]` or `[to person]`.

## Resource IRIs and ARKs

Individual **resources** (the records themselves) are identified by an IRI and
optionally an **ARK** (a persistent identifier URL). Resource-level access is
instance-side and out of scope in v1 — v1 identifies projects, data-models, and
resource-types only.

## Lifting identifiers from output

The typical loop: run a `list` to discover names/shortcodes, then pass one into a
`describe` or `structure`. In prose, identifiers are shown inline; in `-j`/`--csv`
they're discrete fields you can `jq`/`cut` out. See `dsp docs workflows` for
worked chains.

See also: `dsp docs workflows`, `dsp docs concepts`.
