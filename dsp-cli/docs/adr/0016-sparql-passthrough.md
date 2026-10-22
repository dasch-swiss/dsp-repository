# `dsp vre sparql query` is a raw relay and suspends the vocabulary, output and exit rules

`dsp-cli` exists to abstract DSP-API's RDF surface behind domain-expert vocabulary (dsp-cli/ADR-0001) and to render every answer through one of five formats (dsp-cli/ADR-0003). Both assume dsp-cli knows the *shape* of the answer. DSP-API's `POST /admin/sparql/query` breaks that assumption on purpose: the caller writes arbitrary SPARQL, the store chooses the serialization, and the server relays status, media type and bytes untouched. The capability exists because closing the triplestore's port removed staff's only route to questions the curated API does not answer. The project brief's stance is "intelligence lives in the driver, keep the passthrough dumb", and dsp-cli is the primary headless and agent client of that surface.

`dsp vre sparql query` is therefore a **thin, unabstracted relay**. This record states which of dsp-cli's usual rules it narrowly and explicitly suspends. The carve-out is stated here once; the records for the suspended rules point to this one.

## Decision

`dsp vre sparql query` suspends three rules dsp-cli otherwise obeys:

1. **Vocabulary (dsp-cli/ADR-0001).** The command surface still speaks CLI vocabulary, but the **payload** is raw, and the words `SPARQL` and `triplestore` are admissible in this command's help, docs and messages. They are not DSP-API jargon smuggled in: SPARQL is a W3C standard the user invokes directly, and a user who asks for it has opted into the layer below.
2. **Output (dsp-cli/ADR-0003).** No `Renderer`, no `--format`, no envelope. This is the third command with no `Renderer`, after `dsp docs` and `dsp auth token`. The response body is written to stdout byte-verbatim: no added trailing newline, no re-encoding, no control-character stripping. The one deviation from pure relay is a default `Accept` of `application/sparql-results+json`, chosen for the headless and agent audience over the store's XML default, and overridable through `--accept` (an alias table, or a raw media type containing `/`). The aliases are `json`, `xml`, `csv`, `tsv`, `turtle`, `ntriples` and `jsonld`. There is no way to send zero `Accept`: `reqwest`'s public API seeds `Accept: */*` as a client-level default header and re-merges it into any request whose own `Accept` entry is vacant, so a per-request removal does not survive. And `*/*` is not equivalent to an absent header (live-verified: `*/*` returns JSON from this store, an absent `Accept` returns XML). `--accept xml` is the documented way to ask for the store's own default.
3. **Exit semantics (dsp-cli/ADR-0012).** stdout carries results only, byte-exact and **unsanitised by contract**: a user redirecting to a file gets exactly the bytes the store sent. On a non-2xx relay nothing is written to stdout.

**Status mapping.** dsp-api's own typed failures are distinguished by status, never by sniffing content, and map to the existing `Diagnostic` variants and their stable exit codes:

| Status | Diagnostic | Exit |
|---|---|---|
| `401`, `403` | `AuthRequired` | 3 |
| `404` | `NotFound` | 1 |
| `413` | `Usage` | 2 |
| `415` | `Internal` | 1 |
| `500`, `502`, `503`, `504` | `ServerError` | 1 |

`413` is the one caller-fault mapping that gets exit `2`, since it is unambiguously the submitted query text being too big. Any other non-2xx outside that table is a store rejection: it is reported on stderr with the store's own text, sanitised (control characters stripped, `\n` and `\t` kept) and capped at 200 characters because stderr is prose, and exits `1`. dsp-cli never judges the query's *content*, only its outcome.

What is **not** suspended: bearer-token resolution and the `AuthRequired` / exit-`3` contract (a `SystemAdmin` token is required and resolved before any HTTP call, and no anonymous path exists on this endpoint); the `Diagnostic` type and stable exit codes; the no-`unwrap`/`expect` rule; and client-boundary discipline (all DSP-API-shaped status classification stays in `src/client/` and is never re-implemented in the action layer).

### The one relay type

`DspClient::sparql_query` returns `SparqlResponse { status: u16, content_type: Option<String>, body: Vec<u8> }`, the **one** method on the trait that returns a relay struct rather than a parsed domain model. It lives in `src/client/sparql.rs`, deliberately not `src/model/`: every `src/model/` type is a translated domain type (dsp-cli/ADR-0008), and this one is transport-shaped, an HTTP status and a MIME string. `Debug` is hand-written and prints the body's length, not its bytes, so the codebase's standard `assert!(…, "{result:?}")` idiom and any `tracing::debug!(?resp)` never dump unsanitised, potentially large store output into logs.

### Redirects are not followed

The per-call client sets `redirect::Policy::none()`. `POST /admin/sparql/query` never legitimately redirects, and a `307` or `308` **replays the request body**, here the query text, which is treated as privacy-sensitive, to whatever host the `Location` names. Refusing to follow removes the question instead of reasoning about it.

A deployment-level `301` or `302` (an edge proxy, an http-to-https upgrade) therefore arrives at the action as a non-2xx relay and is reported as "the triplestore rejected the query (HTTP 301)", which misattributes an infrastructure redirect to the store. This is accepted. The alternative is a `3xx` row in the status classifier, which is not provided and is worth adding if a redirect is ever seen in practice.

### Accepted risks, recorded rather than mitigated

Two things this design knowingly does not bound, both proportionate to a CLI that staff run against servers they name themselves:

- **stdout is unsanitised store bytes by contract.** A triplestore literal containing ANSI escapes reaches the terminal as-is. The mitigation is documentation (`dsp docs sparql` recommends `less -R`, `cat -v` or a redirect when the data may be hostile) and the fact that the stderr prose path is sanitised and capped. The asymmetry is deliberate.
- **No client-side response ceiling.** 64 MiB is the server's promise (`max-response-bytes`). `reqwest`'s `bytes()` applies no bound of its own, so a mistyped or hostile `--server` is limited only by `--timeout`. A second ceiling would have to be kept in step with the server's.

### No result post-processing

There is no `--csv`-to-table rendering and no binding extraction: that would reintroduce the abstraction this command exists to bypass, and it only works for `SELECT`, since `CONSTRUCT` and `DESCRIBE` return a graph. A curated question deserves a curated command, built separately on top of a real answer shape.

## Consequences

- Raw SPARQL results are outside dsp-cli's output contract: a change to how the store serializes results is not a dsp-cli breaking change.
- `SystemAdmin` is required, and the endpoint is off by default per deployment (`allow-sparql-passthrough`), so `404` is an expected outcome on every environment until the flag is turned on. The message is purpose-written and **hedges across every plausible cause** rather than asserting one (disabled flag, older dsp-api, misconfigured store dataset, wrong `--server`); none is distinguishable from the client side, because an unregistered route returns a bare `404`.
- `--query-file` reads one file the user authored for one logical operation. This is not the file round-tripping or bulk-from-file pattern dsp-cli/ADR-0004 forbids.
- The store's own error body is contractually opaque (dsp-api's end-to-end spec asserts only non-emptiness); dsp-cli relays it and never parses it.
- The update half, `dsp vre sparql update`, would engage the write-operations question of dsp-cli/ADR-0004 separately.

## Considered Options

- **Render SPARQL results through the five formats**: rejected. It reimposes the abstraction this command exists to bypass, and only works for `SELECT`.
- **Pure passthrough with exit `0` on a store `4xx`**: rejected. Silently succeeding on a broken query is a bad default for scripted and agent use.
- **Send no `Accept` at all** (a `--accept none` sentinel): not possible through `reqwest`'s public API (see "Output" above). `--accept xml` is the closest equivalent and is the documented answer.
- **A fully raw `--accept` with no alias table and no default**: rejected. It is simpler, but it makes the common case (`--accept json`) verbose for the exact audience the tool serves, and leaves the out-of-the-box experience as XML.
- **Sanitising control characters on stdout when it is a TTY**: rejected. It would make `dsp … > file` and `dsp …` emit different bytes, a worse and more surprising footgun than the escape sequence it prevents, and unlike anything else in the tool. The risk is documented instead.
- **A separate `dsp admin` area**: rejected. dsp-cli is a power-user tool throughout, and `SystemAdmin` is ordinary server-side access control, the same JWT and permission gate every command already obeys.
- **Reject `--query-file` under dsp-cli/ADR-0004**: rejected. That record's boundary is bulk file round-tripping, not "a command may read a file".
- **Split `DspClient` into a curated trait and a raw one**: deferred, not rejected. One raw method is a one-off exception and two would make a category, so the split is reconsidered when a second raw-relay method, `dsp vre sparql update`, lands.

Not yet in code: `dsp vre sparql update`; it is blocked on a dsp-api update passthrough, which does not exist (DEV-6734).

Enforced by: `dsp-cli/tests/sparql_query_http.rs` (byte relay and content type, request shape, the status mapping, redirects not followed), the action's
relay-split unit tests, and `dsp-cli/tests/cli.rs` (`--format`/`--columns` rejected, the `--query`/`--query-file` parsing), run by `just test`
(**static-analysis**); the live suite against the pinned stack (dsp-cli/ADR-0009); the vocabulary carve-out, the `Debug` that never prints the body, the
no-post-processing rule, and the client-boundary and no-`unwrap` rules, by **review**.
