# A reading capability may keep a derived read model

ADR-0003 makes `sync` the single writer of the Access Area's archive projection in Chischtli and has DPE, CPE
and the SPARQL endpoint read through `sync`'s ports; it rejects each reader embedding a copy of that projection.
A *copy* of the projection is not the same thing as a *read model derived* from it. The projection is RDF
because that is the archive's shape; it is not the shape every reader wants. A presentation model, a full-text
index, a precomputed facet table are each a different shape over the same facts, and computing them per request
against a triplestore is the wrong place to pay for them. This record makes the pattern general, so that CPE's
store (ADR-0007) is its first instance, not an exception. The decision, stated so a violation is describable in
code:

- **A capability that reads the archive projection may keep its own read model.** A read model is a store in
  whatever shape and engine suits how the capability reads — relational tables, a search index, in-process
  structures — holding only the facts the capability needs, transformed the way it needs them. The default is
  still to query `sync`'s ports live; a read model is the choice when the reader's shape differs from the
  projection's or when the transformation is too expensive to repeat per request, and the record of the
  capability names which.
- **A read model is fed by the capability's own ports and nothing else.** Its port against `sync` (ADR-0003,
  consumer-defined) serves the projection's facts; a port for bytes against `media` serves deposited files such
  as a configuration (ADR-0012). The read model is built from what those ports serve: not from the archive's
  channels, not from Chischtli opened directly, not from another capability's store, and not from files in the
  capability's own directory. A read model therefore adds no consumer of either of the archive's channels,
  which is what ADR-0003's one consumer per channel protects.
- **The port against `sync` speaks archive-shaped facts; the remodelling lives with the reader.** The port
  serves the projection as the archive records it (classes, typed values, links, files, ordered membership),
  never the consumer's model. The transformation from those facts into the read model is the reading
  capability's code, in its own crates. `sync` implements the adapter beside its data and knows nothing of what
  the reader builds.
- **The capability is the read model's only writer.** Nothing else writes it, `sync` included; `sync` serves
  facts and change, it never reaches into a reader's store. Inside the capability the read model is a table set
  with a single writer like any other (ADR-0003, data sovereignty).
- **A read model is derived state: disposable, rebuilt, never repaired.** It carries no fact the projection
  does not; losing it loses nothing; it is never hand-edited or migrated in place. Every read model can be
  rebuilt from empty against its ports, and that rebuild is the baseline every capability must support. Applying
  change incrementally, from the changes a port announces, is an optimisation a capability may add on top of
  the rebuild, never a substitute for it.
- **A read model stores rights per row.** A rebuild reads on no one's behalf. Both ports serve what they serve
  with its rights attached and never pre-filter; the read model stores the rights per row, and the presenter
  filters at request time, as the anonymous consumer until an authorisation mechanism exists (ADR-0012,
  ADR-0013).
- **Change reaches a reader only through its port, and the port announces it.** The provider owns the
  trigger; the reader rebuilds the one project the port named and grows no endpoint of its own. Until a port
  carries change, a restart is the rebuild.
- **Staleness is legitimate.** While `sync` has nothing new, or a port is unavailable, the read model serves
  what it has, exactly as the projection does while the archive is unavailable (`areas/archive/CONTEXT.md`,
  `chischtli/CONTEXT.md`).
- **A reading capability's tests may build `sync`'s adapter.** A crate of a reading capability may name
  `sync`'s store crate as a dev-dependency, and only as one; the same holds for `media`'s, the adapter of its
  bytes port (ADR-0012). Cargo gives a dev-dependency to test builds alone,
  so a library or binary as built for use is still fed by its ports only. Such a test constructs
  `LiveArchiveProjection` over the committed snapshot and hands the builder what the port serves; it never opens
  `sync`'s data itself. The tests of `cpe-store-build` and `cpe-server` build Incunabula's store this way.

## Considered Options

- **Derived read models behind the reader's own ports (chosen).**
- **Every read goes live through `sync`'s query ports, no read models** — rejected: it forces every reader into
  the projection's RDF shape and pays the transformation on every request. CPE's presentation model and DPE's
  corpus caches are both read models already.
- **`sync` serves each consumer's shape** — rejected, as ADR-0007 rejects it for CPE: the remodel then lives in
  `sync`, every reader edits `sync` to change how it reads, and `sync` becomes the second writer of every
  reader's model.
- **Each reader consumes the archive's channels itself** — rejected by ADR-0003: one consumer per read-side
  service is the cost the modulith exists to avoid multiplying.
- **One shared read model for the area** (a cache every capability reads) — rejected: a shared shape is what
  ADR-0003 forbids between capabilities, and a store with several readers freezes its layout for all of them;
  ADR-0003's rejected `graph` capability, restated for tables.
- **Read models allowed but bound to one engine** (Chischtli, or SQLite) — not chosen: the engine is the
  shape's choice, and the area binary already links more than one. What is rejected is a copy of the
  projection, in any engine; a derived shape in Chischtli would be a read model like any other.
- **A read model's tests in the composition root, so no capability crate names `sync`'s store** — rejected:
  `ARCH-MAP.md` holds the composition root to wiring only.
- **A second committed copy of the snapshot, as a fixture of the capability** — rejected: two copies of one
  file, and the tests would no longer read through the adapter that serves the capability in use.

## Consequences

- ADR-0003's rejected option of each reader embedding its own Chischtli rejects a copy of the projection; a
  derived read model is not a copy.
- CPE's read-side store (ADR-0007) is the first read model under this record; its rebuild from what the port
  serves is this record's baseline rebuild.
- DPE's corpus caches, fields of `dpe_core::Corpus`, are DPE's read model, built from the corpus on disk. Read
  through a port against `sync`, they are a read model under this record with no change of shape.
- A refresh between starts needs a port that announces change. `sync`'s minimum (ADR-0007) serves the
  committed projection of one shortcode and announces nothing, so a full rebuild at start is the only refresh
  until the ports carry change.
- `ARCH-MAP.md`'s "Durable state" line for a reading capability of the Access Area says which it is: a read
  model behind its ports, single writer the capability, rebuilt from empty; or no store, reading live.

Not yet in code: DPE reads its corpus from disk, not through a port against `sync` (not scheduled).
Not yet in code: neither the port against `sync` nor the bytes port against `media` announces change, and no
reader rebuilds one named project (DEV-7622).
Not yet in code: neither port serves rights, no read model stores them per row, and no presenter filters on
them (not scheduled).

Enforced by: the target is Bazel `visibility` per ADR-0001 for "fed by its own ports": a reading capability's
crates see its own `ports` crate and never `sync`'s store crates or Chischtli, except as a dev-dependency
(**structure**). Today `check-capability-deps.sh`, run by `just check`: a crate of another capability names
`sync-store` or `media-store` as a dev-dependency only (its `DEV_ONLY`), a `ports`, `image-viewer` or `api-*` crate
or a crate of `sync` or `media` not even as one, and no capability's crate names another store crate at all
(**static-analysis**); the composition root, which constructs the adapters, is `check-composition-root-deps.sh`'s.
The rule lets through either adapter as a dev-dependency of any other crate of the area, and it reads no source,
so what a test does with the adapter is review. One test per read model rebuilds it from empty against a
`Fake<Port>` and asserts what the capability serves is unchanged (**static-analysis**; for CPE, `areas/access/cpe/store-build/tests/it/rebuild.rs`). Review for any
other reading capability, for "the port speaks archive-shaped facts" and for "never repaired in place".
