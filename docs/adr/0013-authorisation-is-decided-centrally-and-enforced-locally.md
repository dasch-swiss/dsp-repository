# Authorisation is decided centrally and enforced locally

Every surface of the Access Area answers the same question before it shows anything: may this principal see
this. DPE, CPE, DaTEI, `media` and the SPARQL endpoint all serve the archive's data, and ADR-0012 makes CPE
and DaTEI serve what the requesting user may see. If each capability answered on its own, the answers would
drift: a resource hidden on a CPE page could appear in a DPE listing or come back from the SPARQL endpoint,
and an image `media` refuses could be described in full beside it. This record fixes where the answer comes
from and where it is applied. The decision, phrased so a violation is describable in code:

- **The archive owns the policy data, and it travels with the facts.** Rights are recorded by the archive per
  Representation version, never per value: a version is visible as a whole or not at all. A port that serves
  a resource or a file serves its rights with it (ADR-0012); no capability authors, overrides or infers a
  right.
- **Identity comes from one Access-Area capability through a consumer-defined port.** Who is asking is
  answered by the one capability that owns sessions; each capability that needs it declares its own
  authenticator port and principal DTO in its own `ports` crate, as ADR-0003 prescribes. No surface reads a session, a token or a cookie of its own.
- **The decision is one pure function in a `shared-*` crate.** It takes a principal and a set of rights and
  returns allow or deny, with no I/O: no store, no network, no clock it reads itself. It is the one place
  that says what a right means. Every capability that enforces depends on it, as any capability may depend
  on a `shared-*` crate (ADR-0003), and none re-implements it.
- **Each capability enforces the decision in its own query layer.** A read model or a projection compiles
  the rights it is served into a column per row, and the capability applies the decision as one predicate
  over that column, so a search, a facet count and a listing filter in place and agree with the page a row
  leads to. `media` applies the same function to bytes at request time, in its implementation of Vitrinli's
  Authorisation check, so a file and the page that shows it get the same answer.
- **Until an authentication mechanism exists, every request is the anonymous principal.** Each surface
  evaluates the function for it; that is ADR-0012's interim for CPE and DaTEI.

## Considered Options

- **One pure function in a shared crate, enforced in each capability's query layer (chosen).**
- **A per-request authorisation service**, a decision point over the wire or a capability the others call —
  rejected: a listing, a search or a facet count would need a call per row, or would filter after paging,
  which breaks counts and pages; and every surface would depend on it at runtime.
- **Each capability with rules of its own** — rejected: the semantics drift, and "may this principal see
  this" gets as many answers as there are capabilities. The archive owns the policy data and the shared crate
  the decision; a capability only enforces.
- **A provider that pre-filters, serving each consumer what a given principal may see** — rejected: a read
  model built per principal cannot be shared between requests, and one built for the anonymous consumer
  cannot serve a signed-in user (ADR-0012).
- **Rights per value** — rejected: a row would carry as many rights as it has values, and a listing or a
  count would have no single answer for a resource that is partly visible.

## Consequences

- ADR-0012's presentation clause has this record as its target: CPE's and DaTEI's ports serve rights, their
  read models store them per row and their presenters filter every request with this record's function.
- The port against `sync` serves rights per resource; CPE's port table moves permissions from out to in when
  it does (`areas/access/cpe/CONTEXT.md`, "What the port serves").
- A `shared-*` crate joins `shared/` when the first capability enforces; it depends on nothing that does I/O.
- `media` owns its routes and its authorisation check (ADR-0003); the decision the check applies is this
  record's function.
- The SPARQL endpoint, when it is built, filters in its query layer as the other surfaces do.

Not yet in code: the shared crate and its function, rights on any port, a rights column in any read model,
the predicate in any query layer, `media`'s check and the Access Area's identity port (not scheduled).

Enforced by: review until the crate exists; then the crate graph (`check-capability-deps.sh`, Bazel
`visibility` after ADR-0001) for "the decision lives in one `shared-*` crate with no I/O dependency"
(**structure**); one test per surface that a listing, a facet count and a page agree on a restricted row
(**static-analysis**); "no capability decides a right itself" stays with review.
