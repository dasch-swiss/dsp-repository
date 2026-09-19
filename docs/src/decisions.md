# Decision Records

Architecture decision records (ADRs) capture the "why" behind a decision, not just the "what".

## Where a decision lives, and how it is cited

System-wide decisions live in the root `docs/adr/`, with one sequence starting at 0001. A
root-level component with its own decision history keeps its own `docs/adr/` under the component
directory, with its own sequence also starting at 0001. The two sequences collide by number; the
citation form is what disambiguates:

- A bare `ADR-NNNN` always names a root ADR, from anywhere in the repository.
- A component ADR is always cited qualified, as `<component>/ADR-NNNN` — also from inside that
  component.

An amendment refines a decision in place, under a dated `## Amendment` heading. A decision that
changes rather than narrows becomes a new ADR, and the superseded one takes
`status: superseded by <the new one>`.

`just check` runs `check-adr-refs.sh`, which verifies that every `ADR-NNNN` and
`<component>/ADR-NNNN` reference in the repository (outside `docs/specs/`) resolves to a file.
See [ADR-0006](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md)
for the full decision.

## Root series

| ADR | Title |
|---|---|
| [ADR-0001](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0001-bazel-builds-the-monorepo.md) | Bazel builds the monorepo |
| [ADR-0002](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0002-areas-at-the-repository-root.md) | The repository root is shaped by the three areas of the Trusted Repository |
| [ADR-0003](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0003-one-modulith-per-area.md) | One modulith per area |
| [ADR-0004](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0004-hypermedia-frontends.md) | Every user-facing surface is a hypermedia server |
| [ADR-0005](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0005-fair-landing-pages-in-the-access-area.md) | Every landing page in the Access Area is FAIR-assessable by machine |
| [ADR-0006](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md) | Decision records are colocated with what they govern, and a component's are cited qualified |

## Component series

Listed here as components arrive with their own decision history.
