# Workflows — chaining commands to get real work done

`dsp-cli` has no session state and no piping primitives. Each command stands
alone: you run one, read its output, pull an identifier out of it, and use that
identifier in the next command. *You* (or the agent) are the glue between
commands — that is deliberate (see `dsp docs dsp-cli`, principle 3).

This topic is a set of playbooks for the common tasks. Each shows the chain and
the identifier you carry from one step to the next.

In every example, `-s <server>` selects the server (or set `DSP_SERVER` / a
`.env` file once — see `dsp docs connecting`). The `-p`/`--project` value can be a
shortcode, shortname, or IRI (see `dsp docs identifiers`).

## Explore an unfamiliar project, top to bottom

The core discovery loop — from "which projects exist?" down to "what fields does
this resource-type have?":

```
# 1. Find the project. --filter narrows the list client-side.
dsp vre project list -s prod --filter incunabula
#    → note the shortcode/shortname from the output, e.g. `incunabula` (0803)

# 2. Get the project overview, including its data-models.
dsp vre project describe -s prod -p incunabula
#    → note the data-model names listed, e.g. `incunabula`

# 3. List the resource-types in a data-model.
dsp vre resource-type list -s prod -p incunabula --data-model incunabula
#    → note a resource-type name, e.g. `page`

# 4. Describe the resource-type: its full field list.
dsp vre resource-type describe -s prod -p incunabula \
    --data-model incunabula --resource-type page
```

Step 4 answers "what does this data-model look like?" — the question dsp-cli was
built to answer in one call. Add `--include-builtins` at steps 3–4 to also see the
platform fields every resource inherits.

Add `--count` at steps 3–4 to also fetch instance counts per resource-type (one
extra HTTP call) — counts are non-deleted but not permission-filtered, so a
disclosure note is always shown alongside them.

## See how a data-model hangs together

When you care about the *relationships* between resource-types rather than one
type's fields:

```
dsp vre data-model structure -s prod -p beol --data-model beol
```

This emits a flat list of `link` relations (which resource-type points at which,
via which field) and `inherits` relations (superclass edges). Cross-data-model
targets are tagged `[to <dm>]`.

## Find what links to a given resource-type

There's no dedicated search; compose `structure` with a structured format and a
standard filter:

```
dsp vre data-model structure -s prod -p beol --data-model beol --csv \
  | grep ',person,'        # rows whose target is `person`
```

Use `-l` (lines) or `--csv`/`--tsv` whenever you intend to pipe into `grep`,
`cut`, or a spreadsheet — see `dsp docs output`.

## Archive a project (download a dump)

The one v1 command that needs authentication and writes a file:

```
dsp auth login -s prod -u you@example.org      # once; token is cached
dsp vre project dump -s prod --project 0803     # writes ./0803-<timestamp>.zip
```

Re-running `dump` for the same project is idempotent — it adopts an in-progress or
completed dump rather than erroring. The server holds **one dump at a time across
the whole instance**; if another project occupies the slot, `dump` refuses rather
than touching it. See `dsp vre project dump --help` for `--skip-assets`,
`--output`, `--replace`, and `--delete`.

## Ask a question the curated commands don't answer

When you need something no `list`/`describe`/`structure` command surfaces —
"which resources reference this deleted list node", "how many values of type X
exist across all projects" — drop to a raw SPARQL query against the server's
own triplestore:

```
dsp vre sparql query -s prod --query 'SELECT ?s WHERE { GRAPH <…> { ?s a <…> } } LIMIT 20'
```

This is the one command that does not follow the pattern below — no
`--format`, no curated model, the store's own response relayed verbatim. See
`dsp docs sparql`.

## The pattern behind all of these

1. Run a `list` to find an identifier.
2. Run a `describe` (or `structure`) using that identifier.
3. Repeat one level deeper, carrying the new identifier.

Prefer prose output while exploring (it includes next-step hints); switch to a
structured format (`-j`, `--csv`, `--tsv`, `-l`) the moment you want to pipe.

See also: `dsp docs identifiers`, `dsp docs output`, `dsp docs concepts`, `dsp
docs sparql`.
