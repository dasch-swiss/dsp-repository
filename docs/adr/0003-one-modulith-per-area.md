# One modulith per area

Each area of ADR-0002 is built as one **modulith**: one deployable binary per area, composed at that area's
single composition root, `areas/<area>/server`, out of **capabilities** that are bounded as strictly as if a
network sat between them. Every area has its composition root at `areas/<area>/server` from its first crate on,
and each capability is mounted there through its facade. Between areas nothing changes: they integrate over the
wire (ADR-0002).

The capabilities:

- The Deposit Area: `identity` and `editor` (`areas/deposit/ADR-0004`); the data-model creator, data creation
  and the Deposit Area's `media` are planned beside them.
- The Access Area: DPE, `sync` and CPE (ADR-0007) and the Access Area's `media` (ADR-0012); DaTEI (ADR-0009),
  `profile`, the SPARQL endpoint and the admin view are planned beside them.
- The Archive Area: Spycherli.

One binary per area keeps the option to split into services, if performance demands several copies of one
capability: a capability behind a port can be extracted without rewriting its callers, and the discipline that
makes extraction possible is paid for while it is free rather than retrofitted. It also respects the read
side's right-sizing rule: every read-side service carries its own local store and its own consumer of the
archive's data products, so the number of applications is kept small and a new capability is added inside the
binary, not beside it.

The contract inside an area:

- **Data sovereignty.** A capability owns its tables and is their sole reader and writer, even where
  capabilities share one database file. No cross-capability SQL, no cross-table joins.
- **Consumer-defined ports.** When capability A needs something from capability B, A declares a **port** — a
  trait naming exactly what A needs (for example `ProjectDirectory { fn by_shortcode(&self, shortcode) ->
  Option<ProjectRef> }`), plus its small boundary DTOs — in its own `ports` crate, which depends on nothing but
  `std` and the `shared-*` crates and on no third-party crate. A port is a dependency-inversion interface, not a
  domain or a web concern, so synchronous and asynchronous ports live in the same crate. Ports are never shared
  between consumers: two capabilities needing the same thing from one provider each declare their own port, and
  the provider implements an adapter per port.
- **The provider implements the adapter, beside its data.** B implements A's port in B's own store crate,
  depending on `A/ports` and nothing else of A. Production adapters are named `Live<Port>`
  (`LiveProjectDirectory`), test doubles `Test<Port>` or `Fake<Port>`; the crate location already says which
  provider fulfils the port, so the name does not encode the storage.
- **One permitted capability-to-capability dependency:** a provider's store crate on a consumer's `ports`
  crate. Never on another capability's domain, store or web crate. A port declared by a `shared-*` crate stays
  in that crate; a capability implementing it depends on the shared crate, which is always allowed. The one
  exception is ADR-0008's: a crate of a reading capability may name `sync-store` as a dev-dependency, and only
  as one.
- **References are opaque ids.** A capability stores only the foreign identifier (a shortcode, an entity id) — a
  foreign key by convention, not a database constraint across capabilities — and fetches names, labels and
  details through the port, never by reading the other capability's tables.
- **Request authentication crosses a capability boundary as a consumer-defined port.** A capability that needs
  to know who is asking declares the actor DTO and the authenticator port in its own `ports` crate; the
  capability that owns sessions implements it in its store crate; the consumer's web crate builds its extractors
  over that port. No area holds a shared extractor crate, and no composition root authenticates in a
  middleware: a handler's signature stays the place that says whether it is public.
- **Shared concepts, not shapes.** A value that is the same concept across capabilities lives in a `shared-*`
  crate; a boundary DTO returned by a port belongs to the consumer's `ports` crate and is not shared code.
- **No distributed transaction across capabilities.** Each capability commits within its own transaction; work
  that spans capabilities is a sequence of per-capability transactions, coordinated by id and port call,
  consistent eventually.

Within a capability, the crates are `{capability}-{role}`. The general roles:

- `core` — the framework-free domain: no web framework, no Maud, no database crate.
- `store` — where the capability persists, and where it implements other capabilities' ports.
- `web` — views and, where the capability has no `server` crate, its routes and handlers. It may also depend on
  its area's shell and on `mosaic-tiles`.
- `ports` — the ports the capability declares as a consumer.
- `server` — the capability's host crate, holding its routes, configuration and startup behind its facade.
- `api-*` — a machine-facing protocol surface of the capability (`dpe-api-oai`).

A capability may add a role. The role is named, with its allowed edges, in the record that governs the
capability or its area, and `check-capability-deps.sh` learns it in the same change: `pull-requests` in
`areas/deposit/ADR-0004`; `components`, `image-viewer` and `store-build` in ADR-0007.

An area may hold one **shell** crate, `areas/<area>/shell`: a view library (`fn -> Markup`: the document,
header, footer, error pages) that depends on `mosaic-tiles` and `shared-*` only, never on a capability crate.
Every capability's web crate and the composition root may depend on it. The routes it links are string
literals, a coupling recorded, not compiled.

**The composition root is wiring.** `areas/<area>/server` constructs each provider's adapters and injects them
into the consumers at startup, and holds no adapter logic. It depends on each capability's entry crate — the
`server` crate where the capability has one (DPE, CPE), its `web` crate where it has none (`identity`,
`editor`) — and on a capability's `store` and `ports` crates only to construct and inject adapters. It never
depends on a `core` or `api-*` crate, nor on the `web` crate of a capability that has a `server` crate;
dev-dependencies are held to the same rule. The entry crate exposes one facade that owns the capability's
configuration, startup and router (`Dpe`, `Cpe`, `Identity`, `Editor`), so the root names no domain type.

**Shared engines are libraries behind capabilities, never capabilities of their own.** Two exist (ADR-0002), and
the same rule holds for both: the engine is a root-level library with no routes, no authentication and no
opinion about the area; everything area-specific arrives through the traits its interface takes; the durable
state it holds belongs to exactly one capability per unit of ownership; and the dependency arrow
`capability → engine` never reverses.

- **Vitrinli** exposes deep modules — IIIF rendering, range-served downloads, Service File derivation — whose
  traits cover where bytes come from, who may see them and where a derived file goes. Each area has a `media`
  capability, with the standard anatomy, that depends on Vitrinli, implements those traits over its own store,
  owns the tables that say what is servable and where, owns the routes and enforces authorisation with the
  one shared decision of ADR-0013, and talks to
  sibling capabilities through ordinary ports. In the Deposit Area, `media` receives the Originals a depositor
  uploads in preparation for archiving and has Vitrinli derive the Service Files the deposit frontends display;
  in the Access Area, `media` serves the Service Files the archive produced, and nothing else. An area's rules
  for media live in that area's code, where its other rules live — the typed successor of the Lua scripts that
  configure sipi.
- **Chischtli** is the triplestore engine, and its unit of ownership is the **named graph**: every graph has
  exactly one capability that writes it, and any other capability reads it through that owner's ports, never by
  opening the store. In the Access Area, the `sync` capability is the single writer of every graph that comes
  from the archive — it consumes the archive's projection stream and rebuilds the projection from snapshot plus
  replay — and DPE, CPE and the SPARQL endpoint read through `sync`'s query ports; a `profile` capability owns
  the small graphs of per-user settings it keeps beside them. In the Deposit Area, the capability that owns data
  creation writes the working graphs the same way, with SHACL validation on update where a project opts in. One
  Chischtli instance per area therefore serves several capabilities without becoming a second writer for any of
  them.

**One consumer per channel.** The archive publishes on two channels (ADR-0012). The projection stream,
snapshots and deltas announced on the bus, is consumed by `sync` alone, which writes the projection. The Access
bucket, where Service Files land, is consumed by the Access Area's `media` alone, which serves them. Neither
capability reads the other's channel, and no third capability reads either. A reader may still keep a read
model of its own, derived from what its port against `sync` serves and written by the reader alone; that is not
a second copy of the projection (ADR-0008).

## Considered Options

- **One modulith per area (chosen).**
- **One modulith for the whole platform** — rejected in ADR-0002: the Deposit and Access Areas are separate
  deployables on separate origins for a stated security reason, and the archive is sealed from both.
- **One deployable per capability (DPE, CPE, the editor, … each its own binary)** — rejected: each binary would
  carry its own local store and its own consumer of the archive's data products, multiplying operational
  surface for a boundary the modulith gives for free.
- **Extracting a capability from the modulith into a service of its own** — *deferred*. It comes back when
  performance demands several copies of one capability; a capability behind its ports is extracted without
  rewriting its callers.
- **Capabilities sharing a database freely inside an area** — rejected because it couples schemas, makes
  extraction impossible, and lets "who set this value?" have no single answer. The same applies to a shared
  triplestore: a graph with two writers is a table with two writers.
- **Each reading capability holding its own copy of the archive projection** (DPE, CPE and the SPARQL endpoint
  each with the same graphs) — rejected: three copies of one projection and three consumers of the archive's
  data products, which is the read-side cost the right-sizing rule exists to avoid. A read model of the
  reader's own shape is a different thing and is allowed (ADR-0008).
- **One shared `ports` crate for the whole area** — rejected: it becomes a crate every capability depends on, the
  same failure as a shared domain crate, and it strips each port of the authorship of the consumer that defines
  it.
- **A `graph` capability owning the whole Access-Area store** — rejected: it would make one capability the
  writer of graphs it does not understand (archive projection and user settings alike); ownership per named
  graph keeps each writer with the data it knows.
- **Adapters at the composition root** — rejected: `<area>/server` would accrete adapter logic and couple itself
  to every provider's storage; the narrow `ports` dependency lets the adapter live with its data.
- **One `auth` crate per area holding one extractor for every web crate** — rejected: it needs the session
  lookup, so it depends on the session-owning capability's store crate and every consumer inherits that edge;
  behind a port it is the rule above with one crate more.
- **The shell in the composition root** — rejected: the handlers that need it live in the capabilities' web
  crates, so the root would either implement a shell port per consumer, which is view code at the composition
  root, or rewrite responses in a layer.
- **A `server` host crate per Deposit capability, the root depending on the hosts only, as in the Access Area**
  — rejected: it adds two crates in a role whose only content would be routes the `web` crate already holds.
- **Chischtli as a shared service (one triplestore process per area, reached over a socket)** — not chosen for
  the layout: whether the engine runs in-process or as a supervised sidecar of the area's binary is an
  operational choice inside the area's modulith and does not change who owns which graph or who may depend on
  the crate.
- **Vitrinli as a service on its own origin** — rejected: each area needs it against different inputs (Originals
  in the Deposit Area, archive-made Service Files in the Access Area) and behind its own authentication, which a
  capability inside each modulith gives for free and a shared service would have to re-invent.
- **Vitrinli mounted directly as a capability in both moduliths** — rejected: it would own tables and routes in
  two databases and two routers, and it would have to consume area data through ports whose direction is easy
  to get backwards; a library with an explicit trait interface plus a per-area `media` capability keeps the
  arrow `areas → vitrinli` unambiguous.
- **Vitrinli configured per area by files, as sipi is by Lua scripts** — rejected: untyped per-deployment
  configuration is what no test covers and no agent can trace to an owner; the `media` capability is typed Rust
  that fails to compile when wired wrongly.

## Consequences

- A new capability in an area is a directory beside the existing ones, its own crates, and a mount plus adapter
  wiring at the composition root — never an import of a sibling capability.
- More indirection per cross-capability need (a port plus an adapter), boundary DTOs defined per consumer rather
  than shared, and read paths that would have been one SQL join become a port call plus in-memory composition.
  Accepted.
- A capability's vocabulary goes into its own `CONTEXT.md` once it has terms of its own, placed by the rule in
  ADR-0002 and indexed from the root `CONTEXT.md`.
- Bazel `visibility` (ADR-0001) makes a capability's `ports` crate public within its area and its other crates
  visible only to the capability itself and to `areas/<area>/server`.

Not yet in code: the data-model creator, data creation, `profile`, the SPARQL endpoint, the admin view and
Spycherli have no crate (not scheduled).
Not yet in code: the Deposit Area has no `media` capability; the Access Area's `media` is its hand-fed
stand-in, `media-store` over a committed directory (ADR-0012), with no routes and no authorisation check; and
Vitrinli is not in this repository (not scheduled).
Not yet in code: `sync` serves a committed N-Quads snapshot through `sync-store`, not a Chischtli projection
rebuilt from snapshot plus replay (not scheduled).
Not yet in code: DPE reads its own data directory (`areas/access/dpe/server/data`) through `dpe-core`, not
through a port against `sync` (not scheduled).
Not yet in code: `check-capability-deps.sh` checks only the Deposit Area (`CHECKED_AREAS="deposit"`); it does
not know the Access Area's roles `server`, `api-*`, `components`, `image-viewer` and `store-build`, nor
ADR-0008's dev-dependency on `sync-store`, and `check-cpe-seam.sh` holds CPE's edges meanwhile (DEV-7527).

Enforced by: `check-capability-deps.sh`, run by `just check`, for the crate edges inside the areas it lists
(static-analysis); `check-composition-root-deps.sh` for the composition roots' edges, so each rule has one
enforcer (static-analysis); `check-cpe-seam.sh` for CPE's edges (static-analysis), with Bazel `visibility`
under ADR-0001 as the target (structure), also for "one consumer per channel"; review for data sovereignty, the facade, the no-middleware rule (a middleware is one
`.layer()` line) and the engine rules.
