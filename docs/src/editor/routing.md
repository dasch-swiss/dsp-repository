# Routing and Request Handling

The URL scheme, the method discipline, how a write answers with and without JavaScript, and the two layers that wrap the app.

## URL scheme

Paths are **root-mounted**. There is no `/editor` prefix.

DPE carries `/dpe/…` because it shares `repository.dasch.swiss` with other services. The editor gets its own hostname, so a prefix buys nothing — and adopting one would keep alive the path-routing option this design rejects, for the CSRF reason ([Architecture](./architecture.md#relationship-to-dpe)).

| Path | Method | Access | Purpose |
|------|--------|--------|---------|
| `/` | GET | public | 303 to `/projects`. |
| `/login` | GET, POST | public | The address form, and issuing a one-time code. POST rate-limited per IP. |
| `/login/code` | GET, POST | public | The code form, and spending the code. POST rate-limited per IP. |
| `/logout` | POST | public | Delete the session and clear the cookie. |
| `/projects` | GET | signed in | The projects this account may edit, named from the published set. |
| `/projects/{shortcode}` | GET | signed in + assigned | 303 to the first form section. 403 otherwise. |
| `/states` | GET | signed in | What each depositor-facing state means and the expected wait before Online (REQ-2.6). |
| `/projects/{shortcode}/sections/{section}` | GET, POST | signed in + assigned | One form section, and the save, autosave, submit, withdrawal or discard it makes. 200 even when the project is unpublished. |
| `…/sections/{section}/fields/{field}/add` | POST | signed in + assigned | One more row of a repeatable field. Under the section's URL so it resolves through the same `context()`. |
| `…/sections/{section}/fields/{field}/{key}/remove` | POST | signed in + assigned | Drop one row. The key is in the path, not a button's name and value, because a programmatic submit omits the submitter's. |
| `/projects/{shortcode}/entities/{proposal}` | GET, POST | signed in + assigned | One entity proposal's form, and the save or discard it makes. `{proposal}` is the proposal's `entity_id`. |
| `…/entities/{proposal}/fields/{field}/add` | POST | signed in + assigned | One more row of a repeatable field on the entity form. |
| `…/entities/{proposal}/fields/{field}/{key}/remove` | POST | signed in + assigned | Drop one row, for the reason the section's own row path gives. |
| `/review` | GET | RDU | The review queue: every pending submission oldest first, and every draft. |
| `/review/{shortcode}` | GET, POST | RDU | The field-by-field diff, and the claim, decision save, approve, request-changes or reject it makes. |
| `/depositors` | GET, POST | RDU | The account list, and creating a depositor. |
| `/depositors/new` | GET | RDU | The create form. |
| `/depositors/{id}/edit` | GET, POST | RDU | The edit form, and the change it makes. |
| `/depositors/{id}/remove` | GET, POST | RDU | The removal confirmation, and the removal. |
| `/collection` | GET | RDU | Every approved record and where its collection stands. |
| `/collection/{id}/discard` | GET, POST | RDU | The discard confirmation, and force-discarding a stranded record. |
| `/healthz` | GET | public | Liveness probe. Untraced. |
| `/telemetry/collect` | POST | public | Browser telemetry beacon. Untraced, rate-limited per IP. |
| `/api/v1/approved-records` | GET | public | Every approved record and its accepted entity proposals, as JSON, for the collecting workflow. Rate-limited per IP. |
| `/api/v1/collection-report` | POST | bearer token | Report the outcome of one collection attempt — a pull request and its state, or a failure. |

Everything else is served from the public asset directory, falling back to a 404 rendered in the page shell.

The two row paths are the one exception to the rule below, and deliberately: they are `POST`-only because both change the form and a `GET` that did would be a state change on a `GET`. They never strand a refusal, because they re-render the section rather than redirecting, and a reload of one lands on the section's own `GET`.

Every write shares a URL with the `GET` that renders its form, so a rejected submission re-renders somewhere that still answers `GET`. A write-only path leaves a reloaded rejection at a bare 405, the same dead end the 403 is rendered as a page to avoid.

`/` is a redirect rather than a page so that exactly one place decides what a signed-out visitor gets. It is therefore absent from `page_url.rs`'s `KNOWN_ROUTES`: a redirect renders no beacon script, so no beacon can report it.

There is deliberately no resend endpoint: asking again is another `POST /login`, under the same cooldown, which keeps the number of endpoints that can send mail at one. See [Authentication](./authentication.md).

`/projects` lists the published projects a reader may reach — every project for an RDU member, the intersection of assignments and the published set for a depositor. `/projects/{shortcode}` is a **redirect** into the form's first section, so exactly one place decides where a project link lands, and there is no per-project landing page between the list and the form. It is therefore absent from `page_url.rs`'s `KNOWN_ROUTES` for the same reason `/` is: a redirect renders no beacon script, so no beacon can report it. The redirect target is the same section for both audiences — a destination that depended on the role is one more thing to get wrong in a link shared between a depositor and a reviewer.

Two decisions about that scheme:

- **Form sections are real URLs**, not fragment swaps. Bookmarkable, Back-friendly, and consistent with the repository's URL-based-navigation principle.
- **Review deep-links by shortcode**, not by submission id. A project has at most one pending submission, so the shortcode is unique for the purpose and reads better in a URL shared between reviewers.

## The form's two renderings

`POST /projects/{shortcode}/sections/{section}` is one handler answering two ways, discriminated on the `Datastar-Request` header the vendored bundle sets on every fetch it makes:

| path | outcome | answer |
|---|---|---|
| no script | saved | `303` to this section's marked `GET` |
| no script | refused | `200`, the whole page re-rendered at the same URL |
| Datastar | saved | `200`, the section region as `text/html` |
| Datastar | refused | `200`, the same |

The plain path redirects because a `POST` left in the history re-posts on refresh. The enhanced path does not need to and must not: it never navigated, so a refresh re-issues the last `GET` — and a 303 followed by a full document would hand Datastar an `<html>` to patch. A refusal re-renders on both paths, because a redirect would throw away what the depositor typed.

That "marked" `GET` carries a query-string marker the redirect appends: `?saved=<the stored draft's updated_at, RFC 3339, URL-encoded>` for a save, or `?done=submitted|withdrawn|discarded` for a submit, a withdrawal or a discard. The `GET` renders the matching notice only while the marker still holds: the stamp still matches the stored row for `saved`, and for `done` the project is still in the phase named (locked for `submitted`, unlocked with the latest round `Withdrawn` for `withdrawn`, unlocked with no stored draft for `discarded`). A stale or bookmarked URL therefore confirms nothing. Because a status region filled at the first load is not reliably announced (`editor-web/src/pages/section.rs`'s module doc argues why), that same `GET` also prefixes the document `<title>` with the notice's short form (`Draft saved — …`, `Sent to RDU — …`, `Submission withdrawn — …`, `Draft discarded — …`), since a navigation does announce a title change where a live region filled at load does not. The entity form (see [Entity Proposals](./entity-proposals.md#the-entity-form)) mirrors both halves for its own save (`?saved=<the proposal's own stamp>`) and discard (`?done=discarded`), with `Saved — …` and `Discarded — …` as its short forms.

Three things about the enhanced path fail quietly if changed:

- **A `text/html` response *is* an implicit `datastar-patch-elements`.** With no selector it matches by `id` in `outer` mode, so what comes back is the region under one id, not a document.
- **The region is bigger than the form.** It is the rail, the status and the form together — everything a save can change. Returning only the `<form>` leaves the rail showing the obligation counts from *before* the save, so the depositor fills in the last required field, the field goes quiet, and the rail still says something is missing.
- **A refusal must still answer 200.** Datastar processes a response body only on a 200; any other status aborts the fetch and the message the response was carrying never reaches the page. The outcome goes on the span instead, which is where alerting reads it from — the same reasoning as the account forms' redisplayed 200.

`data-on:submit` carries no `__prevent`: the bundle calls `preventDefault` unconditionally for a `submit` event on a form element, so one would be noise. The form is *not* `novalidate` and no field is `required`, which looks contradictory and is not — see `editor-web/src/pages/section.rs`, where the `type="date"` reason is argued.

## Request middleware

Two layers wrap the app, in this order from the outside in:

1. **CSRF** — `Sec-Fetch-Site: same-origin` is required on every non-`GET`/`HEAD` request, failing closed on everything else including an absent header. It is applied **last** in `build_app`, which makes it outermost and therefore the one layer the positional traced/untraced split cannot route around: inside `build_router` it would have missed `/telemetry/collect`, the only pre-auth POST in the app, with no test failing. See [Authentication](./authentication.md#csrf) for why `SameSite` and `__Host-` do not close this.
2. **OTel** — the traced/untraced split below.

Access control is **not** a third layer. It is two extractors, `Authenticated` and `Rdu`, and that is the design rather than an omission: a handler that names one cannot run without the check, because the argument is what runs it, and a handler that names neither is visibly public at the point anyone reads its signature. A middleware over a sub-router would have added a second positional invariant of exactly the shape this module already regrets — the traced/untraced split is invisible in the route table and reversible by moving one line — and here the failure mode is an unauthenticated route rather than a missing span. See [Authentication](./authentication.md#authorization).

## Traced and untraced routes

An Axum layer wraps only routes declared **before** it. The router therefore has two halves:

- `build_router` — everything wrapped by `OtelInResponseLayer` then `OtelAxumLayer`. `OtelInResponseLayer` is declared first so it runs *inner* and injects the `traceparent` response header; `OtelAxumLayer` is declared second so it runs *outer* and creates the server span.
- `build_app` — adds `/healthz` and `/telemetry/collect` **after** those layers, so neither is traced. A liveness probe every 30 seconds and a telemetry upload on every page view would otherwise mint a span each and bury the real traffic.

That split is positional, so it is invisible in the route table and reversible by moving one line. A test asserts `/healthz` and the beacon are absent from `build_router`.
