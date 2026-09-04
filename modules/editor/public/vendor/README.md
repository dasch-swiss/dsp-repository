# Vendored JavaScript Dependencies

Do not edit these files directly. See update process below.

| File | Package | Version | SHA-256 |
|------|---------|---------|---------|
| `web-vitals-attribution.js` | web-vitals | 5.2.0 | `sha256:580581eec7c7c21eebdf6e4d382389aab3ecf5f42561153d16424b5e35240260` |
| `datastar.js` | starfederation/datastar | 1.0.2 | `sha256:2837d87acf6ee0ba8e4e63765926c25a98d63883b02f88be194a86b81d3fd24a` |

## Sources

- web-vitals: `npm:web-vitals/dist/web-vitals.attribution.js` — byte-identical to DPE's copy
- datastar: <https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.2/bundles/datastar.js>

The Datastar URL is jsdelivr's **`gh/`** (GitHub) route, not `npm/`. The npm package
`@starfederation/datastar` is stale — its latest published version is `1.0.0-beta.11`, so an
`npm/` URL for any 1.0.x tag returns 404. Bundles are published only as repository files.

## Datastar version

The table above is the editor's version of record. DPE vendors its own copy under `modules/dpe/public/vendor/`; nothing is shared between the two directories and each is bumped on its own, so do not read either one's version off the other.

One property of 1.0.x worth knowing: keyed plugin attributes use `:`, not `-` — `data-on:click`, `data-attr:disabled`, `data-class:open`, and `data-init` rather than `data-on-load`. That has been true since RC.6, so it matches DPE's markup too. The old hyphen form fails **semi-silently**: a console error and an inert control, with the page rendering fine and snapshot tests still passing.

## Known dead listener in `telemetry.js`

`telemetry.js` registers `document.addEventListener('datastar-sse-error', ...)`, copied from DPE.
No such event exists in 1.0.2 — the bundle dispatches `datastar-fetch`, `datastar-patch-elements`,
`datastar-patch-signals`, `datastar-prop-change`, `datastar-ready`, `datastar-scope-children` and
`datastar-signal-patch`. The `kind: 'datastar_sse'` beacon signal is therefore unreachable, and the
collector's SSE-error counter will read zero however many fragment requests fail.

Harmless today, because no Datastar endpoint exists yet. Rewire it to `datastar-fetch` (whose detail
carries the failure type) when the first one lands, and verify against a real failing request rather
than by inspection — this is exactly the semi-silent class of failure the delimiter note above warns
about.

## Update process

1. Download the new version from the source URL above
2. Replace the file in this directory
3. Update the version and SHA-256 in the table above: `shasum -a 256 <file>`
4. Confirm the table matches the files: `just verify-checksums` (also run by `just check`)
5. Verify the application works (`just dev-editor`)
