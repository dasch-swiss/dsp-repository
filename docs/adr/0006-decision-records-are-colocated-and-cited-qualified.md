---
status: accepted
date: 2026-09-19
---

# Decision records are colocated with what they govern, and a component's are cited qualified

`ARCH-MAP.md`'s "Colocated docs" line granted a colocated `CONTEXT.md` and `docs/adr/` to a
bounded context and to a shared engine with its own vocabulary; this record widens both to any
root-level component with its own vocabulary or decision history.
`dsp-cli`, which arrives as a root peer of the areas by an amendment to ADR-0002 that lands with
the crate, is the first component to exercise it, with Vitrinli and Chischtli next. No maintained
ADR tool is monorepo-aware — log4brains has been stale since 2022, adr-tools and MADR tooling
assume a single directory — so the convention has to survive on citation discipline alone, not
tooling (checked 2026-09-18, DEV-7330). The decision, stated so a violation is describable:

- System-wide decisions live in the root `docs/adr/` with one sequence. A component with its own
  decision history keeps its own `docs/adr/` under the component directory, with its own sequence
  starting at 0001. The two sequences collide by design; the citation form is what disambiguates.
- A bare `ADR-NNNN` always names a root ADR, from anywhere in the repository.
- A component ADR is always cited qualified, as `<component>/ADR-NNNN` — also from inside that
  component, where the bare form would otherwise read naturally and resolve to the wrong record.
- An amendment refines a decision in place, under a dated `## Amendment` heading. A decision that
  changes rather than narrows becomes a new ADR, and the old one takes
  `status: superseded by <the new one>`. ADR-0005's three amendments predate this record and
  stand as the bold dated paragraphs they were written as; amendments from here on use the
  heading. ADR-0002's in-place revision notes — "layout revised 2026-09-17" at
  `docs/adr/0002-areas-at-the-repository-root.md:10`, and "replaced 2026-09-17" at lines 37 and
  42 — are not amendments at all but pre-acceptance drafting of a record dated 2026-09-16, and
  they set no precedent for revising in place.
- A root ADR that constrains an earlier component ADR names that component ADR in its
  Consequences, and the component ADR gets a dated amendment pointing back. The link is written
  from both ends because neither directory is scanned from the other.
- `dsp-cli/` is the first component to carry its own series, with Vitrinli and Chischtli as the
  next once they have decisions of their own.

## Considered Options

- **One root sequence, one component sequence per component, disambiguated by qualified citation
  (chosen).**
- **One root sequence for everything** — rejected: a component's decisions then need renumbering
  on arrival to avoid colliding with the root series, and the root series carries records no one
  outside the component reads.
- **Per-component sequences with globally unique prefixes** (e.g. a `CLI-0001` form) — rejected:
  the component name is already in the path a qualified citation names, and a prefix has to be
  invented and policed per component for no gain over the path that already exists.
- **Leave the convention unwritten and settle it in review** — rejected: `dsp-cli` arrives with
  its own ADR series before this is written down, and review alone caught neither a dangling
  reference nor an unqualified citation of a component ADR in the drafts that led to this record.

## Consequences

- The gate resolves a bare reference against the root series only, so a stale bare `ADR-0001`
  through `ADR-0006` written inside a component — where a component ADR of that number was
  meant — resolves against the root series and passes. Only review catches that; a one-time
  rewrite of a component's citations to qualified form, done once that component's series exists,
  is what removes the ambiguity for good.
- `docs/specs/**` is not scanned. Specs and journals are point-in-time records that cite intended
  future state by design, including decision records that do not exist yet, so a dangling
  reference there is correct rather than broken.

Enforced by: the citation rule and reference resolution, by
`.github/scripts/check-adr-refs.sh`, run by `just check` (**static-analysis**); the amendment and
supersession rules, and the both-ends link between a root ADR and the component ADR it
constrains, by **review**.
