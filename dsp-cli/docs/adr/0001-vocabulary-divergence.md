# User-facing vocabulary diverges from the DaSCH knowledge hub

`dsp-cli`'s user-facing vocabulary uses `data-model`, `resource-type`, `field`, `value-type` and `vocabulary`, diverging from DSP-API and from the DaSCH knowledge hub glossary,
which canonise `ontology`, `class` (a.k.a. "resource class"), `property`, a Value subclass, and `list` for a controlled vocabulary.
The divergence is accepted because the glossary's words have two recurring failure modes the CLI cannot afford: "property" is routinely confused with "value" by
non-technical users (slot vs. content), and "class" is overloaded with the OOP meaning that dominates AI-agent training data.
The CLI's charter (`docs/src/dsp-cli/overview.md`) is to use domain-expert language that abstracts away DSP-API's RDF surface. Words whose schema/instance pairing is
lexically obvious (`resource-type` ↔ `resource`, `field` ↔ `value`) serve that charter better than mirroring the published glossary.

A request to "just use the same words as DSP-APP" is answered by this record.

## Translation boundary

Translation happens once, at the client/deserialisation boundary in `src/client/`. Everything above the client layer uses dsp-cli vocabulary; everything
`OntologyDto`-shaped stays inside the client. The word "export", which DSP-API uses for what dsp-cli calls a "dump", never appears in help text, error messages,
type names or module names outside `src/client/`.

## Where the mapping lives

The dsp-cli ↔ DSP-API mapping is stated in one canonical place: the "The DSP-API vocabulary mapping" section of the `dsp docs concepts` topic
(`data-model`↔`ontology`, `resource-type`↔`class`, `field`↔`property`, `value-type`↔value subclass).
Noun-group subcommand prose names the equivalence inline ("called 'ontology' in DSP-API"), and schema-noun commands end their `--help` with
"See also: `dsp docs concepts`". Flags carry no `(DSP-API: …)` annotations.

## The vocabulary noun

A project's controlled vocabularies are `dsp vre vocabulary list` / `describe`. DSP-API calls them "list", DSP-APP's UI "Controlled vocabularies".
The hub double-titles the concept (`Controlled Vocabulary (List)` and `List (Controlled Vocabulary)`), the same ambiguity signal as `Class (Resource Class)`.
The word "vocabulary" elsewhere in dsp-cli's docs is a mass noun ("the vocabulary dsp-cli speaks"); here it is a count noun naming a data object.

## Raw surface

`SPARQL` and `triplestore` are admissible in user-facing text only on `dsp vre sparql query`, the one command whose payload is the raw layer.
Every other command keeps translating. The carve-out is stated in [dsp-cli/ADR-0016](0016-sparql-passthrough.md).

## Considered Options

- **(A) Mirror DSP-API verbatim** (`class`, `property`, `ontology`). Rejected: defeats the abstraction goal.
- **(B) Mirror the DaSCH knowledge hub** (`class`/"resource class", `property`, `data-model`). Rejected: keeps the property/value and class/OOP overloads.
  The hub's own entry is titled `Class (Resource Class)`, which signals that bare "class" is already ambiguous.
- **(C, chosen) Disambiguating natural language** (`resource-type`, `field`, `value-type`, `data-model`). The schema/instance distinction is encoded in the word pairs.
- **Per-flag `(DSP-API: knora-api:Property)` annotations in `--help`.** Rejected: they re-expose the RDF vocabulary the CLI exists to hide.
  The mapping belongs in one well-labelled place.
- **`list` as the noun for a controlled vocabulary.** Rejected: DSP-API's own term, but it collides with the CLI's verb `list` in the same grammar slot
  (`dsp vre list list`).
- **`controlled-vocabulary`.** Rejected: 21 characters for a marginal fidelity gain over `vocabulary`.

## Consequences

- Researchers fluent in DSP-APP or the knowledge hub translate `class`↔`resource-type` and `property`↔`field` when moving between tools;
  the concepts topic and the inline equivalences mitigate this.
- AI agents reading DSP-API responses see `ontology`/`class`/`property` and map them via the concepts topic, which is machine-readable markdown.
- If DaSCH revises the knowledge hub to align with this vocabulary, the divergence dissolves. Until then the CLI carries its own canon.

Enforced by: the help snapshots in `dsp-cli/tests/cli.rs`, which pin the noun-group prose naming the DSP-API equivalent and the "See also: `dsp docs concepts`"
pointers, and the `*_snapshots.rs` assertions that rendered output contains no "export" and no `knora-api:` (**static-analysis**); the `src/client/` boundary
and the rest by **review**.
