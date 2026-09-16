---
status: accepted
date: 2026-09-16
---

# Every user-facing surface is a hypermedia server

Every user-facing surface in this repository — today DPE and the metadata editor, tomorrow every capability with a screen in any area's modulith — is a server-rendered HTML application: pages are rendered on the server with Maud and served by Axum, the server is the single source of truth for UI state, and interactivity is added on top of working HTML with Datastar, which patches server-rendered fragments delivered over server-sent events. There is no client-side framework, no WASM bundle, no client-side router or state store, and no backend-for-frontend layer between a browser and the server that owns the data. The rules, phrased so a violation is describable in code:

- Views are `fn(...) -> maud::Markup` in a `web` crate; the server crate composes them into routes. No template engine other than Maud, no JavaScript framework dependency in any `public/vendor/` directory other than the Datastar client.
- Every URL that renders a page is bookmarkable and renders the full page on a plain `GET`; a Datastar-enhanced control keeps a working `href` or a submit button, so the same action works with JavaScript disabled.
- A fragment endpoint renders the same Maud function the full page uses for that region, so the two cannot drift.
- Every state change is a `POST`; a `GET` never writes.
- Client-side script is limited to progressive enhancement of a page that already works; no page depends on script to show its content.
- A page is never rendered differently by header. Machine-readable representations are dedicated URLs beside the page; the one negotiation step, a `303` on `Accept`, is decided in ADR-0005.

Rationale, in the order it matters: a humanities archive is meant to be read for decades, and plain HTML with server-owned state is the one frontend technology with that track record — a framework's rendering model, build toolchain and hydration contract are what rots first (Longevity over features). With the server authoritative, the URL is the state and every screen is one request away, which is what makes pages citable, cacheable and testable with a browser and nothing else (Server authority; URL-based navigation). Interactivity stays contained in the fragment it enhances rather than leaking into a client-wide state model (Contained interactivity). And the code stays legible: an agent or a reviewer reads one Rust function and knows what the browser receives, with no compiler-generated client, no hydration boundary and no second language to keep in sync (Explicit over magic).

FAIR is the fourth reason, and it is what ADR-0005 builds on: FAIR assessors and metadata harvesters read the HTML and the headers the server sent and execute no client application, so with a server-rendered page machine-readability is a matter of what the one response carries, while a client-rendered shell would need a second rendering path built for machines alone. The repository tried the alternative: DPE and Mosaic were built on Leptos with islands and a WASM bundle, and were migrated to Maud + Axum + Datastar in DEV-6642 because the WASM toolchain, the hydration model and the bundle size cost more than the interactivity they bought.

## Considered Options

- **Server-rendered hypermedia with Datastar (chosen).**
- **A single-page application** (an Angular or React client over a JSON API, as the active-research platform's frontend is built) — rejected: the client owns state the server must mirror, every screen needs an API contract and a second codebase, and the result is neither citable by URL nor readable without the framework it was built with.
- **Leptos with islands and a WASM bundle** — tried and removed (DEV-6642): a wasm32 toolchain and a hydration contract for every interactive piece, a large client bundle, and a rendering model split between two runtimes.
- **A backend-for-frontend per surface** — rejected: an extra deployable and an extra store per screen with no owner of its own; the hypermedia server that owns the data serves the browser directly.

## Consequences

- Datastar's attribute syntax is a compile-invisible contract: a wrong delimiter renders fine and does nothing. `.github/scripts/check-datastar-delimiters.sh` fails `just check` on the retired forms; the editor's E2E suite runs every journey twice, with and without JavaScript, because a snapshot test cannot see a control that Datastar has made inert.
- Accessibility is a server-side concern and lives in the Mosaic tiles, which is why the tiles carry the semantic methods and the a11y suites run on tile changes.
- A capability that needs rich client behaviour argues for a contained enhancement inside one fragment, not for a framework; if that argument ever fails, the decision to revisit is this ADR, not a dependency added in one crate.
- Machine-readable metadata is a head-and-headers concern of the landing page plus dedicated URLs (ADR-0005), which only a server-rendered page can satisfy by construction.

Enforced by: `check-datastar-delimiters.sh` and the editor's no-JavaScript E2E pass (static-analysis) for the enhancement rules; review for the absence of a client framework and a BFF (review).
