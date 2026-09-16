# Chischtli

The triplestore engine of the platform, the in-house replacement for Fuseki: a library around an embedded RDF store with graph-granular write dispatch, exact update deltas, full-text search and optional SHACL validation on update. Chischtli (Swiss German, "little box") is designed and built separately today. When it moves into this monorepo it lives at `chischtli/` as a root peer of the areas (ADR-0002), depended on by capabilities in the Deposit Area and the Access Area (ADR-0003). It is neither a service nor a capability: it has no routes and no authentication, and it holds no opinion about which graphs exist or who may read them. Its unit of ownership is the **named graph**, and every graph has exactly one writing capability. This file is the seed that records, ahead of the code, what those capabilities may rely on; the code brings its own vocabulary with it.

## Language

**Named graph**:
The unit of ownership and of write dispatch: a set of triples under one IRI, written by exactly one capability and read by others only through that capability's ports.
_Avoid_: dataset (Fuseki's word for a whole store), collection, table.

**Owning capability**:
The one capability that writes a Named graph — in the Access Area, `sync` for every graph that comes from the archive and `profile` for per-user settings; in the Deposit Area, the capability that owns data creation for the working graphs.
_Avoid_: tenant, client.

**Update**:
A write to one Named graph, dispatched under that graph's lock and applied atomically; Chischtli reports the exact delta (triples added, triples removed) so the owner can publish or audit it.
_Avoid_: transaction (implies a multi-graph scope Chischtli does not promise), commit.

**Query**:
A read over one or more Named graphs, SPARQL or a typed accessor, served without blocking Updates to other graphs. Who may run which Query is the owning capability's decision, enforced in its ports.

**Full-text index**:
The search index Chischtli maintains beside the graphs under a per-graph profile, so a capability's search is a Query and not a second store.

**Shape validation**:
SHACL validation of an Update before it is applied, on for a graph whose owner opts in; the shapes are the owner's, generated from the project's data model.
_Avoid_: schema check, constraint (too general).

**Projection**:
In the Access Area, the graphs `sync` rebuilds from the archive's data products — snapshot plus replay — and DPE, CPE and the SPARQL endpoint read; disposable, never authoritative, rebuilt rather than repaired.
_Avoid_: cache, replica (the store is complete for what it covers and is not a copy of another store), mirror.

**Working store**:
In the Deposit Area, the graphs data creation writes before anything is archived: the successor of the active-research platform's triplestore, whose contents become a SIP when the depositor submits.
_Avoid_: staging (the archive's quarantine bucket), draft store.

## Relationships

- One **Chischtli** library, one instance per area's modulith; each instance holds many **Named graphs**.
- Every **Named graph** has exactly one **Owning capability**; every **Update** to it comes from that capability.
- In the Access Area, `sync` owns the **Projection** graphs and `profile` owns its settings graphs; DPE, CPE and the SPARQL endpoint issue **Queries** through `sync`'s ports and write nothing.
- In the Deposit Area, the data-creation capability owns the **Working store** graphs and may switch **Shape validation** on per project.
- A **Full-text index** belongs to the graph it indexes and follows its owner.

## Boundary commitments

- Chischtli depends on no area crate and knows no area's session, rights or graph names; those arrive through its interface (ADR-0002, ADR-0003). Target enforcement: structure, via Bazel visibility.
- A capability that does not own a graph never opens the store to read it; it goes through the owner's ports. Target enforcement: review, then structure (the store handle is visible only to owning capabilities).
- The Access Area's **Projection** is written by `sync` alone and only from the archive's data products; nothing in the Access Area edits archived data in place.
- Whether an instance runs in-process or as a supervised sidecar of the area's binary is an operational choice inside the modulith and changes no ownership rule.
- Staleness is legitimate on the access side: while the archive is unavailable, the Projection serves what it has.

## Example dialogue

> **Dev:** "CPE needs a new facet over the archive projection. Do I add a graph in CPE?"
> **Domain expert:** "No. The projection graphs are `sync`'s; CPE cannot write them and should not open the store. If the facet is a **Query**, ask for it through `sync`'s ports. If it needs data the projection lacks, that is a change to what `sync` rebuilds from the archive's data products."

> **Dev:** "Where do a user's saved searches go in DPE?"
> **Domain expert:** "In a graph the `profile` capability owns, beside the projection but never inside it. `profile` writes it; DPE reads it through `profile`'s ports."

## Flagged ambiguities

- **"Triplestore"**: the engine (Chischtli), or the instance one area runs, or Fuseki in older material. Resolution: Chischtli for the engine, "the Access-Area Chischtli" or "the working store" for an instance; Fuseki only historically.
- **"Store"**: Chischtli, the editor's SQLite database, and a capability's `store` crate all get called that. Resolution: say Chischtli, "the database", or "the store crate".
- **"Dataset"**: Fuseki's word for a whole store, and a retired contract term (see the root `CONTEXT.md`). Resolution: not used here; say Named graph or instance.
- **"Sync"**: the Access Area's `sync` capability, and the generic act of synchronising. Resolution: `sync` in code font for the capability; "rebuild from snapshot plus replay" for what it does.
