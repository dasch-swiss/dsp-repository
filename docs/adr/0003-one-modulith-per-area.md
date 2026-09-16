---
status: accepted
date: 2026-09-16
---

# One modulith per area

Each area of ADR-0002 is built as one **modulith**: one deployable binary per area, composed at that area's single composition root out of **capabilities** that are bounded as strictly as if a network sat between them. The Deposit Area's modulith grows from the metadata editor (its next capabilities are the data-model creator and data creation); the Access Area's grows from DPE (then CPE, the SPARQL endpoint, the admin view); the Archive Area's is Spycherli. Between areas nothing changes: they still integrate over the wire (ADR-0002).

One binary per area is the decision for now. It keeps the option to rethink towards services later, if performance demands several copies of one capability, because a capability behind a port can be extracted without rewriting its callers — and the discipline that makes extraction possible is paid for now, while it is free, rather than retrofitted (rationale confirmed 2026-09-16). It also respects the read side's right-sizing rule: every read-side service carries its own local store and its own consumer of the archive's data products, so the number of applications is kept small and a new capability is added inside the binary, not beside it.

The contract inside an area:

- **Data sovereignty.** A capability owns its tables and is their sole reader and writer, even where capabilities share one database file. No cross-capability SQL, no cross-table joins.
- **Consumer-defined ports.** When capability A needs something from capability B, A declares a **port** — a trait naming exactly what A needs (a hypothetical example: `ProjectDirectory { fn by_shortcode(&self, shortcode) -> Option<ProjectRef> }`), plus its small boundary DTOs — in its own `ports` crate, which depends on nothing but `std` and the shared `platform-*` crates. A port is a dependency-inversion interface, not a domain or a web concern, so synchronous and asynchronous ports live in the same crate. Ports are never shared between consumers: two capabilities needing the same thing from one provider each declare their own port, and the provider implements an adapter per port.
- **The provider implements the adapter, beside its data.** B implements A's port in B's own store crate, depending on `A/ports` and nothing else of A. Production adapters are named `Live<Port>` (`LiveProjectDirectory`), test doubles `Test<Port>` or `Fake<Port>`; the crate location already says which provider fulfils the port, so the name does not encode the storage.
- **One permitted capability-to-capability dependency:** a provider on a consumer's `ports` crate. Never on another capability's domain, store or web crate. A port declared by a `platform-*` crate stays in that crate; a capability implementing it depends on the platform crate, which is always allowed.
- **References are opaque ids.** A capability stores only the foreign identifier (a shortcode, an entity id) — a foreign key by convention, not a database constraint across capabilities — and fetches names, labels and details through the port, never by reading the other capability's tables.
- **The composition root holds no adapter logic.** `<area>/server` constructs each provider's adapters and injects them into the consumers at startup; it is wiring, nothing else.
- **Shared concepts, not shapes.** A value that is the same concept across capabilities lives in a `platform-*` crate; a boundary DTO returned by a port belongs to the consumer's `ports` crate and is not shared code.
- **No distributed transaction across capabilities.** Each capability commits within its own transaction; work that spans capabilities is a sequence of per-capability transactions, coordinated by id and port call, consistent eventually.

Within a capability, the crate anatomy stays `{capability}-{role}` (ADR-0002): a framework-free domain crate, a store where it persists, a web crate of views and routes, and `ports` when another capability needs something from it.

**Shared engines are libraries behind capabilities, never capabilities of their own.** Two exist (ADR-0002), and the same rule holds for both: the engine is a root-level library with no routes, no authentication and no opinion about the area; everything area-specific arrives through the traits its interface takes; the durable state it holds belongs to exactly one capability per unit of ownership; and the dependency arrow `capability → engine` never reverses.

- **Vitrinli** exposes deep modules — IIIF rendering, range-served downloads, Service File derivation — whose traits cover where bytes come from, who may see them and where a derived file goes. Each area has a `media` capability, with the standard anatomy, that depends on Vitrinli, implements those traits over its own store, owns the tables that say what is servable and where, owns the routes and the authorisation, and talks to sibling capabilities through ordinary ports. In the Deposit Area, `media` receives the Originals a depositor uploads and has Vitrinli derive Service Files for the deposit frontends; in the Access Area, `media` serves the Service Files the archive produced, and nothing else. An area's rules for media live in that area's code, where its other rules live — the typed successor of the Lua scripts that configure sipi today.
- **Chischtli** is the triplestore engine, and its unit of ownership is the **named graph**: every graph has exactly one capability that writes it, and any other capability reads it through that owner's ports, never by opening the store. In the Access Area, the `sync` capability is the single writer of every graph that comes from the archive — it consumes the archive's data products and rebuilds the projection from snapshot plus replay — and DPE, CPE and the SPARQL endpoint read through `sync`'s query ports; a `profile` capability owns the small graphs of per-user settings it keeps beside them. In the Deposit Area, the capability that owns data creation writes the working graphs the same way, with SHACL validation on update where a project opts in. One Chischtli instance per area therefore serves several capabilities without becoming a second writer for any of them.

## Considered Options

- **One modulith per area (chosen).**
- **One modulith for the whole platform** — rejected in ADR-0002: the Deposit and Access Areas are separate deployables on separate origins for a stated security reason, and the archive is sealed from both.
- **One deployable per capability (DPE, CPE, the editor, … each its own binary)** — today's shape, extended. Rejected: each binary would carry its own local store and its own consumer of the archive's data products, multiplying operational surface for a boundary the modulith gives for free.
- **Capabilities sharing a database freely inside an area** — the simplest today; rejected because it couples schemas, makes extraction impossible, and lets "who set this value?" have no single answer. The same applies to a shared triplestore: a graph with two writers is a table with two writers.
- **Each reading capability embedding its own Chischtli** (DPE, CPE and the SPARQL endpoint each with a copy of the archive projection) — rejected: three copies of one projection and three consumers of the archive's data products, which is the read-side cost the right-sizing rule exists to avoid.
- **One shared `ports` crate for the whole area** — rejected: it becomes a crate every capability depends on, the same failure as a shared domain crate, and it strips each port of the authorship of the consumer that defines it.
- **A `graph` capability owning the whole Access-Area store** — rejected: it would make one capability the writer of graphs it does not understand (archive projection and user settings alike); ownership per named graph keeps each writer with the data it knows.
- **Adapters at the composition root** — rejected: `<area>/server` would accrete adapter logic and couple itself to every provider's storage; the narrow `ports` dependency lets the adapter live with its data.

## Consequences

- `dpe-server` and `editor-server` are the seeds of the Access and Deposit composition roots; as a second capability arrives in an area, the capability-specific code moves out of the server crate and the server becomes wiring only.
- A new capability in an area is a directory beside the existing ones, its own crates, and a mount plus adapter wiring at the composition root — never an import of a sibling capability.
- More indirection per cross-capability need (a port plus an adapter), boundary DTOs defined per consumer rather than shared, and read paths that would have been one SQL join become a port call plus in-memory composition. Accepted.
- Each area keeps one `CONTEXT.md` per capability once vocabulary diverges, indexed from the root `CONTEXT.md`; today the area and its first capability coincide.

Enforced by: Bazel `visibility` once ADR-0001 lands — a capability's `ports` crate is public within its area; its domain, store and web crates are visible only to the capability itself and to `<area>/server` (structure). Until then: review.
