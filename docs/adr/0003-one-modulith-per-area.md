---
status: accepted
date: 2026-09-16
---

# One modulith per area

Each area of ADR-0002 is built as one **modulith**: one deployable binary per area, composed at that area's single composition root out of **capabilities** that are bounded as strictly as if a network sat between them. The Deposit Area's modulith grows from the metadata editor (its next capabilities are the data-model creator and data creation); the Access Area's grows from DPE (then CPE, the SPARQL endpoint, the admin view); the Archive Area's is Spycherli. Between areas nothing changes: they still integrate over the wire (ADR-0002).

One binary per area is the decision for now. It keeps the option to rethink towards services later, if performance demands several copies of one capability, because a capability behind a port can be extracted without rewriting its callers — and the discipline that makes extraction possible is paid for now, while it is free, rather than retrofitted (rationale confirmed 2026-09-16). It also respects the read side's right-sizing rule: every read-side service carries its own local store and its own consumer of the archive's data products, so the number of applications is kept small and a new capability is added inside the binary, not beside it.

The contract inside an area:

- **Data sovereignty.** A capability owns its tables and is their sole reader and writer, even where capabilities share one database file. No cross-capability SQL, no cross-table joins.
- **Consumer-defined ports.** When capability A needs something from capability B, A declares a **port** — a trait naming exactly what A needs (a hypothetical example: `ProjectDirectory { fn by_shortcode(&self, shortcode) -> Option<ProjectRef> }`), plus its small boundary DTOs — in its own `ports` crate, which depends on nothing but `std` and the `shared-*` crates. A port is a dependency-inversion interface, not a domain or a web concern, so synchronous and asynchronous ports live in the same crate. Ports are never shared between consumers: two capabilities needing the same thing from one provider each declare their own port, and the provider implements an adapter per port.
- **The provider implements the adapter, beside its data.** B implements A's port in B's own store crate, depending on `A/ports` and nothing else of A. Production adapters are named `Live<Port>` (`LiveProjectDirectory`), test doubles `Test<Port>` or `Fake<Port>`; the crate location already says which provider fulfils the port, so the name does not encode the storage.
- **One permitted capability-to-capability dependency:** a provider on a consumer's `ports` crate. Never on another capability's domain, store or web crate. A port declared by a `shared-*` crate stays in that crate; a capability implementing it depends on the shared crate, which is always allowed.
- **References are opaque ids.** A capability stores only the foreign identifier (a shortcode, an entity id) — a foreign key by convention, not a database constraint across capabilities — and fetches names, labels and details through the port, never by reading the other capability's tables.
- **The composition root holds no adapter logic.** `<area>/server` constructs each provider's adapters and injects them into the consumers at startup; it is wiring, nothing else.
- **Shared concepts, not shapes.** A value that is the same concept across capabilities lives in a `shared-*` crate; a boundary DTO returned by a port belongs to the consumer's `ports` crate and is not shared code.
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
  **Amended 2026-09-28:** the Access Area's root moved to `areas/access/server` ahead of its second capability (ADR-0007's amendment); `dpe-server` is now DPE's library crate.
- A new capability in an area is a directory beside the existing ones, its own crates, and a mount plus adapter wiring at the composition root — never an import of a sibling capability.
- More indirection per cross-capability need (a port plus an adapter), boundary DTOs defined per consumer rather than shared, and read paths that would have been one SQL join become a port call plus in-memory composition. Accepted.
- Each area keeps one `CONTEXT.md` per capability once vocabulary diverges, indexed from the root `CONTEXT.md`; today the area and its first capability coincide.

Enforced by: Bazel `visibility` once ADR-0001 lands — a capability's `ports` crate is public within its area; its domain, store and web crates are visible only to the capability itself and to `<area>/server` (structure). Until then: review.

## Amendment (2026-09-25): authentication ports, the area shell, and enforcement before Bazel

Three rules the Deposit Area's split into two capabilities (`areas/deposit/ADR-0004`) needed and this record did not state. They are general: the Access Area meets all three when its admin view arrives beside DPE.

- **Request authentication crosses a capability boundary as a consumer-defined port, and each web crate owns its own extractors.** A capability that needs to know who is asking declares the principal DTO and the authenticator port in its own `ports` crate; the capability that owns sessions implements it in its store crate; the consumer's web crate builds its extractors over that port. No area holds a shared extractor crate, and no composition root authenticates in a middleware: a handler's signature stays the place that says whether it is public.
- **An area may hold one shell crate, `areas/<area>/shell`.** A view library (`fn -> Markup`: the document, header, footer, error pages) that depends on `mosaic-tiles` and `shared-*` only, never on a capability crate; every capability's web crate and the composition root depend on it. The routes it links are string literals, a coupling recorded, not compiled.
- **Until Bazel visibility (ADR-0001) exists, the crate graph inside an area is checked by `.github/scripts/check-capability-deps.sh`** over `cargo metadata`, run by `just check`. Between two capabilities the only permitted edge is a provider's store crate on a consumer's `ports` crate; a `ports` crate depends on nothing in the workspace but `shared-*`; the shell crate depends on no capability crate; the composition root may depend on anything in its area. Every other edge fails the check.

  Note (2026-10-13, DEV-7375): the composition root may not depend on anything in its area; the amendment of 2026-10-13 below narrows it.

Added to Considered Options:

- **One `auth` crate per area holding one extractor for every web crate** — rejected: it needs the session lookup, so it depends on the session-owning capability's store crate and every consumer inherits that edge; behind a port it is the rule above with one crate more.
- **The shell in the composition root** — rejected: the handlers that need it live in the capabilities' web crates, so the root would either implement a shell port per consumer, which is view code at the composition root, or rewrite responses in a layer.

Enforced by: the crate-graph script (**static-analysis**) once it lands with the first two-capability area, Bazel `visibility` afterwards (**structure**); the no-middleware rule by **review** (a middleware is one `.layer()` line).

## Amendment (2026-09-28) — a derived read model is not a copy of the projection

The rejected option "each reading capability embedding its own Chischtli" rejects a *copy* of the archive projection per reader: the same graphs held three times, with three consumers of the archive's data products. It does not reject a reader keeping its own **read model** — a store of its own shape and engine, holding only what it needs, derived from what its port against `sync` serves, written by the reader alone and rebuilt from empty rather than repaired. The two were not distinguished when this record was written and the distinction is what ADR-0007's CPE store rests on; ADR-0008 records it as a general rule. The clause on Chischtli above is unchanged: `sync` stays the single writer of the projection and the single consumer of the data products, and every reader reaches the projection only through its own port.

## Amendment (2026-10-13): what a composition root depends on, and where the crate-graph gate starts

The amendment of 2026-09-25 lets the composition root "depend on anything in its area". `check-composition-root-deps.sh`, added with the Access Area's root (`docs/specs/2026-09-28-access-area-composition-root/`, DEV-7397), holds a root to each capability's `server` crate and never its `core`, `web` or `api-*` crate, so that the root stays wiring and names no capability's type. The two disagree, and the Deposit Area's target (`areas/deposit/ADR-0004`) fits neither: its capabilities have no `server` crate, and their routes live in the web crate, where this record's crate anatomy puts them ("a web crate of views and routes"). Decided 2026-10-13 (DEV-7375):

- **A composition root depends on each capability's entry crate, and on its `store` and `ports` crates only to construct and inject adapters; never on a `core` or `api-*` crate.** The entry crate is the capability's `server` crate where it has one (DPE, CPE), and its `web` crate where it has none (the Deposit Area). A capability that has a `server` crate keeps its `web` crate out of the root's reach. Dev-dependencies are held to the same rule.
- **The entry crate exposes one facade that owns the capability's configuration, startup and router** (`Dpe`, `Cpe`; the Deposit Area's `Identity` and `Editor`), so the root names no domain type and holds no logic. That plan's reasons for the rule (the root "holds wiring only" and "never names `Corpus`") hold under the narrowed one.
- **`check-capability-deps.sh` checks the areas it lists, the Deposit Area first.** The Access Area joins once the script knows its crate roles beyond this record's anatomy (`components`, `image-viewer`, `store-build`, `api-*` and the capability `server` crate) and ADR-0008's exception for a dev-dependency on `sync-store` (amendment of 2026-10-11); `check-cpe-seam.sh` holds CPE's edges until then. The root's own edges stay with `check-composition-root-deps.sh`, so each rule has one enforcer.

Added to Considered Options:

- **A `server` host crate per Deposit capability, the root depending on the hosts only, as in the Access Area**: rejected (decided 2026-10-13, DEV-7375). It keeps the gate as it is and leaves the handlers where they are today, but adds two crates, in a role the crate anatomy does not list, to hold routes the anatomy places in the web crate.

Enforced by: `check-composition-root-deps.sh`, which learns the `server`-or-`web` distinction in DEV-7375, and `check-capability-deps.sh` for the areas it lists (**static-analysis**); the facade by **review**.
