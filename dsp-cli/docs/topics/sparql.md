# SPARQL passthrough — `dsp vre sparql query`

`dsp vre sparql query` sends a raw SPARQL 1.1 query straight to the server's
underlying triplestore and relays its response, byte-verbatim. It is the one
command in dsp-cli that deliberately does **not** abstract DSP-API — see
`docs/adr/0016-sparql-passthrough.md` for the full design record. Reach for it
when the question isn't on the curated command menu: "which resources reference
this deleted list node", "how many values of type X exist across all projects",
"what does this standoff markup actually look like in the graph".

## Requirements and availability

- **Requires a `SystemAdmin` bearer token.** Not a per-project permission — a
  system-wide administrator account. Resolved the same way every other command
  resolves a token (`DSP_TOKEN` env, then the cached token from `dsp auth
  login`) — no anonymous path exists on this endpoint.
- **Off by default, per deployment.** The server operator must explicitly enable
  the passthrough route. If it isn't enabled — or the server predates the
  endpoint, or `--server` is simply wrong — you get a `404` with no
  distinguishing body, because the route is not registered at all in that case.
  dsp-cli's `404` message names every possibility (including a misconfigured
  store dataset) rather than asserting one; there is no way to tell them apart
  from the client side.
- **Nothing deployed has this endpoint yet** (as of this writing). Every
  `stage`/`prod`/`dev` run will `404` until an operator turns the flag on for
  that environment. This is expected, not a bug report.
- **Operator-facing facts worth knowing, even as a client:** enabling the route
  on production is meant to be a **time-boxed break-glass window** (it also
  becomes visible in the server's own `/api/docs/`), and it has a precondition
  the flag does not itself enforce — the triplestore must have
  `arq:httpServiceAllowed = false` set, or a plain `SELECT … SERVICE <…>` query
  makes the *store* issue outbound HTTP requests on the operator's behalf.

## Input: three ways in

```
dsp vre sparql query -s prod --query 'SELECT * WHERE { ?s ?p ?o } LIMIT 10'
dsp vre sparql query -s prod --query-file my-query.rq
echo 'SELECT * WHERE { ?s ?p ?o } LIMIT 10' | dsp vre sparql query -s prod
```

`--query` and `--query-file` are mutually exclusive; give neither and dsp-cli
reads the query from stdin. If you give neither *and* stdin is a terminal (no
pipe, no redirect), that's a usage error naming all three options — dsp-cli
never hangs waiting on an interactive terminal.

**Privacy note, most-important fact first:** dsp-api logs the **full query
text, together with your username**, on every call — this is deliberate,
attributable server-side logging, and it is what you most need to know before
putting anything sensitive in a query. Separately, and less importantly:
`--query 'SELECT …'` on the command line also lands in your local shell history
and is visible via `ps`/`/proc/<pid>/cmdline` for the life of the process.
Prefer stdin or `--query-file` when a query embeds anything sensitive — though
note the *server-side* logging happens regardless of which input path you use.
The inverse footgun: pointing `--query-file` at the wrong path (e.g. `.env` or
`~/.config/dsp-cli/auth.toml`) uploads that file's contents, verbatim, to
whatever `--server` resolves to.

## `--accept`: choosing the response format

With no `Accept` header at all, the triplestore answers with its own default —
`application/sparql-results+xml`, which is hostile to `jq` and to scripted use.
So dsp-cli sends `Accept: application/sparql-results+json` by default. Override
with `--accept`:

| alias | sent as |
| --- | --- |
| `json` (default) | `application/sparql-results+json` |
| `xml` | `application/sparql-results+xml` |
| `csv` | `text/csv` |
| `tsv` | `text/tab-separated-values` |
| `turtle` | `text/turtle` |
| `ntriples` | `application/n-triples` |
| `jsonld` | `application/ld+json` |

`turtle`/`ntriples`/`jsonld` exist because `CONSTRUCT` and `DESCRIBE` queries
return RDF graphs, not tabular result sets. Any value **containing `/`** is
forwarded verbatim as a raw media type instead of being matched against the
alias table — e.g. `--accept 'text/csv;q=1, */*;q=0.1'` — so a future store
format works without waiting for a dsp-cli release.

⚠️ **Fuseki does not `406` on an `Accept` it cannot satisfy — it silently falls
back to its own default serialization.** A typo'd `--accept` (`--accept
text/csvv`) does not error; it just comes back as XML, unexpectedly. There is
no `--accept none` to ask for zero `Accept` header — that isn't achievable
through dsp-cli's HTTP layer. `--accept xml` is the documented way to get the
store's own default explicitly.

## Guardrails, as the client sees them

The server enforces several limits read once at its own startup (an operator
restarts it to change any of these):

| guardrail | effect from here |
| --- | --- |
| request-body size cap | an over-cap query → `413`, exit `2` (this is the one caller-fault mapping that gets exit `2`, not `1`) |
| response size cap | an over-cap result → `500`, "narrow the query, or page it with LIMIT and OFFSET" |
| server-side query timeout | a slow query the store cancels → `504` |
| max concurrent passthrough calls | too many simultaneous queries server-wide → `503`, rejected immediately, never queued |

Separately, dsp-cli's own **client-side** request timeout defaults to one hour
(`--timeout <seconds>` to bound it down — it can only lower the bound, never
raise the server's own limit). In practice the server's own guardrail (well
under two minutes) ends a slow query long before the client-side timeout would
ever matter; the long default exists so the client is never the thing that
gives up first.

## Exit codes

| outcome | exit |
| --- | --- |
| the store answered (any `2xx`) — result on stdout | `0` |
| the store rejected the query (its own `4xx`, relayed) — its text on stderr | `1` |
| the server or its store failed (`500`/`502`/`503`/`504`) — its message on stderr | `1` |
| no token, or a token that isn't a `SystemAdmin` account | `3` |
| the passthrough route isn't available at `--server` | `1` (`not found`) |
| the submitted query text exceeds the server's size cap | `2` |
| dsp-api itself faulted (buffer overflow, upstream store unreachable, overloaded) | `1` |

A malformed query is **not** distinguished from other store rejections — both
land on exit `1`, with the store's own error text (sanitised and capped) on
stderr. **stdout is unsanitised, byte-exact store output; stderr is sanitised
prose.** If your query might return data containing terminal escape sequences,
redirect to a file or view with `cat -v`/`less -R` rather than trusting a raw
terminal.

## Scoping a query

There is no `-p`/`--project` flag and no `--named-graph`/`--default-graph` flags
in this release — SPARQL here is server-wide, not project-scoped. Scope your own
query with an explicit `GRAPH`/`FROM` clause instead, e.g.:

```
SELECT ?s ?p ?o WHERE { GRAPH <http://www.knora.org/data/0001/anything> { ?s ?p ?o } } LIMIT 10
```

The server *does* relay `default-graph-uri`/`named-graph-uri` dataset
parameters, but their effect depends on the store's union-default-graph
configuration and isn't a stable dsp-cli-level contract — an explicit `GRAPH`/
`FROM` in the query text is the reliable way to scope a query.

## What this command will never do

No result post-processing: no CSV-to-table rendering, no binding extraction, no
`--format`/`-j`/`-l`/`--columns`. The response is the store's own document in
whatever media type it negotiated; wrapping it in dsp-cli's envelope would
either corrupt it or double-encode it. If you find yourself wanting a curated
view of a specific query's results repeatedly, that's a sign a curated command
is the better home for it — not this one.

See also: `dsp docs concepts`, `dsp docs workflows`, `dsp docs errors`.
