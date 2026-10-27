# Every user-facing surface is a hypermedia server

Every user-facing surface in this repository, meaning every capability with a screen in any area's modulith
(DPE, CPE, identity and the editor among them), is a server-rendered HTML application. Pages are rendered on the
server with Maud and served by Axum, the server is the single source of truth for UI state, and interactivity
is added on top of working HTML with Datastar, which patches server-rendered fragments delivered over
server-sent events. There is no client-side framework, no WASM bundle, no client-side router or state store, and
no backend-for-frontend layer between a browser and the server that owns the data. A command-line client such
as dsp-cli has no screen in this sense and is outside this decision (ADR-0002). The rules, phrased so a
violation is describable in code:

- Views are `fn(...) -> maud::Markup` in a `web` crate; the capability's entry crate composes them into routes
  (ADR-0003). No template engine other than Maud, no JavaScript framework dependency in any `public/vendor/`
  directory other than the Datastar client.
- Every URL that renders a page is bookmarkable and renders the full page on a plain `GET`; a Datastar-enhanced
  control keeps a working `href` or a submit button, so the same action works with JavaScript disabled.
- A fragment endpoint renders the same Maud function the full page uses for that region, so the two cannot
  drift.
- Every state change is a `POST`; a `GET` never writes.
- Client-side script is limited to progressive enhancement of a page that already works; no page depends on
  script to show its content. CPE's deep-zoom image viewer is such an enhancement: without script, each image
  is a plain `<img>` and the viewer's controls are links.
- A page is never rendered differently by header. Machine-readable representations are dedicated URLs beside
  the page; the one negotiation step, a `303` on `Accept`, is decided in ADR-0005.
- Every surface with a screen has an end-to-end suite that runs its journeys with JavaScript disabled as well as
  enabled, because a snapshot test cannot see a control that Datastar has made inert.

Rationale, in the order it matters: a humanities archive is meant to be read for decades, and plain HTML with
server-owned state is the one frontend technology with that track record — a framework's rendering model, build
toolchain and hydration contract are what rots first (Longevity over features). With the server authoritative,
the URL is the state and every screen is one request away, which is what makes pages citable, cacheable and
testable with a browser and nothing else (Server authority; URL-based navigation). Interactivity stays contained
in the fragment it enhances rather than leaking into a client-wide state model (Contained interactivity). And
the code stays legible: an agent or a reviewer reads one Rust function and knows what the browser receives, with
no compiler-generated client, no hydration boundary and no second language to keep in sync (Explicit over
magic).

FAIR is the fourth reason, and it is what ADR-0005 builds on: FAIR assessors and metadata harvesters read the
HTML and the headers the server sent and execute no client application, so with a server-rendered page
machine-readability is a matter of what the one response carries, while a client-rendered shell would need a
second rendering path built for machines alone.

## Considered Options

- **Server-rendered hypermedia with Datastar (chosen).**
- **A single-page application** (an Angular or React client over a JSON API, as the active-research platform's
  frontend is built) — rejected: the client owns state the server must mirror, every screen needs an API
  contract and a second codebase, and the result is neither citable by URL nor readable without the framework
  it was built with.
- **Leptos with islands and a WASM bundle** — rejected: a wasm32 toolchain and a hydration contract for every
  interactive piece, a large client bundle, and a rendering model split between two runtimes, which together
  cost more than the interactivity they buy.
- **A backend-for-frontend per surface** — rejected: an extra deployable and an extra store per screen with no
  owner of its own; the hypermedia server that owns the data serves the browser directly.

## Consequences

- Datastar's attribute syntax is a compile-invisible contract: a wrong delimiter, an unknown plugin or a bad
  modifier renders fine and does nothing. Actions (`@get(…)`, `@post(…)`) are typed: they come from the
  `shared-datastar` crate and are spliced into the attribute value. Attribute names are Maud literals and
  cannot be typed, so `.github/scripts/check-datastar-attributes.sh` holds every Datastar-shaped `data-*` name in
  production code to a whitelist grammar and rejects a raw action; `just check` fails on either.
- The gate does not catch a misspelled key-less plugin (`data-sigals=` reads as a custom `data-*` attribute), the
  hyphen-named plugins (`ignore-morph`, `json-signals`, `preserve-attr`), an action built through `PreEscaped`
  or a computed method name, or JavaScript inside an attribute value. The no-JavaScript end-to-end passes are
  the backstop.
- Accessibility is a server-side concern and lives in the components a page is built from: the Mosaic tiles,
  and CPE's own component library (ADR-0007). That is why the tiles carry the semantic methods and the a11y
  suites run on tile changes.
- A capability that needs rich client behaviour argues for a contained enhancement inside one fragment, not for
  a framework; if that argument ever fails, the decision to revisit is this ADR, not a dependency added in one
  crate.
- Machine-readable metadata is a head-and-headers concern of the landing page plus dedicated URLs (ADR-0005),
  which only a server-rendered page can satisfy by construction.

Not yet in code: CPE has no end-to-end suite, so no journey of it runs with JavaScript disabled (DEV-7404).
Not yet in code: DPE's suite (`areas/access/dpe/web-e2e-tests`) disables JavaScript in single specs only, not
for every journey (not scheduled).

Enforced by: `check-datastar-attributes.sh` (static-analysis); the Deposit Area's no-JavaScript Playwright pass
in `areas/deposit/editor/web-e2e-tests` (static-analysis); review for the absence of a client framework and a
BFF (review).
