# CLI command shape: noun-group hierarchy with identifier flags

`dsp-cli` organises commands as `dsp <area> <noun-group> <verb> [--flags]`. Noun-groups provide a hierarchy for progressive-disclosure help,
but resource identifiers (project shortname, data-model name, resource IRI, etc.) are passed as flags rather than baked into the command path as positional elements.
This honours `idea.md`'s "deeply nested sub-commands with meaningful help texts" through the noun-group hierarchy,
while avoiding identifier-stacking friction that hurts AI-agent ergonomics.

## Considered alternatives

- **Style 1 — deeply nested noun paths with positional identifiers**, e.g. `dsp vre project incunabula data-model anything-onto resource-type list`.
  Rejected because every command at depth N restates N−1 parent identifiers; discovery requires enumeration at each level;
  IRI-addressable lookups (which embed the project shortcode and uniquely address the resource) become awkward;
  positional-identifiers-interleaved-with-subcommands is poorly supported by mainstream CLI frameworks.
- **Style 2 — flat noun-verb-args** (no hierarchy). Rejected because it eliminates progressive-disclosure help, which `idea.md` calls out as a design goal.
- **Style 3 — noun-group hierarchy + identifier flags** (chosen). Per-noun-group `--help` screens give the progressive-disclosure surface;
  flags compose freely; IRI-addressable concepts can be looked up directly.

## Consequences

- Help text at every noun-group level is small and scoped. `dsp vre resource-type --help` shows only resource-type verbs;
  `dsp vre resource-type list --help` shows the flags including `--project` and `--data-model`.
- Invocations look like `dsp vre resource-type list --project incunabula --data-model biblio-onto`. Uniformly verbose, but predictable.
- Identifier flags compose freely with shell scripting (variables substitute in once per flag rather than in positional order).
- Resource lookups by IRI can be direct (`dsp vre resource describe <iri>`) without restating the project shortcode — the IRI embeds it.
- Possible future shortcut: a top-level `dsp vre describe <iri>` that dispatches based on IRI type (project / resource / data-model / etc.).
  To be decided when needed; ADR if formalised.
