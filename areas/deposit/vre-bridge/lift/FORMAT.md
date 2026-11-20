# dao-lift interim project snapshot format

The file format `vre-bridge-lift`'s `lift` returns: one VRE project, current state only, as sorted
N-Quads. `dsp-repository`'s `sync-store` (PR #455, DEV-7399) reads it and transcribes its
vocabulary from this document.

Section numbers are stable. A later change may add a sub-section, but never renumbers one, so citations
such as "FORMAT.md §6" in code stay valid. Every change after the first publication is recorded in §16.

## §0 Status

**Temporary — this is NOT DAO.**

This format follows DAO where DAO has decided, and lists every place where it does not (§12). "DAO has
decided" means the Archive decision log `dsp-repository-design/spycherli/decisions-active.md` at commit
`a3a6dd87`, cited below as d.N (decision number N in that file). A term in the `dao:` namespace is not
thereby DAO-decided: §15 marks each term as DAO or names its §12 deviation.

- **Where it lives.** This copy, `areas/deposit/vre-bridge/lift/FORMAT.md` in `dsp-repository`, is
  authoritative. It was ported with the `vre-bridge-lift` crate that writes the format, the library form of
  `dao-lift`'s `snapshot`, from dsp-incubator's `cpe/tools/dao-lift/` at
  `6106ee38166617cd8be5d58411d5bb14741ebc03`; "`dao-lift`" below names that producer.
- **Pin.** The version that counts is this copy as committed beside the reader: a reader implements the
  `FORMAT.md` of its own `dsp-repository` commit, not a later one. `areas/access/sync/data/PROVENANCE` records
  the earlier `dao-lift` commit that produced the committed 0803 fixture, not the ported source.
- **Version.** This is interim format version 1 (`vre-bridge-lift`'s `INTERIM_FORMAT_VERSION`, recorded in each
  Ready token). The version is raised when a reader would read a snapshot differently. A reader may ignore
  `dao:accessRights` and `dao:dataClassification` (§2, §8) until it carries rights itself, and
  `dao:valueHasXml` (§4.1) until it renders markup.
- **Exit condition.** When DAO is published, `dao-lift` and this file move to it: every §12 row is resolved
  either way, and this document is retired.
- **Provisional choices.** The owner decided the link predicate, the annotation marker, the default IRIs and
  the `dao:` namespace (DEV-7443 plan, `cpe/docs/plans/dao-lift-interim-snapshot/implementation-plan.md`,
  "Owner decisions", 2026-09-30), and the U+001E rule (§12 alignment note). Every other choice below was settled
  by the DEV-7443 plan and stays **provisional until the owner verifies it on the draft PR**: value edges under
  the source property (§3), value IRIs from `kb:valueHasUUID` (§3), list values without `rdf:value` (§4.7), SKOS
  list nodes (§9), the colour and geometry datatypes and the omitted kinds (§4.8–§4.10), the value-kind table
  (§4), representations (§8), GREGORIAN and JULIAN as the only calendars (§4.6), the omissions (§11), membership
  and seqnum (§7), the predicate precedence (§6), link/LinkValue agreement (§6), annotations keeping their
  direct links (§10), identifying annotations by `oa:Annotation` (§10), `dao:valueHasOrder` only when
  recorded (§5), deviation terms in `dao:` (§12.1), canonical numeric and boolean lexicals (§1), legal
  information on representations (§8, D14), access rights and data classification as plain literals
  (§2, §8, D15, D16), and a formatted text's XML as `dao:valueHasXml` beside its plain `rdf:value` (§4.1, D12).

## §1 File and graph

- **Encoding.** UTF-8 [N-Quads](https://www.w3.org/TR/n-quads/), one quad per line, every line terminated by
  `\n` (no `\r`), no comment lines, no blank lines.
- **Order.** Lines are sorted bytewise by the whole line (not by subject) and are unique. `LC_ALL=C sort -c -u`
  accepts the file.
- **Determinism.** The same dump gives a byte-identical file under the pinned serialiser, `oxrdf` 0.3.4 (via
  `oxigraph` 0.5.11, `Cargo.Bazel.lock`). The form is deterministic, **not** RDF Dataset
  Canonicalization (RDFC-1.0) canonical. The file has no blank nodes, so RDFC-1.0 would add nothing.
- **One graph.** Every quad is in the named graph `urn:dsp:project:<shortcode>`, for 0803
  `<urn:dsp:project:0803>`. A quad in the default graph or in any other graph is invalid.
- **No blank nodes**, as subject or object. A blank node anywhere in the dump's data graph is a violation
  (`blank-node`).
- **Literals are verbatim**, with two exceptions. A text value's `rdf:value` and `dao:valueHasXml` drop every
  U+001E (§4.1); labels (`rdfs:label`, `skos:prefLabel`) keep it. And every `xsd:integer`, `xsd:decimal` and
  `xsd:boolean` literal, be it a value, an order, a JDN, a seqnum, a dimension or a list position, is written in its XSD 1.1 canonical
  lexical form, whatever form the dump writes: `"1.50"` becomes `"1.5"`, `"2.0"` becomes `"2"`, `"+5"` and `"05"`
  become `"5"`, and a boolean `"1"` or `"0"` becomes `"true"` or `"false"`. Language tags are lowercase;
  a dump tag that is not valid BCP 47 is a violation (`invalid-language-tag`).
- **Escapes.** The serialiser writes ECHAR for `\b \t \n \f \r \" \\`, and UCHAR `\uXXXX` (uppercase hex) for
  every other character in U+0000–U+001F, U+007F, U+FFFE and U+FFFF, so U+001E in a label is written
  `\u001E`. Every other character, including non-ASCII, is written as itself. A reader's parser must accept
  every ECHAR and UCHAR form.
- **`urn:dsp:`** is an unregistered, DaSCH-local URN namespace. The IRIs minted under it here (§1, §3, §8) are
  not dereferenceable and are not the d.12 identity IRIs (§12, D2).

Prefixes used in this document. They are notation only: N-Quads has no prefixes, and the file writes every IRI
in full.

| Prefix | IRI |
|--------|-----|
| `dao:` | `https://ontology.dasch.swiss/dao#` |
| `oa:` | `http://www.w3.org/ns/oa#` |
| `skos:` | `http://www.w3.org/2004/02/skos/core#` |
| `rdf:` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` |
| `rdfs:` | `http://www.w3.org/2000/01/rdf-schema#` |
| `xsd:` | `http://www.w3.org/2001/XMLSchema#` |
| `kb:` | `http://www.knora.org/ontology/knora-base#` |

`kb:` names knora-base, which appears in the file only as data: as a source property or source class used
directly by a project (§2, §6), never as format vocabulary. Project terms such as
`http://www.knora.org/ontology/0803/incunabula#hasTitle` are written `incunabula:hasTitle` below as shorthand
only.

## §2 Resources

A resource is every live subject of the dump's data graph whose one `rdf:type` is a class of a project ontology,
or `kb:Region` or `kb:LinkObj`. A subject typed with a class that is neither one of these nor a knora-base class
§11 lists is a violation (`unknown-class`).

- The subject is the resource's IRI as the dump records it, `http://rdfh.ch/<shortcode>/<id>`.
- `<resource> rdf:type dao:Resource` (d.9, d.15).
- Exactly one `<resource> dao:sourceClass <class IRI>` (d.72). The class IRI is the **full IRI** as the dump
  records it: a project class `http://www.knora.org/ontology/<shortcode>/<ontology>#<local>` (0803:
  `http://www.knora.org/ontology/0803/incunabula#Page`), or `http://www.knora.org/ontology/knora-base#Region`,
  `http://www.knora.org/ontology/knora-base#LinkObj` or a project sub-class of either. A consumer copies it
  verbatim and never compacts it.
- Exactly one plain (untagged) `<resource> rdfs:label "…"`. A resource whose dump label is missing
  (`missing-label`), repeated (`repeated-label`) or not a plain string, a language-tagged one included
  (`non-plain-label`), is a violation.
- No other `rdf:type`, except `oa:Annotation` (§10).
- Exactly one `<resource> dao:accessRights "public"` (§12, D15): the public guard passes a resource only when
  the anonymous user may view it (§11). And exactly one `<resource> dao:dataClassification "unclassified"`
  (§12, D16).

## §3 Value nodes

A value is attached by an edge and described by a node:

```text
<resource> <sourceProperty> <urn:dsp:value:<valueHasUUID>> .
```

- The **edge predicate** is the source property's full IRI, never compacted: a project property such as
  `http://www.knora.org/ontology/0803/incunabula#hasTitle`, or a knora-base property used directly such as
  `http://www.knora.org/ontology/knora-base#hasComment` (§12, D3).
- The **node IRI** is `urn:dsp:value:` followed by the value's `kb:valueHasUUID`, never by the last segment of
  the value's IRI (after an update, the current version carries the original version's UUID). The UUID is
  base64url (`^[A-Za-z0-9_-]+$`, non-empty) and unique in the file (§12, D2). A live value with no
  `kb:valueHasUUID` (`missing-source-literal`), or with one that is not base64url (`invalid-uuid`), is a
  violation. So are two live values that would mint the same node from one `kb:valueHasUUID`, on one resource or
  on two (`duplicate-uuid`). Why the scope is the whole file, not one resource, is recorded in §12.1.
- The node carries:
  - `rdf:type dao:Value`;
  - exactly one `dao:valueHasUUID "<uuid>"`, a plain string literal equal to the IRI's suffix;
  - exactly one `dao:sourceProperty <sourceProperty>`, equal to the edge predicate;
  - exactly one `rdf:value`, whose datatype names the kind (§4); list values have none (§4.7);
  - at most one `dao:valueHasOrder` (§5), and the kind's extra triples (§4.1, §4.6, §4.7).

### §3.1 Telling edges apart, positively

A triple whose subject is a resource (§2) is exactly one of:

| Predicate | Object | It is |
|-----------|--------|-------|
| `rdf:type`, `dao:sourceClass`, `rdfs:label` | — | resource structure (§2, §10) |
| any predicate | a node typed `dao:Value` | a value edge (§3) |
| `dao:isPartOf`, `dao:seqnum` | a resource / an `xsd:integer` | membership (§7) |
| `dao:hasRepresentation` | a node typed `dao:Representation` | a representation (§8) |
| `oa:hasTarget`, `oa:motivatedBy` | a resource / an `oa:` motivation | annotation structure (§10) |
| a predicate outside the `dao:`, `oa:`, `rdf:` and `rdfs:` namespaces | a resource IRI of the same file | a link (§6) |

The producer never emits a link, membership or target whose object is not a resource in the file.

## §4 Value kinds

The kind of a value is the datatype of its `rdf:value`. A reader must know every kind marked "must" and may
omit a value of a kind marked "may". The dump value class decides the kind; the dump predicate named in the
table is where `rdf:value`'s lexical comes from.

| § | Kind | Dump value class | `rdf:value` from | Datatype | Reader | Violation when |
|---|------|------------------|------------------|----------|--------|----------------|
| 4.1 | text | `kb:TextValue` | `kb:valueHasString` | plain, or `rdf:langString` from `kb:valueHasLanguage` | must | source literal missing or repeated; `kb:valueHasLanguage` repeated or not a valid tag; see §4.1 for the XML and the text type |
| 4.2 | integer | `kb:IntValue` | `kb:valueHasInteger` | `xsd:integer` | must | source literal missing or repeated, or not an `xsd:integer` |
| 4.3 | decimal | `kb:DecimalValue` | `kb:valueHasDecimal` | `xsd:decimal` | must | source literal missing or repeated, or not an `xsd:decimal` |
| 4.4 | boolean | `kb:BooleanValue` | `kb:valueHasBoolean` | `xsd:boolean` | must | source literal missing or repeated, or not an `xsd:boolean` |
| 4.5 | URI | `kb:UriValue` | `kb:valueHasUri` | `xsd:anyURI` | must | source literal missing or repeated, or not typed `xsd:anyURI` |
| 4.6 | date | `kb:DateValue` | `kb:valueHasString` | `dao:date` | must | see §4.6 |
| 4.7 | list | `kb:ListValue` | — (no `rdf:value`) | — | must | see §4.7 |
| 4.8 | colour | `kb:ColorValue` | `kb:valueHasColor` | `dao:color` | may | source literal missing or repeated, or not a plain string |
| 4.9 | geometry | `kb:GeomValue` | `kb:valueHasGeometry` | `dao:geometry` | may | source literal missing or repeated, or not a plain string |
| 4.10 | not carried | `kb:IntervalValue`, `kb:TimeValue`, `kb:GeonameValue` | — | — | — | never; omitted and counted |

A value class not in this table, and not a LinkValue or file value (§11), is a violation. The producer never
writes a value node with zero or two `rdf:value`s; every "violation" case fails the run (§11) instead. The
producer reports a missing source literal as `missing-source-literal`, a repeated one as
`repeated-source-literal`, and one that is not a literal, or has the wrong datatype or lexical, as
`invalid-literal`.

### §4.1 Text

`rdf:value` is `kb:valueHasString` with its whitespace kept and every U+001E (INFORMATION SEPARATOR TWO)
removed, wherever it occurs (d.51). It is a plain literal, or, when the dump records `kb:valueHasLanguage`, a
language-tagged literal with that tag, lowercased (`"x"@DE` in a dump is written `"x"@de`; §12, D5). Every other
control character is kept (and escaped, §1).

**Markup** (§12, D12). A value whose dump records `kb:valueHasXml` also carries exactly one `dao:valueHasXml`: a
plain string, that literal with every U+001E removed (d.51) and otherwise verbatim. It is the XML DSP-API renders
from the value's standoff, prolog included (for example
`"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<text><p>…</p></text>"`). `rdf:value` stays the plain text, so a
reader that does not render markup reads the value as before. The standoff itself is never read: a selected
(§11) value with `kb:valueHasStandoff` or `kb:valueHasMapping` but no `kb:valueHasXml` is a violation
(`missing-text-xml`), and so is a `kb:valueHasXml` that is repeated (`repeated-source-literal`) or not a plain
string (`invalid-literal`). The producer checks nothing inside the XML: it is not sanitised and not checked to be
well-formed, and a link in it may name a resource that is not in the file. A reader that renders it treats it as
untrusted input.
DSP-API writes `kb:valueHasXml` when it creates or updates a formatted text value; values written before it did
so get it from its backfill (`POST /admin/maintenance/backfill-value-has-xml`), which an operator runs before the
project can be lifted.

**Text type.** DSP-API writes `kb:hasTextValueType` on the text values it creates; older values have none. It is
read and never emitted, and a value without it is read as above. When present it is exactly one IRI: a repeated
one is `repeated-source-literal`, one that is not an IRI `invalid-literal`, and an IRI other than
`kb:UnformattedText`, `kb:FormattedText`, `kb:CustomFormattedText` and `kb:UndefinedTextType` is a violation
(`unknown-text-type`). `kb:FormattedText` and `kb:CustomFormattedText` require `kb:valueHasXml` like standoff
does, so a value of either type without it is `missing-text-xml`. A `kb:UnformattedText` value with
`kb:valueHasXml`, `kb:valueHasStandoff` or `kb:valueHasMapping` is a violation (`text-type-mismatch`): the type
and the markup contradict each other, and the producer does not guess which one is right. `kb:UndefinedTextType`
says nothing about the markup, so the value is read as one without a type.

### §4.2 Integer

`"<lexical>"^^xsd:integer`, the lexical in canonical form (§1).

### §4.3 Decimal

`"<lexical>"^^xsd:decimal`, the lexical in canonical form (§1): the dump's `"1.50"` is written `"1.5"`.

### §4.4 Boolean

`"true"^^xsd:boolean` or `"false"^^xsd:boolean`, in canonical form (§1) even when the dump writes `"1"` or `"0"`.

### §4.5 URI

`"<lexical>"^^xsd:anyURI`, the lexical exactly as the dump writes it. The dump must type it `xsd:anyURI`; any
other datatype is a violation (`invalid-literal`).

### §4.6 Dates

`rdf:value` is the DSP lexical form from `kb:valueHasString`, typed `dao:date` (d.73(a)), for example
`"JULIAN:1492 CE"^^dao:date`. The node also carries these deviation triples (§12, D6), each exactly once:

| Predicate | Object | From |
|-----------|--------|------|
| `dao:dateCalendar` | `"GREGORIAN"` or `"JULIAN"` (plain) | `kb:valueHasCalendar` |
| `dao:dateStartJDN` | `xsd:integer` | `kb:valueHasStartJDN` |
| `dao:dateEndJDN` | `xsd:integer` | `kb:valueHasEndJDN` |
| `dao:dateStartPrecision` | `"YEAR"`, `"MONTH"` or `"DAY"` (plain) | `kb:valueHasStartPrecision` |
| `dao:dateEndPrecision` | `"YEAR"`, `"MONTH"` or `"DAY"` (plain) | `kb:valueHasEndPrecision` |

A date is invalid when it lacks any of these or its `kb:valueHasString` (`missing-source-literal`), repeats one
(`repeated-source-literal`), names another calendar (`unknown-calendar`), has an unknown precision
(`unknown-precision`), has a JDN that is not an integer (`invalid-literal`), or has a start JDN after its end JDN
(`inverted-date`). The producer never emits an invalid date: it reports a violation instead.

### §4.7 List values

A list value carries `dao:sourceListNode <list node IRI>` (d.70, from `kb:valueHasListNode`) and **no**
`rdf:value`, which departs from d.70 (§12, D9). The labels are on the list node (§9). The node named is always
a list node in the file: a list value naming a node that is missing, deleted, or inside a deleted subtree is a
violation (`list-value-target`). So is a list value whose `kb:valueHasListNode` is missing
(`missing-source-literal`), repeated (`repeated-source-literal`) or not an IRI (`invalid-literal`).

### §4.8 Colour

`rdf:value` is knora-base's colour lexical, typed `dao:color`, for example `"#ff3333"^^dao:color` (d.48 lexical,
§12 D10). The dump must write it as a plain string; any other literal is a violation (`invalid-literal`).

### §4.9 Geometry

`rdf:value` is knora-base's geometry JSON verbatim, typed `dao:geometry` (d.48 lexical, §12 D10). The geometry is
never turned into a selector (§10). The dump must write it as a plain string; any other literal is a violation
(`invalid-literal`).

### §4.10 Kinds not carried

Interval, time and geoname values are omitted: the edge together with the node (§12, D13). The producer counts
them. A note for readers: an `rdf:value` datatype the reader does not know means "omit this value", never "read
it as text". `dao:date` is a must-know datatype.

## §5 Order

`dao:valueHasOrder "<n>"^^xsd:integer` sits on a value node **only when the dump records `kb:valueHasOrder`**
for that value (§12, D4). Readers order the values of one property on one resource ascending by it, count a
missing order as 0, and break ties by UUID, comparing the UUID strings' ASCII bytes (base64url, so byte order is
string order). Links carry no order (§6): readers count them as 0 and tie them by target IRI.

## §6 Links

A link is a direct resource-to-resource triple:

```text
<resource> <sourceProperty> <target resource> .
```

- The predicate is the source property's full IRI, including knora-base properties used directly
  (`http://www.knora.org/ontology/knora-base#hasLinkTo` on LinkObjs,
  `http://www.knora.org/ontology/knora-base#isRegionOf` on Regions). This departs from d.69's canonical aliases
  (§12, D3).
- A link carries no UUID and no order. LinkValue reifications are never carried (d.69 strips them).
- Source properties are compared and passed on by full IRI. `http://www.knora.org/ontology/knora-base#hasComment`
  and `http://www.knora.org/ontology/0803/incunabula#hasComment` are distinct properties, and a consumer must
  never shorten either to a local name.
- **A link is emitted only when its LinkValue agrees:** a direct triple is emitted as a link (or as a membership,
  §7) iff a live, non-superseded LinkValue with the same `rdf:subject`, `rdf:predicate` and `rdf:object` exists.
  A direct triple with no LinkValue at all (`missing-link-value`), and a live LinkValue whose direct triple is
  missing (`unpaired-link-value`), are violations. So is a live LinkValue without exactly one IRI each of
  `rdf:subject`, `rdf:predicate` and `rdf:object` (`invalid-link-value`). A live LinkValue whose subject and
  predicate are sound but whose object is not stands in for `missing-link-value` on the direct triples under
  that subject and predicate, so one mistake is reported once.
  A direct triple whose only LinkValues are deleted or superseded is dropped and counted. A deleted LinkValue
  beside a live one for the same triple (a link deleted and re-created) does not drop it.
- A link whose target is not a live resource of the dump is dropped and counted (§11).

**Predicate precedence.** A resource-level predicate `p` is classified by the first rule that matches, in this
order. `⊑*` is the reflexive `rdfs:subPropertyOf*` closure, so a knora-base property used directly matches its
own rule.

1. **Membership:** `p ⊑* kb:isPartOf` → `dao:isPartOf` (§7).
2. **Seqnum:** `p ⊑* kb:seqnum` → `dao:seqnum` (§7).
3. **Standoff link:** `p ⊑* kb:hasStandoffLinkTo` → always omitted (§11).
4. **Link:** any other `p` whose object is an IRI that is not a value, LinkValue or file value of the dump →
   a link, emitted under the rules above.
5. **LinkValue companion:** the object is a `kb:LinkValue` (for example `incunabula:hasBandTypeLValue`) → never
   emitted; it only feeds the agreement check.
6. **File value:** the object's class is a knora-base file-value class (for example via
   `kb:hasStillImageFileValue`) → the §8 representation, never a value edge. This is decided by the object's
   class, because knora-base's own `hasFileValue` hierarchy is not in the dump.
7. **Value:** the object is a value (§4) → a value edge (§3).

Every rule except 3 needs an IRI object: a literal under a membership or seqnum predicate, or under any predicate
of rules 4–7, fits no rule. A resource-level predicate that fits none of these and is not in §11's resource list
is a violation (`unknown-predicate`).

**Closures.** Every `*` closure (`rdfs:subPropertyOf*`, `rdfs:subClassOf*`) is reflexive, is computed once over
**every project ontology graph of the dump** and only those, is seeded with the knora-base IRIs named in this
document, and follows only IRI objects (the ontology graphs also hold blank-node restrictions). knora-base
itself is not in the dump, so its own hierarchy never applies: `kb:isPartOf`, `kb:isRegionOf` and
`kb:hasStandoffLinkTo` are sub-properties of `kb:hasLinkTo` in knora-base, but not here. A super-property or
super-class outside the data is tolerated. Cycles and diamonds terminate: each closure is walked once with a
visited set.

In 0803, `incunabula:hasBandTypeL` and `incunabula:hasBandTypeR` are `⊑ kb:hasLinkTo`, and
`incunabula:hasComment` is `⊑ kb:hasComment`; the precedence and the per-class target rules of §10 decide how
each is read.

## §7 Membership

- `<child> dao:isPartOf <parent>` (d.69) for every direct triple under an `rdfs:subPropertyOf*` of `kb:isPartOf`
  (0803: `incunabula:isPartOfBook`). The sub-property itself is not recorded. A child may have several parents.
  The `dao:isPartOf` graph has no cycle: a resource on a cycle is a violation (`membership-cycle`).
- `<child> dao:seqnum "<n>"^^xsd:integer` (d.69) for every value under an `rdfs:subPropertyOf*` of `kb:seqnum`
  (0803: `incunabula:hasSeqnum`, an IntValue); `<n>` is its `kb:valueHasInteger`, in canonical form (§1). A
  `kb:valueHasInteger` that is missing (`missing-source-literal`), repeated (`repeated-source-literal`) or not an
  `xsd:integer` (`invalid-literal`) is a violation. A resource has at most one seqnum: two live seqnum values on
  one resource are a violation (`repeated-seqnum`). A seqnum whose object is not a `kb:IntValue` of the dump is a
  violation (`invalid-seqnum`).
- Neither also appears as a link or as a value node. A membership to a parent that is not a live resource is
  dropped (§11); the child keeps its `dao:seqnum`.

## §8 Representations

```text
<resource> dao:hasRepresentation <urn:dsp:representation:<file value UUID>> .
```

- At most one per resource: two live file values on one resource are a violation (`multiple-representations`).
- The node IRI is `urn:dsp:representation:` followed by the file value's `kb:valueHasUUID`, never its IRI suffix.
  It is base64url and unique among representations: a live file value whose `kb:valueHasUUID` is missing
  (`missing-source-literal`), repeated (`repeated-source-literal`) or not base64url (`invalid-uuid`, §3), and two
  live file values sharing one (`duplicate-uuid`, §3), are violations. The two prefixes keep representation and
  value nodes apart (§12, D2).
- The node carries `rdf:type dao:Representation` (d.15, d.48 (iii)), exactly one `dao:representationType` and exactly
  one `dao:internalFilename` (a plain string, from `kb:internalFilename`) (§12, D7).
- `dao:representationType` is one of:

  | Type | Dump file-value class |
  |------|-----------------------|
  | `dao:StillImageRepresentation` | `kb:StillImageFileValue` |
  | `dao:AudioRepresentation` | `kb:AudioFileValue` |
  | `dao:MovingImageRepresentation` | `kb:MovingImageFileValue` |
  | `dao:DocumentRepresentation` | `kb:DocumentFileValue` |

- A still image also carries exactly one `dao:dimX` and one `dao:dimY` (`xsd:integer`, from `kb:dimX`/`kb:dimY`,
  canonical lexical (§1), fitting a `u32`). A `kb:internalFilename`, or a still image's `kb:dimX` or `kb:dimY`,
  that is missing (`missing-source-literal`) or repeated (`repeated-source-literal`) is a violation, and so is a
  filename that is not a plain string or a dimension that is not an `xsd:integer` fitting a `u32`
  (`invalid-literal`).
- **Legal information** (§12, D14), on any representation type, both optional: one `dao:authorship` (a plain
  string) per `kb:hasAuthorship` of the file value, and at most one `dao:copyrightHolder` (a plain string, from
  `kb:hasCopyrightHolder`). No order among the authors is promised: N-Quads carry none. An author or copyright
  holder that is not a plain string (`invalid-literal`), and a second `kb:hasCopyrightHolder`
  (`repeated-source-literal`), are violations. A file value's `kb:hasLicense` is not carried and stays outside the
  closed list (§11).
- Archive, text and external still-image file values (`kb:ArchiveFileValue`, `kb:TextFileValue`,
  `kb:StillImageExternalFileValue`) are omitted and counted (§12, D1).
- A deleted or superseded file value gives no representation.
- **Access rights** (§12, D15, D16): exactly one `dao:accessRights`, `"restricted-view"` when the file value
  grants the anonymous user restricted view only, which the public guard passes on a `kb:StillImageFileValue`
  alone (§11), and `"public"` otherwise; and exactly one `dao:dataClassification "unclassified"`. Both are plain
  strings.
- A reader omits a representation whose `dao:representationType` it does not know.

## §9 List nodes

The file carries every live `kb:ListNode` of the project's data graph (§12, D8), whether or not a value uses it.

- `<node> rdf:type skos:Concept` (d.70). The node IRI is the dump's.
- Zero or more `skos:prefLabel`, one per `rdfs:label` in the dump, verbatim: language-tagged when the dump tags
  it (tag lowercased), plain otherwise. At most one label per language tag, counting "untagged" as one tag
  (SKOS S14). A label that is neither a plain nor a language-tagged string is a violation (`invalid-list-label`).
- A **root** is a node with `kb:isRootNode true`. It has no `skos:broader` and no position.
- A **non-root's parent** is the subject of the one `kb:hasSubListNode` triple naming it. A non-root carries
  exactly one `<node> skos:broader <parent>` and, when the dump records `kb:listNodePosition`, one
  `<node> dao:listNodePosition "<n>"^^xsd:integer` (canonical lexical, §1; fitting a `u32`; unique among siblings).
- No node name (`kb:listNodeName`) and no list comment (`rdfs:comment`) are carried.
- A deleted node is dropped together with its whole subtree.
- Each of these is a violation: a second label with the same tag (`duplicate-list-label`), a node with two
  parents (`multiple-parents`), a non-root with no parent (`orphan-list-node`), a root with a parent
  (`root-with-parent`), a root with a position (`root-with-position`), a position that is not one `xsd:integer`
  fitting a `u32` (`invalid-list-position`), two siblings sharing a position (`duplicate-sibling-position`), a
  cycle in `skos:broader` (`list-node-cycle`), and a `kb:hasSubListNode` naming no list node of the dump
  (`dangling-sub-list-node`).
- "Two parents" counts every `kb:hasSubListNode` naming the node, a deleted parent's included: a node with a
  live and a deleted parent is a violation, not a child of the live one, since it would otherwise vanish with
  the deleted parent's subtree.

## §10 Annotations

A resource whose class is `kb:Region` or `kb:LinkObj`, or reaches either by `rdfs:subClassOf*` in the project
ontology graphs, is an annotation (d.71). It is additionally `rdf:type oa:Annotation`, so it is
`a dao:Resource, oa:Annotation`, and it carries:

- exactly one `oa:motivatedBy`:
  - `oa:linking` for a LinkObj (or a sub-class);
  - for a Region (or a sub-class): `oa:commenting` when it has a live value under an `rdfs:subPropertyOf*` of
    `kb:hasComment`, else `oa:highlighting`. "Live" is §11's: neither deleted nor superseded;
  - a class that reaches both `kb:Region` and `kb:LinkObj` has no one motivation, so a resource of it is a
    violation (`ambiguous-annotation-class`);
- one `oa:hasTarget <target>` per distinct target of the emitted links (§6) of the annotation: for a Region, the
  links under an `rdfs:subPropertyOf*` of `kb:isRegionOf`; for a LinkObj, those under an `rdfs:subPropertyOf*` of
  `kb:hasLinkTo`. `kb:hasStandoffLinkTo` is never a target. At least one target is required (the Web Annotation
  Data Model requires one), so an annotation with none is a violation (`missing-annotation-target`).

No selector and no body are carried (d.71 minimal, §12 D11): the geometry stays a value (§4.9). A LinkObj's link
destinations sit in `oa:hasTarget`, not in a body, which is not the Web Annotation `linking` pattern (§12, D11).
The annotation keeps its direct links beside `oa:hasTarget` (`<region> kb:isRegionOf <page>`,
`<linkobj> kb:hasLinkTo <target>`), and its values, links and representation follow §3–§8 unchanged.
`oa:hasTarget` is annotation structure and never a link.

**Consumers identify annotations by `rdf:type oa:Annotation`, never by `dao:sourceClass`,** because a project may
sub-class `kb:Region` or `kb:LinkObj`.

## §11 Filtering and omissions

The file carries current state only. No tombstone or marker of a dropped item remains.

- A resource or value with `kb:isDeleted true` is dropped. A dropped resource takes its values, links and
  representation with it.
- Every object of `kb:previousValue` (a superseded version) is dropped, whether or not it is still attached to
  its resource. A version chain whose current head is deleted drops the whole value.
- A deleted LinkValue drops its direct link, unless another live LinkValue for the same triple exists (§6).
- Links, memberships and targets whose object is not a live resource of the dump (deleted, or an IRI outside
  it) are dropped and counted.
- A deleted list node is dropped with its subtree (§9).
- **Never carried:** users, permissions, `kb:attachedToProject`, creation and modification dates, standoff nodes
  and mappings, `kb:hasStandoffLinkTo`, `kb:hasTextValueType`, `kb:valueHasComment`, `kb:listNodeName`, list
  comments, LinkValues.
- The dump's admin and permission graphs are never read. The ontology graphs are read only for the closures of
  §6; nothing from them is emitted.

**The selection and the public guard.** The file is made from the selection: every resource that is not
deleted, and every value, file value and LinkValue that is neither deleted nor superseded and belongs to a
selected resource (one that names it as an object or, for a LinkValue, its `rdf:subject`). Before anything is
written, every selected subject, kinds omitted whole included, is checked against its own `kb:hasPermissions`
in the data graph: one plain string of `|`-separated entries `<token> <group>[,<group>…]`, tokens `RV` < `V` <
`M` < `D` < `CR`, in any order. Only an entry naming `knora-admin:UnknownUser` (or its full IRI) counts, and the
anonymous user's grant is the highest token among them. `V` or better is public. `RV` passes only on a
`kb:StillImageFileValue` and makes its representation `"restricted-view"` (§8). Anything else refuses the whole
project, giving no snapshot and listing every case by subject, kind (resource or value) and class: no grant, `RV`
on any other subject, no `kb:hasPermissions`, or one that is not one plain string of known tokens and non-empty
groups. Unselected subjects are never checked. List nodes (§9) are not checked and carry no rights: a dump
records no `kb:hasPermissions` on them, and every live one is written. Permissions themselves are never carried.

**The closed list.** Every subject of the data graph has exactly one known kind, and every predicate and
`rdf:type` on it is one the emitter handles for that kind. Anything outside the list is a violation, and a run
with violations fails with all of them listed at once, giving no snapshot. Deleted and superseded subjects are
dropped before their predicates are checked, but still need a known kind. The producer names a predicate
outside the list by its subject's kind: `unknown-predicate` (resource, §6), `unknown-value-predicate`,
`unknown-file-value-predicate`, `unknown-link-value-predicate` and `unknown-list-node-predicate`. A subject with
no kind is `untyped-subject`, one with two `rdf:type`s `multiple-types`, and one whose type is no class listed
here `unknown-class`.

Project-namespace predicates cannot be listed in a generic format, so on a resource they are classified by rule
instead: by the §6 precedence and the kind of their object (value, LinkValue, file value, resource). A project
predicate that fits no rule is a violation. The resource list below names the knora-base predicates that go
through the same rule when a project uses them directly.

Standoff nodes are identified as the subtree reached from a text value by `kb:valueHasStandoff` and are omitted
whole, whatever their predicates and types (0803: `kb:StandoffLinkTag` and the
`http://www.knora.org/ontology/standoff#` tag classes). A text value's markup comes from `kb:valueHasXml` (§4.1),
never from its standoff.

Handling: **emitted** (as named), **read** (drives filtering or a check, never emitted itself), **omitted**.

**Resource** (its `rdf:type` is a project class, `kb:Region`, `kb:LinkObj` or a sub-class):

| Predicate | Handling |
|-----------|----------|
| `rdf:type` | read: `dao:Resource`, `dao:sourceClass` (§2), `oa:Annotation` (§10); exactly one per subject |
| `rdfs:label` | emitted (§2) |
| `kb:isDeleted` | read (filter) |
| `kb:attachedToProject`, `kb:attachedToUser`, `kb:creationDate`, `kb:lastModificationDate` | omitted |
| `kb:hasPermissions` | read (the public guard) |
| `kb:isPartOf` | §6 rule 1, membership (§7) |
| `kb:seqnum` | §6 rule 2, seqnum (§7) |
| `kb:hasStandoffLinkTo` | §6 rule 3, omitted |
| `kb:hasLinkTo`, `kb:isRegionOf` | §6 rule 4, link (§6) and annotation target (§10) |
| `kb:hasLinkToValue`, `kb:isRegionOfValue`, `kb:isPartOfValue`, `kb:hasStandoffLinkToValue` | §6 rule 5, omitted |
| `kb:hasStillImageFileValue` and any predicate whose object is a file value | §6 rule 6, representation (§8) |
| `kb:hasComment`, `kb:hasColor`, `kb:hasGeometry` | §6 rule 7, value (§3) |
| any project-namespace predicate | by the §6 precedence |

**Value** (`kb:TextValue`, `kb:IntValue`, `kb:DecimalValue`, `kb:BooleanValue`, `kb:UriValue`, `kb:DateValue`,
`kb:ListValue`, `kb:ColorValue`, `kb:GeomValue`), every class:

| Predicate | Handling |
|-----------|----------|
| `rdf:type` | read: `dao:Value` and the kind (§4) |
| `kb:valueHasUUID` | emitted as the node IRI and `dao:valueHasUUID` (§3) |
| `kb:valueHasOrder` | emitted as `dao:valueHasOrder` (§5) |
| `kb:isDeleted`, `kb:previousValue` | read (filter) |
| `kb:valueHasComment`, `kb:valueCreationDate`, `kb:attachedToUser` | omitted |
| `kb:hasPermissions` | read (the public guard) |

Per value class, in addition:

| Value class | Emitted | Read | Omitted |
|-------------|---------|------|---------|
| `kb:TextValue` | `kb:valueHasString`, `kb:valueHasXml` (as `dao:valueHasXml`) (§4.1) | `kb:valueHasLanguage` (the tag, §4.1); `kb:valueHasMapping`, `kb:valueHasStandoff`, only whether present (§4.1); `kb:hasTextValueType` (checked against the markup, never emitted, §4.1) | `kb:valueHasMaxStandoffStartIndex`; the objects of `kb:valueHasMapping` and `kb:valueHasStandoff` (the subtree) |
| `kb:IntValue` | `kb:valueHasInteger` (§4.2, or `dao:seqnum` §7) | — | `kb:valueHasString` |
| `kb:DecimalValue` | `kb:valueHasDecimal` (§4.3) | — | `kb:valueHasString` |
| `kb:BooleanValue` | `kb:valueHasBoolean` (§4.4) | — | `kb:valueHasString` |
| `kb:UriValue` | `kb:valueHasUri` (§4.5) | — | `kb:valueHasString` |
| `kb:DateValue` | `kb:valueHasString`, `kb:valueHasCalendar`, `kb:valueHasStartJDN`, `kb:valueHasEndJDN`, `kb:valueHasStartPrecision`, `kb:valueHasEndPrecision` (§4.6) | — | — |
| `kb:ListValue` | `kb:valueHasListNode` (§4.7) | — | `kb:valueHasString` |
| `kb:ColorValue` | `kb:valueHasColor` (§4.8) | — | `kb:valueHasString` |
| `kb:GeomValue` | `kb:valueHasGeometry` (§4.9) | — | `kb:valueHasString` |

`kb:IntervalValue`, `kb:TimeValue` and `kb:GeonameValue` are known kinds omitted whole (§4.10); their predicates
are not checked.

**File value** (`kb:StillImageFileValue`, `kb:AudioFileValue`, `kb:MovingImageFileValue`,
`kb:DocumentFileValue`):

| Predicate | Handling |
|-----------|----------|
| `rdf:type` | read: the representation type (§8) |
| `kb:valueHasUUID` | emitted as the representation IRI (§8) |
| `kb:internalFilename` | emitted as `dao:internalFilename` |
| `kb:dimX`, `kb:dimY` | emitted as `dao:dimX`, `dao:dimY` for a still image; accepted and not emitted on the other file-value classes |
| `kb:hasAuthorship`, `kb:hasCopyrightHolder` | emitted as `dao:authorship`, `dao:copyrightHolder` (§8, D14) |
| `kb:isDeleted`, `kb:previousValue` | read (filter) |
| `kb:internalMimeType`, `kb:originalFilename`, `kb:originalMimeType`, `kb:valueHasString`, `kb:valueHasOrder`, `kb:valueCreationDate`, `kb:attachedToUser` | omitted |
| `kb:hasPermissions` | read (the public guard; `dao:accessRights` of the representation) |

`kb:ArchiveFileValue`, `kb:TextFileValue` and `kb:StillImageExternalFileValue` are known kinds omitted whole
(§8); their predicates are not checked.

**LinkValue** (`kb:LinkValue`), never emitted:

| Predicate | Handling |
|-----------|----------|
| `rdf:type` | read |
| `rdf:subject`, `rdf:predicate`, `rdf:object` | read (link agreement, §6) |
| `kb:isDeleted`, `kb:previousValue` | read (filter) |
| `kb:valueHasUUID`, `kb:valueHasOrder`, `kb:valueHasString`, `kb:valueHasRefCount`, `kb:valueHasComment`, `kb:valueCreationDate`, `kb:attachedToUser` | omitted |
| `kb:hasPermissions` | read (the public guard) |

**List node** (`kb:ListNode`):

| Predicate | Handling |
|-----------|----------|
| `rdf:type` | read: `skos:Concept` (§9) |
| `rdfs:label` | emitted as `skos:prefLabel` (§9) |
| `kb:listNodePosition` | emitted as `dao:listNodePosition` (§9) |
| `kb:isRootNode`, `kb:hasSubListNode` | read: the root and `skos:broader` (§9) |
| `kb:isDeleted` | read (filter) |
| `kb:hasRootNode`, `kb:listNodeName`, `rdfs:comment`, `kb:attachedToProject` | omitted |

**Standoff node** (reached by `kb:valueHasStandoff`): omitted whole; its predicates and types are not checked.

## §12 Deviations from DAO

Ids are assigned in row order and never renumbered; a later deviation is appended with the next id.

| Id | What | DAO decision | Interim choice | Reason | Exit |
|----|------|--------------|----------------|--------|------|
| D1 | Archive, text and external still-image files | d.73(c): external files become `dao:Representation`s flagged `externallyHosted`; d.48 (iii): file values become representations | omitted and counted (§8) | the port has no representation kind for them, and 0803 has none | carry them when DAO's representation shape is published |
| D2 | Minted value and representation IRIs | d.48: a value is a blank node carrying `valueHasUUID`; d.12: identity IRIs are UUIDv7-based URNs (`urn:dsp:rep:…`) | `urn:dsp:value:<valueHasUUID>` (§3), `urn:dsp:representation:<valueHasUUID>` (§8), from the VRE UUIDs | a snapshot consumer needs a stable node to address and diff; blank nodes are not stable across runs | DAO's value identity and d.12 URNs |
| D3 | Source property as predicate | d.69: structural links under canonical `dao:` aliases (the d.69 spike: `dao:isRegionOf`); d.73(b): directly-used kb descriptive predicates as `urn:dsp:kb:term:<local>` | every value edge and link uses the source property's full IRI (§3, §6), `kb:isRegionOf` and `kb:hasLinkTo` included; only membership uses `dao:isPartOf`/`dao:seqnum` (§7) | the sync PRD's REQ-R3.5: every value and link names its source property as a full IRI; no term scheme for project predicates is decided | DAO's published term scheme |
| D4 | `dao:valueHasOrder` | d.48 prunes `valueHasOrder` as VRE working state | carried when the dump records it (§5) | CPE orders repeated values; missing order must stay observable | DEV-7445 (order upstream) |
| D5 | Text language | d.48 flattens text to an `xsd` string literal, with no language | `rdf:value` is language-tagged from `kb:valueHasLanguage` (§4.1) | the language is part of the value for a multilingual reader | DEV-7445 (language upstream) |
| D6 | Date deviation triples | d.73(a): the lexical form typed `dao:date`, with the JDN dropped as recomputable (d.48) | lexical kept, plus `dao:dateCalendar`, `dao:dateStartJDN`, `dao:dateEndJDN`, `dao:dateStartPrecision`, `dao:dateEndPrecision` (§4.6) | a reader should not have to parse DSP's lexical form to sort, filter or range-check | Q29's date standard (d.73(a)) |
| D7 | Representation shape | d.20/d.33: file facts as blank-node-structured RDF on the representation; no `dao:File` | `dao:representationType`, `dao:internalFilename`, `dao:dimX`, `dao:dimY` directly on the node (§8) | the reader needs exactly these to build IIIF URLs; the file-fact shape is not published | DAO's representation and file-fact terms |
| D8 | Project lists in the snapshot | d.70: lists are profile-side `dao:ValueList` / `skos:ConceptScheme`s, with sibling order by an unnamed position property | every live list node in the project graph as a `skos:Concept`, with `dao:listNodePosition` (§9) | there is no profile yet (§13); a reader needs the labels and the tree | the application profile |
| D9 | List values without a label `rdf:value` | d.70: the value carries all label variants in `rdf:value` plus `dao:sourceListNode` | `dao:sourceListNode` only (§4.7) | #455 treats a value node with two `rdf:value`s as invalid; the labels are on the node | when DAO's value reader supports multi-valued `rdf:value`, or d.70 changes |
| D10 | `dao:color` and `dao:geometry` datatypes | d.48: self-contained custom datatypes for Color and Geom, unnamed; `dao:date` (d.73(a)) names only dates | `rdf:value` typed `dao:color` / `dao:geometry` (§4.8, §4.9) | a typed lexical lets a reader skip a kind it does not know instead of reading it as text | DAO's datatype names |
| D11 | Minimal Web Annotation | d.71: selectors (`FragmentSelector`, `SvgSelector`), `styleClass`, motivations; WADM `linking` puts destinations in a body | `oa:Annotation`, `oa:motivatedBy`, `oa:hasTarget` only; geometry stays a value; LinkObj destinations in `oa:hasTarget`, not in a body (§10) | enough to identify and drop annotations; no consumer needs selectors yet | the full d.71 mapping, when a consumer needs selectors (CPE Incunabula Slice 6) |
| D12 | Markup as DSP-API's XML beside the plain text | d.51: standard-mapped standoff kept as one XML value with U+001E stripped, custom-mapped as XML plus XSLT | `rdf:value` stays the plain `kb:valueHasString`; the XML from `kb:valueHasXml`, U+001E removed, beside it as the plain string `dao:valueHasXml` (§4.1); standoff nodes, mappings, a custom mapping's XSLT and `kb:hasStandoffLinkTo` omitted (§11) | DSP-API stores the XML beside the standoff, so no standoff reader is needed; a plain `rdf:value` leaves every reader that renders no markup unchanged, and #455 reads one `rdf:value` per node | DAO's term for the XML value, and the XSLT of custom-mapped text (d.51) |
| D13 | Interval, time and geoname values | d.48: time as an `xsd` literal, interval and geoname as self-contained custom datatypes | omitted with their edge and counted (§4.10) | the port has no kind for them, and 0803 has none | when the port gains these kinds |
| D14 | Legal information on representations | none published: DAO has no rights terms yet | `dao:authorship` (0..n) and `dao:copyrightHolder` (0..1) plain strings on the representation node (§8); `kb:hasLicense` not carried | a presentation shows each file's authors and copyright holder (DEV-7630, project 0854) | DAO's rights terms |
| D15 | Access rights | none published: DAO has no rights terms yet | `dao:accessRights`, the plain string `"public"` or `"restricted-view"`, on every resource and representation node (§2, §8) | the Access Area serves a resource or a file only with its rights | DAO's rights terms |
| D16 | Data classification | none published | `dao:dataClassification`, the plain string `"unclassified"`, wherever `dao:accessRights` is written (§2, §8) | a second value with the rights' granularity; one value exists today | DAO's classification terms |

**Alignment note (not a deviation).** Text values drop every U+001E from `rdf:value`, following d.51's stripping
of INFORMATION SEPARATOR TWO from DAO's standoff XML (owner decision 2026-09-30). Labels keep it (§1).

### §12.1 Choices where DAO is silent

- **The graph IRI** `urn:dsp:project:<shortcode>`. d.95, amending d.94, leaves how a project is stored, graph
  included, to each consumer ("Graph" is a triplestore-internal notion). The prefix matches d.12's
  `urn:dsp:project:`, but the suffix is the shortcode, not a d.12 UUID.
- **Resource IRIs** stay the VRE's `http://rdfh.ch/<shortcode>/<id>`. Resource identity is out of the sync PRD's
  scope.
- **Deviation terms stay in the `dao:` namespace**, frozen at `https://ontology.dasch.swiss/dao#` (the owner decision
  recorded in §0). §15 names the status of each term, so "in `dao:`" never implies "DAO-decided".
- **Value node identity is file-global.** A value node's IRI is minted from its `kb:valueHasUUID` alone (§3,
  D2), so one UUID names one node across the whole file, not only within one resource. The 2026-10 run on
  project 0803's production dump found no `duplicate-uuid`. That measurement is not the guarantee: the
  guarantee is that `duplicate-uuid` fails the run before any file is written, so `sync-store` can never
  receive one node standing for two values and merge them. If a dump ever does reuse a UUID across
  resources, the fallback is a per-resource IRI, `urn:dsp:value:<resource-id>/<uuid>`, which needs the
  matching change in `sync-store`.

## §13 Non-goals

- No application profile: no `dao:PropertyUsage` and no `dao:ClassUsage` records (d.69, d.72). Labels for
  properties and classes come from CPE's KDL configuration.
- No standoff: a formatted text's markup is carried only as DSP-API's XML (§4.1), and a custom mapping's XSLT not
  at all (§12, D12).
- No permissions (§11), only the access rights the public guard derives from them (§2, §8, D15), and no legal
  information beyond a file's authorship and copyright holder (§8, D14).
- No change objects, events or tombstones: the file is one full current-state snapshot, not d.94's
  per-resource change stream.

## §14 Worked example

An excerpt of the 0803 snapshot: book `70aWaB2kWsuiN6ujYgM0ZQ`'s `incunabula:hasPubdate` date node, Page `b7v`
(`_Y6g7jY_WTqi9N-ofH9xEg`) with its membership, seqnum, one link and its representation, and the Region on that
page with its colour value and the comment that makes it `oa:commenting`, each with its access rights and data
classification. Other lines of those subjects, and the
target resources, are left out; the lines shown are valid, sorted and unique N-Quads, exactly as the file writes
them.

```nquads
<http://rdfh.ch/0803/70aWaB2kWsuiN6ujYgM0ZQ> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.knora.org/ontology/knora-base#hasColor> <urn:dsp:value:MB-Cj5wySn-XGaFWAA1d-w> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.knora.org/ontology/knora-base#hasComment> <urn:dsp:value:b_NykVy4T3uPwcLFN8PmSg> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.knora.org/ontology/knora-base#isRegionOf> <http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/ns/oa#Annotation> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.w3.org/2000/01/rdf-schema#label> "Seitenbezeichnung : b7v" <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.w3.org/ns/oa#hasTarget> <http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <http://www.w3.org/ns/oa#motivatedBy> <http://www.w3.org/ns/oa#commenting> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <https://ontology.dasch.swiss/dao#accessRights> "public" <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <https://ontology.dasch.swiss/dao#dataClassification> "unclassified" <urn:dsp:project:0803> .
<http://rdfh.ch/0803/FMz9WIT5XYOOGtQG8Bi38w> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/knora-base#Region> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <http://www.knora.org/ontology/0803/incunabula#hasBandTypeL> <http://rdfh.ch/0803/VNdAPbnOXgqJv1PlcMPj6g> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Resource> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <http://www.w3.org/2000/01/rdf-schema#label> "b7v" <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <https://ontology.dasch.swiss/dao#accessRights> "public" <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <https://ontology.dasch.swiss/dao#dataClassification> "unclassified" <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <https://ontology.dasch.swiss/dao#hasRepresentation> <urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <https://ontology.dasch.swiss/dao#isPartOf> <http://rdfh.ch/0803/cpQ3-JfqVZOkd7hUQ26kTg> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <https://ontology.dasch.swiss/dao#seqnum> "32"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<http://rdfh.ch/0803/_Y6g7jY_WTqi9N-ofH9xEg> <https://ontology.dasch.swiss/dao#sourceClass> <http://www.knora.org/ontology/0803/incunabula#Page> <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Representation> <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <https://ontology.dasch.swiss/dao#accessRights> "public" <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <https://ontology.dasch.swiss/dao#dataClassification> "unclassified" <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <https://ontology.dasch.swiss/dao#dimX> "3297"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <https://ontology.dasch.swiss/dao#dimY> "5141"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <https://ontology.dasch.swiss/dao#internalFilename> "8F1KTUpKKra-CaFgCYlt8im.jp2" <urn:dsp:project:0803> .
<urn:dsp:representation:sdSIAoxiS4-7ONN-oeHxjA> <https://ontology.dasch.swiss/dao#representationType> <https://ontology.dasch.swiss/dao#StillImageRepresentation> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "JULIAN:1492 CE"^^<https://ontology.dasch.swiss/dao#date> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#dateCalendar> "JULIAN" <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#dateEndJDN> "2266376"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#dateEndPrecision> "YEAR" <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#dateStartJDN> "2266011"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#dateStartPrecision> "YEAR" <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/0803/incunabula#hasPubdate> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#valueHasOrder> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:value:EDo-E58nRcql5glzoMvnYw> <https://ontology.dasch.swiss/dao#valueHasUUID> "EDo-E58nRcql5glzoMvnYw" <urn:dsp:project:0803> .
<urn:dsp:value:MB-Cj5wySn-XGaFWAA1d-w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
<urn:dsp:value:MB-Cj5wySn-XGaFWAA1d-w> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "#ff3333"^^<https://ontology.dasch.swiss/dao#color> <urn:dsp:project:0803> .
<urn:dsp:value:MB-Cj5wySn-XGaFWAA1d-w> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/knora-base#hasColor> <urn:dsp:project:0803> .
<urn:dsp:value:MB-Cj5wySn-XGaFWAA1d-w> <https://ontology.dasch.swiss/dao#valueHasOrder> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:value:MB-Cj5wySn-XGaFWAA1d-w> <https://ontology.dasch.swiss/dao#valueHasUUID> "MB-Cj5wySn-XGaFWAA1d-w" <urn:dsp:project:0803> .
<urn:dsp:value:b_NykVy4T3uPwcLFN8PmSg> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <https://ontology.dasch.swiss/dao#Value> <urn:dsp:project:0803> .
<urn:dsp:value:b_NykVy4T3uPwcLFN8PmSg> <http://www.w3.org/1999/02/22-rdf-syntax-ns#value> "Derselbe Holzschnitt wird auf Seite c6r der lateinischen Ausgabe des Narrenschiffs verwendet. " <urn:dsp:project:0803> .
<urn:dsp:value:b_NykVy4T3uPwcLFN8PmSg> <https://ontology.dasch.swiss/dao#sourceProperty> <http://www.knora.org/ontology/knora-base#hasComment> <urn:dsp:project:0803> .
<urn:dsp:value:b_NykVy4T3uPwcLFN8PmSg> <https://ontology.dasch.swiss/dao#valueHasOrder> "0"^^<http://www.w3.org/2001/XMLSchema#integer> <urn:dsp:project:0803> .
<urn:dsp:value:b_NykVy4T3uPwcLFN8PmSg> <https://ontology.dasch.swiss/dao#valueHasUUID> "b_NykVy4T3uPwcLFN8PmSg" <urn:dsp:project:0803> .
```

## §15 Vocabulary index

Every IRI the format fixes, one per row: the section that defines it, and its status, `DAO` or the §12 id of its
deviation. Source properties and classes (project and knora-base IRIs) are data, not format vocabulary, and are not
listed. The URN templates `urn:dsp:project:`, `urn:dsp:value:` and `urn:dsp:representation:` are in §1, §3 and §8.
Plain and language-tagged strings are written without a datatype IRI, so `xsd:string` and `rdf:langString` are
not listed.

Parse contract (tests read this table): it is the only table under this heading; column 1 is exactly one
backticked full IRI, unique across rows; column 3 is `DAO` or a §12 id (`D` and a number) and nothing else, and
every such id is a row of the §12 table; columns 2 and 4 are informational, and column 4 may be empty.

| IRI | Section | Status | Basis |
|-----|---------|--------|-------|
| `http://www.w3.org/1999/02/22-rdf-syntax-ns#type` | §2 | DAO | RDF |
| `http://www.w3.org/1999/02/22-rdf-syntax-ns#value` | §3 | DAO | d.48, d.70 |
| `http://www.w3.org/2000/01/rdf-schema#label` | §2 | DAO | RDFS; the resource label of the d.69 spike |
| `http://www.w3.org/2001/XMLSchema#integer` | §4.2 | DAO | d.48 |
| `http://www.w3.org/2001/XMLSchema#decimal` | §4.3 | DAO | d.48 |
| `http://www.w3.org/2001/XMLSchema#boolean` | §4.4 | DAO | d.48 |
| `http://www.w3.org/2001/XMLSchema#anyURI` | §4.5 | DAO | d.48 |
| `https://ontology.dasch.swiss/dao#Resource` | §2 | DAO | d.9, d.15 |
| `https://ontology.dasch.swiss/dao#sourceClass` | §2 | DAO | d.72 |
| `https://ontology.dasch.swiss/dao#accessRights` | §2 | D15 | |
| `https://ontology.dasch.swiss/dao#dataClassification` | §2 | D16 | |
| `https://ontology.dasch.swiss/dao#Value` | §3 | DAO | d.48; class name of the d.69 spike |
| `https://ontology.dasch.swiss/dao#valueHasUUID` | §3 | DAO | d.48 |
| `https://ontology.dasch.swiss/dao#sourceProperty` | §3 | DAO | d.69, d.72 |
| `https://ontology.dasch.swiss/dao#valueHasOrder` | §5 | D4 | |
| `https://ontology.dasch.swiss/dao#valueHasXml` | §4.1 | D12 | d.51 keeps the XML; the term is the interim's |
| `https://ontology.dasch.swiss/dao#date` | §4.6 | DAO | d.73(a) |
| `https://ontology.dasch.swiss/dao#dateCalendar` | §4.6 | D6 | |
| `https://ontology.dasch.swiss/dao#dateStartJDN` | §4.6 | D6 | |
| `https://ontology.dasch.swiss/dao#dateEndJDN` | §4.6 | D6 | |
| `https://ontology.dasch.swiss/dao#dateStartPrecision` | §4.6 | D6 | |
| `https://ontology.dasch.swiss/dao#dateEndPrecision` | §4.6 | D6 | |
| `https://ontology.dasch.swiss/dao#sourceListNode` | §4.7 | DAO | d.70 |
| `https://ontology.dasch.swiss/dao#color` | §4.8 | D10 | |
| `https://ontology.dasch.swiss/dao#geometry` | §4.9 | D10 | |
| `https://ontology.dasch.swiss/dao#isPartOf` | §7 | DAO | d.69 |
| `https://ontology.dasch.swiss/dao#seqnum` | §7 | DAO | d.69 |
| `https://ontology.dasch.swiss/dao#hasRepresentation` | §8 | DAO | d.48 (iii), d.69; term of the d.69 spike |
| `https://ontology.dasch.swiss/dao#Representation` | §8 | DAO | d.15, d.48 (iii) |
| `https://ontology.dasch.swiss/dao#representationType` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#StillImageRepresentation` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#AudioRepresentation` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#MovingImageRepresentation` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#DocumentRepresentation` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#internalFilename` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#dimX` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#dimY` | §8 | D7 | |
| `https://ontology.dasch.swiss/dao#authorship` | §8 | D14 | |
| `https://ontology.dasch.swiss/dao#copyrightHolder` | §8 | D14 | |
| `http://www.w3.org/2004/02/skos/core#Concept` | §9 | DAO | d.70 |
| `http://www.w3.org/2004/02/skos/core#prefLabel` | §9 | DAO | d.70 |
| `http://www.w3.org/2004/02/skos/core#broader` | §9 | DAO | d.70 |
| `https://ontology.dasch.swiss/dao#listNodePosition` | §9 | D8 | |
| `http://www.w3.org/ns/oa#Annotation` | §10 | DAO | d.71 |
| `http://www.w3.org/ns/oa#motivatedBy` | §10 | DAO | d.71 |
| `http://www.w3.org/ns/oa#hasTarget` | §10 | DAO | d.71 |
| `http://www.w3.org/ns/oa#commenting` | §10 | DAO | d.71 |
| `http://www.w3.org/ns/oa#highlighting` | §10 | DAO | d.71 |
| `http://www.w3.org/ns/oa#linking` | §10 | DAO | d.71 |

## §16 Revisions

- 2026-09-30 — initial publication (DEV-7443 Phase 1).
- 2026-09-30 — review changes made before the file was first pushed, so part of what #455 first read (DEV-7443):
  §0 and §1 give repository paths for the plan and the lockfile; §2 and §3 state the label and UUID violations;
  §4.1 cites D5; §5 and §6 no longer cite d.69 for link order; §8 and §15 give `dao:Representation`'s basis as
  d.15, d.48 (iii); §12.1 points at §0 for the `dao:` namespace decision; §15 gains its parse contract. §4.10 and
  §12 gain **D13**: interval, time and geoname values are omitted. D13 was chosen by the DEV-7443 plan, not by the
  owner, and stays provisional (§0).
- 2026-09-30 — §0, §1, §4.2–§4.4, §7, §8, §9: `xsd:integer`, `xsd:decimal` and `xsd:boolean` lexicals are written in
  XSD canonical form, no longer as the dump writes them (DEV-7443 Phase 5). `dao-lift` reads the dump through an
  Oxigraph store, which keeps these datatypes as values and writes them back canonically, so a verbatim lexical
  could not be promised. The output stays deterministic, and 0803 has no non-canonical such lexical, so its
  snapshot is unchanged.
- 2026-09-30 — a note on the entry above: canonical lexicals were decided while the DEV-7443 plan ran, not by the
  owner, and can be reversed by reading `data.nq` without the store; like D13, they stay provisional (§0).
- 2026-09-30 — §1–§4, §4.5–§4.9, §6–§11: every violation rule the producer reports is named, by the name it
  prints, in the section its report cites. Cases the text did not state before: a URI not typed `xsd:anyURI`, a
  colour or geometry that is not a plain string, a malformed `kb:valueHasListNode`, a malformed LinkValue, a
  literal under a membership or seqnum predicate, a seqnum that is not an IntValue, a list label that is not a
  string, a dangling `kb:hasSubListNode`, a live and a deleted list parent, and a class reaching both `kb:Region`
  and `kb:LinkObj` (DEV-7443). §10 says what a live comment is, and that `oa:hasTarget` is written once per
  distinct target; §11 says `kb:dimX`/`kb:dimY` are accepted on every file value. §4.10 is reflowed. None of this
  changes a valid snapshot: the producer already behaved so, and 0803's is unchanged.
- 2026-09-30 — §14 shows the Region's comment value, and §15's parse contract requires every deviation id to be a
  §12 row (DEV-7443).
- 2026-09-30 — §7 and §8 name the shared rules they cite: a seqnum's `kb:valueHasInteger`, a file value's
  `kb:valueHasUUID`, `kb:internalFilename` and still-image dimensions report `missing-source-literal`,
  `repeated-source-literal` and `invalid-literal` (DEV-7443). No valid snapshot changes: the producer already
  reported these rules so.
- 2026-10-07 — §12.1 records that value node identity is file-global, with the 0803 production run that found no
  `duplicate-uuid` and the per-resource fallback; §3 points to it (DEV-7455). Documentation only: no output
  changes.
- 2026-10-23 — §8, §11, §12, §13, §15: a representation carries its file's legal information, `dao:authorship`
  (0..n) and `dao:copyrightHolder` (0..1), from `kb:hasAuthorship` and `kb:hasCopyrightHolder`; deviation **D14**
  (DEV-7630). `kb:hasLicense` stays outside the closed list. §13 no longer lists legal information as a non-goal;
  permissions stay one. A snapshot of a dump without legal information, 0803's included, is unchanged; a reader
  that does not know the two predicates must now accept them on a representation. D14 was chosen by the DEV-7630
  plan and stays provisional (§0).
- 2026-11-20 — §0: this file moved to `dsp-repository` with its producer, now the `vre-bridge-lift` crate, and
  this copy is authoritative (DEV-7980). §1: the pinned serialiser is `oxrdf` 0.3.4 via `oxigraph` 0.5.11, no
  longer 0.3.3 via 0.5.8; `real_dump_matches_the_incubator_lift` compares the facts with `dao-lift`'s on a real
  dump.
- 2026-11-20 — interim format version 1 (DEV-7982). §2, §8, §12, §15: every resource and representation node
  carries `dao:accessRights` (`"public"`, or `"restricted-view"` on a raster still image's representation) and
  `dao:dataClassification "unclassified"`, deviations **D15** and **D16**. §11: the file is made from the
  selection, and the public guard refuses a project with anything in it the anonymous user may not view;
  `kb:hasPermissions` is read, no longer only omitted. §0 states the version rule; §13 and §14 follow. A snapshot
  of the same dump gains two lines per resource and representation and is otherwise unchanged. D15 and D16 were
  chosen by the P-DEV-301 plan and stay provisional (§0).
- 2026-11-20 — still interim format version 1, by the owner's decision, since no snapshot of it had been
  published (DEV-8012). §0, §1, §3, §4.1, §11, §12, §13, §15: a text value carries `dao:valueHasXml`, DSP-API's
  XML from `kb:valueHasXml` without U+001E, beside its plain `rdf:value` (D12, reworded). The standoff is never read; a selected text value
  with standoff or a mapping but no `kb:valueHasXml` refuses the project (`missing-text-xml`). A dump with
  `kb:valueHasXml`, refused before as `unknown-value-predicate`, now lifts; one with standoff and no XML, lifted
  before as plain text, is now refused; any other dump's snapshot is unchanged. A reader that does not know
  `dao:valueHasXml` reads the plain text as before. Carrying the XML and refusing a value without it are the
  owner's decisions; the term, its plain-string literal and the rule's name were chosen by the DEV-8012 run and
  stay provisional (§0).
- 2026-11-23 — still interim format version 1, before any snapshot was published (DEV-8012). §4.1, §11: a text
  value's `kb:hasTextValueType`, which DSP-API writes on new text values, is read and never emitted; a dump with
  it, refused before as `unknown-value-predicate`, now lifts, and its snapshot equals that of the same dump
  without it. A type other than DSP-API's four is `unknown-text-type`; `kb:FormattedText` or
  `kb:CustomFormattedText` without `kb:valueHasXml` is `missing-text-xml`; `kb:UnformattedText` with XML, standoff
  or a mapping is `text-type-mismatch`. A dump without the predicate, 0803's stage dump included, gives the same
  snapshot as before. The two rules and refusing a contradicting type were chosen by the run that added them and
  stay provisional (§0).
