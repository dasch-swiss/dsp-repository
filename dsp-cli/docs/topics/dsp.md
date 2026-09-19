# dsp — the DaSCH Service Platform, in brief

DaSCH is the Swiss National Data and Service Center for the Humanities. It runs
the **DaSCH Service Platform (DSP)** — digital infrastructure for creating,
archiving, and publishing qualitative humanities research data. This topic is
just enough orientation to use `dsp-cli`; it is not a full DSP manual.

## Two areas: VRE and Repository

The DSP has two main areas:

- **DSP VRE** (Virtual Research Environment) — where research data is *created* and
  edited. This is what `dsp-cli` v1 works with.
- **DSP Repository** — for long-term archival and publication. It is still being
  built; until it takes over, the VRE also serves as archive and presentation
  environment.

Because the Repository isn't ready, `dsp-cli` v1 targets the **VRE only**.

## How data is organised in the VRE

A **project** owns one or more **data-models**; a data-model defines
**resource-types**; a resource-type defines **fields**; and a researcher's actual
records are **resources** holding **values**. The full vocabulary is in
`dsp docs concepts`.

## The existing ways to talk to the DSP

- **DSP-APP** — an Angular web application for viewing and editing data. The main
  client for humans; not suitable for agents.
- **DSP-TOOLS** — a Python CLI (and library) for file-driven, declarative bulk work:
  initialising projects, building data-models from files, bulk-ingesting data. See
  `dsp docs dsp-tools` for the boundary with `dsp-cli`.
- **DSP-API** — the public HTTP API both clients use. It is only partially RESTful;
  many endpoints speak RDF and return verbose responses that burn an agent's
  context window.

## Where dsp-cli fits

`dsp-cli` is a third, complementary surface: it wraps DSP-API's verbose RDF in
high-level commands and compact output, filling the gap between DSP-APP (human-only)
and DSP-API (verbose and complex). It does not replace either, and it has a sharp
scope boundary with `dsp-tools`.

See also: `dsp docs concepts`, `dsp docs dsp-tools`, `dsp docs connecting`.
