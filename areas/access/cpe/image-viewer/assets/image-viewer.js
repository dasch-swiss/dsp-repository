// <image-viewer> — a IIIF deep-zoom image-viewer custom element.
//
// The public surface is the <image-viewer> element, its attributes, its
// methods and its `viewer-*` events; the deep-zoom engine is a vendored
// OpenSeadragon, the only one. This is the one hand-written piece of the
// cpe-image-viewer crate: deliberately plain, dependency-free, no-build
// ES-module JavaScript so the checked-in file IS the source (mirrors CPE's
// datastar.js vendoring discipline). See the crate README and
// dsp-incubator cpe/docs/adr/0010-openseadragon-integration.md.
//
// ⚠️ EDITING THIS FILE: it is embedded into the crate at compile time
// (`include_bytes!` rows in the `ASSETS` table in `src/lib.rs`), which cargo
// tracks per file, so an edit here rebuilds the crate. See the crate
// README, "Editing the assets".
//
// Design contract (why a custom element and not an inline init script):
//   * Upgrade is tied to DOM PRESENCE, not to a <script> running. The element
//     initialises whether it arrived on a full MPA page load OR via a DataStar
//     fragment swap — injected <script> tags do NOT execute on a morph, an
//     inline init would silently no-op there.
//   * The OpenSeadragon-managed DOM lives inside a shadow root, which
//     idiomorph (DataStar's morph) treats as opaque — so a surrounding
//     fragment swap can never reach in and corrupt the viewer's internal DOM.
//   * The server owns state: everything the viewer needs comes in through
//     HTML attributes (server-rendered). The element is an interactivity
//     island with a narrow contract — attributes in, custom events out.
//
// Attributes (all read on connect; the reactive ones re-open on change):
//   engine        the deep-zoom engine. Only "osd-wc" (OpenSeadragon in this shadow
//                 root) exists; any other value renders OpenSeadragon too. The Rust
//                 `Viewer` always emits "osd-wc".
//   tile-sources  JSON array of IIIF info.json URLs (or raw OSD tile-source
//                 objects). One entry → single image; many → paged sequence.
//   auth-mode     "none" (default) | "cookie". "cookie" sends credentials with
//                 info.json + tile requests (for SIPI cookie-gated images).
//                 NOTE: bearer-token auth is intentionally NOT implemented here
//                 — putting a token in a server-rendered attribute leaks it into
//                 the DOM. See the README "Auth" section before adding it.
//   navigator     presence/"true" shows the overview navigator thumbnail.
//   sequence      "true" forces sequence mode; otherwise it is auto-enabled
//                 when tile-sources has more than one entry.
//   adjust        presence/"true" adds an "Adjust" toggle (bottom-left); the
//                 image-adjustment panel (brightness, contrast, exposure,
//                 saturation, gamma, hue, invert) is collapsed by default and
//                 expands on click — CSS filters applied to the tiled-image
//                 canvas. Zero pixel cost: GPU-composited,
//                 no per-tile reprocessing. `exposure`/`gamma` have no native
//                 CSS filter equivalent and are CSS-filter APPROXIMATIONS — see
//                 `setExposure`/`setGamma` below and the comment at `_applyFilter`.
//   label         accessible label for the viewer region.
//   chrome        "false" opts OUT of OSD's own stock navigation/sequence/
//                 rotation controls (all default ON) — for a host that renders
//                 its own paging/zoom chrome and drives it via the public JS
//                 methods below. Read once at init; NOT in observedAttributes
//                 (host chrome-mode is a per-mount choice, not a reactive one).
//   page          the 1-based page to open a multi-source sequence on, instead
//                 of always page 1 (e.g. a takeover reopening on the clicked
//                 image). Read once at init. Deliberately NOT in
//                 observedAttributes: unlike `tile-sources`, patching this
//                 attribute (e.g. across a DataStar swap) must never tear the
//                 viewer down — see `goToPage()` below, which is the reactive
//                 sibling of this static initial value.
//
// An `svg[data-iv-overlay]` plus `[data-iv-tag]` elements among the light-DOM
// children are an annotation OVERLAY: drawn as-is without JS, and cloned onto
// the deep-zoom canvas on every `open` (osd-wc only; crate README, "Overlay
// layer").
//
// Light-DOM children (e.g. a <img> IIIF thumbnail) are the no-JS / pre-upgrade
// FALLBACK: shown until the viewer opens. On a LOAD FAILURE they are NOT
// revealed — a static image that mimics a working deep-zoom viewer would hide
// the failure. Instead a visible error state is shown and `viewer-error` is
// emitted. (A fallback is optional; the builder supports
// one for a consumer that wants a pre-upgrade placeholder.)
//
// Public JS control surface (host-rendered chrome)
// A host that renders its OWN chrome (buttons, pager, adjust rail — e.g. CPE's
// zoom takeover) drives the element via these public methods instead of
// mutating attributes (mutating an OBSERVED attribute like `tile-sources`
// tears the viewer down and rebuilds it — see attributeChangedCallback below).
// All are safe to call before the viewer has opened (they queue/no-op rather
// than throwing) — the module script + lazy OSD load are async, so a click
// handler wired at render time may run before `_init` finishes.
//
//   zoomIn() / zoomOut()   step the OSD viewport zoom. No-op before open.
//   goHome()               reset pan/zoom to the home view. No-op before open.
//   goToPage(n)             page a multi-source sequence to the 1-based page
//                          `n`, driving OSD's OWN `sequenceMode` (auto-on for
//                          >1 source) — NOT by mutating `tile-sources`. Queues
//                          until OSD's `open` fires if called before then (or
//                          before a subsequent re-init settles); a no-op when
//                          the viewer is single-image (sequenceMode off).
//   setBrightness(v) / setContrast(v) / setExposure(v) / setSaturation(v) /
//   setGamma(v) / setHue(v) / setInvert(on)
//                          set one of the seven CSS-filter adjustment operators
//                          directly (brightness/contrast/saturation 0–200%,
//                          exposure -1..1, gamma 0.5..2, hue -180..180deg,
//                          invert boolean) — for a host that renders its own
//                          Adjust rail UI (this crate's own built-in panel,
//                          `adjust[true]`/`Viewer::adjust`, stays independent
//                          and off by default). Safe before open — reasserted
//                          on `open`/re-init. `setExposure`/`setGamma` have no
//                          native CSS filter equivalent and are APPROXIMATED
//                          via the brightness filter chain — see the comment
//                          at `_applyFilter` for the full explanation
//                          (a true per-pixel implementation would
//                          apply the real operators).
//   resetAdjust()          reset all seven operators to their defaults.
//   setOverlayState({visible, focus})
//                          show/hide the overlay layer and emphasise one shape
//                          (crate README, "Overlay layer"). Stored until open.
//   zoomToShape(id, {fraction, maxZoom})
//                          animate onto one overlay shape. No-op before open.
//
// Events `viewer-overlay-hover` (`detail: { id }`, `id` null on leave and when
// the overlay is replaced) and
// `viewer-overlay-click` (`detail: { id }`) bubble from the element when the
// pointer enters/leaves or quick-clicks an overlay shape or its tag.
//
// Event: `viewer-zoom` (bubbling CustomEvent) fires on every zoom change
// (`detail: { zoomPercent, atHome, atNative, atMin, atMax, ratioToHome }`) — for a
// host-rendered scale readout. `zoomPercent`/`ratioToHome` are null until an
// image has opened. `zoomPercent` is screen-px-per-source-px (1 = native
// resolution) — NOT relative to the fit/home zoom, so a host must not read it
// as a "×N from FIT" scale (a modest zoom-in from a heavily-downscaled FIT can
// still read well under 1). `ratioToHome` is `getZoom() / getHomeZoom()` —
// exactly 1 at FIT/home, and the value a host-rendered "×N" readout should use
// so zooming in from FIT always reads above ×1.

const BUILTIN_ENGINE = "osd-wc";

// Resolve the vendored OpenSeadragon next to this module, wherever the host
// mounted the asset directory — the mount path is NOT hardcoded here.
const OSD_SCRIPT = new URL("./vendor/openseadragon.min.js", import.meta.url).href;
const PREFIX_URL = new URL("./vendor/images/", import.meta.url).href;

// Load OpenSeadragon once per document, lazily, on first viewer connect. Heavy
// (~340 KB) so pages without a viewer never pay for it.
let _osdPromise = null;
function loadOpenSeadragon() {
  if (window.OpenSeadragon) return Promise.resolve(window.OpenSeadragon);
  if (_osdPromise) return _osdPromise;
  _osdPromise = new Promise((resolve, reject) => {
    const s = document.createElement("script");
    s.src = OSD_SCRIPT;
    s.async = true;
    s.onload = () =>
      window.OpenSeadragon
        ? resolve(window.OpenSeadragon)
        : reject(new Error("OpenSeadragon did not define a global after load"));
    s.onerror = () => reject(new Error(`failed to load ${OSD_SCRIPT}`));
    document.head.appendChild(s);
  });
  return _osdPromise;
}

function parseSources(raw) {
  if (!raw) return [];
  try {
    const v = JSON.parse(raw);
    return Array.isArray(v) ? v : [v];
  } catch {
    // Tolerate a bare URL string that isn't JSON-quoted.
    return [raw];
  }
}

// The image adjustments. Each maps to a CSS filter function; grayscale is just
// saturation 0, so it needs no own control. Hue rotation shifts the colour
// cast of discoloured parchment; like brightness, contrast, saturation and
// invert it is one function in the same filter string.
// `exposure`/`gamma` have NO native CSS filter equivalent, so they are
// CSS-filter APPROXIMATIONS,
// composited into the `brightness()` term at apply-time (see `_applyFilter`).
//
// NOTE the unit split: `brightness`/`contrast`/`saturate` are percentages
// (0–200, default 100); `hue` is in degrees over a SIGNED -180…+180 range;
// `exposure` is signed -1..1 (default 0); `gamma` is 0.5..2 (default 1).
// That's why the slider rows carry their own min/max rather than sharing one pair.
//
// Signed rather than 0–360 because the hue wheel is cyclic — both spans cover it
// completely, but a signed one puts the neutral default (0 = unrotated) in the
// middle of the track, so the control reads as "warmer / cooler from here" and
// the reset position is where the thumb visibly sits at rest. `hue-rotate()`
// takes negative angles fine. (-180 and +180 land on the same colour; harmless.)
const ADJUST_DEFAULTS = {
  brightness: 100,
  contrast: 100,
  saturate: 100,
  hue: 0,
  invert: 0,
  exposure: 0,
  gamma: 1,
};

// Zoom-out floor as a fraction of the home (fit) zoom. Pinned at fit (1.0),
// NOT slightly below: a sub-fit floor (e.g. 0.9 = 10% past fit) gives
// softer haptics but an ugly visual oscillation at the boundary —
// below fit the image is SMALLER than the viewport, so OSD's image-visibility/
// pan constraint and its zoom constraint fight each other at the floor and the
// render jitters (~0.88↔0.90×). At/above fit the image always fills the
// viewport, so there is no such conflict and the floor is clean. The wheel/
// gesture path is bound by OSD's `minZoomImageRatio` (which is what oscillates
// below fit), so a twitch-free sub-fit floor is not achievable here. Used by
// the OSD `minZoomImageRatio` option (wheel/gesture), the `zoomOut()` clamp
// (button), and `_emitZoom`'s `atMin` edge — one shared floor.
const MIN_HOME_ZOOM_RATIO = 1.0;

// Clamp `n` into `[min, max]` rather than throwing —
// a public setter must never hand OSD/CSS a
// nonsensical value just because a caller passed one. This is the raw
// range-clamp primitive: a non-finite input falls back to `min`, which is
// fine where `min` genuinely is the safe/destructive-but-valid extreme. The
// six numeric adjust setters (`setBrightness`/`setContrast`/`setSaturation`/
// `setHue`/`setExposure`/`setGamma`) do NOT rely on that bare `min` fallback
// directly — they route through `_setNumericAdjust`, which falls back to
// each operator's own NEUTRAL default on non-finite input instead (see that
// method for why).
function clamp(n, min, max) {
  return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : min;
}

// An overlay clone carries the overlay's structure and nothing of the host's
// behaviour: `class` and every data attribute outside `data-iv-*` (DataStar
// bindings, host state) are dropped, and so are the element's own state
// attributes, so all state comes via `setOverlayState`. Nor can it carry
// script into the shadow root: event-handler (`on*`) and link (`href`,
// `xlink:href`) attributes are dropped, and so is every descendant that is
// not a shape, group or text element (`script`, `foreignObject`, `a`, …);
// a tag keeps only `span` descendants.
const OVERLAY_STATE_ATTRS = ["data-iv-off", "data-iv-focused", "data-iv-em"];
const OVERLAY_SVG_ELEMENTS = new Set([
  "g", "rect", "ellipse", "circle", "polygon", "polyline", "line", "path",
  "text", "tspan", "title", "desc",
]);
function sanitizeOverlayClone(root) {
  const svg = root.namespaceURI === "http://www.w3.org/2000/svg";
  for (const el of root.querySelectorAll("*")) {
    const name = el.localName;
    if (svg ? !OVERLAY_SVG_ELEMENTS.has(name) : name !== "span") el.remove();
  }
  for (const el of [root, ...root.querySelectorAll("*")]) {
    for (const name of el.getAttributeNames()) {
      if (
        name === "class" ||
        name.startsWith("on") ||
        name === "href" ||
        name === "xlink:href" ||
        OVERLAY_STATE_ATTRS.includes(name) ||
        (name.startsWith("data-") && !name.startsWith("data-iv-"))
      ) {
        el.removeAttribute(name);
      }
    }
  }
  return root;
}

// The OSD placement naming which point of a tag sits on its anchor: its
// bottom (above the anchor, the default) or top (`data-iv-place="below"`),
// at its left edge or, for `data-iv-align="center"`, its centre.
function tagPlacement(tag) {
  const P = OpenSeadragon.Placement;
  const centre = tag.dataset.ivAlign === "center";
  if (tag.dataset.ivPlace === "below") return centre ? P.TOP : P.TOP_LEFT;
  return centre ? P.BOTTOM : P.BOTTOM_LEFT;
}

// A shape group's box in the overlay's 0..100 units, read from the geometry
// attributes rather than layout, so a hidden shape has one too.
function shapeBox(g) {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  const add = (x, y) => {
    if (!Number.isFinite(x) || !Number.isFinite(y)) return;
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  };
  for (const el of g.children) {
    const n = (name) => Number(el.getAttribute(name));
    switch (el.localName) {
      case "rect":
        add(n("x"), n("y"));
        add(n("x") + n("width"), n("y") + n("height"));
        break;
      case "ellipse":
        add(n("cx") - n("rx"), n("cy") - n("ry"));
        add(n("cx") + n("rx"), n("cy") + n("ry"));
        break;
      case "circle":
        add(n("cx") - n("r"), n("cy") - n("r"));
        add(n("cx") + n("r"), n("cy") + n("r"));
        break;
      case "polygon":
      case "polyline": {
        const v = (el.getAttribute("points") || "").trim().split(/[\s,]+/).map(Number);
        for (let i = 0; i + 1 < v.length; i += 2) add(v[i], v[i + 1]);
        break;
      }
    }
  }
  return x0 <= x1 && y0 <= y1 ? { x: x0, y: y0, w: x1 - x0, h: y1 - y0 } : null;
}

// Per-document counters so each viewer's Adjust / Layers panels get unique ids
// for their toggle's `aria-controls` (a page can host multiple <image-viewer>s).
let osdAdjustSeq = 0;
let osdLayersSeq = 0;

// One-click blend presets for the OSD multispectral stack, keyed by modality
// LABEL (a modality not named here is turned off; a named-but-missing one is
// skipped). `top:true` raises that layer so its blend composites over the rest.
// The labels are the ones a host puts on its modalities.
function osdLayerPresets() {
  return [
    {
      label: "Colour",
      hint: "Just the calibrated visible-light photograph.",
      spec: { "Accurate color": { on: true, opacity: 1, blend: "source-over" } },
    },
    {
      label: "Reveal undertext",
      hint:
        "Infrared enhancement DIFFERENCED over accurate colour: the shared " +
        "overtext cancels toward black, the erased undertext survives as bright " +
        "residue.",
      spec: {
        "Accurate color": { on: true, opacity: 1, blend: "source-over" },
        "Infrared (RLE01)": {
          on: true,
          opacity: 1,
          blend: "difference",
          top: true,
        },
      },
    },
    {
      label: "Reinforce ink",
      hint:
        "Infrared MULTIPLIED over accurate colour: strokes dark in both layers " +
        "are reinforced; noise dark in only one washes out.",
      spec: {
        "Accurate color": { on: true, opacity: 1, blend: "source-over" },
        "Infrared (RLE01)": {
          on: true,
          opacity: 1,
          blend: "multiply",
          top: true,
        },
      },
    },
    {
      label: "Surface + text",
      hint:
        "Raking light (surface texture) SCREENED over the infrared reading, so " +
        "surviving strokes sit in their physical context.",
      spec: {
        "Infrared (RLE01)": { on: true, opacity: 1, blend: "source-over" },
        "Raking light 55°": {
          on: true,
          opacity: 0.6,
          blend: "screen",
          top: true,
        },
      },
    },
  ];
}

// Parse the structured `layers` attribute:
// [{src,label,opacity,visible}, …], order = z-order (bottom
// first). Tolerant; a malformed/empty value yields [].
function parseLayers(raw) {
  if (!raw) return [];
  let arr = raw;
  if (!Array.isArray(arr)) {
    try {
      arr = JSON.parse(raw);
    } catch {
      return [];
    }
  }
  if (!Array.isArray(arr)) return [];
  return arr
    .filter((m) => m && m.src)
    .map((m) => ({
      src: m.src,
      label: m.label || m.src,
      opacity: typeof m.opacity === "number" ? m.opacity : 1,
      visible: m.visible !== false,
    }));
}

class ImageViewer extends HTMLElement {
  static get observedAttributes() {
    return [
      "engine",
      "tile-sources",
      "auth-mode",
      "navigator",
      "sequence",
      "adjust",
      "layers",
      "label",
    ];
  }

  constructor() {
    super();
    this._viewer = null;
    this._filter = { ...ADJUST_DEFAULTS };
    // The shadow root is NOT attached here: `_init` attaches it lazily
    // (`_ensureOsdShadow`). Until then the element has no shadow root and renders
    // its light-DOM children (the fallback) as-is.
    this._root = null;
    // Overlay layer: the host's last `setOverlayState`, kept across re-inits,
    // and the current open's shadow clones ({ svg, shapes, tags } or null).
    this._overlayState = { visible: true, focus: null };
    this._overlay = null;
    this._hoverId = null; // the last `viewer-overlay-hover` id sent
  }

  // Lazily attach the OSD shadow root + template on first init.
  // Idempotent: subsequent _init calls reuse the existing shadow tree.
  _ensureOsdShadow() {
    if (this._root) return;
    this._root = this.attachShadow({ mode: "open" });
    this._root.innerHTML = `
      <style>
        :host { display: block; position: relative; min-height: 240px; background: #111; }
        .stage { position: absolute; inset: 0; }
        .stage[hidden] { display: none; }
        ::slotted(img) { width: 100%; height: 100%; object-fit: contain; }
        .error {
          position: absolute; inset: 0; display: flex;
          align-items: center; justify-content: center; text-align: center;
          padding: 1rem; box-sizing: border-box;
          font: 13px/1.4 system-ui, sans-serif; color: #f4b4b4; background: #1a0e0e;
        }
        .error[hidden] { display: none; }
        /* Adjust TOGGLE — bottom-left corner; reveals the (collapsed-by-default)
           slider panel, which stacks directly above it. Clear of the overview
           navigator (top-right) and the zoom/nav controls (top-left). */
        .adjust-toggle {
          position: absolute; bottom: .5rem; left: .5rem; z-index: 3;
          font: 12px/1 system-ui, sans-serif; color: #fff;
          background: rgba(20,20,22,.82); backdrop-filter: blur(3px);
          border: 1px solid rgba(255,255,255,.6); border-radius: 6px;
          padding: .35rem .6rem; min-height: 28px; cursor: pointer;
        }
        .adjust-toggle[hidden] { display: none; }
        .adjust-toggle::after { content: " ▸"; }
        .adjust-toggle[aria-expanded=true]::after { content: " ▾"; }
        /* Panel — stacked ABOVE its toggle (~28px + .5rem inset). */
        .adjust {
          position: absolute; bottom: calc(.5rem + 34px); left: .5rem; z-index: 2;
          display: grid; grid-template-columns: auto 1fr; gap: .25rem .5rem;
          align-items: center; padding: .6rem .7rem;
          font: 12px/1.3 system-ui, sans-serif; color: #fff;
          background: rgba(20,20,22,.82); border-radius: 6px;
          backdrop-filter: blur(3px); max-width: 15rem;
        }
        .adjust[hidden] { display: none; }
        /* Over-zoom badge — bottom-center, shown only past native 1:1. */
        .overzoom {
          position: absolute; bottom: .5rem; left: 50%; transform: translateX(-50%);
          z-index: 3; font: 11px/1 system-ui, sans-serif; color: #ffe08a;
          background: rgba(20,20,22,.82); backdrop-filter: blur(3px);
          border: 1px solid rgba(255,224,138,.5); border-radius: 6px;
          padding: .3rem .55rem; pointer-events: none; white-space: nowrap;
        }
        .overzoom[hidden] { display: none; }
        .adjust label { white-space: nowrap; }
        .adjust input[type=range] { width: 100%; }
        .adjust__foot { grid-column: 1 / -1; display: flex; justify-content: space-between; align-items: center; margin-top: .15rem; }
        .adjust button { font: inherit; color: #fff; background: transparent; border: 1px solid rgba(255,255,255,.6); border-radius: 4px; padding: .1rem .5rem; min-height: 24px; cursor: pointer; }
        .adjust__invert { display: flex; align-items: center; gap: .3rem; min-height: 24px; }
        .adjust input[type=checkbox] { width: 16px; height: 16px; }
        /* Layers (multispectral) — bottom-RIGHT corner + panel above it, mirroring
           the Adjust affordance. Used only by a layers stack. */
        .layers-toggle {
          position: absolute; bottom: .5rem; right: .5rem; z-index: 3;
          font: 12px/1 system-ui, sans-serif; color: #fff;
          background: rgba(20,20,22,.82); backdrop-filter: blur(3px);
          border: 1px solid rgba(255,255,255,.6); border-radius: 6px;
          padding: .35rem .6rem; min-height: 28px; cursor: pointer;
        }
        .layers-toggle[hidden] { display: none; }
        .layers-toggle::after { content: " ▸"; }
        .layers-toggle[aria-expanded=true]::after { content: " ▾"; }
        .layers {
          position: absolute; bottom: calc(.5rem + 34px); right: .5rem; z-index: 2;
          padding: .5rem .6rem; max-width: 19rem;
          font: 12px/1.3 system-ui, sans-serif; color: #fff;
          background: rgba(20,20,22,.82); border-radius: 6px; backdrop-filter: blur(3px);
        }
        .layers[hidden] { display: none; }
        .layers__presets { display: flex; flex-wrap: wrap; gap: .25rem; margin-bottom: .5rem; padding-bottom: .5rem; border-bottom: 1px solid rgba(255,255,255,.2); }
        .layers__presets button { font: 11px/1 system-ui, sans-serif; color: #fff; background: rgba(255,255,255,.08); border: 1px solid rgba(255,255,255,.5); border-radius: 4px; padding: .25rem .45rem; cursor: pointer; }
        .layers__presets button:hover { background: rgba(255,255,255,.18); }
        .layers__rows { display: flex; flex-direction: column; gap: .3rem; }
        .layers__row { display: flex; align-items: center; gap: .35rem; min-height: 24px; }
        .layers__row.is-failed { opacity: .5; }
        .layers__name { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 7rem; }
        .layers__opacity { width: 4rem; }
        .layers select { font: inherit; color: #fff; background: rgba(0,0,0,.3); border: 1px solid rgba(255,255,255,.5); border-radius: 4px; }
        .layers__reorder button { font: inherit; color: #fff; background: transparent; border: 1px solid rgba(255,255,255,.6); border-radius: 4px; width: 22px; height: 22px; cursor: pointer; line-height: 1; padding: 0; }
        .layers__reorder button:disabled { opacity: .35; cursor: default; }
        .layers input[type=checkbox] { width: 16px; height: 16px; }
        /* Overlay layer (crate README, "Overlay layer"). The clones live in this
           shadow root, so no light-DOM CSS reaches them: colour comes from each
           shape's and tag's inline --iv-c / --iv-on, the rest from inheritable
           --iv-* tokens a host may set on <image-viewer>. A host that also
           draws the light-DOM copy keeps a twin of these rules (CPE:
           areas/access/cpe/components/style/media.css, ANNOTATIONS block);
           change both. */
        svg[data-iv-overlay] { overflow: visible; pointer-events: none; }
        [data-iv-shape] > * {
          vector-effect: non-scaling-stroke; fill: none;
          transition: stroke-width var(--iv-dur, 240ms);
        }
        [data-iv-part=halo] { stroke: var(--iv-halo, oklch(1 0 0 / .62)); stroke-width: 4px; pointer-events: none; }
        [data-iv-part=line] { stroke: var(--iv-c, #f33); stroke-width: 1.5px; pointer-events: visible; cursor: pointer; }
        [data-iv-shape][data-iv-em] > [data-iv-part=line] { stroke-width: 2.5px; }
        [data-iv-shape][data-iv-em] > [data-iv-part=halo] { stroke-width: 5px; }
        [data-iv-focused] [data-iv-shape]:not([data-iv-em]) { opacity: .85; }
        [data-iv-focused] [data-iv-shape]:not([data-iv-em]) > [data-iv-part=line] { stroke-width: 1px; stroke-dasharray: 4 3; }
        [data-iv-focused] [data-iv-shape]:not([data-iv-em]) > [data-iv-part=halo] { stroke-width: 3.5px; }
        [data-iv-off] [data-iv-shape]:not([data-iv-em]) { display: none; }
        /* OSD positions a tag through a wrapper of its own and writes
           display:block onto the tag on every draw, so the host's zero-JS
           positioning is neutralised here and a hidden tag uses visibility. */
        [data-iv-tag] {
          position: static !important; transform: none !important; margin: 0 0 3px !important;
          box-sizing: border-box; min-width: 18px; height: 18px; padding: 0 5px;
          font: 600 11px/16px var(--iv-mono, ui-monospace, monospace);
          text-align: center; white-space: nowrap; border-radius: 1px;
          border: 1px solid var(--iv-c, #f33); background: var(--iv-c, #f33); color: var(--iv-on, #000);
          box-shadow: 0 0 0 1px var(--iv-halo, oklch(1 0 0 / .62));
          pointer-events: auto; cursor: pointer;
          transition: background-color var(--iv-dur, 240ms), color var(--iv-dur, 240ms);
        }
        [data-iv-tag][data-iv-place=below] { margin: 3px 0 0 !important; }
        [data-iv-tag][data-iv-em] {
          background: var(--iv-ink, oklch(.18 0 0)); color: var(--iv-surface, oklch(1 0 0));
          border-color: var(--iv-ink, oklch(.18 0 0));
        }
        [data-iv-tag][data-iv-off] { visibility: hidden; }
      </style>
      <div class="stage" part="stage"></div>
      <div class="overzoom" part="overzoom" hidden></div>
      <button class="adjust-toggle" part="adjust-toggle" type="button" aria-expanded="false" hidden>Adjust</button>
      <div class="adjust" part="adjust" hidden></div>
      <button class="layers-toggle" part="layers-toggle" type="button" aria-expanded="false" hidden>Layers</button>
      <div class="layers" part="layers" hidden></div>
      <slot part="fallback"></slot>
    `;
    this._stage = this._root.querySelector(".stage");
    this._overzoom = this._root.querySelector(".overzoom");
    this._toggle = this._root.querySelector(".adjust-toggle");
    this._panel = this._root.querySelector(".adjust");
    this._panel.id = `iv-osd-adjust-${osdAdjustSeq++}`;
    this._toggle.setAttribute("aria-controls", this._panel.id);
    this._toggle.addEventListener("click", () => {
      const open = this._panel.hidden;
      this._panel.hidden = !open;
      this._toggle.setAttribute("aria-expanded", String(open));
    });
    this._layersToggle = this._root.querySelector(".layers-toggle");
    this._layersPanel = this._root.querySelector(".layers");
    this._layersPanel.id = `iv-osd-layers-${osdLayersSeq++}`;
    this._layersToggle.setAttribute("aria-controls", this._layersPanel.id);
    this._layersToggle.addEventListener("click", () => {
      const open = this._layersPanel.hidden;
      this._layersPanel.hidden = !open;
      this._layersToggle.setAttribute("aria-expanded", String(open));
    });
    this._slot = this._root.querySelector("slot");
    // Fallback visible until the viewer successfully opens.
    this._stage.hidden = true;
  }

  connectedCallback() {
    this._scheduleInit();
  }

  disconnectedCallback() {
    this._teardown();
  }

  attributeChangedCallback() {
    // Any observed attribute change → rebuild.
    if (this.isConnected) this._scheduleInit();
  }

  // Coalesce the burst of attributeChangedCallbacks that fires during upgrade
  // (one per observed attribute) + connectedCallback into a SINGLE _init on the
  // next microtask. Without this, each attribute would spawn its own OSD
  // instance and they would stack (multiple viewers, unfiltered canvases on
  // top of each other). A later real attribute change coalesces the same way.
  _scheduleInit() {
    if (this._initScheduled) return;
    this._initScheduled = true;
    queueMicrotask(() => {
      this._initScheduled = false;
      if (this.isConnected) this._init();
    });
  }

  _teardown() {
    this._endHover();
    if (this._viewer) {
      this._viewer.clearOverlays();
      this._viewer.destroy();
      this._viewer = null;
    }
    this._overlay = null;
  }

  // Viewer opened → reveal the OSD stage and hide the light-DOM fallback (else
  // the fallback <img> renders behind/around the canvas as a double image). The
  // stage was display:none while OSD initialised, so OSD captured a degenerate
  // (1x1) container; now that it is laid out, tell OSD the real size and re-fit.
  _onOpen(gen) {
    this._stage.hidden = false;
    // Keep the light-DOM fallback visible UNTIL OSD's image is actually drawn
    // (see the rAF poll below), then hide it. The OSD canvas is transparent
    // until it paints and sits ABOVE the fallback (`.stage` is absolute over
    // the slot), so during the open→first-paint window the still-visible
    // fallback shows through — instead of the empty dark canvas flashing
    // between the sharp preview and the first paint (`_forceFirstPaint` can
    // defer that paint by up to 350ms). Once OSD has drawn, the canvas covers
    // the fallback, so hiding it is seamless.
    if (this._error) this._error.hidden = true; // clear a prior error state
    // Hide the fallback once OSD's image is actually drawn, by POLLING
    // `getFullyLoaded()` on a rAF loop — NOT via a `tile-drawn`/`animation-finish`
    // handler, which does not fire for a single-resolution IIIF source on the
    // 2D-canvas drawer (so `viewer-rendered`, which is fired from those
    // handlers, never emits for such a source). The bounded loop gives up after ~4s
    // and simply leaves the fallback shown (the correct graceful state on a
    // load that never completes).
    const hideFallback = () => {
      if (gen === this._gen && this.isConnected) this._slot.hidden = true;
    };
    let paintFrames = 0;
    const hideWhenDrawn = () => {
      if (gen !== this._gen || !this._viewer || !this.isConnected || this._slot.hidden) return;
      const w = this._viewer.world;
      const item = w && w.getItemCount() ? w.getItemAt(0) : null;
      if (item && item.getFullyLoaded && item.getFullyLoaded()) {
        hideFallback();
        return;
      }
      if (++paintFrames > 240) return;
      requestAnimationFrame(hideWhenDrawn);
    };
    requestAnimationFrame(hideWhenDrawn);
    // Backstop: `getFullyLoaded()` (and every OSD paint event) can stay false
    // forever for a single-resolution IIIF source on the 2D-canvas drawer, so
    // the rAF poll above may never fire here. OSD has certainly painted well
    // within 700ms (info.json + one sized-derivative fetch + decode), and the
    // opaque canvas covers the fallback, so hiding it then is invisible — it
    // just ends the overlap. Where the poll DOES work it hides earlier.
    setTimeout(hideFallback, 700);
    const r = this._stage.getBoundingClientRect();
    if (r.width > 0 && r.height > 0) {
      this._viewer.viewport.resize(new OpenSeadragon.Point(r.width, r.height), false);
      this._viewer.viewport.goHome(true);
    }
    // The initial `page` attr or a queued goToPage() call goes first: when it
    // turns the page, this open is not the page that stays, and the overlay
    // is adopted by the `open` of the page it turns to.
    const turning = this._tryApplyPendingPage();
    if (!turning) this._adoptOverlay(gen);
    this._applyFilter(); // re-assert filter (the canvas is (re)created on open)
    this._forceFirstPaint(gen); // timed from `open`, not viewer creation
    if (this._stack) this._seedStack(gen); // add the remaining modalities
    this._emitZoom();
  }

  // Adopt the host's light-DOM overlay (crate README, "Overlay layer") as OSD
  // overlays. A fresh clone every `open`: OSD's `close()`, run by every open
  // including a sequence `goToPage`, clears all overlays, and `addOverlay`
  // skips an element it still tracks. Elements, never ids: OSD resolves an id
  // with `document.getElementById`, which cannot see into this shadow root.
  _adoptOverlay(gen) {
    if (gen !== this._gen) return;
    this._endHover();
    this._overlay = null;
    const v = this._viewer;
    const item = v && v.world.getItemCount() ? v.world.getItemAt(0) : null;
    const src = this.querySelector("svg[data-iv-overlay]");
    if (!item || !src) return;
    const bounds = item.getBounds();
    const svg = sanitizeOverlayClone(src.cloneNode(true));
    v.addOverlay({ element: svg, location: bounds });
    const hover = (el, id) => {
      el.addEventListener("pointerenter", () => this._emitOverlay("viewer-overlay-hover", id));
      el.addEventListener("pointerleave", () => this._emitOverlay("viewer-overlay-hover", null));
    };
    const shapes = new Map();
    for (const g of svg.querySelectorAll("g[data-iv-shape]")) {
      shapes.set(g.dataset.ivShape, g);
      hover(g, g.dataset.ivShape);
    }
    const tags = new Map();
    for (const t of this.querySelectorAll("[data-iv-tag]")) {
      const x = Number(t.dataset.ivX);
      const y = Number(t.dataset.ivY);
      if (src.contains(t) || !Number.isFinite(x) || !Number.isFinite(y)) continue;
      const tag = sanitizeOverlayClone(t.cloneNode(true));
      v.addOverlay({
        element: tag,
        location: new OpenSeadragon.Point(
          bounds.x + (x / 100) * bounds.width,
          bounds.y + (y / 100) * bounds.height,
        ),
        placement: tagPlacement(tag),
        checkResize: false,
        rotationMode: OpenSeadragon.OverlayRotationMode.NO_ROTATION,
      });
      const id = tag.dataset.ivTag;
      tags.set(id, [...(tags.get(id) || []), tag]);
      hover(tag, id);
    }
    this._overlay = { svg, shapes, tags };
    this._applyOverlayState();
  }

  // Reflect `_overlayState` onto the current clones. The `data-iv-off`,
  // `data-iv-focused` and `data-iv-em` attributes are this method's alone.
  _applyOverlayState() {
    const o = this._overlay;
    if (!o) return;
    const { visible } = this._overlayState;
    let { focus } = this._overlayState;
    // An id this open does not draw (a stale pin after a page turn) is no focus.
    if (!o.shapes.has(focus) && !o.tags.has(focus)) focus = null;
    o.svg.toggleAttribute("data-iv-off", !visible);
    o.svg.toggleAttribute("data-iv-focused", focus !== null);
    for (const [id, g] of o.shapes) g.toggleAttribute("data-iv-em", id === focus);
    for (const [id, list] of o.tags) {
      for (const t of list) {
        t.toggleAttribute("data-iv-em", id === focus);
        t.toggleAttribute("data-iv-off", !visible && id !== focus);
      }
    }
  }

  // Public JS control surface (host-rendered chrome)
  // See the file-header doc comment "Public JS control surface" for the
  // contract. All methods below are safe to call before the viewer has opened.

  // Overlay state, replaced whole: `visible: false` draws only `focus`; a
  // `focus` id is drawn emphasised with its siblings dashed. A missing
  // `visible` means true, an empty or missing `focus` means none. Stored, so a
  // call before `open` (or before a re-init settles) applies once it opens.
  setOverlayState(state) {
    const focus = typeof state?.focus === "string" && state.focus !== "" ? state.focus : null;
    this._overlayState = { visible: state?.visible !== false, focus };
    this._applyOverlayState();
  }

  // Animate onto shape `id`: its box padded to `size / fraction` about its
  // centre, then held between fit and `maxZoom` × fit. That cap is relative to
  // fit on purpose; OSD's own `maxZoomPixelRatio` is relative to native pixels.
  // A no-op before open and for an id the current image does not draw.
  zoomToShape(id, opts) {
    const v = this._viewer;
    const item = v && v.world.getItemCount() ? v.world.getItemAt(0) : null;
    const g = this._overlay && this._overlay.shapes.get(id);
    const box = g && shapeBox(g);
    if (!item || !box) return;
    const { fraction = 0.6, maxZoom = 4 } = opts || {};
    const f = Number.isFinite(fraction) && fraction > 0 && fraction <= 1 ? fraction : 0.6;
    const cap = Number.isFinite(maxZoom) && maxZoom >= 1 ? maxZoom : 4;
    const b = item.getBounds();
    const w = Math.max(((box.w / 100) * b.width) / f, 1e-9);
    const h = Math.max(((box.h / 100) * b.height) / f, 1e-9);
    const centre = new OpenSeadragon.Point(
      b.x + ((box.x + box.w / 2) / 100) * b.width,
      b.y + ((box.y + box.h / 2) / 100) * b.height,
    );
    const vp = v.viewport;
    vp.fitBoundsWithConstraints(new OpenSeadragon.Rect(centre.x - w / 2, centre.y - h / 2, w, h));
    const home = vp.getHomeZoom();
    const z = vp.getZoom(false);
    const held = Math.min(home * cap, Math.max(home, z));
    if (held !== z) {
      // No refPoint: OSD would hold that point at its current screen
      // position instead of centring it.
      vp.zoomTo(held);
      vp.panTo(centre);
      vp.applyConstraints();
    }
  }

  _emitOverlay(type, id) {
    if (type === "viewer-overlay-hover") this._hoverId = id;
    this.dispatchEvent(new CustomEvent(type, { bubbles: true, detail: { id } }));
  }

  // A removed clone never fires `pointerleave`, so a hover still open when
  // the overlay goes away is closed here. From `disconnectedCallback` the
  // element is already out of the document, so that event reaches listeners
  // on the element only, never its ancestors: a host that removes the viewer
  // clears its own hover state.
  _endHover() {
    if (this._hoverId != null) this._emitOverlay("viewer-overlay-hover", null);
  }

  zoomIn() {
    if (this._viewer) this._viewer.viewport.zoomBy(1.5);
  }

  zoomOut() {
    if (!this._viewer) return;
    // Clamp at the zoom-out floor (`MIN_HOME_ZOOM_RATIO` × fit). OSD's `zoomBy`
    // does NOT apply the `minZoomImageRatio` constraint the way the wheel/
    // gesture path does, so an unclamped `zoomBy(1/1.5)` let the button shrink
    // the image far below the floor (to ~0.3× fit) and, spring-fighting the
    // constraint, produced an ugly twitch and a scale readout stuck on `FIT`.
    // Clamping the target here (with `minZoomImageRatio` covering the wheel/
    // gesture path) makes the floor consistent for every zoom-out affordance.
    const vp = this._viewer.viewport;
    const floor = vp.getHomeZoom() * MIN_HOME_ZOOM_RATIO;
    vp.zoomTo(Math.max(floor, vp.getZoom() / 1.5));
  }

  goHome() {
    if (this._viewer) this._viewer.viewport.goHome();
  }

  // Page a multi-source sequence to the 1-based page `n`, via OSD's OWN
  // sequenceMode (`viewer.goToPage`, 0-based) — never by mutating
  // `tile-sources` (an OBSERVED attribute; mutating it tears the viewer down).
  // Queues (`_pendingPage`) until OSD's `open` fires (or the next one, on a
  // re-init) if the viewer isn't open yet.
  goToPage(n) {
    this._pendingPage = n;
    this._tryApplyPendingPage();
  }

  // Applies `_pendingPage` (set by `goToPage()` or the initial non-observed
  // `page` attribute — see `_initOsd`) once the viewer has actually opened.
  // A single-image viewer (sequenceMode off) has no page to go to; the pending
  // value is dropped rather than left to retry forever, as is the page already
  // shown. Returns whether it turned the page (a new `open` follows).
  _tryApplyPendingPage() {
    if (this._pendingPage == null || !this._viewer || !this._viewer.isOpen()) {
      return false;
    }
    const page = Math.max(0, this._pendingPage - 1);
    this._pendingPage = null;
    if (!this._viewer.sequenceMode || page === this._viewer.currentPage()) return false;
    this._viewer.goToPage(page);
    return true;
  }

  // Shared non-finite-safety for the six numeric adjust operators: `clamp()`
  // itself falls back to `min` on a non-finite input, which is fine for an
  // operator whose destructive extreme genuinely IS `min` in isolation, but
  // every one of these six has a semantically NEUTRAL default in the middle
  // of its range (100/100/100/0/0/1) — silently clamping a bad/transient
  // value (e.g. `undefined` from an uninitialized signal) to `min` can mean a
  // literally black or maximally-distorted canvas instead of a harmless no-op.
  // So every numeric setter falls back to ITS OWN default via this shared
  // helper, not to `clamp()`'s raw `min`.
  _setNumericAdjust(key, v, min, max) {
    const n = Number(v);
    this._setAdjust(key, Number.isFinite(n) ? clamp(n, min, max) : ADJUST_DEFAULTS[key]);
  }

  setBrightness(v) {
    this._setNumericAdjust("brightness", v, 0, 200);
  }

  setContrast(v) {
    this._setNumericAdjust("contrast", v, 0, 200);
  }

  setSaturation(v) {
    this._setNumericAdjust("saturate", v, 0, 200);
  }

  setHue(v) {
    this._setNumericAdjust("hue", v, -180, 180);
  }

  setInvert(on) {
    this._setAdjust("invert", on ? 100 : 0);
  }

  // APPROXIMATION — no native CSS filter for exposure. See `_applyFilter`.
  setExposure(v) {
    this._setNumericAdjust("exposure", v, -1, 1);
  }

  // APPROXIMATION — no native CSS filter for gamma. See `_applyFilter`.
  setGamma(v) {
    this._setNumericAdjust("gamma", v, 0.5, 2);
  }

  resetAdjust() {
    this._filter = { ...ADJUST_DEFAULTS };
    this._applyFilter();
    this._syncAdjustControls();
  }

  // Shared setter: update filter state, re-apply the CSS filter, and keep this
  // crate's OWN built-in Adjust panel in sync IF it happens to be built (a host
  // driving these methods normally has `adjust(false)` — the panel is never
  // built — but a consumer combining both must not see them desync).
  _setAdjust(key, value) {
    this._filter[key] = value;
    this._applyFilter();
    this._syncAdjustControls();
  }

  _syncAdjustControls() {
    const c = this._adjustControls;
    if (!c) return;
    for (const key of ["brightness", "contrast", "saturate", "hue", "exposure", "gamma"]) {
      if (c[key]) c[key].value = String(this._filter[key]);
    }
    if (c.invert) c.invert.checked = this._filter.invert !== 0;
  }

  // Compute + emit the `viewer-zoom` CustomEvent (see the file-header doc
  // comment). Fired on every OSD zoom change and once on `open`.
  _emitZoom() {
    const ratio = this._zoomRatio();
    const atNative = ratio !== null && ratio > 0.95 && ratio < 1.05;
    const vp = this._viewer && this._viewer.viewport;
    const homeZoom = !!vp && vp.getHomeZoom();
    const maxZoom = vp ? vp.getMaxZoom() : 0;
    // Read `getZoom(false)`, the committed/target zoom, which tracks the zoom
    // level. `getZoom(true)` (the spring "current") stays pinned at the HOME
    // value in this shadow-DOM + canvas-drawer setup while the image zooms;
    // reading it freezes the host readout on `FIT` and mis-fires every
    // disabled-state edge below.
    const currentZoom = vp ? vp.getZoom(false) : null;
    const minZoom = homeZoom * MIN_HOME_ZOOM_RATIO;
    // Three distinct edges the host chrome disables its controls on:
    //   atHome — at fit (disables the fit/home reset button)
    //   atMin  — at the zoom-out floor (disables zoom-out)
    //   atMax  — at max magnification (disables zoom-in)
    // `atHome`'s band is RELATIVE, 2% of `getHomeZoom()`, never an absolute
    // epsilon: `_onOpen`'s resize()+goHome(true) pair can settle the viewport
    // up to ~0.5% off `getHomeZoom()`, and with an absolute `1e-6` a
    // host-rendered scale readout then shows a ratio (e.g. `×0.23`) instead of
    // `FIT` on first open. 2% is above that settling noise and far below the
    // smallest real zoom step (`zoomIn()`/`zoomOut()` are ×1.5/÷1.5), so a
    // real zoom-in is never misread as home; scaling by the home zoom keeps
    // the check meaningful across the range of home-zoom magnitudes. The band
    // is tight so it names ONLY fit, not the whole fit→floor range (a zoom-out
    // step below fit must still read as "not home" so the reset button
    // re-enables). `atNative` (a 5% band around screen 1:1) stays a
    // readout-only marker, NOT the zoom-in disable edge — magnifying past
    // native to `maxZoomPixelRatio` is allowed.
    const atHome = !!vp && Math.abs(currentZoom - homeZoom) < Math.max(homeZoom * 0.02, 1e-6);
    const atMin = !!vp && !!homeZoom && currentZoom <= minZoom * 1.01;
    const atMax = !!vp && maxZoom > 0 && currentZoom >= maxZoom * 0.99;
    // `ratioToHome`: home-relative scale (1 at FIT), independent of
    // `zoomPercent`'s native-resolution-relative scale — see the file-header
    // doc comment. `homeZoom` is `0` (falsy) before an image has opened, same
    // guard `atHome` already relies on.
    const ratioToHome = !vp || !homeZoom ? null : currentZoom / homeZoom;
    this.dispatchEvent(
      new CustomEvent("viewer-zoom", {
        bubbles: true,
        detail: {
          zoomPercent: ratio === null ? null : Math.round(ratio * 100),
          atHome,
          atNative,
          atMin,
          atMax,
          ratioToHome,
        },
      }),
    );
  }

  // A seed modality failed to open. Retry with the next modality; only when
  // EVERY modality has failed do we surface the error state. The current seed is
  // recorded as failed so the Layers panel greys it.
  _onStackSeedFailed(gen) {
    if (gen !== this._gen || !this.isConnected) return;
    const failed = this._stack[this._seedIndex];
    this._failedLayers.push(failed);
    this._emitWarning(`layer "${failed.label}" failed to load`);
    const nextSeed = this._seedIndex + 1;
    if (nextSeed >= this._stack.length) {
      // All modalities failed → the standard error state (the one fatal case).
      this._fail(new Error("image-viewer: no layer could be loaded"));
      return;
    }
    // Re-init OSD seeded on the NEXT modality, carrying the retry progress so
    // _initOsd resumes from there (it reads seedIndex synchronously to pick the
    // seed tile source).
    this._initOsd(
      this._stack.map((m) => m.src),
      this._stack,
      nextSeed,
      this._failedLayers,
    );
  }

  // Stack the remaining modalities over the seed via addTiledImage, then build
  // the Layers panel. Each item is co-located (same viewport coords) so they
  // overlay pixel-aligned. A per-item load failure is non-fatal (greyed row).
  _seedStack(gen) {
    if (gen !== this._gen || !this._viewer) return;
    const stack = this._stack;
    const seedIdx = this._seedIndex;
    // The seed (index seedIdx) is already OSD's world item 0. Map each modality
    // to its world item as items are added, in stack order.
    this._layerItems = new Array(stack.length).fill(null);
    this._layerItems[seedIdx] = this._viewer.world.getItemAt(0) || null;

    // Add every modality EXCEPT the seed and any already-failed ones (from a
    // seed-retry). Each addTiledImage carries its own error callback (non-fatal),
    // so no separate add-item-failed handler is needed.
    //
    // Do NOT pass a fixed `index` here: additions resolve out of order and the
    // world has fewer items than the stack when any modality is skipped/failed,
    // so a stack-index passed as a world-index gaps or misorders. Instead append
    // each item, then, once ALL have settled, deterministically restack the
    // world in stack order (bottom first) over the survivors.
    const failedSet = new Set(this._failedLayers);
    let pending = 0;
    const settle = () => {
      if (pending === 0) {
        this._restackWorld(gen);
        this._buildLayersPanel(gen);
      }
    };
    stack.forEach((m, i) => {
      if (i === seedIdx || failedSet.has(m)) return;
      pending += 1;
      this._viewer.addTiledImage({
        tileSource: m.src,
        opacity: m.visible ? m.opacity : 0,
        success: (evt) => {
          if (gen !== this._gen) return;
          this._layerItems[i] = evt.item;
          pending -= 1;
          settle();
        },
        error: () => {
          if (gen !== this._gen) return;
          this._failedLayers.push(m);
          this._emitWarning(`layer "${m.label}" failed to load`);
          pending -= 1;
          settle();
        },
      });
    });
    if (pending === 0) settle(); // single-modality stack (nothing to add)
  }

  // Set the world z-order deterministically from stack order (bottom = stack
  // index 0), skipping modalities that never produced an item. Called once all
  // addTiledImage calls have settled, so item count no longer changes under us.
  _restackWorld(gen) {
    if (gen !== this._gen || !this._viewer) return;
    const world = this._viewer.world;
    let target = 0;
    this._stack.forEach((_m, i) => {
      const item = this._layerItems[i];
      if (item && world.getIndexOfItem(item) !== -1) {
        world.setItemIndex(item, target);
        target += 1;
      }
    });
  }

  // Show the over-zoom badge when magnified past native 1:1. The ratio is
  // "screen pixels per source pixel": at native it is 1; at 2× the source pixels
  // are drawn 2× larger. Derived from OSD's zoom (viewport units per source
  // pixel via the tiled image) times the container width in source-pixel units.
  _updateOverzoom() {
    if (!this._overzoom || !this._viewer) return;
    const ratio = this._zoomRatio();
    if (ratio === null) {
      this._overzoom.hidden = true;
      return;
    }
    this._setOverzoom(ratio);
  }

  // Screen pixels per source pixel: 1 at native resolution, >1 past it.
  // Shared by `_updateOverzoom` (the over-zoom badge) and `_emitZoom` (the
  // `viewer-zoom` event's `zoomPercent`/`atNative`). Returns null before an
  // image has opened (no world item to measure against yet).
  //
  // imageToViewportZoom(1) = viewport units spanned by 1 source pixel; the
  // container spans `1 / getZoom()` viewport units across its pixel width. So
  // screen-px per source-px = getZoom() * containerPx / imageWidthPx.
  _zoomRatio() {
    const v = this._viewer;
    if (!v) return null;
    const item = v.world && v.world.getItemCount() ? v.world.getItemAt(0) : null;
    if (!item) return null;
    const containerPx = v.viewport.getContainerSize().x;
    const imageW = item.getContentSize().x;
    // getZoom(false) — the committed zoom that tracks reality; getZoom(true)
    // is pinned at home here (see `_emitZoom`).
    return (v.viewport.getZoom(false) * containerPx) / imageW;
  }

  // Shared badge renderer. ratio = screen-px per source-px.
  _setOverzoom(ratio) {
    const el = this._overzoom;
    if (!el) return;
    if (ratio > 1.05) {
      el.textContent = `Over-zoom ${ratio.toFixed(1)}× · beyond native resolution`;
      el.hidden = false;
    } else {
      el.hidden = true;
    }
  }

  _emitWarning(message) {
    this.dispatchEvent(
      new CustomEvent("viewer-warning", { bubbles: true, detail: message }),
    );
  }

  // `viewer-rendered`: a bubbling event for hosts, fired from a REAL engine
  // render-complete (OSD `tile-drawn` / `animation-finish`). Its detail
  // carries only the engine slug and a `performance.now()` timestamp, not
  // what caused the render: a listener that needs to tell a first paint from
  // a pan, zoom or page change does so by its own timing. It is a signal
  // only and changes no rendering behavior. NOT tied to `_forceFirstPaint`'s
  // timer, which is a paint workaround, not a completion signal.
  _emitRendered(engine) {
    this.dispatchEvent(
      new CustomEvent("viewer-rendered", {
        bubbles: true,
        detail: { engine, t: performance.now() },
      }),
    );
  }

  // Work around OSD's incomplete shadow-DOM rendering (see the crate README):
  // inside a shadow root OSD does not paint the home view until the viewport
  // actually *changes*, so on load the viewer stays blank until the user
  // zooms/pans. An imperceptible (1e-7) pan forces the paint. It must be
  // deferred: applied synchronously in `open`/`tile-loaded`
  // the delta is coalesced away before OSD's drawer is ready. Fire it at two
  // points after `open` so a slower tile-load is still covered without depending
  // on a single exact moment. Remove this (and the `drawer:"canvas"` override)
  // if a future OSD release fixes its shadow-DOM handling.
  _forceFirstPaint(gen) {
    const nudge = () => {
      if (gen !== this._gen || !this._viewer || !this.isConnected) return;
      this._viewer.viewport.panBy(new OpenSeadragon.Point(1e-7, 0));
      this._viewer.viewport.applyConstraints(true);
    };
    setTimeout(nudge, 350);
    setTimeout(nudge, 1200);
  }

  async _init() {
    // Multispectral / layered path: a `layers` attribute carries N modalities
    // and NO `tile-sources`. It is a STACK, not a paged sequence, so it is
    // initialised differently below.
    const layers = parseLayers(this.getAttribute("layers"));
    const sources = parseSources(this.getAttribute("tile-sources"));
    if (sources.length === 0 && layers.length === 0) return; // nothing to show yet

    // Attach the OSD shadow root on first use (lazily, not in the constructor).
    this._ensureOsdShadow();
    // A layers stack uses the modality URLs as OSD tile sources but stacks them
    // (all at once, sequenceMode off) rather than paging — see _initOsd.
    if (layers.length > 0) {
      return this._initOsd(
        layers.map((m) => m.src),
        layers,
      );
    }
    return this._initOsd(sources);
  }

  async _initOsd(sources, layers, seedIndex = 0, failedSoFar = []) {
    // Generation guard: if another _init starts while we await OSD, ours is
    // stale — abort before creating a second viewer.
    const gen = (this._gen = (this._gen || 0) + 1);
    this._teardown();
    // A multispectral STACK (layers present) is NOT a paged sequence. Remember
    // it so `open` seeds the extra layers and the Layers panel is built.
    // `_seedIndex` = which modality is currently seeding OSD's first image
    // (advances on a seed failure); `_failedLayers` collects modalities that
    // failed to load → greyed panel rows. Both are passed in on a seed-failure
    // retry so the re-init resumes from the next modality rather than restarting.
    this._stack = layers && layers.length ? layers : null;
    this._seedIndex = seedIndex;
    this._failedLayers = failedSoFar;

    let OpenSeadragon;
    try {
      OpenSeadragon = await loadOpenSeadragon();
    } catch (err) {
      if (gen === this._gen) this._fail(err);
      return;
    }
    // Superseded by a newer _init, or disconnected while the script loaded.
    if (gen !== this._gen || !this.isConnected) return;

    const authCookie = this.getAttribute("auth-mode") === "cookie";
    // A stack is never a paged sequence (mutually exclusive). Otherwise the
    // rule is: explicit sequence="true", or auto for >1 source.
    const sequence =
      !this._stack &&
      (this.getAttribute("sequence") === "true" || sources.length > 1);

    // Stock-chrome opt-out (Viewer::chrome(false)): a host driving its own
    // paging/zoom chrome (the public methods above) sets `chrome="false"` so
    // OSD never draws a second, out-of-band pager/rotator alongside it. Default
    // true.
    const chromeOn = this.getAttribute("chrome") !== "false";

    // Initial page (Viewer::page(n), 1-based) — read ONCE here, not reactively:
    // `page` is deliberately absent from observedAttributes (see the
    // file-header doc comment), so this only ever runs at a real (re-)init, not
    // on every attribute patch. Don't clobber a `goToPage()` call already
    // queued (e.g. issued between connect and this init settling).
    if (this._pendingPage == null) {
      const initialPage = Number(this.getAttribute("page"));
      if (Number.isFinite(initialPage) && initialPage >= 1) {
        this._pendingPage = initialPage;
      }
    }

    // Stack: seed with the modality at `_seedIndex` (0 on first try; a seed
    // failure re-inits with the next one). The rest are added over it via
    // addTiledImage after `open` (see _onOpen → _seedStack). Non-stack: hand OSD
    // the whole source array as usual.
    const tileSources = this._stack
      ? [this._stack[this._seedIndex].src]
      : sources;

    const options = {
      element: this._stage,
      prefixUrl: PREFIX_URL,
      // Use the 2D-canvas drawer, not OSD 6's default WebGL drawer: the WebGL
      // drawer renders a blank canvas here ("Error creating texture in WebGL"
      // in Chrome) — OSD's WebGL path does not cope with this
      // shadow-DOM + IIIF setup. The canvas drawer renders reliably, and it is
      // also the drawer whose `globalCompositeOperation` backs per-layer blend
      // modes (setCompositeOperation) for the stack.
      drawer: "canvas",
      tileSources,
      sequenceMode: sequence,
      showNavigator: this.hasAttribute("navigator"),
      // Stock chrome — navigation (prev/next), sequence, and rotation controls
      // — default ON; `chrome="false"` (Viewer::chrome(false))
      // turns off all three for a host rendering its own. `showNavigator`
      // (the overview-thumbnail navigator) is a SEPARATE, opt-IN affordance
      // (`Viewer::navigator`) and is untouched by this opt-out.
      showNavigationControl: chromeOn,
      showSequenceControl: chromeOn,
      showRotationControl: chromeOn,
      gestureSettingsTouch: { pinchRotate: false },
      // Over-zoom past native 1:1 (enlarged pixels) — editors keep magnifying a
      // detail even once the tiles stop resolving finer. OSD's default is ~1.1
      // (barely any overstep); 4 lets you zoom to 4× native.
      maxZoomPixelRatio: 4,
      // Zoom-out floor for the wheel/gesture path, at `MIN_HOME_ZOOM_RATIO` ×
      // fit (fit itself — see the constant's note on why not below fit).
      // `zoomOut()` clamps the button path to the same floor.
      minZoomImageRatio: MIN_HOME_ZOOM_RATIO,
      // Request tiles as CORS (`crossorigin="anonymous"` on the tile <img>) so a
      // Service Worker can cache them as inspectable, non-opaque responses (an
      // opaque response cannot be served from Cache Storage byte for byte, nor
      // a miss detected). Applies to the ANONYMOUS <img> tile path only; when
      // `loadTilesWithAjax` is on (the cookie path below) OSD loads via XHR and
      // this is inert — so it is safe to set unconditionally and does not touch
      // the credentialed path.
      crossOriginPolicy: "Anonymous",
      // Credentialed requests for cookie-gated IIIF (e.g. SIPI). Applies to
      // the info.json fetch and every tile request. See README "Auth".
      loadTilesWithAjax: authCookie,
      ajaxWithCredentials: authCookie,
    };

    this._viewer = OpenSeadragon(options);
    this._viewer.addHandler("open", () => this._onOpen(gen));
    // Stack: a failed SEED modality must not be fatal — try the next one; only
    // when every modality has failed do we surface the error. For a non-stack
    // an open failure is fatal.
    this._viewer.addHandler("open-failed", (e) => {
      if (this._stack) this._onStackSeedFailed(gen);
      else this._fail(e);
    });

    // `viewer-rendered` (see `_emitRendered`). `tile-drawn` marks real
    // pixels reaching the canvas (first-paint); `animation-finish` marks a
    // pan, zoom or page change settling. Observation only — no render change.
    this._viewer.addHandler("tile-drawn", () => this._emitRendered("osd-wc"));
    this._viewer.addHandler("animation-finish", () =>
      this._emitRendered("osd-wc"),
    );

    // Over-zoom badge: on every zoom, show how far past native 1:1 we are (image
    // pixels per screen pixel). Editors need to know the pixels are enlarged, not
    // resolving finer. Hidden at/below native.
    this._viewer.addHandler("zoom", () => this._updateOverzoom());
    this._viewer.addHandler("open", () => this._updateOverzoom());
    // `viewer-zoom` (host-rendered scale readout) — every zoom change, plus the
    // one `_onOpen` fires above.
    this._viewer.addHandler("zoom", () => this._emitZoom());

    // Overlay clicks come through OSD's own quick-click event, never a native
    // `click` on the shapes: OSD captures the pointer on its canvas, so a
    // native click may retarget away from them, while `originalTarget` is the
    // pointerdown target. Only a hit cancels OSD's default click-to-zoom.
    this._viewer.addHandler("canvas-click", (e) => {
      if (!e.quick) return;
      const hit = e.originalTarget?.closest?.("[data-iv-shape],[data-iv-tag]");
      if (!hit) return;
      e.preventDefaultAction = true;
      this._emitOverlay("viewer-overlay-click", hit.dataset.ivShape ?? hit.dataset.ivTag);
    });

    // A bare aria-label on a role-less <div> is ignored by screen readers, so
    // give the stage an explicit role. "group" names the viewer region without
    // trapping all keystrokes the way role="application" would.
    this._stage.setAttribute("role", "group");
    const label = this.getAttribute("label");
    if (label) this._stage.setAttribute("aria-label", label);

    // When `adjust` is set — OR this is a multispectral stack, where the global
    // CSS filter over the whole composited canvas is the deliberate choice
    // (one filter for the whole composite, no per-layer filter) — expose the Adjust
    // toggle. The panel stays collapsed until opened (filters off by default);
    // sliders are built lazily. Otherwise neither is shown.
    if (this.hasAttribute("adjust") || this._stack) {
      this._buildAdjustPanel();
      this._toggle.hidden = false;
    } else {
      this._toggle.hidden = true;
      this._panel.hidden = true;
    }
  }

  // Image adjustment (CSS filters on the tiled-image canvas)
  // One CSS `filter` string is composed and set on the
  // drawing surface `_filterTarget` picks. GPU-composited, no tile
  // reprocessing. Re-asserted on `open` (the canvas is recreated).

  _filterTarget() {
    // The drawer's own <canvas>, never `viewer.canvas`: that container also
    // holds the overlays, which must keep their own colour. A drawer that is
    // not the forced canvas one falls back to the container (it still
    // excludes the navigator and the nav buttons).
    const v = this._viewer;
    const d = v && v.drawer;
    if (d && d.canvas && typeof d.getType === "function" && d.getType() === "canvas") {
      return d.canvas;
    }
    return (v && v.canvas) || this._stage.querySelector(".openseadragon-canvas");
  }

  _applyFilter() {
    const t = this._filterTarget();
    if (!t || !t.style) return;
    const f = this._filter;

    // ⚠️ APPROXIMATION — `exposure` and `gamma` have NO native CSS `filter`
    // function. This engine (osd-wc) applies adjust operators as a CSS-filter
    // chain over the OSD canvas, so both are folded into the `brightness()`
    // term below rather than applied as independent per-pixel operators.
    //   - `exposure`: `1 + exposure` is a linear multiplier, the same form as a
    //     per-pixel exposure operator `(exposure + 1.0) * color.rgb`; the
    //     approximation is purely about *compositing* (folded into this
    //     brightness() chain instead of applied standalone).
    //   - `gamma`: this IS a genuine mathematical divergence. A true gamma is a
    //     per-channel power law (`pow(color.rgb, vec3(1.0 / gamma))`); this uses
    //     a plain linear multiplier. Gamma is the more approximate of the two.
    // This is a deliberate tradeoff (the
    // approximation is acceptable because Adjust is transient/never
    // persisted), NOT a bug and NOT something to "fix" by adding shader-like
    // math here. A per-pixel (WebGL) implementation is the way to exact
    // photographic semantics, if they are ever required.
    //   exposure (-1..1, default 0) → linear brightness multiplier 0..2
    //   gamma    (0.5..2, default 1) → linear brightness multiplier 0.5..2
    // Both multiply the base `brightness` percent so the neutral defaults
    // (exposure 0, gamma 1) leave `brightness` completely unchanged.
    const exposureMult = 1 + f.exposure; // -1..1 -> 0..2
    const gammaMult = f.gamma; // 0.5..2, already a multiplier
    const brightnessPct = f.brightness * exposureMult * gammaMult;

    // Order matters: hue-rotate after saturate (rotating a desaturated image does
    // nothing visible, which is the intuitive reading of the two controls), and
    // invert last so it flips the finished result.
    t.style.filter =
      `brightness(${brightnessPct}%) contrast(${f.contrast}%) ` +
      `saturate(${f.saturate}%) hue-rotate(${f.hue}deg) invert(${f.invert}%)`;
  }

  _buildAdjustPanel() {
    // Build the slider DOM only; visibility is owned by the toggle (the panel
    // stays collapsed by default — filters off until the user opens it).
    const p = this._panel;
    if (p.dataset.built) return; // idempotent across re-inits
    p.dataset.built = "1";
    // [state key, label, min, max, step] — the range/step is per-row: `hue` is
    // in degrees (-180..180); `exposure`/`gamma` are fractional (step 0.01,
    // ranges pinned by `tests/render.rs` — see the `ADJUST_DEFAULTS` comment
    // and `_applyFilter`'s APPROXIMATION note); the rest are integer percentages
    // (0–200, step 1, the default when omitted).
    const rows = [
      ["brightness", "Brightness", 0, 200],
      ["contrast", "Contrast", 0, 200],
      ["exposure", "Exposure", -1, 1, 0.01],
      ["saturate", "Saturation", 0, 200],
      ["gamma", "Gamma", 0.5, 2, 0.01],
      ["hue", "Hue", -180, 180],
    ];
    const sliders = {}; // key → input, for a clean Reset (no label round-trip)
    // Exposed on `this` so the public setBrightness()/…/setInvert()/
    // resetAdjust() methods can keep this built-in panel's own controls in
    // sync if a consumer happens to drive both (normally `adjust(false)` means
    // this panel is never built at all — see `_setAdjust`/`_syncAdjustControls`).
    this._adjustControls = sliders;
    for (const [key, text, min, max, step] of rows) {
      const id = `iv-adj-${key}`;
      const label = document.createElement("label");
      label.textContent = text;
      label.htmlFor = id; // real label↔control association (clickable, named)
      const input = document.createElement("input");
      input.id = id;
      input.type = "range";
      input.min = String(min);
      input.max = String(max);
      if (step != null) input.step = String(step);
      input.value = String(this._filter[key]);
      input.addEventListener("input", () => {
        this._filter[key] = Number(input.value);
        this._applyFilter();
      });
      sliders[key] = input;
      p.append(label, input);
    }
    const foot = document.createElement("div");
    foot.className = "adjust__foot";
    const invertWrap = document.createElement("label");
    invertWrap.className = "adjust__invert";
    const invert = document.createElement("input");
    invert.type = "checkbox";
    invert.addEventListener("change", () => {
      this._filter.invert = invert.checked ? 100 : 0;
      this._applyFilter();
    });
    sliders.invert = invert; // so the public setInvert()/resetAdjust() sync it
    invertWrap.append(invert, document.createTextNode("Invert"));
    const reset = document.createElement("button");
    reset.type = "button";
    reset.textContent = "Reset";
    reset.addEventListener("click", () => {
      this._filter = { ...ADJUST_DEFAULTS };
      invert.checked = false;
      for (const [key] of rows) sliders[key].value = String(this._filter[key]);
      this._applyFilter();
    });
    foot.append(invertWrap, reset);
    p.append(foot);
  }

  // Build the multispectral Layers panel (OSD stack). One row per modality with
  // a visibility toggle, opacity slider, blend-mode select (OSD's winning axis:
  // setCompositeOperation → the canvas drawer's globalCompositeOperation), and
  // up/down reorder (world.setItemIndex). Failed modalities render greyed.
  //
  // OSD has no per-item colour filter — the CSS Adjust filter (see _filterTarget)
  // hits the whole composited canvas — so there is NO active-layer concept here;
  // a per-layer filter is not offered.
  _buildLayersPanel(gen) {
    if (gen !== this._gen || !this._viewer) return;
    this._layersToggle.hidden = false;
    const p = this._layersPanel;
    p.textContent = "";

    // Row state: remembered (last non-zero) opacity per modality so re-showing a
    // hidden layer restores its slider rather than reading 0 back from the item;
    // `blend` tracks each modality's current composite operation so the UI (and
    // presets) stay in sync with the item.
    const stack = this._stack;
    const world = this._viewer.world;
    const remembered = stack.map((m) => (m.opacity > 0 ? m.opacity : 1));
    const blend = stack.map(() => "source-over");

    const itemOf = (i) => this._layerItems[i] || null;

    // Presets — one click sets each modality's visibility / opacity / blend and
    // (for the blend-mode ones) which layer is on top. Matched to the four
    // Jubilees modalities BY LABEL so a missing/failed modality is simply skipped.
    // Each entry: { <label>: { on, opacity?, blend?, top? } }. `top:true` raises
    // that modality to the top of the stack (so its blend composites over the rest).
    const PRESETS = osdLayerPresets();

    const rows = document.createElement("div");
    rows.className = "layers__rows";

    // Render top-of-stack first (highest world index) so the panel order matches
    // the visual stack.
    const render = (focusKey) => {
      rows.textContent = "";
      // Order rows by current world z-order, top first.
      const order = stack
        .map((m, i) => ({ m, i }))
        .filter((x) => itemOf(x.i))
        .sort((a, b) => world.getIndexOfItem(itemOf(b.i)) - world.getIndexOfItem(itemOf(a.i)));

      order.forEach(({ m, i }, rowPos) => {
        const item = itemOf(i);
        const row = document.createElement("div");
        row.className = "layers__row";

        const vis = document.createElement("input");
        vis.type = "checkbox";
        vis.checked = m.visible;
        vis.setAttribute("aria-label", `Show ${m.label}`);
        vis.addEventListener("change", () => {
          m.visible = vis.checked;
          item.setOpacity(m.visible ? remembered[i] : 0);
        });

        const name = document.createElement("span");
        name.className = "layers__name";
        name.textContent = m.label;

        const op = document.createElement("input");
        op.type = "range";
        op.min = "0";
        op.max = "100";
        op.value = String(Math.round(remembered[i] * 100));
        op.className = "layers__opacity";
        op.setAttribute("aria-label", `${m.label} opacity`);
        op.addEventListener("input", () => {
          remembered[i] = Number(op.value) / 100;
          if (m.visible) item.setOpacity(remembered[i]);
        });

        const blendSel = document.createElement("select");
        blendSel.setAttribute("aria-label", `${m.label} blend mode`);
        for (const mode of ["source-over", "multiply", "difference", "screen"]) {
          const opt = document.createElement("option");
          opt.value = mode;
          opt.textContent = mode === "source-over" ? "normal" : mode;
          blendSel.appendChild(opt);
        }
        blendSel.value = blend[i]; // reflect current state (e.g. after a preset)
        blendSel.addEventListener("change", () => {
          blend[i] = blendSel.value;
          item.setCompositeOperation(
            blendSel.value === "source-over" ? null : blendSel.value,
          );
        });

        // Reorder — world index 0 is the BOTTOM; the top row raises the item.
        const up = document.createElement("button");
        up.type = "button";
        up.textContent = "▲";
        up.setAttribute("aria-label", `Move ${m.label} up`);
        up.disabled = rowPos === 0;
        up.dataset.focus = `${i}:up`; // for focus restore after re-render
        up.addEventListener("click", () => {
          const idx = world.getIndexOfItem(item);
          world.setItemIndex(item, Math.min(idx + 1, world.getItemCount() - 1));
          render(`${i}:up`); // re-render, then restore focus to this control
        });
        const down = document.createElement("button");
        down.type = "button";
        down.textContent = "▼";
        down.setAttribute("aria-label", `Move ${m.label} down`);
        down.disabled = rowPos === order.length - 1;
        down.dataset.focus = `${i}:down`;
        down.addEventListener("click", () => {
          const idx = world.getIndexOfItem(item);
          world.setItemIndex(item, Math.max(idx - 1, 0));
          render(`${i}:down`);
        });
        const reorder = document.createElement("span");
        reorder.className = "layers__reorder";
        reorder.append(up, down);

        row.append(vis, name, op, blendSel, reorder);
        rows.appendChild(row);
      });

      // Failed modalities → greyed, non-interactive rows.
      this._failedLayers.forEach((m) => {
        const row = document.createElement("div");
        row.className = "layers__row is-failed";
        const warn = document.createElement("span");
        warn.textContent = `⚠ ${m.label} (failed to load)`;
        row.appendChild(warn);
        rows.appendChild(row);
      });

      // Restore focus after a reorder re-render (the clicked button was
      // destroyed). If that button is now disabled (the item hit an end), fall
      // back to the row's other reorder button so keyboard focus stays put.
      if (focusKey) {
        const [idx, dir] = focusKey.split(":");
        const other = dir === "up" ? "down" : "up";
        const btn =
          rows.querySelector(`[data-focus="${idx}:${dir}"]:not([disabled])`) ||
          rows.querySelector(`[data-focus="${idx}:${other}"]:not([disabled])`);
        if (btn) btn.focus();
      }
    };

    // Apply a preset (see PRESETS): set each modality's visibility / opacity /
    // blend, optionally raise one to the top so its blend composites over the
    // rest, then re-render the rows to reflect the new state.
    const applyPreset = (spec) => {
      let toRaise = null;
      stack.forEach((m, i) => {
        const s = spec[m.label];
        const item = itemOf(i);
        if (!item) return;
        if (!s) {
          // Not named by the preset → turn it off.
          m.visible = false;
          item.setOpacity(0);
          return;
        }
        m.visible = s.on !== false;
        if (typeof s.opacity === "number") remembered[i] = s.opacity;
        item.setOpacity(m.visible ? remembered[i] : 0);
        blend[i] = s.blend || "source-over";
        item.setCompositeOperation(
          blend[i] === "source-over" ? null : blend[i],
        );
        if (s.top) toRaise = item;
      });
      if (toRaise) {
        world.setItemIndex(toRaise, world.getItemCount() - 1);
      }
      render();
    };

    // Preset bar (top of the panel). One button per PRESETS entry + a Reset.
    const bar = document.createElement("div");
    bar.className = "layers__presets";
    for (const preset of PRESETS) {
      const b = document.createElement("button");
      b.type = "button";
      b.textContent = preset.label;
      b.title = preset.hint;
      b.addEventListener("click", () => applyPreset(preset.spec));
      bar.appendChild(b);
    }

    p.append(bar, rows);
    render();
  }

  _fail(detail) {
    // Surface the failure as a visible ERROR state, and emit `viewer-error` so a
    // host can react (e.g. DataStar `data-on:viewer-error`). Builtin/OSD path.
    //
    // We deliberately do NOT reveal the light-DOM `<img>` fallback here: a static
    // image that visually mimics a working deep-zoom viewer hides the failure
    // (the user thinks it works, tries to zoom, nothing happens). Honest states
    // only — working viewer, or a visible error.
    this._stage.hidden = true;
    this._slot.hidden = true;
    if (!this._error) {
      this._error = document.createElement("div");
      this._error.className = "error";
      this._error.setAttribute("part", "error");
      this._error.setAttribute("role", "alert");
      this._root.appendChild(this._error);
    }
    this._error.hidden = false;
    this._error.textContent = `Viewer failed to load: ${
      detail && detail.message ? detail.message : detail
    }`;
    this.dispatchEvent(
      new CustomEvent("viewer-error", { bubbles: true, detail }),
    );
  }
}

if (!customElements.get("image-viewer")) {
  customElements.define("image-viewer", ImageViewer);
}
