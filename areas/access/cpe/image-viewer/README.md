# cpe-image-viewer

An IIIF deep-zoom image viewer for CPE, packaged as an `<image-viewer>` **web
component** with a thin [Maud](https://maud.lambda.xyz/) helper. The engine is a
vendored [OpenSeadragon](https://openseadragon.github.io/), running in a shadow
root.

## An interim, cut-down copy

This crate is a cut-down copy of `shared/dasch-image-viewer` in
`dasch-swiss/dsp-incubator`, taken at commit
`4b1cb8201648157abb502a9b0f3bf1c1e25745c2`. It exists so that CPE's component
library can render a viewer from this repository. The incubator keeps developing
its viewer and chooses the final engine there; the result then replaces this
crate. The decision is recorded in
[`docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md`](../../../../docs/adr/0007-cpe-joins-the-access-area-as-its-second-capability.md).

Until then:

- **A fix lands here first** and is applied by hand to the incubator's crate.
  The two copies are not kept in step by any tool.
- **CPE is the only consumer.** The crate depends on `maud` alone and on no
  other crate of the capability.

### What was removed

Compared with the incubator's crate at that commit:

- The OpenLayers engine (`assets/engines/`, `assets/vendor/openlayers/`,
  `vendor-src/`, `scripts/vendor-openlayers.sh`) and the placeholder geo data
  (`assets/geo/`).
- The light-DOM OpenSeadragon engine slot and the engine switch: the private
  `Engine` and `Strategy` enums, the builders `Viewer::osd_wc`, `Viewer::osd`
  and `Viewer::openlayers`, and in the JS the engine gate, `_mountEngine`,
  `_failLight`, the engine handle and the `map` getter.
- The builders `Viewer::region`, `Viewer::rotation` and
  `Viewer::tile_transition`, which only the removed engines read.
- The types `MapView`, `LayeredView` and `Modality`.
- `include_dir` and `build.rs`: the assets are compiled in through an
  `include_bytes!` table (see [Editing the assets](#editing-the-assets)).
- The incubator's `docs/`, its `justfile`, and its showcase.

Two things were left in place although nothing here uses them:

- **The `layers` mode in the JS has no Rust builder.** `image-viewer.js` still
  implements a layered-image stack driven by a `layers` attribute. No builder
  in this crate emits that attribute.
- **An `engine` value other than `osd-wc` renders OpenSeadragon.** The element
  no longer reads the `engine` attribute. It still observes it, so changing the
  attribute re-opens the viewer. `Viewer` always emits `engine="osd-wc"`, so
  that the markup equals the incubator's.

## What's in the box

| Layer | Path | What it is |
|---|---|---|
| **0, the bundle** | `assets/image-viewer.js` | Hand-written, no-build ES-module custom element. The core logic. |
| | `assets/vendor/openseadragon.min.js` | Vendored OpenSeadragon (pinned, see `assets/vendor/VENDOR.lock`). |
| | `assets/vendor/images/*.png` | OpenSeadragon's 40 navigation-control sprites. |
| | `assets/LICENSE-openseadragon.txt` | OpenSeadragon's licence text at the vendored version. |
| **1, the crate** | `src/lib.rs` | `asset()`, `head_tag()` and the `Viewer` Maud builder. Renders attributes and serves bytes; **no viewer logic**. |

Layer 1 holds no viewer logic, so the bundle can be given to a non-Rust consumer
without a rewrite. The web component reads everything from HTML attributes, and
the Rust helper only renders the element.

## Why a web component

A custom element upgrades whenever it enters the DOM: on a full page load **or**
through a DataStar fragment swap. An inline `new OpenSeadragon(...)` init script
only runs on full loads; an injected `<script>` tag does not execute on a morph,
so it would do nothing after a swap. The viewer's DOM lives in a shadow root,
which idiomorph (DataStar's morph) treats as opaque, so a surrounding swap
cannot reach in and corrupt it.

Being a custom element and attaching a shadow root are two independent
decisions, and only the shadow root costs the workarounds in
[Rendering notes](#rendering-notes-openseadragon-inside-a-shadow-root).

## Consuming it from a Rust / Maud host

**1. Depend on it** by path; the crate is not published.

**2. Serve the assets from one route.** `asset()` rejects a path with an empty,
`.` or `..` segment, so an untrusted request tail can go straight in:

```rust
use axum::{extract::Path, http::{header, StatusCode}, response::{IntoResponse, Response}, routing::get};

async fn serve_viewer_asset(Path(path): Path<String>) -> Response {
    match cpe_image_viewer::asset(&path) {
        Some(a) => (
            [
                (header::CONTENT_TYPE, a.content_type),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            a.bytes,
        ).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// cpe_image_viewer::ASSET_PREFIX is "/_static/image-viewer"
let app = app.route("/_static/image-viewer/{*path}", get(serve_viewer_asset));
```

The route should send `X-Content-Type-Options: nosniff`, as the sample does:
`asset()` returns the bytes and a content type and sets no header itself.

`ASSET_PREFIX` is a default, and a host replaces it. ADR-0007 serves the
engine's shared files under each project's own prefix, at
`<base>/<project id>/_static/…`. The JS finds the vendored
OpenSeadragon and its sprites relative to its own module URL, so no path is
fixed in the bundle; the host passes the prefix it mounted to `head_tag()`.

`asset()` serves every file under `assets/`, the licence text and `VENDOR.lock`
included.

**3. Register the element** once, in `<head>`:

```rust
html! {
    head {
        // … your other head tags …
        (cpe_image_viewer::head_tag(cpe_image_viewer::ASSET_PREFIX))
    }
}
```

**4. Render a viewer** wherever you need one:

```rust
use cpe_image_viewer::{Viewer, Auth};
use maud::html;

Viewer::new("page-viewer", vec![info_json_url])
    .auth(Auth::Cookie)              // credentialed IIIF (SIPI); default None
    .navigator(true)                 // overview thumbnail
    .label("Page 1 recto")
    .fallback(html! {                // no-JS / pre-upgrade
        img src=(thumb_url) alt="Page 1 recto";
    })
```

The builders are `auth`, `navigator`, `sequence`, `adjust`, `label`, `fallback`,
`chrome` and `page`.

`Viewer::new` trusts its caller for the tile-source URLs: it encodes them safely
into the attribute and does not check where they point, and with `Auth::Cookie`
the browser fetches them with credentials. CPE filters each URI through
`cpe_core::util::is_safe_external_url` before it builds a viewer.

Give the viewer a **stable `id`**: idiomorph keys on it, so a stable id lets a
surrounding DataStar fragment swap preserve a running viewer instead of tearing
it down. Size the viewer with CSS on the `image-viewer` element (it defaults to
`display:block; min-height:240px`), e.g. `image-viewer { height: 80vh; }`.

### Client → server (optional)

The element emits a `viewer-error` `CustomEvent` (bubbling) when an image fails
to open. A host can react with DataStar, e.g.
`data-on:viewer-error="@post('/…')"`. Keep the contract narrow: attributes in,
events out. Do not wire OpenSeadragon state into DataStar signals.

## Public JS control surface (host-rendered chrome)

This section is for a host that renders its **own** chrome (a paging strip, zoom
chips, an Adjust rail) instead of relying on OpenSeadragon's stock controls; CPE's
zoom takeover does. The rule that makes it necessary: **never mutate an
*observed* HTML attribute to drive interaction.** `tile-sources` (and every
other attribute in `ImageViewer.observedAttributes`) is watched by
`attributeChangedCallback`, which tears the whole viewer down and rebuilds it on
any change. That is right for a new image and wrong for "page to the next image"
or "zoom in". The methods and attributes below exist so that a host never has to
do it.

### Builder-time (Rust)

- **`Viewer::chrome(false)`** opts out of OSD's own stock navigation
  (prev/next), sequence and rotation controls (all default **on**;
  `Viewer::chrome(true)` and never calling it render the same markup). Set it
  when the host renders its own paging and zoom chrome and drives the element
  through the methods below: without it, a multi-source viewer draws a **second**
  pager that pages the canvas without going through the host's own state.
- **`Viewer::page(n)`** is the page (see [1-based](#1-based-not-0-based)) to
  open a multi-source sequence on, instead of page 1. It is rendered as a plain
  `page="N"` attribute and read **once** at element init. It is deliberately
  **not** in `observedAttributes`, so a host can re-render or patch it (across a
  DataStar fragment swap, for example) without triggering the viewer's teardown.

### Runtime (JS, on the `<image-viewer>` element itself)

All of these are safe to call **before the viewer has opened**: the module
script and the lazy-loaded OpenSeadragon bundle are both async, so a click
handler wired at render time may run before `_init` finishes. `zoomIn`,
`zoomOut`, `goHome` and `zoomToShape` do nothing until then; `goToPage` and
`setOverlayState` **queue** and apply themselves once OSD's `open` event fires.

| Method | What it does |
| --- | --- |
| `zoomIn()` | Step the OSD viewport zoom in. |
| `zoomOut()` | Step the OSD viewport zoom out. |
| `goHome()` | Reset pan and zoom to the home view. |
| `goToPage(n)` | Page a multi-source sequence to page `n` (1-based), through OSD's own `sequenceMode`, never by mutating `tile-sources`. Queues until `open` if called early; does nothing on a single-image viewer. |
| `setBrightness(v)` / `setContrast(v)` / `setSaturation(v)` | Set one of the three percentage CSS-filter operators (clamped `0..=200`). |
| `setExposure(v)` | Set the exposure operator (clamped `-1..=1`, default `0`). **Approximated** as a CSS filter, which has no native equivalent: folded into `brightness()`. |
| `setGamma(v)` | Set the gamma operator (clamped `0.5..=2`, default `1`). **Approximated** as a CSS filter, which has no native equivalent: folded into `brightness()`. |
| `setHue(v)` | Set the hue-rotation operator (degrees, clamped `-180..=180`). |
| `setInvert(on)` | Toggle the invert operator (`boolean`). |
| `resetAdjust()` | Reset all seven operators to their defaults. |
| `setOverlayState({visible, focus})` | Show or hide the [overlay layer](#overlay-layer) and emphasise one shape. Replaces the whole state: a missing `visible` is `true`, an empty or missing `focus` is none. Stored, so a call before `open` applies once the image opens. Default `{visible: true, focus: null}`. |
| `zoomToShape(id, {fraction, maxZoom})` | Animate onto overlay shape `id`: its box padded to `size / fraction` (default `0.6`) about its centre, held between fit and `maxZoom` × fit (default `4`). Does nothing before open and for an id the image does not draw. |

With these setters a host renders its **own** adjust panel. The crate's built-in
panel (`Viewer::adjust(true)`) stays off by default, and the two need not
coexist; if they do, the built-in panel's controls stay in sync.

### `viewer-zoom` event

A bubbling `CustomEvent("viewer-zoom")` fires on every OSD zoom change (and once
right after `open`), for a host-rendered scale readout:

```js
el.addEventListener("viewer-zoom", (e) => {
  const { zoomPercent, atHome, atNative, atMin, atMax, ratioToHome } = e.detail;
  // zoomPercent/ratioToHome are `null` until an image has actually opened.
  // atHome: at fit · atMin: at the zoom-out floor (fit) · atMax: at max
  // magnification · atNative: screen 1:1 (a readout marker, not the zoom-in
  // limit — you can magnify past native up to `maxZoomPixelRatio`).
});
```

### `viewer-rendered` and `viewer-warning` events

- `viewer-rendered` (bubbling) fires each time OpenSeadragon draws a tile or an animation settles; its
  `detail` is `{ engine, t }`, the engine slug (`"osd-wc"`) and a `performance.now()` timestamp.
- `viewer-warning` (bubbling) fires when one layer of a `layers` stack fails to load; its `detail` is the
  message, a string.

### Overlay events

Two bubbling `CustomEvent`s on the element link the overlay to a host-rendered
list (the names are lowercase, so a DataStar `data-on:viewer-overlay-hover`
binding works on an ancestor):

| Event | `detail` | Fires when |
| --- | --- | --- |
| `viewer-overlay-hover` | `{ id }`, `id` is `null` on leave | the pointer enters or leaves a shape or its tag |
| `viewer-overlay-click` | `{ id }` | a quick click (OSD's click thresholds) lands on a shape or its tag |

A drag that starts on a shape still pans and the wheel still zooms over one;
only a click on a shape or tag suppresses OSD's click-to-zoom. The crate never
emphasises anything by itself: a host answers these events with
`setOverlayState`.

### 1-based, not 0-based

`goToPage(n)` and `Viewer::page(n)` are both **1-based**, as is every other page
or sequence index that crosses between CPE and this crate (`?member=`, `?i=`,
the `.pg` pager labels): page 1 is the first image. The element translates it to
OpenSeadragon's 0-based `sequenceMode` index (`n - 1`) at the `goToPage` call.

## Overlay layer

Annotation shapes and their numbers, drawn on the deep-zoom canvas so that they
track pan and zoom.

The host renders the overlay as ordinary light-DOM children of the element,
typically inside the `.fallback()` beside the plain image, so the same markup is
the no-JS rendering:

```html
<image-viewer id="v" tile-sources="[…]">
  <img src="…/full/900,/0/default.jpg" alt="…">
  <svg data-iv-overlay viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
    <g data-iv-shape="a1" style="--iv-c:#ff3333;--iv-on:#000">
      <rect data-iv-part="halo" x="17" y="15" width="40" height="30"/>
      <rect data-iv-part="line" x="17" y="15" width="40" height="30"/>
    </g>
  </svg>
  <span data-iv-tag="a1" data-iv-x="17" data-iv-y="15" data-iv-align="start"
        style="--iv-c:#ff3333;--iv-on:#000" aria-hidden="true">1</span>
</image-viewer>
```

- **`svg[data-iv-overlay]`**, one per element: `viewBox="0 0 100 100"` and
  `preserveAspectRatio="none"`, so coordinates are percentages of the image's
  width and height.
- **`g[data-iv-shape=<id>]`**, one per shape, with inline `--iv-c` (the shape's
  colour) and `--iv-on` (the text colour on it). Its geometry is given twice as
  direct children, `data-iv-part="halo"` first, then `data-iv-part="line"`:
  `rect`, `ellipse`, `circle`, `polygon` or `polyline`. Later shapes paint on
  top and win the hit, so put a nested smaller shape after the one containing
  it.
- **`[data-iv-tag=<id>]`**, an HTML element holding the shape's number, with the
  same `--iv-c`/`--iv-on`. `data-iv-x`/`data-iv-y` are its anchor in the same
  percentages; `data-iv-align="start"` (its left edge on the anchor, the
  default) or `"center"`; `data-iv-place="above"` (default, 3px above the
  anchor) or `"below"` (3px below it, for a shape at the image's top edge). Any
  `style` positioning the host adds for its no-JS copy is neutralised in the
  viewer.

On every `open` (including a sequence `goToPage`) the element clones that
structure into its shadow root: the SVG becomes one overlay over the full image
rectangle, each tag a point overlay that keeps its screen size. The clone loses
`class` and every `data-*` attribute outside `data-iv-*`, so host bindings
(DataStar `data-class`, `data-on:*`) never reach it; its state comes only from
`setOverlayState`. `data-iv-off`, `data-iv-focused` and `data-iv-em` are the
element's own state attributes; a host copy of them is dropped too. The clone
carries no script either: `on*`, `href` and `xlink:href` attributes are dropped,
and so is any descendant other than a shape, group or text element (`g`, `rect`,
`ellipse`, `circle`, `polygon`, `polyline`, `line`, `path`, `text`, `tspan`,
`title`, `desc`; a tag keeps only `span`s). A hover still open when the overlay
is replaced (a re-init, a page turn) is closed with a `viewer-overlay-hover` of
`null`. Removing the element sends that event too late for an ancestor to see it
(the element has already left the document), so a host that removes the viewer
clears its own hover state.

The clone is styled by the shadow stylesheet, never by host CSS: halo and line
strokes are non-scaling, with rest, emphasised (the `focus` id) and sibling
(dashed, while another is emphasised) weights. Inheritable custom properties set
on `<image-viewer>` carry the host's tokens: `--iv-halo` (halo colour),
`--iv-ink` and `--iv-surface` (the emphasised tag's fill and text), `--iv-mono`
(the tag's font), `--iv-dur` (transition duration). The Adjust filters apply to
the scan only, not to the overlay, on the canvas drawer the element forces. Were
OSD ever to pick another drawer, the filters fall back to the whole OSD
container and tint the overlay too.

## Rendering notes (OpenSeadragon inside a shadow root)

OpenSeadragon's [shadow-DOM support is incomplete](https://github.com/openseadragon/openseadragon/issues/1677)
(it uses some document-global DOM queries), and this component runs OSD inside a
shadow root by design, so that idiomorph treats the viewer's DOM as opaque. Three
consequences are worked around in `image-viewer.js`:

- **Canvas drawer, not WebGL.** OSD 6 defaults to the WebGL drawer, which
  renders a blank canvas here (`Error creating texture in WebGL`, reproduced in
  Chrome). The element forces `drawer: "canvas"`.
- **Re-fit on `open`.** The stage is `display:none` while OSD initialises, so it
  captures a 1×1 container; on `open` the element reveals it, resizes and calls
  `goHome()` at the real size.
- **First-paint nudge.** Even correctly sized, OSD does not paint the home view
  until the viewport *changes*; `goHome()` and `forceRedraw()` alone do not
  trigger it. The element applies an imperceptible (`1e-7`) pan at two points
  after `open` to force that first paint.

**These cannot be verified from an automated browser.** The harness backgrounds
the tab, which throttles OSD's render loop, so the viewer cannot paint on load.
Verify the on-load render in a real, foregrounded browser.

All three exist to pay for the shadow root. What it took, what was ruled out,
and the question of which engine to keep are in the incubator's brief:
`dsp-incubator shared/dasch-image-viewer/docs/engine-comparison-spike.md`.

## Auth

OpenSeadragon fetches `info.json` and then many individual tile images; auth has
to cover both.

- **`Auth::None`** (default): public images, no credentials.
- **`Auth::Cookie`**: sends credentials (cookies) with the `info.json` fetch and
  every tile request, for cookie-gated IIIF such as SIPI. The image server must
  allow credentialed CORS.

**Bearer tokens are intentionally not supported.** A token rendered into a
server-rendered HTML attribute leaks into the DOM. If DaSCH images turn out to
be bearer-gated, add it through an out-of-band hook (fetch the token in JS and
set `ajaxHeaders` on the OSD options), not through an attribute.

## Editing the assets

`src/lib.rs` compiles every file under `assets/` into the crate through the
`ASSETS` table: one `include_bytes!` row per file. `include_bytes!` registers
each file with cargo, so an edit to `image-viewer.js` rebuilds the crate without
a build script.

- **A new file needs a new row.** A file under `assets/` without a row is not
  served. The test `asset_returns_every_file_under_assets` in `tests/render.rs`
  walks the directory and fails for it.
- **A removed file needs its row removed.** A row whose file is missing does not
  compile.
- **A new file extension needs an arm in `content_type_for`**, or the file is
  served as `application/octet-stream`.

## Updating OpenSeadragon

The version is pinned in one place: `OSD_VERSION` in
[`scripts/vendor-osd.sh`](scripts/vendor-osd.sh). To change it, edit
`OSD_VERSION` and run the script:

```bash
areas/access/cpe/image-viewer/scripts/vendor-osd.sh
```

It downloads the JS and the sprites from the official npm package (through
jsDelivr) and regenerates `assets/vendor/VENDOR.lock`, a SHA-256 of every
vendored file. Review the lock's diff, add a row to `ASSETS` for any file that
is new, replace `assets/LICENSE-openseadragon.txt` with the licence text of the
new version, and commit.

**The vendored files are checked against `VENDOR.lock` by hand.**
`just verify-checksums` does not read this crate; it covers the vendored
JavaScript under a module's `public/vendor/`. To check:

```bash
cd areas/access/cpe/image-viewer/assets/vendor
grep -E '^[0-9a-f]{64}  ' VENDOR.lock | shasum -a 256 -c -
```

### Licence

OpenSeadragon and its sprites are under the BSD 3-Clause licence. Its text, as
upstream ships it at the vendored version (`LICENSE.txt` at the tag `v6.0.2`),
is `assets/LICENSE-openseadragon.txt`. That file states the terms without the
licence's name; the name is the one upstream's `package.json` gives at the same
tag (`BSD-3-Clause`).

## Testing

```bash
cargo test -p cpe-image-viewer    # integration tests and doc tests
just test                         # the workspace's tests; runs no doc test
```

The tests pin the Rust contract (attribute rendering, safe URL encoding, asset
lookup and traversal rejection) and some text of `image-viewer.js`. The viewer's
runtime behaviour lives in `image-viewer.js`, and no test in this repository
runs it in a browser.
