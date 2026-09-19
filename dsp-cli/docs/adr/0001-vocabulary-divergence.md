# User-facing vocabulary diverges from DaSCH knowledge hub

`dsp-cli`'s user-facing vocabulary uses `resource-type`, `field`, `value-type`, and `data-model` — diverging from the DaSCH knowledge hub glossary, which
canonises `class` (a.k.a. "resource class"), `property`, and `ontology`.
We accept this divergence because the existing vocabulary has two recurring failure modes the CLI cannot afford: "property" is routinely confused with "value" by
non-technical users (slot vs. content), and "class" is overloaded with the OOP meaning that dominates AI-agent training data.
The CLI's charter (per `idea.md`) is to use domain-expert language that abstracts away DSP-API's RDF surface — picking words whose schema/instance pairing is
lexically obvious (`resource-type` ↔ `resource`, `field` ↔ `value`) serves that
charter better than mirroring the published glossary.

## Considered alternatives

- **(A) Mirror DSP-API verbatim** (`class`, `property`, `ontology`). Rejected — defeats the abstraction goal in `idea.md`.
- **(B) Mirror the DaSCH knowledge hub** (`class`/"resource class", `property`, `data-model`). Rejected — keeps the property/value and class/OOP overloads that cause
  real, recurring confusion.
  The hub's own entry for "class" is titled `Class (Resource Class)`, signalling that bare "class" is already ambiguous.
- **(C, chosen) Lead with disambiguating natural language** (`resource-type`, `field`, `value-type`, `data-model`). The schema/instance distinction is encoded
  directly in the word pairs.

## Consequences

- Researchers fluent in DSP-APP / knowledge hub must mentally translate `class`↔`resource-type` and `property`↔`field` when moving between tools. Mitigated by the
  centralised mapping table in `dsp docs concepts` (the "## The DSP-API vocabulary mapping" section) and by the noun-group subcommand prose that already names the
  equivalence inline (e.g. `data-model` subcommand: "called 'ontology' in DSP-API"; `resource-type` subcommand: "called 'class' in DSP-API").
  Every schema-noun command links to `dsp docs concepts` via a "See also: `dsp docs concepts`" pointer in its `--help` output.
- AI agents reading DSP-API responses see `ontology`/`class`/`property` and must map to CLI vocabulary. Mitigated by `dsp docs concepts` — the mapping table is
  machine-readable markdown, and LLMs absorb explicit mappings trivially.
  The per-command "See also" pointers guide agents to the table.
- If DaSCH ever revises the knowledge hub to align with this vocabulary, the divergence dissolves. If not, the CLI carries its own canon — acceptable, given the
  linguistic case is principled.
- This ADR is the entry point for any future request to "just use the same words as DSP-APP" — the answer is no, and here is why.

## Amendment — 2026-06-12

The original "Consequences" bullets prescribed surfacing the dsp-cli↔DSP-API mapping in "every relevant `--help` text" via per-flag annotations
(e.g. `field (DSP-API: knora-api:Property)`).
That prescription was never implemented and is now superseded.

**Reason for amendment:** Per-flag `(DSP-API: …)` annotations re-expose the RDF vocabulary the CLI exists to hide, defeating the abstraction goal in `idea.md`.
The mapping belongs in one canonical, well-labelled place — not scattered across individual flag doc-comments.

**Actual mitigation (as amended above):** The `dsp docs concepts` topic (shipped in Phase 6, plan 018) already contains a "## The DSP-API vocabulary mapping" table
covering `data-model`↔`ontology`, `resource-type`↔`class`, `field`↔`property`, `value-type`↔value subclass, plus inline "(DSP-API calls this …)" notes.
Every schema-noun command (`project list/describe`, `data-model list/describe/structure`, `resource-type list/describe`) carries a "See also: `dsp docs concepts`"
pointer in its `--help` output (audited and completed in plan 021, 2026-06-12). The noun-group subcommand prose already names the equivalence inline (wording
unchanged since Phase 5).

## Amendment — 2026-07-30

`vocabulary` joins the divergent-term list (plan 034). `dsp-cli` says `vocabulary`; DSP-API says "list"; DSP-APP's UI section is titled
"Controlled vocabularies". `dsp vre vocabulary list`/`describe` surface a project's controlled vocabularies and one vocabulary's node tree.

**Considered and rejected:**

- **`list`** — DSP-API's own term, but it collides with the CLI's own **verb** `list`, used across every other noun-group, in the **same grammar slot**:
  `dsp vre list list` would put a noun and a verb next to each other reading as a repeated word. Unusable as a noun-group name.
- **`controlled-vocabulary`** — 21 characters, for a marginal fidelity gain over the shorter `vocabulary`.

**Supporting evidence for the ambiguity**, the same signal this ADR already reads in the `Class (Resource Class)` case above: the DaSCH knowledge hub
double-titles the identical concept — `### Controlled Vocabulary (List)` and `### List (Controlled Vocabulary)`, with identical bodies
(`dasch-swiss-website/content/knowledge-hub/reference-glossary.md:59,163`). A bare term double-titled like that is already the pattern this ADR treats as
evidence of ambiguity; this is the same evidence, applied to a new term.

**Objection considered and dismissed:** dsp-cli's own docs already use the word "vocabulary" elsewhere (e.g. `docs/topics/concepts.md`'s title "the
vocabulary dsp-cli speaks", `docs/topics/dsp-cli.md`), but every existing occurrence is a **mass noun about words in general** ("the vocabulary dsp-cli
speaks"), never a **count noun** naming a data object (`a vocabulary`, `three vocabularies`). Introducing the count-noun sense as a new command-surface
term does not create real-world ambiguity with the existing mass-noun usage.

## Amendment (2026-08-07, plan 035)

### Deliberate raw-surface carve-out: `dsp vre sparql query`

This ADR's translation duty is about DSP-API's *invented* jargon for domain concepts (`ontology` →
data-model, `class` → resource-type). `dsp vre sparql query` ([ADR-0016](0016-sparql-passthrough.md))
is different: SPARQL is a W3C standard the user invokes **directly**, so its own vocabulary is the
user's, not DSP-API's, and renaming it would make the command undiscoverable. The words `SPARQL` and
`triplestore` are therefore admissible in user-facing text **only** on this command's surface and in
its messages — the user has already opted into the raw layer by reaching for this command. This does
not license the words anywhere else; every other command keeps translating.
