# Decision Records

Architecture decision records (ADRs) capture the "why" behind a decision, not just the "what".

## Where a decision lives, and how it is cited

System-wide decisions live in the root `docs/adr/`, with one sequence starting at 0001. An area
keeps its own series at `areas/<area>/docs/adr/`, and a root-level component with its own
decision history keeps one at `<component>/docs/adr/`, each also starting at 0001. The sequences
collide by number; the citation form is what disambiguates:

- A bare `ADR-NNNN` always names a root ADR, from anywhere in the repository.
- An area or component ADR is always cited qualified, as `areas/<area>/ADR-NNNN` or
  `<component>/ADR-NNNN` — also from inside that area or component.

## An ADR is the current decision

An ADR states the decision in force now, or the target the code is being moved to, with any gap
named against a Linear issue. It is rewritten in place when the decision changes, and retired when
the decision no longer holds. An ADR number is an identifier and is never reused: a retired record
keeps its file as a one-line tombstone that says where its decision lives now, so every citation
still resolves, and its row below stays, marked retired. The history of a decision is the
git history of its file: the commit that changes a record says what it decided before and why
([Git conventions](git-conventions.md#history-lives-in-git-documents-hold-the-current-state)). The
writing rules are in
[`docs/adr/CLAUDE.md`](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/CLAUDE.md).

Specs in `docs/specs/` are not rewritten: an ADR they cite may have been rewritten since the
spec's date.

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
| [ADR-0006](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md) | Decision records are colocated with what they govern, hold only the current decision, and a component's are cited qualified |
| [ADR-0007](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md) | CPE joins the Access Area as its second capability |
| [ADR-0008](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0008-a-reading-capability-may-keep-a-derived-read-model.md) | A reading capability may keep a derived read model |
| [ADR-0009](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0009-datei-joins-the-access-area-as-its-third-capability.md) | DaTEI joins the Access Area as its third capability |
| [ADR-0010](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0010-the-port-also-serves-per-resource-curation.md) | *Retired, see ADR-0012.* The port also serves per-resource curation |
| [ADR-0011](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0011-cpe-crates-move-in-dependency-order.md) | *Retired, see ADR-0007.* CPE's crates move in dependency order |
| [ADR-0012](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0012-build-time-inputs-come-through-two-channels-one-consumer-each.md) | Build-time inputs come through two channels, one consumer each |
| [ADR-0013](https://github.com/dasch-swiss/dsp-repository/blob/main/docs/adr/0013-authorisation-is-decided-centrally-and-enforced-locally.md) | Authorisation is decided centrally and enforced locally |

## Component series

Listed here as components arrive with their own decision history.

| Component | Decisions | Citation form |
|---|---|---|
| [`dsp-cli`](https://github.com/dasch-swiss/dsp-repository/blob/main/dsp-cli/docs/adr/) | its own sequence, starting at 0001 | `dsp-cli/ADR-NNNN`, also from inside `dsp-cli/` |
| [`areas/deposit`](https://github.com/dasch-swiss/dsp-repository/blob/main/areas/deposit/docs/adr/) | the Deposit Area's own sequence, starting at 0001 | `areas/deposit/ADR-NNNN`, also from inside `areas/deposit/` |
