# SPARQL passthrough as a deliberate raw-surface carve-out

`dsp-cli` exists to abstract DSP-API's RDF surface behind domain-expert vocabulary
([ADR-0001](0001-vocabulary-divergence.md)) and to render every answer through one of five formats
([ADR-0003](0003-chaining-and-output.md)). Both assume dsp-cli knows the *shape* of the answer.
DSP-API's new `POST /admin/sparql/query` (dsp-api `e4ef2e348`, 2026-08-07, DEV-6734 milestone 1)
breaks that assumption on purpose: the caller writes arbitrary SPARQL, the store chooses the
serialization, and the server relays status, media type and bytes untouched. The capability exists
because closing the triplestore's port removed staff's only route to questions the curated API does
not answer; the project brief's stance is *"intelligence lives in the driver, keep the passthrough
dumb."* dsp-cli is named the primary headless/agent client of that surface. This ADR records how
`dsp vre sparql query` ships as a **thin, unabstracted relay**, and which of dsp-cli's usual rules it
narrowly and explicitly suspends to do so. Depends on ADR-0001, ADR-0002, ADR-0003, ADR-0004,
ADR-0007, ADR-0012. Amends ADR-0001, ADR-0003, ADR-0007, ADR-0008, ADR-0009, ADR-0010, ADR-0012.

Full design decisions (D1–D19) live in plan
[035-vre-sparql-query](../design/plans/035-vre-sparql-query/implementation-plan.md); this ADR
records the load-bearing shape for future readers who won't read the plan.

## Decision

`dsp vre sparql query` suspends three rules dsp-cli otherwise obeys:

1. **Vocabulary (ADR-0001).** The command surface still speaks CLI vocabulary, but the **payload**
   is raw, and the words `SPARQL` and `triplestore` are admissible in this command's help, docs and
   messages. They are not DSP-API jargon being smuggled in: SPARQL is a W3C standard the user
   invokes directly, and a user who asks for it has opted into the layer below.
2. **Output (ADR-0003).** No `Renderer`, no `--format`, no envelope. The response body is written to
   stdout byte-verbatim — no added trailing newline, no re-encoding, no control-character stripping.
   The one deviation from pure relay is a default `Accept` of `application/sparql-results+json`,
   chosen for the headless/agent audience over the store's XML default, overridable via `--accept`
   (an alias table, or a raw media type containing `/`). There is no way to send zero `Accept`:
   `reqwest`'s public API seeds `Accept: */*` as a client-level default header and re-merges it into
   any request whose own `Accept` entry is vacant, so a per-request removal does not survive — and
   `*/*` is not equivalent to a genuinely absent header (live-verified: `*/*` returns JSON from this
   store, absent `Accept` returns XML). `--accept xml` is the documented way to ask for the store's
   own default explicitly.
3. **Exit semantics (ADR-0012).** stdout carries results only, byte-exact and **unsanitised by
   contract** — a user redirecting to a file gets exactly the bytes the store sent. A store rejection
   (any non-`2xx` outside dsp-api's own typed-error table) is reported on stderr with the store's own
   text — there sanitised (control characters stripped, kept `\n`/`\t`) and capped at 200 characters,
   because stderr is prose — and exit `1`. dsp-api's own typed failures (`401`/`403`/`404`/`413`/`415`/
   `500`–`504`) map by status to the existing `Diagnostic` variants and their stable exit codes
   (`AuthRequired`/3, `NotFound`/1, `Usage`/2 for `413` only, `Internal`/1, `ServerError`/1). dsp-cli
   never judges the query's *content*, only its outcome.

What is **not** suspended: bearer-token resolution and the `AuthRequired`/exit-`3` contract (a
`SystemAdmin` token is required and resolved before any HTTP call — no anonymous path exists on this
endpoint); the `Diagnostic` type and stable exit codes; the no-`unwrap`/`expect` rule; the
client-boundary discipline (all DSP-API-shaped status classification stays in `src/client/`, never
re-implemented in the action layer).

### The one relay type

`DspClient::sparql_query` returns `SparqlResponse { status: u16, content_type: Option<String>, body:
Vec<u8> }` — the **one** method on the trait that returns a relay struct rather than a parsed domain
model. It lives in `src/client/sparql.rs`, deliberately not `src/model/`: every `src/model/` type is
a *translated domain* type (ADR-0008's layer-4 contract), and this is transport-shaped (an HTTP
status and a MIME string), not a parsed model. `Debug` is hand-written (prints the body's length, not
its bytes) so the codebase's standard `assert!(…, "{result:?}")` idiom and any `tracing::debug!(?resp)`
never dump unsanitised, potentially large store output into logs.

### Redirects are not followed

The per-call client sets `redirect::Policy::none()`. `POST /admin/sparql/query` never legitimately
redirects, and a `307`/`308` **replays the request body** — here, the query text, which this ADR treats
as privacy-sensitive — to whatever host the `Location` names. Refusing to follow removes the question
instead of reasoning about it. Consequence to know: a deployment-level `301`/`302` (an edge proxy, an
http→https upgrade) now arrives at the action as a non-2xx relay and is reported as *"the triplestore
rejected the query (HTTP 301)"*, which misattributes an infrastructure redirect to the store. Accepted
for now — the alternative is a `3xx` row in the classifier, which is worth adding if it ever happens in
practice.

### Accepted risks, recorded rather than mitigated

Two things this design knowingly does not bound, both proportionate to a CLI that staff run against
servers they name themselves:

- **stdout is unsanitised store bytes by contract** (D3). A triplestore literal containing ANSI escapes
  reaches the terminal as-is. Sanitising conditionally on stdout being a TTY was considered and
  rejected: it would make `dsp … > file` and `dsp …` emit *different bytes*, a worse and more
  surprising footgun than the escape sequence it prevents. The mitigation is documentation
  (`dsp docs sparql` recommends `less -R` / `cat -v` / redirect when the data may be hostile) and the
  fact that the **stderr prose path is sanitised and capped** — the asymmetry is deliberate.
- **No client-side response ceiling.** 64 MiB is the *server's* promise (`max-response-bytes`);
  `reqwest`'s `bytes()` applies no bound of its own, so a typo'd or hostile `--server` is limited only
  by `--timeout`. Accepted rather than adding a second ceiling that would have to be kept in step with
  the server's.

### No result post-processing

There is no `--csv`-to-table rendering and no binding extraction: that would re-introduce the
abstraction this command exists to bypass, and only works for `SELECT` (`CONSTRUCT`/`DESCRIBE` return
a graph). A curated question deserves a curated command, built separately, on top of a real answer
shape.

## Consequences

- A third no-Renderer command (after `dsp docs` and `dsp auth token`), so that pattern is now a real
  category — ADR-0003 names the rule, not just a third data point.
- Raw SPARQL results are outside dsp-cli's output contract: a future change to how the store
  serializes results is not a dsp-cli breaking change.
- `SystemAdmin` is required, and the endpoint is off by default per deployment
  (`allow-sparql-passthrough`), so `404` is an expected outcome on every environment until the flag is
  turned on, with a purpose-written message that **hedges across every plausible cause** rather than
  asserting one (disabled flag, older dsp-api, misconfigured store dataset, wrong `--server`) — none of
  them is distinguishable from the client side, because an unregistered route returns a bare `404`.
- The `update` half (`dsp vre sparql update`) is deferred to DEV-6734 milestone 2 (not yet started
  server-side) and will engage ADR-0004's write-operations question separately. It is also the
  trigger to reconsider whether `DspClient` should split into a curated trait and a raw one: one
  outlier method names a one-off exception; two would make a category.
- `--query-file` reads one file the user authored for one logical operation — this is not the
  file-roundtripping/bulk-from-file pattern ADR-0004 forbids, and is recorded here so it is not
  re-argued.
- The store's own error body is contractually opaque (dsp-api's E2E spec asserts only non-emptiness);
  dsp-cli relays it, never parses it.

## Considered alternatives

- **Render SPARQL results through the five formats.** Rejected — re-imposes the abstraction this
  command exists to bypass, and only works for `SELECT`.
- **Keep pure passthrough with exit `0` on a store `4xx`.** Rejected — silently succeeding on a
  broken query is a bad default for scripted and agent use.
- **Send no `Accept` at all (a `--accept none` sentinel).** Considered, then dropped after
  implementation proved it unreachable through `reqwest`'s public API (see "Output" above). `--accept
  xml` is the closest available equivalent and is the documented answer.
- **A fully raw `--accept` with no alias table and no default** (type the media type or get the
  store's XML). Rejected by the owner — simpler, but makes the common case (`--accept json`) verbose
  for the exact audience the tool serves, and leaves the out-of-the-box experience as XML.
- **Sanitising control characters on stdout when it is a TTY.** Rejected — it would make `dsp … >
  file` and `dsp …` emit different bytes, a worse and more surprising footgun than the escape sequence
  it prevents, and unlike anything else in the tool. The risk is documented instead; stderr, the prose
  channel, is sanitised.
- **A separate `dsp admin` area.** Rejected — dsp-cli is a power-user tool throughout, and
  `SystemAdmin` is ordinary server-side access control, the same JWT/permission gate every command
  already obeys.
- **Reject `--query-file` under ADR-0004.** Rejected on reading — that ADR's boundary is bulk
  file-roundtripping, not "a command may read a file".
