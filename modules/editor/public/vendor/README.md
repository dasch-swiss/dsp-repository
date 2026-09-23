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

## The Datastar failure listener in `telemetry.js`

`telemetry.js` listens for `datastar-fetch`, the one event 1.0.x dispatches for every request outcome, and reports `type: 'error'` (a response status of 400 or above, with the status in `detail.argsRaw`) and the first `type: 'retrying'` or `type: 'retries-failed'` of each request as error kind `datastar_sse`. A network failure or dropped stream produces no `error`: Datastar retries it (even with `retry: 'never'`, which covers only HTTP statuses) and dispatches `retries-failed` only after about 3 minutes, so the first retry is where it is reported. The kind name is kept so the collector's bounded set and existing dashboards stay valid. There is no `datastar-sse-error` event in any 1.0.x release.

The editor answers `200` on its Datastar paths by design, rendering refusals into the patched region, so here the listener fires only on a transport failure or on a route that breaks that convention. A bump that renames the event or its `type` values silences the listener without an error, so check `detail` against the new bundle on every update.

## Update process

1. Download the new version from the source URL above
2. Replace the file in this directory
3. Update the version and SHA-256 in the table above: `shasum -a 256 <file>`
4. Confirm the table matches the files: `just verify-checksums` (also run by `just check`)
5. Verify the application works (`just dev-editor`)
