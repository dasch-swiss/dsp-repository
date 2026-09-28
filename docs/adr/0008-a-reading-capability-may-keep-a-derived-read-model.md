---
status: accepted
date: 2026-09-28
---

# A reading capability may keep a derived read model

ADR-0003 makes `sync` the single writer of the Access Area's archive projection in Chischtli and has
DPE, CPE and the SPARQL endpoint read through `sync`'s ports; it rejects each reader embedding a copy
of that projection. ADR-0007 then gives CPE a read-side store of its own, built from `sync`. Both are
right, and the distinction between them was implicit: a *copy* of the projection is not the same thing
as a *read model derived* from it. The projection is RDF because that is the archive's shape; it is not
the shape every reader wants. A presentation model, a full-text index, a precomputed facet table are
each a different shape over the same facts, and computing them per request against a triplestore is the
wrong place to pay for them. This record makes the pattern general so that CPE is its first instance,
not an exception (raised in review of ADR-0007). The decision, stated so a violation is
describable in code:

- **A capability that reads the archive projection may keep its own read model.** A read model is a
  store in whatever shape and engine suits how the capability reads — relational tables, a search index,
  in-process structures — holding only the facts the capability needs, transformed the way it needs
  them. The default is still to query `sync`'s ports live; a read model is the choice when the reader's
  shape differs from the projection's or when the transformation is too expensive to repeat per request,
  and the record of the capability names which.
- **The only feed is the capability's own port against `sync`.** The read model is built from what the
  port (ADR-0003, consumer-defined) serves, and from nothing else: not from the archive's data products,
  not from Chischtli opened directly, not from another capability's store. `sync` therefore stays the
  Access Area's single consumer of the archive's data products, and a read model adds no consumer,
  which is what ADR-0003's right-sizing rule protects.
- **The port speaks archive-shaped facts; the remodelling lives with the reader.** The port serves
  the projection as the archive records it (classes, typed values, links, files, ordered membership),
  never the consumer's model. The transformation from those facts into the read model is the reading
  capability's code, in its own crates. `sync` implements the adapter beside its data and knows nothing
  of what the reader builds.
- **The capability is the read model's only writer.** Nothing else writes it, `sync` included; `sync`
  serves facts and change, it never reaches into a reader's store. Inside the capability the read model
  is a table set with a single writer like any other (ADR-0003, data sovereignty).
- **A read model is derived state: disposable, rebuilt, never repaired.** It carries no fact the
  projection does not; losing it loses nothing; it is never hand-edited or migrated in place. Every read
  model can be rebuilt from empty against its port, and that rebuild is the baseline every capability
  must support. Applying change incrementally, from the changes the port announces, is an optimisation a
  capability may add on top of the rebuild, never a substitute for it.
- **Staleness is legitimate.** While `sync` has nothing new, or the port is unavailable, the read model
  serves what it has, exactly as the projection does while the archive is unavailable
  (`areas/archive/CONTEXT.md`, `chischtli/CONTEXT.md`).

## Considered Options

- **Derived read models behind the reader's own port (chosen).**
- **Every read goes live through `sync`'s query ports, no read models** — rejected: it forces every
  reader into the projection's RDF shape and pays the transformation on every request. CPE's
  presentation model and DPE's in-process caches (today built straight from the corpus, tomorrow from
  `sync`) are both read models already; the rule would be violated on the day it was written.
- **`sync` serves each consumer's shape** — rejected, as ADR-0007 rejects it for CPE: the remodel then
  lives in `sync`, every reader edits `sync` to change how it reads, and `sync` becomes the second
  writer of every reader's model.
- **Each reader consumes the archive's data products itself** — rejected by ADR-0003: one consumer of
  the data products per read-side service is the cost the modulith exists to avoid multiplying.
- **One shared read model for the area** (a cache every capability reads) — rejected: a shared shape is
  what ADR-0003 forbids between capabilities, and a store with several readers freezes its layout for
  all of them; ADR-0003's rejected `graph` capability, restated for tables.
- **Read models allowed but bound to one engine** (Chischtli, or SQLite) — not chosen: the engine is
  the shape's choice, and the area binary already links more than one. What is rejected is a copy of the
  projection, in any engine; a derived shape in Chischtli would be a read model like any other.

## Consequences

- ADR-0003's rejected option "each reading capability embedding its own Chischtli" is narrowed by
  amendment: a copy of the projection stays rejected; a derived read model is not a copy.
- ADR-0007's second clause, CPE's read-side store, is the first read model under this record; its
  "rebuilt from what the port serves" is this record's baseline rebuild.
- `dpe-core`'s process-global caches are, in this vocabulary, DPE's read model built from the corpus on
  disk. When DPE's reading of the corpus moves behind a port against `sync` (ADR-0007, Consequences),
  the caches become a read model under this record with no change of shape.
- Incremental refresh needs a port that announces change. `sync`'s minimum in ADR-0007 serves the
  committed projection of one shortcode and announces nothing, so a full rebuild is the only refresh
  until a port carries change; a capability that needs more asks for it in its own port.
- `ARCH-MAP.md`'s "Durable state" line for a reading capability of the Access Area says which it is:
  a read model behind its port against `sync`, single writer the capability, rebuilt from empty; or no
  store, reading live. The `areas/access/cpe` entry gains this with ADR-0007's other updates
  (`/dune:dune-map`).
- `chischtli/CONTEXT.md`'s example dialogue on a new facet in CPE gains the third answer: a facet the
  projection can serve but in another shape is CPE's read model, built through its port.

Enforced by: review until the capabilities exist; then Bazel `visibility` per ADR-0001 for "the only
feed is the port" (a reading capability's crates see its own `ports` crate and never `sync`'s store
crates or Chischtli; **structure**), and one test per read model that rebuilds it from empty against a
`Fake<Port>` and asserts what the capability serves is unchanged (**static-analysis**); "the port
speaks archive-shaped facts" and "never repaired in place" stay with review.
