# Archiving Manual

**Status: design in progress.** This manual specifies the **AIP** — how the Archive stores, validates, and preserves what it receives — together with the **DAO write-side model** (layer 2) and its **OCFL serialization** (layer 3). It supersedes the storage narrative in `dao-discovery.md` §3.6/§3.7 following decision 52.

**Documentation map** — the four-layer model maps onto the OAIS information packages:

| Layer | Package | Home |
|---|---|---|
| 1 | SIP (Producer input) | [`producer-deposit-manual.md`](./producer-deposit-manual.md) |
| 2–3 | AIP (model + storage) | **this manual** |
| 4 | DIP (Consumer output) | [`consumer-manual.md`](./consumer-manual.md) |

Rationale + decision log: [`dao-discovery.md`](./dao-discovery.md). Glossary: [`CONTEXT.md`](./CONTEXT.md) and [`../../UBIQUITOUS_LANGUAGE.md`](../../UBIQUITOUS_LANGUAGE.md). When this manual and `dao-discovery.md` disagree, `dao-discovery.md`'s decision log is authoritative until reconciled.

---

## 1. The DAO model (layer 2)

DAO models the **write side**: persistent-identity entities and events. Versions are read-side projections, **not** DAO classes (§3.4, decisions 11/16). Write-side classes (`dao-discovery.md` §6; decisions 9/15/26/45):

`dao:Resource`, `dao:Representation`, `dao:Project`, `dao:Agent`, `dao:Event` (+ subclasses), `dao:Deposition`, `dao:DepositAgreement`, `dao:PreservationAction`, `dao:AccessPolicy`.

The canonical truth is the append-only **event log** (§5, decision 5); current state is a fold over it. The DAO ontology + its **stored** SHACL profile are the authoritative schema (one DAO vocabulary, multiple SHACL profiles — submission/stored/dissemination; the *submission* profile is the Producer manual's concern). Ontology + SHACL files: TBD.

## 2. Events — the canonical record

- **Vocabulary:** `dao-discovery.md` §5.1 (`ResourcePublished`, `RepresentationCreated`, `DepositionAccepted`, `FixityChecked`, `FormatMigrated`, `AccessRuleChanged`, `AccessPolicyCreated`/`Retired`, `Tombstoned`, `Redacted`, `PreservationActionExecuted`).
- **Fat events** (decision 52): every state-committing event carries the **full DAO metadata snapshot** (not deltas — decision 24/§5.2) + bitstream **byte-references (multihash)**. Entity state is a projection of these events, never stored separately.
- **Embedded fixity** (decision 53): each event carries the multihash of every referenced bitstream (per-file content fixity) and a **per-event payload checksum** (SHA-256, decision 39) over its canonical RDF. Each AIU is independently verifiable.
- **Format:** NDJSON-JSON-LD, one event per line, with `@id`, `@type`, `global_offset`, `stream_id`, `stream_version`, `timestamp`, `event_schema_version`, payload, `crc32` (decision 37). WORM.

## 3. Storage (layer 3) — supersedes §3.7 (decision 52)

OCFL holds exactly **two** things:

1. **Preservation File bitstreams**, content-addressed by multihash.
2. **The event log**, as sealed NDJSON-JSON-LD segments in open, preservation-grade form (decision 37) — the durable canonical copy.

There is **no entity-state substrate** (decision 52 dropped decision 36's per-entity versioned OCFL objects). `dao:Resource`/`dao:Representation` RDF lives only in events; a `dao:Representation`'s OCFL object holds only its **bytes** (amends decisions 33/34).

- **Read / serving = Redb** (decision 38), fully rebuildable from the event log. **One instance, two read models** (decision 54): **A** — command-validation + the Archive GUI; **B** — synchronous `QueryAPI`.
- **Write path:** command → **Tier-2 validation** against read-model A (§5) → append event(s) to the log + write referenced bitstreams to OCFL → update both read models → emit on the `EventStream`.
- **Recovery:** rebuild Redb by replaying the event log; verify bitstreams by multihash.
- **Fixity** (decision 52 amends 40): per event-log **segment** (SHA-256 manifest, `sha256sum -c`-compatible, decision 39) + per **bitstream object** + per-event payload SHA-256 + per-line CRC32 (decisions 37/53). `FixityChecked` events bind to segments and bitstream objects, not to per-Representation OCFL objects.
- **Embedded store, not off-the-shelf** (decision 52): append-log + Redb index, no operational event-store dependency.

## 4. The AIP (decision 53)

- **Virtual / on-demand** — no AIP is stored as a package. The preserved substrate is the event log + bitstreams; an AIP is a *view*.
- **AIU = a Resource:** its event stream (its `ResourcePublished` versions + `AccessRuleChanged` + the events of the Representation Versions it pins) ∪ the referenced Preservation File bitstreams.
- **AIC = a Project / Deposition** (collection of AIUs).
- **PDI-completeness invariant:** events must carry enough PDI per AIU — Provenance (event history), Fixity (embedded checksums + `FixityChecked`), Reference (URN/ARK), Context (relationships), Access Rights — that a complete, verifiable AIP is reconstitutable.
- **On-demand serialization** to BagIt / RO-Crate for export, audit, succession, and display (decision 42). A future **Archive-side GUI** displays AIPs on demand (consumer of read-model A).
- **Documented deviation** (decision 27): OAIS requires the information preserved, not a physically constituted AIP.

## 5. Command validation (decision 54)

Commands are rejectable intents; events are emitted only after validation (decision 14). All mutations are commands to the internal `CommandAPI` (decision 47).

- **Tier 1 — Ingest Service (stateless):** SHACL *submission* profile + format-ID + ClamAV. Producer-facing edge; the Producer manual owns its rules.
- **Tier 2 — Archive command handler (stateful, authoritative):** referential integrity (referenced `dao:Resource`/`dao:Representation`/`dao:AccessPolicy` exist; pinned Representation Version real; policy not retired), domain invariants (no publish onto a `Tombstoned` entity, etc.), optimistic concurrency (`expected_stream_version`), validated against **read-model A**. **Only Tier 2 emits events.**
- **Two command paths:** Producer (Ingest → commands) and **Preservation Management** (admin tooling → commands directly, skipping Tier 1 — they are not SIPs — but passing Tier 2). Both converge on the single stateful gate.

## 6. Public interfaces (decision 47)

`CommandAPI` (internal-only, mTLS), `EventStream` (gRPC server-streaming → subscribers; see the Consumer manual), Binary retrieval (`/bitstreams/{multihash}`), `QueryAPI` (synchronous, read-model B), `/metrics` (operational telemetry, decision 41). OCFL is exclusive to the Archive boundary.

## 7. Certification evidence

The event log is the provenance + fixity evidence base across CTS → nestor → ISO 16363. See `dao-discovery.md` §8.1 (evidence index).

> **Pending:** inline amendment notes on decisions 24/33/34/36/40 in `dao-discovery.md` (decision 52 enumerates them); the DAO ontology + stored SHACL profile files; the on-demand AIP serializer specifics.
