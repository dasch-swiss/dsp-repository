# Routing and Request Handling

The URL scheme, the method discipline, how a write answers with and without JavaScript, and the layers that wrap the app.

## URL scheme

Paths are **root-mounted**. There is no `/editor` prefix.

DPE carries `/dpe/…` because it shares `repository.dasch.swiss` with other services. The editor gets its own hostname, so a prefix buys nothing — and adopting one would keep alive the path-routing option this design rejects, for the CSRF reason ([Architecture](./architecture.md#relationship-to-dpe)).

| Path | Method | Access | Purpose |
|------|--------|--------|---------|
| `/` | GET | public | 303 to `/projects`. |
| `/login` | GET, POST | public | The address form, and issuing a one-time code. POST rate-limited per IP. |
| `/login/code` | GET, POST | public | The code form, and spending the code. POST rate-limited per IP. |
| `/logout` | POST | public | Delete the session and clear the cookie. |
| `/projects` | GET | signed in | The projects this account may edit, named from the published set or, for a created project, from its draft. |
| `/projects/{shortcode}` | GET | signed in + assigned | 303 to the first form section. 403 otherwise. |
| `/states` | GET | signed in | What each depositor-facing state means and the expected wait before Online (REQ-2.6). |
| `/projects/{shortcode}/sections/{section}` | GET, POST | signed in + assigned | One form section, and the save, autosave, submit, withdrawal or discard it makes. 200 even when the project is unpublished. |
| `…/sections/{section}/fields/{field}/add` | POST | signed in + assigned | One more row of a repeatable field. Under the section's URL so it resolves through the same `context()`. |
| `…/sections/{section}/fields/{field}/{key}/remove` | POST | signed in + assigned | Drop one row. The key is in the path, not a button's name and value, because a programmatic submit omits the submitter's. |
| `…/sections/{section}/agents?for=&q=` | GET | signed in + assigned | One person-and-organisation search's results, as the fragment the picker's results morph to. Never writes; 200 for every query, 404 for a `for` that names no picker of the section. |
| `…/sections/{section}/vocabulary?for=&q=` | GET | signed in + assigned | One discipline, place or period search's results, as the fragment the picker's results morph to. Never writes; 200 for every query, 404 for a `for` that names no picker of the section. |
| `/projects/{shortcode}/entities/{proposal}` | GET, POST | signed in + assigned | One entity proposal's form, and the save or discard it makes. `{proposal}` is the proposal's `entity_id`. |
| `…/entities/{proposal}/fields/{field}/add` | POST | signed in + assigned | One more row of a repeatable field on the entity form. |
| `…/entities/{proposal}/fields/{field}/{key}/remove` | POST | signed in + assigned | Drop one row, for the reason the section's own row path gives. |
| `…/entities/{proposal}/agents?for=&q=` | GET | signed in + assigned | The same search for the entity form's affiliations, without the propose controls. |
| `/projects/new` | GET, POST | RDU | The create-a-project form, and registering the project with its seeded draft. |
| `/projects/{shortcode}/remove` | GET, POST | RDU | The confirmation, and deleting a created project with its draft and review rounds, and withdrawing its entity proposals not yet published, while it has no submission or approved record. |
| `/review` | GET | RDU | The review queue: every pending submission oldest first, and the drafts in progress. |
| `/review/{shortcode}` | GET, POST | RDU | The field-by-field diff, and the claim, decision save, approve, request-changes or reject it makes. |
| `/depositors` | GET, POST | RDU | The account list, and creating a depositor. |
| `/depositors/new` | GET | RDU | The create form. |
| `/depositors/{id}/edit` | GET, POST | RDU | The edit form, and the change it makes. |
| `/depositors/{id}/remove` | GET, POST | RDU | The removal confirmation, and the removal. |
| `/approved` | GET | RDU | Every approved record, its status against the published set, and a link to its pull requests on GitHub. |
| `/approved/{id}/discard` | GET, POST | RDU | The discard confirmation, and the discard. |
| `/healthz` | GET | public | Liveness probe. Untraced. |
| `/telemetry/collect` | POST | public | Browser telemetry beacon. Untraced, rate-limited per IP. |
| `/api/v1/approved-records` | GET | public | Every approved record and its accepted entity proposals, as JSON, for the Pull-request run. Rate-limited per IP. |

Everything else is served from the public asset directory, falling back to a 404 rendered in the page shell.

The two row paths are the one exception to the rule below, and deliberately: they are `POST`-only because both change the form and a `GET` that did would be a state change on a `GET`. They never strand a refusal, because they re-render the section rather than redirecting, and a reload of one lands on the section's own `GET`.

Every write shares a URL with the `GET` that renders its form, so a rejected submission re-renders somewhere that still answers `GET`. A write-only path leaves a reloaded rejection at a bare 405, the same dead end the 403 is rendered as a page to avoid.

`/` is a redirect rather than a page so that exactly one place decides what a signed-out visitor gets.

There is deliberately no resend endpoint: asking again is another `POST /login`, under the same cooldown, which keeps the number of endpoints that can send mail at one. See [Authentication](./authentication.md).

`/projects` lists the projects a reader may reach — for an RDU member the created projects, then every published one; for a depositor each assigned shortcode the published set or a created project carries, with any assigned shortcode that names neither named below. `/projects/{shortcode}` is a **redirect** into the form's first section, so exactly one place decides where a project link lands, and there is no per-project landing page between the list and the form. The redirect target is the same section for both audiences — a destination that depended on the role is one more thing to get wrong in a link shared between a depositor and a reviewer.

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

The plain path redirects because a `POST` left in the history re-posts on refresh. The enhanced path does not need to and must not: it never navigated, so a refresh re-issues the last `GET` — and a 303 followed by a full document would hand Datastar an `<html>` to patch. A refusal re-renders on both paths, because a redirect would throw away what the depositor typed. A write whose answer is another page (a change proposal opening its entity form, an entity discard returning to its section) answers the plain path with the `303` and the enhanced path with a `200` `text/javascript` body that navigates there (`handlers::navigate`): Datastar runs such a body as a script, once per response.

The pickers add seven verbs to the section form's `intent` pair (the entity form takes the agent ones): `find-agent` runs a search and re-renders, `choose-agent:{picker}:{id}` and `add-agent:{field}:{id}` set a picker's value and append a row, `change-agent:{picker}` opens a card's search, `remove-role:{prefix}:{role}` drops a role, `find-entry` is a vocabulary search handled like `find-agent`, and `pick:{prefix}:{value}` (section form only) fills a discipline, place or period row. The grammar splits on the first colon after the verb, so a `pick` value may hold colons. `apply_picker_intents` rewrites the posted pairs for the choice, the add, the removal and the pick (into the row's own pairs) before the appliers run, so the appliers read an ordinary form body; find and change change nothing and re-render with the body kept.

That "marked" `GET` carries a query-string marker the redirect appends: `?saved=<the stored draft's updated_at, RFC 3339, URL-encoded>` for a save, or `?done=submitted|withdrawn|discarded` for a submit, a withdrawal or a discard. The `GET` renders the matching notice only while the marker still holds: the stamp still matches the stored row for `saved`, and for `done` the project is still in the phase named (locked for `submitted`, unlocked with the latest round `Withdrawn` for `withdrawn`, unlocked with no stored draft, or with a created project's draft equal to its seed, for `discarded`). A stale or bookmarked URL therefore confirms nothing. Because a status region filled at the first load is not reliably announced (`editor-web/src/pages/section.rs`'s module doc argues why), that same `GET` also prefixes the document `<title>` with the notice's short form (`Draft saved — …`, or `Draft saved, 1 value needs fixing — …` while the draft holds a value submit would refuse, `Sent to RDU — …`, `Submission withdrawn — …`, `Draft discarded — …`), since a navigation does announce a title change where a live region filled at load does not. RDU's `/projects` takes one more marker, `?removed=<shortcode>` after a created project's removal, with a weaker guarantee: it can only check that neither a registration nor the published set holds the shortcode, so it confirms any shortcode nothing holds; its line is prefixed to the title as well (`Project 08AB removed — …`). The entity form (see [Entity Proposals](./entity-proposals.md#the-entity-form)) mirrors both halves for its own save (`?saved=<the proposal's own stamp>`), with `Saved — …` as its short form. Its discard leaves the form: it returns to the section the proposal was opened from with `?discarded_proposal=<entity_id>`, with `Proposal discarded — …` as the short form. Its guarantee is weaker, like `?removed=`: a withdrawn proposal stays withdrawn, so the marker confirms again on a reload or a shared URL, and confirms nothing only once the entity has a live proposal again.

Four things about the enhanced path fail quietly if changed:

- **A `text/html` response *is* an implicit `datastar-patch-elements`.** With no selector it matches by `id` in `outer` mode, so what comes back is the region under one id, not a document.
- **The region is bigger than the form.** It is the rail, the status and the form together — everything a save can change. Returning only the `<form>` leaves the rail showing the obligation counts from *before* the save, so the depositor fills in the last required field, the field goes quiet, and the rail still says something is missing.
- **The autosave's region hides some format findings.** A finding renders at its control on every render of the draft except the autosave's, which leaves out one on a value typed since the last write, since the depositor may still be typing it; a `GET` or a reload of the same draft shows it ([Project form](./project-form.md)).
- **A refusal must still answer 200.** Datastar processes a response body only on a 200; any other status aborts the fetch and the message the response was carrying never reaches the page. The outcome goes on the span instead, which is where alerting reads it from — the same reasoning as the account forms' redisplayed 200.

`data-on:submit` carries no `__prevent`: the bundle calls `preventDefault` unconditionally for a `submit` event on a form element, so one would be noise. The form is *not* `novalidate` and no field is `required`, which looks contradictory and is not — see `editor-web/src/pages/section.rs`, where the `type="date"` reason is argued.

## Request middleware

Three layers wrap the app, in this order from the outside in, and a fourth, the request body limit, wraps the traced routes only:

1. **Security headers** — `Content-Security-Policy: frame-ancestors 'none'`, `X-Frame-Options: DENY` and `X-Content-Type-Options: nosniff` on every response, set outside CSRF so its refusals carry them too. See [Authentication](./authentication.md#framing) for why framing is refused. There is no full CSP: Google Fonts and the vendored Datastar would need an allowlist, a separate decision. The combobox tile's inline `onkeydown` handler would need a hash or a rewrite into a Datastar `data-on:keydown` under a strict `script-src`. HSTS is not the app's to set. Every response except a file `ServeDir` found also carries `Cache-Control: private, no-store`, so neither the browser nor a shared cache stores a signed-in page, and a new route is covered whatever its content type. `ServeDir` marks the files it serves, which keep its `Last-Modified` and no `Cache-Control`. The cost is that Back re-fetches a page rather than restoring it.
2. **CSRF** — `Sec-Fetch-Site: same-origin` is required on every non-`GET`/`HEAD` request, failing closed on everything else including an absent header. It is applied in `build_app` **after every route**, so the positional traced/untraced split cannot route around it: inside `build_router` it would have missed `/telemetry/collect`, the only pre-auth POST in the app, with no test failing. See [Authentication](./authentication.md#csrf) for why `SameSite` and `__Host-` do not close this.
3. **OTel** — the traced/untraced split below.

The body limit is `DefaultBodyLimit::max(256 KiB)` on every editor and identity route, in place of axum's 2 MiB default. The largest committed project (29 KB of JSON) form-encodes to about 40 KB as a whole and a request posts one section or one entity; the text inputs carry no `maxlength`, so the limit is what bounds a stored value. `/healthz` and the telemetry beacon are outside it (the beacon has its own limits).

Access control is **not** a layer. It is two extractors, `Authenticated` and `Rdu`, and that is the design rather than an omission: a handler that names one cannot run without the check, because the argument is what runs it, and a handler that names neither is visibly public at the point anyone reads its signature. A middleware over a sub-router would have added a second positional invariant of exactly the shape this module already regrets — the traced/untraced split is invisible in the route table and reversible by moving one line — and here the failure mode is an unauthenticated route rather than a missing span. See [Authentication](./authentication.md#authorization).

## Traced and untraced routes

An Axum layer wraps only routes declared **before** it. The router therefore has two halves:

- `build_router` — everything wrapped by `OtelInResponseLayer` then `OtelAxumLayer`. `OtelInResponseLayer` is declared first so it runs *inner* and injects the `traceparent` response header; `OtelAxumLayer` is declared second so it runs *outer* and creates the server span.
- `build_app` — adds `/healthz` and `/telemetry/collect` **after** those layers, so neither is traced. A liveness probe every 30 seconds and a telemetry upload on every page view would otherwise mint a span each and bury the real traffic.

That split is positional, so it is invisible in the route table and reversible by moving one line. A test asserts `/healthz` and the beacon are absent from `build_router`.
