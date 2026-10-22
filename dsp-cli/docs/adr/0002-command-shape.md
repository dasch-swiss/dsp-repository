# CLI command shape: noun-group hierarchy with identifier flags

`dsp-cli` organises commands as `dsp <area> <noun-group> <verb> [--flags]`. Noun-groups provide a hierarchy for progressive-disclosure help,
but resource identifiers (project shortname, data-model name, resource IRI, etc.) are passed as flags rather than baked into the command path as positional elements.
This honours the nested sub-commands with help text at every level that `docs/src/dsp-cli/overview.md` calls for through the noun-group hierarchy,
while avoiding identifier-stacking friction that hurts AI-agent ergonomics.

The one positional argument is the topic of `dsp docs <topic>`, which names an embedded documentation topic, not a platform resource.

## Considered Options

- **Style 1 — deeply nested noun paths with positional identifiers**, e.g. `dsp vre project incunabula data-model anything-onto resource-type list`.
  Rejected because every command at depth N restates N−1 parent identifiers; discovery requires enumeration at each level;
  IRI-addressable lookups (which embed the project shortcode and uniquely address the resource) become awkward;
  positional-identifiers-interleaved-with-subcommands is poorly supported by mainstream CLI frameworks.
- **Style 2 — flat noun-verb-args** (no hierarchy). Rejected because it eliminates progressive-disclosure help, which `docs/src/dsp-cli/overview.md` calls out as a design goal.
- **Style 3 — noun-group hierarchy + identifier flags** (chosen). Per-noun-group `--help` screens give the progressive-disclosure surface;
  flags compose freely; IRI-addressable concepts can be looked up directly.
- **A top-level `dsp vre describe --iri <iri>` that dispatches on the IRI's type** (project / resource / data-model / …). *Deferred*, not rejected:
  it would be added if lookups by IRI of unknown type become a recurring friction. It would take the IRI as a flag, and a record would formalise it.

## Consequences

- Help text at every noun-group level is small and scoped. `dsp vre resource-type --help` shows only resource-type verbs;
  `dsp vre resource-type list --help` shows the flags including `--project` and `--data-model`.
- Invocations look like `dsp vre resource-type list --project incunabula --data-model biblio-onto`. Uniformly verbose, but predictable.
- Identifier flags compose freely with shell scripting (variables substitute in once per flag rather than in positional order).
- Resource lookups by IRI are direct (`dsp vre resource describe --resource <iri>`) without restating the project shortcode — the IRI embeds it.

Enforced by: the help snapshots in `dsp-cli/tests/cli.rs`, which pin every noun-group's verbs and every leaf's flags (**static-analysis**);
the rest by **review** (the clap structs in `src/cli/mod.rs` are the only place a command's identifiers are declared).
