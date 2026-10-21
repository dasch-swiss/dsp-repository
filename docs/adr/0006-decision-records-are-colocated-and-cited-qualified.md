# Decision records are colocated with what they govern, hold only the current decision, and a component's are cited qualified

A decision record is read by agents and people who trust it to describe the system, and the
repository holds more than one series of them. No maintained ADR tool is monorepo-aware —
log4brains is unmaintained, adr-tools and MADR tooling assume a single directory — so the
convention survives on citation discipline, not tooling. The decision, stated so a violation is
describable:

- System-wide decisions live in the root `docs/adr/` with one sequence. An area keeps its own
  series at `areas/<area>/docs/adr/`, and a root-level component with its own vocabulary or
  decision history keeps one at `<component>/docs/adr/`; each sequence starts at 0001. A
  capability has no series of its own: its decisions go to its area's, the one directory that
  outlives every capability inside it.
- A bare `ADR-NNNN` always names a root ADR, from anywhere in the repository.
- An area or component ADR is always cited qualified, as `areas/<area>/ADR-NNNN` or
  `<component>/ADR-NNNN` — also from inside that area or component, where the bare form would
  read naturally and resolve to the wrong record.
- An ADR holds the decision in force now, or the target the code is being moved to, never the
  history of how it got there. A changed decision is rewritten in place, and a decision that no
  longer holds is retired. The commit that changes a record says what it decided before and why.
  The writing rules are in `docs/adr/CLAUDE.md`.
- An ADR number is an identifier and is never reused. No record is renumbered; a retired record
  keeps its file as a one-line tombstone that names where its decision lives now, so every
  citation ever written still resolves to the record it meant.
- A root ADR that constrains an area or component ADR names it, and the constrained ADR names
  the root ADR back. The link is written from both ends because neither directory is scanned
  from the other.

## Considered Options

- **One root sequence, one sequence per area or component, disambiguated by qualified citation
  (chosen).**
- **One root sequence for everything** — rejected: a component's decisions then need renumbering
  on arrival to avoid colliding with the root series, and the root series carries records no one
  outside the component reads.
- **Per-component sequences with globally unique prefixes** (e.g. a `CLI-0001` form) — rejected:
  the component name is already in the path a qualified citation names, and a prefix has to be
  invented and policed per component for no gain over the path that already exists.
- **A series per capability** (`areas/deposit/editor/docs/adr/`) — rejected: the decisions that
  matter most in an area are the ones between its capabilities, which no capability owns, and a
  capability that moves or splits would take its records to a path that no longer names them.
- **Amend and supersede, keeping every record as written** — rejected: records accumulate dated
  amendments and superseded predecessors, and a reader has to replay the chain to learn what is
  decided. Git already keeps every earlier version of a record and the reason it changed.
- **A densely numbered series, renumbered when a record is deleted** — rejected: a number cited in
  a Linear issue, a learning, a spec or the design repository would silently come to name a
  different record, and no gate can tell.

## Consequences

- The gate resolves a bare reference against the root series only, so a bare `ADR-NNNN` written
  inside an area or component, where that series' record was meant, resolves against the root
  series and passes. Only review catches that.
- The gate proves a citation resolves to a file, not to the record that was meant. Because a
  number is never reused and a retired record keeps its tombstone, a resolving citation names the
  record its author meant, or a tombstone that points on from it.
- `docs/specs/**` is not scanned. Specs and journals are point-in-time records: a record they cite
  may have been rewritten since, so they are read in their historic context
  (`docs/specs/CLAUDE.md`).

Enforced by: the citation rule and reference resolution, by
`.github/scripts/check-adr-refs.sh`, run by `just check` (**static-analysis**); the current-state
rule, the never-reused number and the both-ends link between a root ADR and the ADR it constrains,
by **review**.
