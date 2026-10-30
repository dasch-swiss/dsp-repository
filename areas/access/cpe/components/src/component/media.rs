//! Renderer for the **record viewer** (inline deep-zoom media column beside
//! the properties) and helpers it shares with other media surfaces.
//!
//! Primary export: [`render_media_viewer`]. Per
//! `dsp-incubator cpe/docs/adr/0008-images-are-embedded-only-resources.md`
//! an image is its own Resource; a host (Character/Location/Chapter, …)
//! embeds the images that illustrate it via an ordered `link` property (the
//! host→image "media property", named per-class in
//! `Project::media_properties()`). The store pre-fetches those Image resources
//! (`PageData.media_items`, threaded as `RenderContext.media_items`) and this module
//! mounts `cpe-image-viewer`'s `<image-viewer>` in `.rv` —
//! banner / canvas / bar — with Details/Adjust drawers, WIDER, sequence
//! paging (`.pg` + strip), and host rights rendered elsewhere (`.pgroup` in
//! `resource_detail`).
//!
//! `.pg` is the shared one-at-a-time paging primitive (`← n / N →`,
//! `component::pager`); compound-viewer foot nav uses it too.
//!
//! Markup class names follow the record-viewer design bundle
//! (`style/media.css`)
//! — `.rv*`, `.em-*` strip/caption leftovers, `.pg*`. Interactive chrome is
//! DataStar signal state (no URL write per `dsp-incubator
//! cpe/docs/adr/0007-fragment-swaps-are-transient.md`); zero-JS falls back to real `?img=` /
//! `?strip=` navigation.

use cpe_core::iiif::{iiif_plate_url, iiif_strip_thumb_url};
use cpe_core::ir::{PropertyValue, RepresentationKind, Resource};
use cpe_core::lang::Lang;
use cpe_core::util::{escape_js_string_literal, is_safe_external_url};
use maud::{html, Markup};

use crate::chrome::{plate_pager_labels, ChromeMsg};
use crate::component::pager::override_query_param;

/// Render the record viewer (`.rv-col`) for a host resource-detail page.
///
/// Mounts the `cpe-image-viewer` crate's `<image-viewer>` custom element.
///
/// Emits the `.rv-unit`/`.rv`/`.rv__bar` frame
/// with the picture's legal run (author · copyright · licence,
/// `compose_legal_run`), the access line when restricted, the
/// zoom chips wired via DataStar (DataStar drives the crate's JS control
/// surface via `data-on:click` and reads its `viewer-zoom` event via
/// `data-on:viewer-zoom`), and a caption below the frame (the image's own
/// `description`, falling back to `alternative-description`).
///
/// The `$rail` signal (three-state — `closed`|`adjust`|`details`) drives
/// both drawers: the bar's Details/Adjust toggles and the rails' `x`
/// close buttons all write `$rail`; the rails read it via a reactive
/// `data-attr` object binding (`{'aria-hidden': …}`), and `.rv` reveals the open rail via a
/// reactive `.rail-adjust`/`.rail-details` CLASS (`data-class`) — never a reactive
/// `data-*` attribute, which collides with DataStar's own `data-*` namespace
/// (see the `.rv` element's inline note). The WIDER chip (`data-expand`)
/// must use the same class-toggle idiom, not a reactive `data-expand`
/// attribute. There is no whole-column fallback component — the
/// zero-JS/pre-upgrade path is the crate's own `.fallback()` markup (a plain
/// `<img>`) rendered per-image inside the canvas.
pub fn render_media_viewer(
    items: &[Resource],
    fallback_alt: &str,
    selected: usize,
    host_path: &str,
    host_query: Option<&str>,
    strip_open: bool,
    lang: &Lang,
) -> Markup {
    let n = items.len();
    debug_assert!(
        n > 0,
        "render_media_viewer must only be called with a non-empty items slice \
         (callers gate on RenderContext.media_items being non-empty)"
    );
    if n == 0 {
        // Defensive fallback for a misuse this crate's own callers don't
        // reach in practice — still carries `.page` (the LAYOUT CONTRACT
        // applies to every branch, this one included).
        return html! {
            div id="record-figure" tabindex="-1" class="rv-col page" {}
        };
    }
    // `selected` is 1-based (`?img=`, the record viewer's own convention); clamp
    // into range (a stale/hostile `?img=` can point past the set) → 0-based.
    let sel0 = selected.clamp(1, n) - 1;
    let label = ChromeMsg::PlateIllustrationsLabel.text(lang);
    // The crate requires a DOM `id` **stable across re-renders** (idiomorph
    // keys on it). The viewer is ONE multi-source element for the whole
    // host, so its id is per-HOST, not per-image: the first image's
    // id is stable across `?img=` reloads (the item LIST is unchanged; only the
    // selection differs) and unique to this host's viewer.
    let viewer_id = format!("rv-{}", items[0].id);

    // Sequence paging (`.pg` pager + strip) — the SAME gate the
    // unit applies internally (`render_viewer_unit`'s `can_sequence`).
    let can_sequence = has_sequence_pager(items);

    html! {
        div id="record-figure" tabindex="-1" class="rv-col page" {
            div.em-head {
                div.em-head__group {
                    span.em-head__title { (label) }
                    span.em-head__count { (n) }
                }
            }
            ({
                render_viewer_unit(
                    items,
                    sel0,
                    &viewer_id,
                    fallback_alt,
                    strip_open,
                    None,
                    lang,
                )
            })
            // Pager + caption + strip live in `.rv-col`, OUTSIDE the frame
            // (the pager outside the frame; the strip in the viewer column).
            // All three page via the `$plate` signal + `goToPage()` (JS) or a
            // real `?img=N` full reload (zero-JS) — NEVER a URL write and
            // NEVER the `_plate` `@get` fragment endpoint (the record viewer's
            // model is pre-render + toggle).
            @if can_sequence { (render_rv_pager(sel0, n, &viewer_id, host_path, host_query, lang)) }
            (render_caption_blocks(items, sel0, can_sequence, lang))
            @if can_sequence {
                ({
                    render_rv_strip(
                        items,
                        sel0,
                        strip_open,
                        &viewer_id,
                        host_path,
                        host_query,
                        lang,
                    )
                })
            }
        }
    }
}

/// The `data-attr` binding a WIDER host puts on the blocks the wide viewer
/// steps over (the record's head + body, the `pages` reader's member props):
/// genuinely `inert` + `aria-hidden` while `$expand` is set, not merely
/// `display:none`'d in CSS, so focus/AT cannot reach the stepped-aside
/// controls. `inert`/`aria-hidden` are NOT `data-*` attributes, so binding
/// them reactively is safe (only reactive `data-*` writes collide). Explicit
/// `'true'`/`false`: a bare-bool `aria-hidden=""` is read by AT as NOT hidden,
/// and `false` (not `'false'`) tells DataStar to REMOVE `inert` when collapsed
/// rather than leave `inert="false"` (a boolean attribute, inert by presence).
///
/// One constant for both hosts (`resource_detail`, `compound_viewer`), so the
/// two can never drift. Only meaningful beside a [`render_viewer_unit`], which
/// declares `$expand`.
pub(crate) const HIDDEN_WHEN_WIDE_ATTR: &str =
    "{'inert': $expand ? 'true' : false, 'aria-hidden': $expand ? 'true' : 'false'}";

/// The record viewer's **unit** — `div.rv-unit > div.rv { banner, canvas, bar }`
/// — extracted from [`render_media_viewer`] so
/// the compound viewer's `pages` stage mounts the SAME surface, not a fork.
///
/// Takes its items, the 0-based selection `sel0` (already clamped into
/// `0..items.len()`), and the viewer's DOM id (`rv-{first image id}` — the
/// bar's toggles and the rails derive their own ids from it) as data; it reads
/// nothing from the host article. `fallback_alt` is the host's title (the
/// `<img>` alt fallback), `strip_open` seeds the `stripOpen` signal.
/// `overlay` is the overlay layer's markup (`annotation::render_overlay`):
/// drawn only where a single-image `<image-viewer>` mounts and the image has
/// both dimensions, inside a fit box with the fallback `<img>`; ignored
/// anywhere else, which then renders exactly as with `None`.
///
/// The unit declares the viewer's page-global signals (`rail`, `expand`,
/// `plate`, `stripOpen`, the Adjust values; the canvas adds the zoom flags).
/// WIDER's visual effect lives on the HOST, which reads `$expand` (see
/// [`HIDDEN_WHEN_WIDE_ATTR`]), so a page holds at most one unit
/// (`DiagnosticKind::MultipleImageViewers`).
///
/// `items` must be non-empty (both callers gate on it).
pub(crate) fn render_viewer_unit(
    items: &[Resource],
    sel0: usize,
    viewer_id: &str,
    fallback_alt: &str,
    strip_open: bool,
    overlay: Option<&Markup>,
    lang: &Lang,
) -> Markup {
    let n = items.len();
    // Sequence paging (multi-source `<image-viewer>` + `.pg` pager + strip +
    // the `$plate` per-image toggle) is offered only when EVERY
    // image is IIIF-viewable, so `goToPage(n)`'s page index stays aligned 1:1
    // with the pre-rendered per-image blocks and the `?img=` selection. A set
    // with any non-IIIF image (no current Alice host — the demo is all IIIF)
    // degrades to the single selected image with no paging; the zero-JS `?img=`
    // reload still selects any image server-side. `n == 1` is the trivial
    // single-source case (no pager/strip either).
    let sources: Vec<String> = items.iter().filter_map(iiif_source_for).collect();
    let can_sequence = n > 1 && sources.len() == n;

    let selected = &items[sel0];
    // `data-restricted` is a STATIC data-* CSS hook (DataStar skips unknown
    // static `data-*`); it carries the SELECTED image's restriction for the
    // initial-load / zero-JS reload paint. In sequence mode the border tint
    // also tracks JS paging via a reactive `.is-restricted` CLASS keyed on a
    // per-image flag array indexed by `$plate` (a class, never a reactive
    // `data-restricted` write — that would hit the `data-*` collision).
    let restricted_sel = find_bool_property(selected, "limited-view").unwrap_or(false);
    let restricted_class: Option<String> = can_sequence.then(|| {
        let flags = items
            .iter()
            .map(|it| {
                if find_bool_property(it, "limited-view").unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{{'is-restricted': [{flags}][$plate]}}")
    });

    // Control gates: the zoom cluster's `data-on:click` targets
    // `getElementById(viewer_id)` and its disabled bindings read
    // `$atHome`/`$atNative`, which only exist when a mounted `<image-viewer>`
    // is present. In sequence mode the multi-source viewer always mounts (every
    // image is IIIF); otherwise the single selected image decides.
    let has_mounted_viewer = if can_sequence {
        true
    } else {
        viewer_will_mount(selected)
    };
    // The Details toggle/rail stays reachable even when no viewer mounts.
    let has_details = if can_sequence {
        true
    } else {
        details_will_render(selected)
    };

    // The `plate` signal defaults to `sel0` so the reactive `$plate != i`
    // bindings agree with the server-emitted static `hidden` (rendered on every
    // block where `i != sel0`) — no flash-of-wrong-block on hydration.
    // `stripOpen` defaults to the server's `?strip=` state for the same reason
    // (the thumb strip's fold is transient client-side state; `?strip=` is the
    // zero-JS full-reload fallback only, never written back — `dsp-incubator
    // cpe/docs/adr/0007-fragment-swaps-are-transient.md`).
    let strip_open_js = if strip_open { "true" } else { "false" };
    let signals = format!(
        "{{rail:'closed', expand:false, plate:{sel0}, stripOpen:{strip_open_js}, brightness:1, \
         contrast:1, saturation:1, exposure:0, gamma:1, hue:0, invert:false}}"
    );

    html! {
        div .rv-unit
            data-restricted=(if restricted_sel { "true" } else { "false" })
            data-class=[restricted_class.as_deref()]
        {
            // `rail` (three-state: closed|adjust|details) and the 7
            // Adjust signals are declared here, on `.rv` — the
            // nearest common ancestor of both the bar's Details/Adjust
            // toggles (which write `$rail`) and the rail markup inside
            // `.rv__canvas` (which reads it) — rather than on
            // `.rv__canvas` alongside the zoom-cluster signals, which are
            // canvas-scoped only.
            //
            // The drawer-open CSS state is a reactive CLASS toggle
            // (`data-class`), NOT a reactive `data-*` attribute. `data-*`
            // is DataStar's OWN reserved namespace: its internal
            // MutationObserver watches every `data-*` attribute change, so
            // having DataStar WRITE a `data-*` attribute (e.g. a `data-attr`
            // whose object sets `data-rail` from `$rail`, or `data-touched` on `.adj`)
            // re-enters that observer and derails DataStar's whole
            // binding-application pass — signals still initialise, but no
            // `data-on`/`data-attr` binding on the page ever wires up, so
            // the toggles silently do nothing. `data-class` writes the
            // plain `class` attribute (outside the `data-*` namespace), so
            // the observer is never re-triggered. The CSS keys on the
            // `.rail-adjust`/`.rail-details` classes accordingly. (Reactive
            // `aria-*` bindings are fine — `aria-*` is not `data-*`. Only
            // reactively-WRITTEN `data-*` attributes collide; a STATIC
            // `data-*` CSS hook like `data-restricted` is an unknown plugin
            // DataStar simply skips.) There is no zero-JS drawer state:
            // drawers are transient client-side enhancement,
            // so closed-by-default with no JS is correct.
            // `expand` (WIDER) is declared here alongside `rail`
            // and the adjust values so all viewer signals live in one
            // block. It is read by an ANCESTOR — `article.record--viewer`
            // toggles its `.is-wide` class from `$expand` via `data-class`
            // (resource_detail.rs) — which works because DataStar signals
            // are global, not DOM-scoped (the zoom signals below are read
            // cross-element the same way). `.rv` sits in `.rv-col`, a
            // sibling of the WIDER-hidden head/body, so this declaration
            // stays live while expanded. `plate` is
            // declared here too — the single source of truth for which
            // image shows: it toggles every per-image bar/Details/caption
            // block, drives the strip's `aria-current`, and is set (with a
            // `goToPage()` call) by the pager + strip. Default = `sel0`.
            div .rv
                data-signals=(signals)
                data-class="{'rail-adjust': $rail === 'adjust', 'rail-details': $rail === 'details'}"
            {
                // The picture's legal record
                // sits in a top banner ROW (never an
                // overlay over the picture — "nothing persistent is ever
                // set over the picture" still stands). `.rv`'s
                // three grid rows are banner / canvas / bar, in that
                // document order.
                (render_viewer_banner(items, sel0, can_sequence, lang))
                ({
                    render_viewer_canvas(
                        items,
                        sel0,
                        can_sequence,
                        &sources,
                        viewer_id,
                        fallback_alt,
                        overlay,
                        lang,
                    )
                })
                (render_viewer_bar(viewer_id, has_mounted_viewer, has_details, lang))
            }
        }
    }
}

/// The per-image show/hide wiring as a reactive `data-attr`
/// expression: block `i` is visible iff it is the selected `$plate`. The
/// caller pairs this with a STATIC maud `hidden[i != sel0]` boolean attribute
/// (the zero-JS path — with no JS only the server-selected block shows, so the
/// N legal records are never simultaneously visible). Object form
/// ONLY — NEVER the `data-attr-hidden` hyphen suffix (the vendored bundle
/// splits plugin from arg on a COLON and silently ignores hyphen args), and
/// NEVER `data-show` (JS-only `display` toggle — leaves all N visible with JS
/// off). `hidden` is a plain HTML attribute, not `data-*`, so writing it
/// reactively does not hit the `data-*` MutationObserver collision.
fn plate_hidden_attr(i: usize) -> String {
    format!("{{'hidden': $plate != {i}}}")
}

/// The `.rv__canvas`-level zoom signals' initial-value body (keys and
/// values only, no surrounding braces) — the identical literal both
/// `render_viewer_canvas` call sites declare via `data-signals`, and the
/// second half of [`viewer_reset_signals_json`]'s combined reset literal. A
/// single `const` so the two declaration sites and the reset fn can never
/// drift on this half's key set.
const CANVAS_SIGNALS_DEFAULT: &str =
    "atHome: true, atNative: false, atMin: true, atMax: false, viewerError: false, scale: 'FIT'";

/// The `.rv`-level signals' RESET body — the same nine keys
/// `render_viewer_unit`'s own `signals` local declares (`rail`, `expand`,
/// `plate`, `stripOpen`, and the 7 Adjust operators), but with every value
/// pinned to its DEFAULT rather than a render-time value (`plate: 0` /
/// `stripOpen: true`, not `sel0` / the request's `?strip=` state) — this is
/// what a `_compound` `layout="pages"` swap needs: a freshly-swapped-in
/// viewer unit always resets to its defaults, never carries forward the
/// previous member's `$plate`/`$rail` state.
const RV_SIGNALS_RESET: &str = "rail: 'closed', expand: false, plate: 0, stripOpen: true, \
     brightness: 1, contrast: 1, saturation: 1, exposure: 0, gamma: 1, hue: 0, invert: false";

/// The viewer unit's full DEFAULT/reset signal set — the `.rv`-level group
/// (`RV_SIGNALS_RESET`) and the `.rv__canvas`-level group
/// (`CANVAS_SIGNALS_DEFAULT`) combined into ONE `{key: value, ...}`
/// literal — the two
/// groups have disjoint top-level keys, so one object literal covers both.
/// The router's `layout="pages"` `_compound` swap patches this single
/// literal as one `PatchSignals` event, resetting a freshly-swapped-in
/// `<image-viewer>` unit to its just-mounted defaults. Keep this fn's
/// output IDENTICAL to what `render_viewer_unit`/`render_viewer_canvas`
/// declare at first render (modulo `plate`/`stripOpen`, which are
/// RENDER-TIME values there and RESET defaults here, by design) — a future
/// key rename/addition at either declaration site must be mirrored in
/// `RV_SIGNALS_RESET`/`CANVAS_SIGNALS_DEFAULT`, or the router's reset
/// silently stops covering it. Never add an annotation key here: the
/// annotation signals have their own reset
/// (`annotation::annotation_reset_signals_json`), which deliberately leaves
/// the toggle alone so it survives a page turn. `pub`, not `pub(crate)`,
/// because `dsp-incubator cpe/bin` (a separate crate) calls it directly.
pub fn viewer_reset_signals_json() -> String {
    format!("{{{RV_SIGNALS_RESET}, {CANVAS_SIGNALS_DEFAULT}}}")
}

/// Escape `s` as a JS **double-quoted** string literal (quotes included in
/// the output). Rust's `{:?}` Debug formatting is NOT an exact substitute for
/// this — Debug escapes some code points (e.g. bidi-control characters) as
/// braced `\u{XXXX}`, while JS string literals require unbraced 4-digit
/// `\uXXXX` — so embedding a Debug-formatted string directly into a JS
/// expression can emit a syntax error for text
/// containing such code points. This escapes exactly what a JS
/// double-quoted literal needs: backslash, double quote, and C0 control
/// characters; everything else (including non-ASCII text) passes through
/// unchanged. The `shared-datastar` path encoding is a different, narrower tool
/// (percent-encoding a handful of chars for a single-quoted `@get('…')` URL
/// context) and is not a fit here.
fn js_dq_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The IIIF `info.json` tile-source for one Image resource, or `None` when it
/// has no safe IIIF-shaped representation (used to decide multi-source
/// sequence eligibility and to build the `<image-viewer>` sources list).
fn iiif_source_for(item: &Resource) -> Option<String> {
    let repr = image_repr(item)?;
    cpe_core::iiif::iiif_info_json_url(&repr.uri)
}

/// Whether [`render_media_viewer`] offers the `.pg` sequence pager (the
/// `can_sequence` in that function) for `items` — i.e. more than one linked
/// image AND every one of them is IIIF-sequenceable. Exposed so
/// `dsp-incubator cpe/web`'s `page` module's breadcrumb/share-control wiring
/// can gate the record surface's reactive `?img=` `href` tracking on the
/// SAME condition `render_media_viewer` itself uses to decide whether the
/// `$plate` signal has anything to page through — a record with 0/1 images,
/// or a mixed IIIF/non-IIIF set, has no pager and so nothing for the share
/// control to track client-side.
pub fn has_sequence_pager(items: &[Resource]) -> bool {
    let n = items.len();
    n > 1 && items.iter().filter_map(iiif_source_for).count() == n
}

/// Render the per-image caption blocks below the frame (`.em-col__meta`). In
/// sequence mode every image's caption is pre-rendered and toggled by `$plate`;
/// otherwise only the selected image's caption renders. A caption is
/// the image's own `description` (falling back to `alternative-description`);
/// an image with neither contributes an empty block so the `$plate` index
/// alignment holds (its `.em-col__meta` is simply empty).
fn render_caption_blocks(items: &[Resource], sel0: usize, can_sequence: bool, lang: &Lang) -> Markup {
    let caption_of = |image: &Resource| {
        find_text_property(image, "description", lang)
            .or_else(|| find_text_property(image, "alternative-description", lang))
    };
    if !can_sequence {
        return html! {
            @if let Some(caption) = caption_of(&items[sel0]) {
                div.em-col__meta {
                    p.em-caption { (caption) }
                }
            }
        };
    }
    html! {
        @for (i, image) in items.iter().enumerate() {
            div.em-col__meta hidden[i != sel0] data-attr=(plate_hidden_attr(i)) {
                @if let Some(caption) = caption_of(image) {
                    p.em-caption { (caption) }
                }
            }
        }
    }
}

/// One inline SVG icon from the viewer-chrome glyph set — 16×16, `stroke-width:1.1`,
/// `fill:none`, `stroke:currentColor`, matching `.ico` in `media.css`.
/// Scoped to viewer chrome only — never reach for this outside `component::media`; the browse
/// surfaces deliberately stay typographic (`→ ↗ › ·`).
enum Icon {
    Plus,
    Minus,
    Home,
    Sliders,
    Maximize,
    X,
}

/// Render one [`Icon`] as `<svg class="ico" aria-hidden="true">…</svg>`. The
/// caller is responsible for an accessible name (`title`+`aria-label` on the
/// enclosing control) when the icon is not purely decorative —
/// none of the viewer-chrome glyphs are ever the *sole*
/// accessible label on their own (every chip pairs one with a `title`).
///
/// Each variant maps to one or more path `d` strings plus, for
/// [`Icon::Sliders`] only, three `<circle>` "thumb" marks — the only glyph in this set that isn't
/// pure strokes. [`Icon::Home`] is two overlapping strokes (roofline + walls).
fn ico(icon: Icon) -> Markup {
    let ds: &[&str] = match icon {
        Icon::Plus => &["M8 3.4V12.6M3.4 8h9.2"],
        Icon::Minus => &["M3.4 8h9.2"],
        Icon::Home => &["M2.6 7.3 8 2.9l5.4 4.4", "M4.3 6.6V13h7.4V6.6"],
        Icon::Sliders => &["M2.5 4.3h11M2.5 8h11M2.5 11.7h11"],
        Icon::Maximize => &["M6 2.6H2.6V6M10 2.6h3.4V6M13.4 10v3.4H10M6 13.4H2.6V10"],
        Icon::X => &["M4.3 4.3 11.7 11.7M11.7 4.3 4.3 11.7"],
    };
    // Only Sliders has thumb circles; every other variant's slice is empty,
    // so this branch is a no-op for them.
    let circles: &[(&str, &str, &str)] = match icon {
        Icon::Sliders => &[("10.4", "4.3", "1.5"), ("5.6", "8", "1.5"), ("11.4", "11.7", "1.5")],
        _ => &[],
    };
    html! {
        svg class="ico" viewBox="0 0 16 16" aria-hidden="true" {
            @for d in ds {
                path d=(d) {}
            }
            @for (cx, cy, r) in circles {
                circle cx=(cx) cy=(cy) r=(r) {}
            }
        }
    }
}

/// The `data-on:viewer-zoom` expression bound on `.rv__canvas` (a bubbling
/// listener — the crate's `Viewer` builder has no extension point for host
/// attributes on the `<image-viewer>` element itself, so this rides the
/// canvas wrapper `<div>` instead; the crate's `viewer-zoom` `CustomEvent`
/// is documented as bubbling for exactly this reason).
///
/// Sets the `$scale` signal `.rv__scale`'s `data-text` reads (the
/// continuous-zoom readout): `FIT` at home (or before the crate has an
/// opened image — `ratioToHome` is `null` until then), else `×N` from
/// `ratioToHome`, with `· native` appended at the top of the ladder.
///
/// Reads `evt.detail.ratioToHome` (home-relative: exactly 1 at FIT), NOT
/// `evt.detail.zoomPercent` (native-resolution-relative — the crate's
/// original field). A prior version of this expression read `zoomPercent`
/// directly, which correctly showed `FIT` at rest but showed a raw OSD
/// zoom fraction (e.g. `×0.35`) after a plain zoom-in from FIT, because
/// `zoomPercent` is screen-px-per-source-px, not a scale relative to the
/// fit/home zoom — a modest zoom-in on a large source image can still sit
/// well under 1 on that scale. `ratioToHome` (added to the crate's
/// `viewer-zoom` event additively for this fix) is exactly the scale a
/// "×N from FIT" readout needs: 1 at home, and always above 1 after any
/// zoom-in. `data-on:*` expressions are the trimmed function body verbatim
/// (no implicit `return`) —
/// `evt` is the crate's `CustomEvent`, `$scale` is the signal.
///
/// The disabled-state wiring extends the same expression to also
/// set `$atHome`/`$atNative` from the event payload — one bubbling listener,
/// three signals, rather than three separate `data-on:viewer-zoom` bindings
/// on the same element (DataStar allows only one `data-on:<event>` per
/// element per key). DataStar `data-on:*` expressions accept multiple
/// semicolon-separated statements (mirrors `facet_panel.rs`'s two-statement
/// clear click, `document.getElementById('q').value = ''; @get('…')`) — each assignment is its own
/// statement, evaluated left to right.
const VIEWER_ZOOM_EXPR: &str = "$scale = (evt.detail.atHome || evt.detail.ratioToHome == null) ? 'FIT' : ('×' + evt.detail.ratioToHome.toFixed(2) + (evt.detail.atNative ? ' · native' : '')); $atHome = evt.detail.atHome; $atNative = evt.detail.atNative; $atMin = evt.detail.atMin; $atMax = evt.detail.atMax";

/// The `data-on:viewer-error` expression bound alongside [`VIEWER_ZOOM_EXPR`]
/// on `.rv__canvas` (same bubbling-listener rationale). A `viewer-error`
/// is a quiet mono line in the bar, not a dialog. The crate's `viewer-error`
/// `CustomEvent` detail is either an `Error` (has `.message`) or a plain
/// value (see `cpe-image-viewer`'s `assets/image-viewer.js`'s
/// `_fail`) — never a guaranteed human-readable string, so this
/// deliberately does NOT surface `evt.detail` verbatim to the visitor (that
/// would risk showing a raw technical error). It only flips a boolean
/// signal; the translated line text itself
/// ([`ChromeMsg::RvViewerErrorLabel`]) is static SSR markup shown/hidden by
/// a static `hidden` attribute plus a reactive `data-attr` binding (never
/// `data-show` — see the `data-show` prohibition).
const VIEWER_ERROR_EXPR: &str = "$viewerError = true";

/// Whether [`render_viewer_canvas`] will mount a real `<image-viewer>`
/// element for `image` — i.e. it has a safe-external-URL `Image`
/// representation AND that URI is IIIF-shaped
/// ([`cpe_core::iiif::iiif_info_json_url`] returns `Some`). Shares
/// [`render_viewer_canvas`]'s exact filter chain so the two functions
/// can never disagree about which branch fired — used by
/// [`render_viewer_bar`] to gate the zoom cluster: the chips'
/// `data-on:click` targets `document.getElementById(viewer_id)` and their
/// disabled-state bindings read `$atHome`/`$atNative`, both only ever valid
/// when a real viewer is mounted (otherwise the degraded/
/// no-viewer branches would render live-looking, enabled-by-
/// default zoom chips wired to a `getElementById` that resolves to `null`).
fn viewer_will_mount(image: &Resource) -> bool {
    image_repr(image).is_some_and(|repr| cpe_core::iiif::iiif_info_json_url(&repr.uri).is_some())
}

/// Whether [`render_viewer_canvas`] will emit a Details rail for `image` —
/// i.e. it has a safe-external-URL `Image` representation (the same first
/// filter [`viewer_will_mount`] applies), regardless of whether that URI is
/// IIIF-shaped. Unlike [`viewer_will_mount`], this does NOT require IIIF —
/// [`render_details_rail`] only ever reads `repr`/EAV data, never the
/// mounted `<image-viewer>` element, so it renders in both of
/// [`render_viewer_canvas`]'s branches (the Details drawer must stay
/// reachable for a degraded/non-IIIF image, not just a fully mounted one).
/// Used by [`render_viewer_bar`] to gate the Details toggle button
/// independently of `has_viewer`.
fn details_will_render(image: &Resource) -> bool {
    image_repr(image).is_some()
}

/// Both of an image's stored dimensions, when positive: the ratio the
/// overlay's fit box needs.
fn fit_dims(repr: &cpe_core::ir::Representation) -> Option<(i64, i64)> {
    match (repr.width, repr.height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
        _ => None,
    }
}

/// Whether a single-image unit for `image` draws an overlay passed to it:
/// its `<image-viewer>` mounts and the image has both dimensions. The same
/// gates [`render_viewer_canvas`] applies, so a caller can wire the overlay
/// exactly where it is drawn.
pub(crate) fn overlay_will_draw(image: &Resource) -> bool {
    viewer_will_mount(image) && image_repr(image).and_then(fit_dims).is_some()
}

/// Render `.rv__canvas` — the crate's territory,
/// plus both drawers, mounted here because
/// `.rv__rail--adjust`/`.rv__rail--details` sit inside `.rv__canvas`
/// alongside `<image-viewer>` — they cover the canvas row only, never the
/// bar. Mounts `Viewer::new` with `tile-sources` from
/// [`cpe_core::iiif::iiif_info_json_url`]; degrades to the plain
/// plate `<img>` inside `.rv__stand`, with **no** `<image-viewer>` element at
/// all, when the stored URI is not IIIF-shaped (never `.expect()` a
/// `Some`). Missing/unsafe representations degrade further to the shared
/// image-placeholder note, same as the plate. The Details rail is emitted in
/// BOTH the `Some(info_json)` and `None` arms — it only needs `repr`/EAV data,
/// never the mounted `<image-viewer>` element — mirroring [`render_viewer_bar`]'s
/// `has_details` gate ([`details_will_render`]); the Adjust rail stays
/// `Some(info_json)`-only (mirrors `has_viewer`/[`viewer_will_mount`]), since
/// Adjust genuinely needs the crate's element to act on. Both gates share the
/// same filter chain as their bar counterparts, so the two can never disagree
/// about which branch fired. `overlay`: see [`render_viewer_unit`].
#[allow(clippy::too_many_arguments)]
fn render_viewer_canvas(
    items: &[Resource],
    sel0: usize,
    can_sequence: bool,
    sources: &[String],
    viewer_id: &str,
    fallback_alt: &str,
    overlay: Option<&Markup>,
    lang: &Lang,
) -> Markup {
    let selected = &items[sel0];
    // No safe Image representation for the selected image (missing
    // Representation, or a URI that fails the safe-external-URL allowlist)
    // degrades to the shared designed absent mark, rendered directly
    // into the canvas's own `.rv__stand` box. No
    // Details/Adjust rail renders here either — both are gated on the same
    // `image_repr`/`details_will_render` check, which also fails here.
    let Some(repr_sel) = image_repr(selected) else {
        let aria_label = ChromeMsg::ImagePlaceholderAriaLabel.text(lang);
        let label = ChromeMsg::ImageAbsentLabel.text(lang);
        let why = ChromeMsg::ImageAbsentPlate.text(lang);
        return html! {
            div.rv__canvas aria-label=(aria_label) {
                div.rv__stand {
                    div.absent {
                        span.absent__label { (label) }
                        span.absent__why { (why) }
                    }
                }
            }
        };
    };

    let plate_src = iiif_plate_url(&repr_sel.uri, repr_sel.width).unwrap_or_else(|| repr_sel.uri.clone());
    let alt = repr_sel.alt.as_deref().filter(|s| !s.is_empty()).unwrap_or(fallback_alt);
    // EAGER + high fetch priority: this is the viewer's pre-upgrade preview
    // (the crate's `.fallback()`), the FIRST thing a visitor should see. Lazy
    // loading left the viewer blank through OSD's init/info.json/tile window
    // (~100–150ms) because the preview never loaded promptly; eager shows it
    // immediately (~60ms cold), and OSD reuses the SAME sized-derivative URL so
    // its own fetch hits cache. (Contrast the row/strip thumbnails, which stay
    // lazy — they are genuinely below-the-fold secondary content.) In sequence
    // mode this fallback is the SELECTED image's plate only — one pre-hydration
    // preview, matching `?img=`/`.page(sel0+1)`.
    let fallback_img = html! {
        img src=(plate_src) alt=(alt) decoding="async" loading="eager" fetchpriority="high";
    };

    // The canvas-scoped zoom signals + the two crate-event listeners are shared
    // across the whole sequence (one `<image-viewer>`, paged by `goToPage`). In
    // sequence mode the multi-source viewer always mounts.
    if can_sequence {
        // Sequence mode auto-enables for >1 source; `.page(sel0+1)` opens on the
        // server-selected image (1-based). `.chrome(false)` opts out of OSD's
        // own stock sequence/nav controls — CPE provides the `.pg` pager + strip.
        let viewer = cpe_image_viewer::Viewer::new(viewer_id.to_owned(), sources.to_vec())
            .sequence(true)
            .page((sel0 + 1) as u32)
            .chrome(false)
            .auth(cpe_image_viewer::Auth::None)
            .label(alt)
            .fallback(fallback_img);
        return html! {
            div .rv__canvas
                data-signals=(format!("{{{CANVAS_SIGNALS_DEFAULT}}}"))
                data-on:viewer-zoom=(VIEWER_ZOOM_EXPR)
                data-on:viewer-error=(VIEWER_ERROR_EXPR)
            {
                (viewer)
                (render_details_rail(items, sel0, true, viewer_id, lang))
                (render_adjust_rail(viewer_id, lang))
            }
        };
    }

    // Single selected image (n == 1, or a set that isn't uniformly IIIF).
    match cpe_core::iiif::iiif_info_json_url(&repr_sel.uri) {
        Some(info_json) => {
            // The fit box takes the image's own ratio, so the zero-JS overlay
            // lines up with the image rather than the canvas; without both
            // dimensions there is no ratio, and so no overlay.
            let fallback = match (overlay, fit_dims(repr_sel)) {
                (Some(overlay), Some((w, h))) => html! {
                    div.anno-fit style=(format!("--r:{w} / {h}")) { (fallback_img) (overlay) }
                },
                _ => fallback_img,
            };
            let viewer = cpe_image_viewer::Viewer::new(viewer_id.to_owned(), vec![info_json])
                .chrome(false)
                .auth(cpe_image_viewer::Auth::None)
                .label(alt)
                .fallback(fallback);
            html! {
                div .rv__canvas
                    data-signals=(format!("{{{CANVAS_SIGNALS_DEFAULT}}}"))
                    data-on:viewer-zoom=(VIEWER_ZOOM_EXPR)
                    data-on:viewer-error=(VIEWER_ERROR_EXPR)
                {
                    (viewer)
                    (render_details_rail(items, sel0, false, viewer_id, lang))
                    (render_adjust_rail(viewer_id, lang))
                }
            }
        }
        // A non-IIIF/malformed URI degrades to the plain plate for this
        // image — never mount `<image-viewer>` on a `None` tile-source. The
        // Details rail still renders — it only needs `repr`/EAV data.
        None => html! {
            div.rv__canvas {
                div.rv__stand { (fallback_img) }
                (render_details_rail(items, sel0, false, viewer_id, lang))
            }
        },
    }
}

/// The safe IIIF-shaped `Image` representation of one resource, if any — the
/// shared `find(Image).filter(safe-url)` chain used across the viewer for
/// tile-source, plate, dimensions, and IIIF-source reads. `pub(crate)`
/// — `compound_viewer.rs`'s `pages` layout
/// scan/stage shares this exact body since it needs the same "has a scan" gate.
pub(crate) fn image_repr(item: &Resource) -> Option<&cpe_core::ir::Representation> {
    item.representations
        .iter()
        .find(|r| r.kind == RepresentationKind::Image)
        .filter(|r| is_safe_external_url(&r.uri))
}

/// Whether a viewer record's frame is tall: every linked image has stored
/// pixel dimensions and is tall ([`cpe_core::ir::is_tall`]). The viewer pages
/// its images client-side, so the frame cannot follow the image shown: one
/// image that is not tall, or of unknown size, keeps the default split.
/// `true` renders `data-frame="tall"`, which selects the tall-frame rules in
/// `style/media.css`.
pub(crate) fn frame_is_tall(items: &[Resource]) -> bool {
    !items.is_empty()
        && items.iter().all(|item| {
            image_repr(item)
                .and_then(|r| r.width.zip(r.height))
                .is_some_and(|(w, h)| cpe_core::ir::is_tall(w, h))
        })
}

/// Render `.rv__rail.rv__rail--details`
/// — the picture's second-order fields: the restricted-access prose (only when
/// `restricted`), identifier, dimensions, IIIF source. Never the bar's
/// author/copyright/licence/access-state fields (those stay in
/// [`render_viewer_bar`] only). No DESCRIPTION row — the plate
/// caption below the frame already shows the image's `description` (falling
/// back to `alternative-description`), so a DESCRIPTION row here duplicated
/// visible content whenever a description was present.
///
/// `repr` is the same `Representation` [`render_viewer_canvas`] already found
/// — dimensions and the IIIF source both read it directly, so unlike the
/// bar's fields this drawer never needs its own existence check for either.
/// Called from BOTH of [`render_viewer_canvas`]'s branches (
/// `viewer_id` is still needed here even in the `None`/no-mounted-viewer
/// branch, purely as the DOM id of the bar's Details toggle button, which
/// exists whenever this rail does; see [`details_will_render`]) — so it
/// never assumes a `<image-viewer>` element exists.
fn render_details_rail(items: &[Resource], sel0: usize, can_sequence: bool, viewer_id: &str, lang: &Lang) -> Markup {
    let close_label = ChromeMsg::RvDrawerCloseLabel.text(lang);
    let region_label = ChromeMsg::RvDetailsToggleLabel.text(lang);
    // Closing the rail via its own X restores focus to the bar
    // toggle that opened it (mirrors `reset_click`'s multi-statement
    // `format!` idiom below in `render_adjust_rail`).
    // Move focus to the toggle BEFORE flipping `$rail`,
    // so focus never sits inside a region that has just gone
    // `aria-hidden="true"` (this rail is always visible/focusable, so no
    // `setTimeout` is needed for this direction, unlike the open case
    // below which waits on the just-opened rail's DOM to settle).
    // `preventScroll: true` guards against scrolling the canvas's
    // `overflow:hidden` box while the OTHER rail is still transitioning
    // off-screen.
    // `viewer_id` is escaped for safe interpolation into this JS string
    // literal — see `escape_js_string_literal`'s doc comment.
    let viewer_id_js = escape_js_string_literal(viewer_id);
    let close_click = format!(
        "document.getElementById('{viewer_id_js}-details-toggle')?.focus({{ preventScroll: true }}); \
         $rail = 'closed'"
    );
    let n = items.len();

    html! {
        div .rv__rail
            .rv__rail--details
            id=(format!("{viewer_id}-details"))
            role="region"
            aria-label=(region_label)
            data-attr="{'aria-hidden': $rail !== 'details' ? 'true' : 'false'}"
        {
            div.rv__rail-head {
                span { (region_label) }
                button
                    type="button"
                    class="vctl"
                    data-on:click=(close_click)
                    title=(close_label)
                    aria-label=(close_label)
                { (ico(Icon::X)) }
            }
            div.rv__rail-body {
                // The rail SHELL (head + close + aria-hidden) is shared across
                // the sequence; only its FIELD content is per-image. In
                // sequence mode every image's `.rv-details` is pre-rendered and
                // toggled by `$plate` (static `hidden` + reactive `data-attr` —
                // see `plate_hidden_attr`); otherwise only the selected image.
                @if can_sequence {
                    @for (i, image) in items.iter().enumerate() {
                        div.rv-details hidden[i != sel0] data-attr=(plate_hidden_attr(i)) {
                            (render_details_fields(image, i, n, can_sequence, lang))
                        }
                    }
                } @else {
                    div.rv-details {
                        ({
                            render_details_fields(
                                &items[sel0],
                                sel0,
                                n,
                                can_sequence,
                                lang,
                            )
                        })
                    }
                }
            }
        }
    }
}

/// Render one image's `dl.rf` Details fields: restricted-access prose
/// (when the image is `limited-view`), identifier (`<id> · plate i of n` when
/// the host has more than one image AND those images are reachable via the
/// pager/strip — `can_sequence`; the plate suffix is omitted
/// for a lone image, and ALSO omitted when `n > 1` but `!can_sequence` — some
/// of the host's images aren't IIIF, so there is no pager/strip to reach
/// them, and printing "plate i of n" there would falsely imply other
/// reachable images exist), dimensions, and IIIF source. `i` is the image's
/// 0-based position; `n` the total. Reads the image's own IIIF representation
/// for dimensions + source (`image_repr` — always `Some` in sequence mode,
/// and `Some` for the selected image in the single case since the canvas
/// already gated on it; the rows are simply omitted if it is somehow absent).
fn render_details_fields(image: &Resource, i: usize, n: usize, can_sequence: bool, lang: &Lang) -> Markup {
    let restricted = find_bool_property(image, "limited-view").unwrap_or(false);
    let repr = image_repr(image);
    let dims = repr.and_then(image_dimensions);
    html! {
        dl.rf {
            @if restricted {
                div.rf__row.rf__row--access {
                    dt { (ChromeMsg::RvFieldAccess.text(lang)) }
                    dd { (ChromeMsg::RvAccessRestrictedLong.text(lang)) }
                }
            }
            div.rf__row {
                dt { (ChromeMsg::RvFieldIdentifier.text(lang)) }
                dd.rf__mono {
                    (image.id)
                    @if can_sequence && n > 1 { " · " (crate::chrome::rv_plate_of(i + 1, n, lang)) }
                }
            }
            @if let Some(dims) = dims {
                div.rf__row {
                    dt { (ChromeMsg::RvFieldDimensions.text(lang)) }
                    dd.rf__mono { (dims) }
                }
            }
            @if let Some(repr) = repr {
                div.rf__row {
                    dt { (ChromeMsg::RvFieldIiifSource.text(lang)) }
                    dd.rf__mono { (repr.uri) }
                }
            }
        }
    }
}

/// Render `.rv__rail.rv__rail--adjust` — the crate's `Viewer::adjust(true)` panel skin: 7 operators
/// (brightness · contrast · saturation · exposure · gamma · hue · invert),
/// `RESET ALL`, and the transient-looking note.
///
/// **The units bridge (load-bearing):** the slider ranges are
/// display units the crate's setters do NOT take directly. Brightness/
/// contrast/saturation are 0–1-ish multipliers here (defaults `1`) but the
/// crate's OSD setters take **percent** (`setBrightness(100)` ==
/// `brightness(100%)`), so each `data-on:input` multiplies the signal by 100
/// before calling the setter. Exposure/gamma/hue are pass-through — the
/// crate's setters already take the same native units (0-centered fraction,
/// a gamma multiplier, and degrees respectively) the readout shows.
fn render_adjust_rail(viewer_id: &str, lang: &Lang) -> Markup {
    let close_label = ChromeMsg::RvDrawerCloseLabel.text(lang);
    let region_label = ChromeMsg::RvAdjustToggleLabel.text(lang);
    // `viewer_id` is escaped for safe interpolation into every JS string
    // literal below — see `escape_js_string_literal`'s doc comment. The
    // `id=(…)` DOM id attributes further down stay on the bare `viewer_id`
    // (maud escapes those for HTML on its own).
    let viewer_id_js = escape_js_string_literal(viewer_id);
    let reset_click = format!(
        "$brightness = 1; $contrast = 1; $saturation = 1; $exposure = 0; \
         $gamma = 1; $hue = 0; $invert = false; \
         document.getElementById('{viewer_id_js}').resetAdjust()"
    );
    // Closing the rail via its own X restores focus to the bar
    // toggle that opened it. Focus moves BEFORE `$rail`
    // flips (see `render_details_rail`'s matching comment).
    // `preventScroll: true` guards the transitioning-off-screen rail case.
    let close_click = format!(
        "document.getElementById('{viewer_id_js}-adjust-toggle')?.focus({{ preventScroll: true }}); \
         $rail = 'closed'"
    );

    html! {
        div .rv__rail
            .rv__rail--adjust
            id=(format!("{viewer_id}-adjust"))
            role="region"
            aria-label=(region_label)
            data-attr="{'aria-hidden': $rail !== 'adjust' ? 'true' : 'false'}"
        {
            div.rv__rail-head {
                span { (region_label) }
                button
                    type="button"
                    class="vctl"
                    data-on:click=(close_click)
                    title=(close_label)
                    aria-label=(close_label)
                { (ico(Icon::X)) }
            }
            div.rv__rail-body {
                div.adj data-class="{untouched: $brightness === 1}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustBrightness.text(lang)) }
                    span.adj__val data-text="$brightness.toFixed(2)" { "1.00" }
                    input
                        .adj__in
                        type="range"
                        min="0.4"
                        max="1.6"
                        step="0.01"
                        data-bind="brightness"
                        aria-label=(ChromeMsg::RvAdjustBrightness.text(lang))
                        data-on:input=({
                            format!(
                                "document.getElementById('{viewer_id_js}').setBrightness($brightness * 100)",
                            )
                        });
                }
                div.adj data-class="{untouched: $contrast === 1}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustContrast.text(lang)) }
                    span.adj__val data-text="$contrast.toFixed(2)" { "1.00" }
                    input
                        .adj__in
                        type="range"
                        min="0.4"
                        max="1.6"
                        step="0.01"
                        data-bind="contrast"
                        aria-label=(ChromeMsg::RvAdjustContrast.text(lang))
                        data-on:input=({
                            format!(
                                "document.getElementById('{viewer_id_js}').setContrast($contrast * 100)",
                            )
                        });
                }
                div.adj data-class="{untouched: $saturation === 1}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustSaturation.text(lang)) }
                    span.adj__val data-text="$saturation.toFixed(2)" { "1.00" }
                    input
                        .adj__in
                        type="range"
                        min="0"
                        max="2"
                        step="0.01"
                        data-bind="saturation"
                        aria-label=(ChromeMsg::RvAdjustSaturation.text(lang))
                        data-on:input=({
                            format!(
                                "document.getElementById('{viewer_id_js}').setSaturation($saturation * 100)",
                            )
                        });
                }
                div.adj data-class="{untouched: $exposure === 0}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustExposure.text(lang)) }
                    span.adj__val data-text="$exposure.toFixed(2)" { "0.00" }
                    input
                        .adj__in
                        type="range"
                        min="-1"
                        max="1"
                        step="0.01"
                        data-bind="exposure"
                        aria-label=(ChromeMsg::RvAdjustExposure.text(lang))
                        data-on:input=({
                            format!(
                                "document.getElementById('{viewer_id_js}').setExposure($exposure)",
                            )
                        });
                }
                div.adj data-class="{untouched: $gamma === 1}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustGamma.text(lang)) }
                    span.adj__val data-text="$gamma.toFixed(2)" { "1.00" }
                    input
                        .adj__in
                        type="range"
                        min="0.5"
                        max="2"
                        step="0.01"
                        data-bind="gamma"
                        aria-label=(ChromeMsg::RvAdjustGamma.text(lang))
                        data-on:input=({
                            format!(
                                "document.getElementById('{viewer_id_js}').setGamma($gamma)",
                            )
                        });
                }
                div.adj data-class="{untouched: $hue === 0}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustHue.text(lang)) }
                    span.adj__val data-text="$hue + '°'" { "0°" }
                    input
                        .adj__in
                        type="range"
                        min="-180"
                        max="180"
                        step="1"
                        data-bind="hue"
                        aria-label=(ChromeMsg::RvAdjustHue.text(lang))
                        data-on:input=({
                            format!(
                                "document.getElementById('{viewer_id_js}').setHue($hue)",
                            )
                        });
                }
                div.adj.adj--flag data-class="{untouched: $invert === false}" {
                    span.adj__lbl { (ChromeMsg::RvAdjustInvert.text(lang)) }
                    button
                        type="button"
                        class="vctl"
                        data-attr="{'aria-pressed': $invert ? 'true' : 'false'}"
                        data-on:click=({
                            format!(
                                "$invert = !$invert; document.getElementById('{viewer_id_js}').setInvert($invert)",
                            )
                        })
                    { (ChromeMsg::RvAdjustInvert.text(lang)) }
                }
                div.rv__rail-note { (ChromeMsg::RvAdjustNote.text(lang)) }
            }
            div.rv__rail-foot {
                button type="button" class="vctl" data-on:click=(reset_click) {
                    (ChromeMsg::RvAdjustResetLabel.text(lang))
                }
            }
        }
    }
}

/// Render one image's legal record: the restricted ACCESS line
/// (when the image is `limited-view`) above the author · copyright · licence
/// run (`compose_legal_run` — omits absent fields, shows copyright only when
/// present ∧ ≠ licence). Used per-image by [`render_viewer_banner`]'s
/// `.rv__legal` (one block per image in sequence mode, toggled by `$plate`;
/// the selected image alone otherwise). Never the Details-drawer fields.
fn render_legal_block(image: &Resource, lang: &Lang) -> Markup {
    let restricted = find_bool_property(image, "limited-view").unwrap_or(false);
    let author = find_text_property(image, "authorship", lang);
    let copyright = find_text_property(image, "copyright", lang);
    let license = find_text_property(image, "license", lang);
    let run = compose_legal_run(author.as_deref(), copyright.as_deref(), license.as_deref());
    html! {
        @if restricted {
            span.em-access {
                b { (ChromeMsg::RfAccessRestricted.text(lang)) }
            }
        }
        @if !run.is_empty() {
            span.em-legal__run {
                @for (i, part) in run.iter().enumerate() {
                    @if i > 0 {
                        span.em-legal__dot { " · " }
                    }
                    (part)
                }
            }
        }
    }
}

/// Render `.rv__banner` — a solid-bed row above the canvas holding ONLY the
/// picture's legal record (access line + author · copyright · licence run).
/// It has its own top row so the bar can stay a
/// controls-only strip. This is a banner ROW, not an overlay — it never
/// composites over the canvas (the "nothing persistent is ever set over the
/// picture" rule of `style/media.css` section 3 still stands; a banner row is simply a
/// second use of the same rule, at the top instead of the bottom). It draws
/// on the SAME near-black bed as the rest of `.rv` (no background of its
/// own) and the same `--on-bed`/`--on-bed-muted`/`--on-bed-faint` ink ramp —
/// never a hand-rolled white-alpha value (see media.css's "ONE floor for
/// quiet text on the dark bed" note).
///
/// In sequence mode every image's legal block is pre-rendered and toggled by
/// `$plate` (static `hidden` + reactive `data-attr`, so the N legal
/// records are never simultaneously visible with JS off); otherwise
/// only the selected image's block. `min-height: var(--bar-h)` (shared with
/// the bar) is only a FLOOR — the banner's content varies per image (an
/// ACCESS line when restricted, a licence run that wraps differently per
/// string and language). The picture does not resize while paging because
/// media.css stacks all of these pre-rendered plates into ONE grid cell of
/// `.rv__legal`, which sizes to the tallest of them; hidden plates therefore
/// hide with `visibility: hidden`, not `display: none`, so their boxes stay
/// in flow. See media.css's `.rv__legal` rule and its `[hidden]` guard.
fn render_viewer_banner(items: &[Resource], sel0: usize, can_sequence: bool, lang: &Lang) -> Markup {
    html! {
        div.rv__banner {
            span.rv__legal {
                @if can_sequence {
                    @for (i, image) in items.iter().enumerate() {
                        span.rv__legal-plate hidden[i != sel0] data-attr=(plate_hidden_attr(i)) {
                            (render_legal_block(image, lang))
                        }
                    }
                } @else {
                    span.rv__legal-plate { (render_legal_block(&items[sel0], lang)) }
                }
            }
        }
    }
}

/// Render `.rv__bar` — the full `.rv__ctrls` row: the Details/Adjust drawer
/// toggles beside the zoom cluster (out/in + fit-to-view, the
/// continuous-zoom disabled-state wiring, and the quiet
/// `viewer-error` line) plus the `WIDER` toggle (a layout-only
/// control rendered whenever the bar renders). Details/Adjust write the
/// SAME single-valued `$rail` signal ([`render_media_viewer`]'s `.rv`
/// `data-signals`) via a toggle-or-close ternary each — mutual exclusion
/// falls out of that shared signal, not from any coordination between
/// the two buttons.
///
/// The picture's legal record is not in this
/// bar but in [`render_viewer_banner`], the top row — the bar is controls-
/// only. The `viewer-error` status line stays here ("a quiet mono
/// line in the bar, not a dialog").
///
/// `has_viewer` (from [`viewer_will_mount`]) gates the scale readout, the
/// three zoom chips, and the Adjust toggle + `viewer-error` status line.
/// [`render_viewer_canvas`] only emits `$atHome`/`$atNative`/`$viewerError`
/// (via `data-signals`) and the `viewer-zoom`/`viewer-error` listeners on the
/// `Some(info_json)` mounted-viewer branch; when it degrades to the plain
/// `.rv__stand` fallback (non-IIIF URI or no safe representation) there
/// is no `<image-viewer>` element for `document.getElementById(viewer_id)` to
/// find and no signals for the `aria-disabled`/`hidden` bindings (object-form
/// `data-attr`) to read.
/// Rendering the cluster anyway would produce chips that read as *enabled*
/// (an undeclared DataStar signal is falsy, so `aria-disabled="$atHome"`
/// never fires) and whose `data-on:click` throws against `null` — silently
/// broken chrome over a plain `<img>`. Omitting the cluster entirely is the
/// correct degradation, matching "never render a live-looking control
/// with nothing behind it" spirit.
///
/// `has_details` (from [`details_will_render`]) gates the Details
/// toggle button INDEPENDENTLY of `has_viewer` — it only needs `repr`/EAV
/// data, so it renders even when no `<image-viewer>` mounted (a degraded/
/// non-IIIF image), keeping the Details drawer reachable in that case. The
/// outer `.rv__ctrls` renders whenever either gate is true.
fn render_viewer_bar(viewer_id: &str, has_viewer: bool, has_details: bool, lang: &Lang) -> Markup {
    let zoom_out_label = ChromeMsg::RvZoomOutLabel.text(lang);
    let zoom_in_label = ChromeMsg::RvZoomInLabel.text(lang);
    let zoom_home_label = ChromeMsg::RvZoomHomeLabel.text(lang);
    let viewer_error_label = ChromeMsg::RvViewerErrorLabel.text(lang);
    // `viewer_id` is this module's own `format!("rv-{}", image.id)`. Resource
    // ids are `[A-Za-z0-9_-]+` by CPE's convention throughout, but that
    // convention is data-authored (`translate.py`/a future project's own
    // pipeline), not schema-enforced — `viewer_id_js` escapes it for safe
    // interpolation into a single-quoted JS string literal regardless
    // (see `util::escape_js_string_literal`'s doc comment).
    // The bare `viewer_id` stays unescaped where it's used as a DOM id
    // attribute (`id=(…)`) — maud escapes those for HTML on its own.
    let viewer_id_js = escape_js_string_literal(viewer_id);
    let zoom_out_click = format!("document.getElementById('{viewer_id_js}').zoomOut()");
    let zoom_in_click = format!("document.getElementById('{viewer_id_js}').zoomIn()");
    let zoom_home_click = format!("document.getElementById('{viewer_id_js}').goHome()");
    let details_label = ChromeMsg::RvDetailsToggleLabel.text(lang);
    let adjust_label = ChromeMsg::RvAdjustToggleLabel.text(lang);
    let wider_label = ChromeMsg::RvWiderToggleLabel.text(lang);
    // WIDER: a pure layout toggle — the properties step aside and
    // the viewer column takes the whole measure. It flips the `$expand` boolean
    // signal; `article.record--viewer` reacts by toggling its `.is-wide` class
    // (via `data-class`, resource_detail.rs) and by making its head + body
    // `inert`/`aria-hidden`. It does NOT call into the `<image-viewer>` element,
    // so it renders whenever the bar renders (even the degraded plate case),
    // unlike the zoom cluster and Adjust toggle which are `has_viewer`-gated. A
    // single constant label + reactive `aria-pressed` conveys the toggle state
    // (mirroring the Details/Adjust toggles) — no label swap on expand.
    let wider_click = "$expand = !$expand";
    // Opening a rail (never closing) moves focus into it once the DOM
    // update settles — the rails render before this bar in `.rv__canvas`, so
    // forward-Tab from the toggle alone would skip right over the just-opened
    // rail's content. Focuses the rail's own close button (`.vctl` inside
    // `.rv__rail-head`) — the simplest, always-present target.
    // `preventScroll: true` guards against scrolling the
    // canvas's `overflow:hidden` box, since the target rail may still be
    // `transform: translateX(100%)` at the moment focus lands.
    let details_click = format!(
        "const open = $rail !== 'details'; $rail = open ? 'details' : 'closed'; \
         if (open) setTimeout(() => document.getElementById('{viewer_id_js}-details')?.querySelector('.rv__rail-head .vctl')?.focus({{ preventScroll: true }}), 0)"
    );
    let adjust_click = format!(
        "const open = $rail !== 'adjust'; $rail = open ? 'adjust' : 'closed'; \
         if (open) setTimeout(() => document.getElementById('{viewer_id_js}-adjust')?.querySelector('.rv__rail-head .vctl')?.focus({{ preventScroll: true }}), 0)"
    );

    html! {
        div.rv__bar {
            // "viewer-error → a quiet mono line in the bar, not a
            // dialog." Hidden via a static `hidden` attribute + a reactive
            // `data-attr` binding, never `data-show` (see the `data-show`
            // prohibition — `data-show` only hides reactively client-side,
            // leaving the line visible pre-hydration/with JS off, and
            // `viewer-error` only ever fires from the crate's own JS
            // regardless) — see `VIEWER_ERROR_EXPR`'s doc comment for why
            // the message text stays static/translated rather than echoing
            // the raw event detail. `aria-live` mirrors this module's other
            // reactive small-status idiom (`facet_panel.rs`'s
            // `div.filterbar`) — a small status element, never the whole
            // swapped region. `compound_viewer.rs`'s own `span.pg__count`
            // also carries `aria-live`, a second independent instance of the same
            // idiom. Gated by `has_viewer` — see the function doc comment.
            @if has_viewer {
                span
                    .rv__err
                    aria-live="polite"
                    aria-atomic="true"
                    hidden
                    data-attr="{'hidden': !$viewerError}"
                { (viewer_error_label) }
            }
            @if has_viewer || has_details {
                div.rv__ctrls {
                    @if has_viewer {
                        span.rv__scale data-text="$scale" { "FIT" }
                    }
                    @if has_details {
                        button
                            type="button"
                            class="vctl"
                            id=(format!("{viewer_id}-details-toggle"))
                            data-on:click=(details_click)
                            data-attr="{'aria-pressed': $rail === 'details' ? 'true' : 'false'}"
                            title=(details_label)
                            aria-label=(details_label)
                        {
                            span { (details_label) }
                        }
                    }
                    @if has_viewer {
                        button
                            type="button"
                            class="vctl vctl--icon"
                            id=(format!("{viewer_id}-adjust-toggle"))
                            data-on:click=(adjust_click)
                            data-attr="{'aria-pressed': $rail === 'adjust' ? 'true' : 'false'}"
                            title=(adjust_label)
                            aria-label=(adjust_label)
                        {
                            (ico(Icon::Sliders))
                            span { (adjust_label) }
                        }
                        span.vgroup {
                            button
                                type="button"
                                class="vctl vctl--icon"
                                data-on:click=(zoom_out_click)
                                data-attr="{'aria-disabled': $atMin ? 'true' : 'false'}"
                                title=(zoom_out_label)
                                aria-label=(zoom_out_label)
                            { (ico(Icon::Minus)) }
                            button
                                type="button"
                                class="vctl vctl--icon"
                                data-on:click=(zoom_home_click)
                                data-attr="{'aria-disabled': $atHome ? 'true' : 'false'}"
                                title=(zoom_home_label)
                                aria-label=(zoom_home_label)
                            { (ico(Icon::Home)) }
                            button
                                type="button"
                                class="vctl vctl--icon"
                                data-on:click=(zoom_in_click)
                                data-attr="{'aria-disabled': $atMax ? 'true' : 'false'}"
                                title=(zoom_in_label)
                                aria-label=(zoom_in_label)
                            { (ico(Icon::Plus)) }
                        }
                    }
                    // WIDER — rendered whenever the bar renders (a layout-only
                    // toggle; see `wider_click`'s note above), so it stays
                    // outside the `@if has_viewer` block.
                    button
                        type="button"
                        class="vctl vctl--icon vctl--wider"
                        id=(format!("{viewer_id}-wider-toggle"))
                        data-on:click=(wider_click)
                        data-attr="{'aria-pressed': $expand ? 'true' : 'false'}"
                        title=(wider_label)
                        aria-label=(wider_label)
                    { (ico(Icon::Maximize)) }
                }
            }
        }
    }
}

// record-viewer sequence paging
//
// The record viewer pages a SINGLE multi-source `<image-viewer>` client-side:
// the `.pg` pager and the thumb strip set the `$plate` signal and call
// `goToPage(n)` on the element, while a real `?img=N` href is the zero-JS
// full-reload fallback. This is the pre-render+toggle model — NEVER the
// plate's `_plate` `@get` fragment swap (which would re-init the viewer and
// lose zoom/adjust state), and NEVER a URL write / `pushState` (`dsp-incubator
// cpe/docs/adr/0007-fragment-swaps-are-transient.md`).

/// The shared JS action for a pager step or strip thumb navigating to the
/// 1-based image `target`: set `$plate` (0-based) — the single source of truth
/// that toggles every per-image block — and page the one multi-source
/// `<image-viewer>` via `goToPage` (1-based). No `@get`, no URL write.
fn rv_page_click(target1: usize, viewer_id: &str) -> String {
    // Escape for safe interpolation into this JS string literal.
    let viewer_id_js = escape_js_string_literal(viewer_id);
    format!(
        "$plate = {plate0}; document.getElementById('{viewer_id_js}').goToPage({target1})",
        plate0 = target1 - 1
    )
}

/// The record-viewer `.pg` pager. Reuses the shared `.pg`/`.pg__step`/
/// `.pg__count` markup primitive but is **fully reactive**: unlike a
/// server-rendered pager whose prev/next hrefs fix `?img=` targets at render
/// time, both arrows derive their step from the LIVE `$plate` signal at click time
/// (`$plate = $plate ± 1; goToPage($plate + 1)`), and their disabled state binds
/// to `$plate` too. This is essential because paging is client-side with no
/// re-render — a server-fixed target would page image 1→2
/// once and then keep pointing at image 2, with prev never re-enabling.
///
/// Both ends are always an `<a>` (never swapped for an inert `<span>`), so the
/// reactive `data-attr` can re-enable them as `$plate` moves off a boundary; the
/// server-rendered static `aria-disabled` carries the zero-JS/initial end-state.
/// `.pg__step[aria-disabled="true"]` is `pointer-events:none` in CSS, so a
/// disabled end is inert for BOTH the JS and zero-JS paths. The `href` stays a
/// real `?img=N` full-reload link (zero-JS), suppressed by `__prevent` under JS.
/// Only called when `n > 1`.
fn render_rv_pager(
    sel0: usize,
    n: usize,
    viewer_id: &str,
    host_path: &str,
    host_query: Option<&str>,
    lang: &Lang,
) -> Markup {
    debug_assert!(
        n > 1,
        "render_rv_pager must only be called when n > 1 (callers gate on multi-image hosts)"
    );
    let (prev_label, next_label) = plate_pager_labels(lang);
    let nav_aria = ChromeMsg::PlatePagerNavLabel.text(lang);
    let count_text = format!("{} / {}", sel0 + 1, n);
    let last = n - 1; // last 0-based index
                      // Zero-JS full-reload targets (1-based, clamped): the neighbour image, or
                      // self at a boundary (the end is `aria-disabled` + `pointer-events:none`
                      // anyway, so a boundary href is never actually followed).
    let prev_href = build_plate_href(host_path, host_query, if sel0 > 0 { sel0 } else { 1 });
    let next_href = build_plate_href(host_path, host_query, if sel0 + 1 < n { sel0 + 2 } else { n });
    // JS actions read `$plate` live and clamp — never a fixed server target.
    // `viewer_id` is escaped for safe interpolation into these JS string
    // literals — see `escape_js_string_literal`'s doc comment.
    let viewer_id_js = escape_js_string_literal(viewer_id);
    let prev_click = format!(
        "if ($plate > 0) {{ $plate = $plate - 1; document.getElementById('{viewer_id_js}').goToPage($plate + 1); }}"
    );
    let next_click = format!(
        "if ($plate < {last}) {{ $plate = $plate + 1; document.getElementById('{viewer_id_js}').goToPage($plate + 1); }}"
    );

    html! {
        nav.pg aria-label=(nav_aria) {
            a   .pg__step
                href=(prev_href)
                aria-label=(prev_label)
                aria-disabled=[(sel0 == 0).then_some("true")]
                data-attr="{'aria-disabled': $plate <= 0 ? 'true' : 'false'}"
                data-on:click__prevent=(prev_click)
            { "←" }
            // `.pg__count` reflects the LIVE plate (`$plate + 1 / n`) so the
            // readout tracks JS paging, with the server-rendered `sel0+1 / n`
            // as the zero-JS/pre-hydration value.
            span
                .pg__count
                aria-live="polite"
                aria-atomic="true"
                data-text=(format!("($plate + 1) + ' / ' + {n}"))
            { (count_text) }
            a   .pg__step
                href=(next_href)
                aria-label=(next_label)
                aria-disabled=[(sel0 + 1 >= n).then_some("true")]
                data-attr=(format!("{{'aria-disabled': $plate >= {last} ? 'true' : 'false'}}"))
                data-on:click__prevent=(next_click)
            { "→" }
        }
    }
}

/// The record-viewer thumb strip (`.em-col__strip`). Like the pager it
/// reuses the plate's strip markup but drives the `$plate`/`goToPage` action.
/// The fold is client-side transient state (`$stripOpen`, defaulting to the
/// server's `?strip=`): the `ul.em-strip` is ALWAYS rendered (so the JS toggle
/// needs no re-render) and shown/hidden by `$stripOpen` — static `hidden` when
/// the server state is closed, reactive `data-attr` for the JS path. The toggle
/// is a real `?strip=` link (zero-JS full reload) with `__prevent` + a
/// `$stripOpen` flip for JS. Only called when `n > 1`.
fn render_rv_strip(
    items: &[Resource],
    sel0: usize,
    open: bool,
    viewer_id: &str,
    host_path: &str,
    host_query: Option<&str>,
    lang: &Lang,
) -> Markup {
    let n = items.len();
    let collapse_label = ChromeMsg::PlateStripCollapseLabel.text(lang).to_owned();
    let expand_label = format!("{} ({n})", ChromeMsg::PlateStripExpandLabel.text(lang));
    // The toggle's zero-JS href flips to the OPPOSITE server state; the visible
    // label + `aria-expanded` reflect the CURRENT state, reactively bound so the
    // JS path (which never reloads) stays in sync with `$stripOpen`.
    let toggle_target = if open { "0" } else { "1" };
    let toggle_qs = override_query_param(host_query, "strip", toggle_target);
    let toggle_href = format!("{host_path}?{toggle_qs}#record-figure");

    html! {
        div.em-col__strip {
            a   .em-strip-toggle
                href=(toggle_href)
                aria-expanded=(if open { "true" } else { "false" })
                data-attr="{'aria-expanded': $stripOpen ? 'true' : 'false'}"
                data-on:click__prevent="$stripOpen = !$stripOpen"
            {
                span.em-strip-toggle__mark { "▾" }
                // The label text follows `$stripOpen` (collapse when open, else
                // the "Show (n)" expand label). `js_dq_string` — not Rust's
                // `{:?}` Debug escaping — builds each JS string literal (Fix
                // 6: Debug escaping is not an exact JS-literal escape).
                span
                    data-text=({
                        format!(
                            "$stripOpen ? {collapse} : {expand}",
                            collapse = js_dq_string(&collapse_label),
                            expand = js_dq_string(&expand_label),
                        )
                    })
                { (if open { &collapse_label } else { &expand_label }) }
            }
            ul.em-strip hidden[!open] data-attr="{'hidden': !$stripOpen}" {
                @for (k, item) in items.iter().enumerate() {
                    li {
                        ({
                            render_rv_thumb(
                                item,
                                k,
                                n,
                                sel0,
                                viewer_id,
                                host_path,
                                host_query,
                                lang,
                            )
                        })
                    }
                }
            }
        }
    }
}

/// Render one `.em-thumb` in the record-viewer strip: a zero-JS `<a>` to the
/// target image, enhanced with the `$plate`/`goToPage` action. `k` is the
/// thumb's 0-based position; `sel0` the server-selected index. `aria-current`
/// (the selected-thumb highlight, `media.css` `.em-thumb[aria-current="true"]`)
/// is static for the initial paint and reactively bound to `$plate` so it
/// follows JS paging. `data-restricted` marks a limited-view thumb (dashed
/// border) — a static data-* hook, unchanged per image.
#[allow(clippy::too_many_arguments)]
fn render_rv_thumb(
    item: &Resource,
    k: usize,
    n: usize,
    sel0: usize,
    viewer_id: &str,
    host_path: &str,
    host_query: Option<&str>,
    lang: &Lang,
) -> Markup {
    let href = build_plate_href(host_path, host_query, k + 1);
    let is_current = k == sel0;
    let aria_current: Option<&str> = is_current.then_some("true");
    let restricted = find_bool_property(item, "limited-view").unwrap_or(false);
    let repr = image_repr(item);
    let aria_label = crate::chrome::rv_plate_of(k + 1, n, lang);

    html! {
        a   .em-thumb
            href=(href)
            aria-label=(aria_label)
            data-restricted=(if restricted { "true" } else { "false" })
            aria-current=[aria_current]
            data-attr=(format!("{{'aria-current': $plate == {k} ? 'true' : false}}"))
            data-on:click__prevent=(rv_page_click(k + 1, viewer_id))
        {
            @if let Some(repr) = repr {
                @let thumb_src = iiif_strip_thumb_url(&repr.uri)
                    .unwrap_or_else(|| repr.uri.clone());
                img src=(thumb_src) alt="" loading="lazy";
            }
        }
    }
}

/// Find `name`'s first `Boolean` value on `resource` (the plate access
/// line's `limited-view` reader).
pub(crate) fn find_bool_property(resource: &Resource, name: &str) -> Option<bool> {
    let property = resource.properties.iter().find(|p| p.name == name)?;
    property.values.iter().find_map(|v| match v {
        PropertyValue::Boolean(b) => Some(*b),
        _ => None,
    })
}

/// Find `name`'s first language-picked `Text`/`Richtext` value on `resource`,
/// non-empty (mirrors [`find_bool_property`], but
/// for the Image resource's own legal/descriptive EAV fields — `authorship`,
/// `copyright`, `license`, `description` — which are text-typed and therefore
/// subject to the same requested-lang → universal → any-present fallback
/// chain every other text property on the record uses (`pick_values`).
/// Richtext is returned as its raw markdown text — this crate does not render
/// markdown inside the viewer's bar/Details drawer (those fields are plain
/// mono prose, not a richtext flow).
pub(crate) fn find_text_property(resource: &Resource, name: &str, lang: &Lang) -> Option<String> {
    let property = resource.properties.iter().find(|p| p.name == name)?;
    crate::component::values::pick_values(property, lang)
        .into_iter()
        .find_map(|v| match v {
            PropertyValue::Text { text, .. } | PropertyValue::Richtext { text, .. } if !text.is_empty() => {
                Some(text.clone())
            }
            _ => None,
        })
}

/// Compose the picture's legally load-bearing run — author · copyright ·
/// licence, in that order — for the viewer bar's `.rv__legal .em-legal__run`.
///
/// Each field is omitted when absent or empty (never a dangling separator or
/// an empty leading entry). Copyright joins the run **only when it is present
/// and differs from the licence** (exact string comparison) — "No known
/// copyright restrictions" beside "Public domain" states the same fact twice;
/// a legal record that repeats itself teaches a reader to skim it. When
/// copyright and licence are equal only the licence is emitted.
pub(crate) fn compose_legal_run(author: Option<&str>, copyright: Option<&str>, license: Option<&str>) -> Vec<String> {
    let author = author.filter(|s| !s.is_empty());
    let copyright = copyright.filter(|s| !s.is_empty());
    let license = license.filter(|s| !s.is_empty());

    let mut run = Vec::with_capacity(3);
    if let Some(a) = author {
        run.push(a.to_owned());
    }
    if let Some(c) = copyright {
        if Some(c) != license {
            run.push(c.to_owned());
        }
    }
    if let Some(l) = license {
        run.push(l.to_owned());
    }
    run
}

/// Format a Representation's stored pixel dimensions for the viewer's Details
/// drawer DIMENSIONS row, e.g. `"690 × 1024 px"`.
///
/// `None` when either dimension is unknown or non-positive — the
/// `representations.width`/`.height` columns are nullable, and a 0×0 or
/// negative reading is not a real dimension to display.
pub(crate) fn image_dimensions(repr: &cpe_core::ir::Representation) -> Option<String> {
    match (repr.width, repr.height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => Some(format!("{w} × {h} px")),
        _ => None,
    }
}

/// Build the no-JS full-page-reload href for a plate navigation to the
/// 1-based `target` image, preserving every other query param.
fn build_plate_href(host_path: &str, host_query: Option<&str>, target: usize) -> String {
    let qs = override_query_param(host_query, "img", &target.to_string());
    format!("{host_path}?{qs}#record-figure")
}

#[cfg(test)]
mod tests {
    use cpe_core::ir::{Representation, RepresentationKind, Resource, SiteConfig};

    use super::*;

    /// The annotation signals have their own reset, which keeps the toggle.
    #[test]
    fn viewer_reset_carries_no_annotation_signal() {
        assert!(!viewer_reset_signals_json().contains("anno"));
    }

    fn lang() -> Lang {
        let site = SiteConfig {
            title: "Test".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        cpe_core::lang::resolve(&["en".to_owned()], &site)
    }

    fn image(id: &str, uri: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            class: "Image".to_owned(),
            properties: vec![],
            representations: vec![Representation {
                kind: RepresentationKind::Image,
                uri: uri.to_owned(),
                width: Some(1000),
                height: Some(1200),
                alt: Some("An illustration".to_owned()),
                attribution: Some("John Tenniel · Public Domain".to_owned()),
                seq: 0,
            }],
        }
    }

    /// Same as [`image`] but with an explicit `limited-view` boolean
    /// property, for the record viewer's ACCESS-line coverage.
    fn image_restricted(id: &str, uri: &str, restricted: bool) -> Resource {
        let mut img = image(id, uri);
        img.properties.push(cpe_core::ir::Property {
            name: "limited-view".to_owned(),
            values: vec![cpe_core::ir::PropertyValue::Boolean(restricted)],
        });
        img
    }

    // find_text_property

    fn image_with_text_props(id: &str, props: &[(&str, &str)]) -> Resource {
        let mut img = image(id, "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg");
        for (name, text) in props {
            img.properties.push(cpe_core::ir::Property {
                name: (*name).to_owned(),
                values: vec![PropertyValue::Text { text: (*text).to_owned(), lang: None }],
            });
        }
        img
    }

    #[test]
    fn find_text_property_returns_universal_value() {
        let img = image_with_text_props("I_001", &[("authorship", "John Tenniel")]);
        assert_eq!(find_text_property(&img, "authorship", &lang()).as_deref(), Some("John Tenniel"));
    }

    #[test]
    fn find_text_property_missing_property_returns_none() {
        let img = image("I_001", "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg");
        assert_eq!(find_text_property(&img, "authorship", &lang()), None);
    }

    #[test]
    fn find_text_property_empty_value_returns_none() {
        let img = image_with_text_props("I_001", &[("copyright", "")]);
        assert_eq!(find_text_property(&img, "copyright", &lang()), None);
    }

    #[test]
    fn find_text_property_reads_richtext_as_plain_text() {
        let mut img = image("I_001", "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg");
        img.properties.push(cpe_core::ir::Property {
            name: "description".to_owned(),
            values: vec![PropertyValue::Richtext {
                text: "A fearsome creature.".to_owned(),
                lang: Some("en".to_owned()),
            }],
        });
        assert_eq!(
            find_text_property(&img, "description", &lang()).as_deref(),
            Some("A fearsome creature.")
        );
    }

    // compose_legal_run

    #[test]
    fn compose_legal_run_all_present_and_distinct() {
        let run = compose_legal_run(
            Some("John Tenniel"),
            Some("No known copyright restrictions"),
            Some("Public domain"),
        );
        assert_eq!(run, vec!["John Tenniel", "No known copyright restrictions", "Public domain"]);
    }

    #[test]
    fn compose_legal_run_omits_copyright_when_equal_to_license() {
        // The real Alice case (CH_026 / I_051): copyright == license ==
        // "Public Domain" — stating the same fact twice teaches skimming.
        let run = compose_legal_run(Some("John Tenniel"), Some("Public Domain"), Some("Public Domain"));
        assert_eq!(
            run,
            vec!["John Tenniel", "Public Domain"],
            "copyright must be omitted when it duplicates the licence, got: {run:?}"
        );
    }

    #[test]
    fn compose_legal_run_keeps_copyright_when_license_absent() {
        let run = compose_legal_run(Some("John Tenniel"), Some("No known copyright restrictions"), None);
        assert_eq!(run, vec!["John Tenniel", "No known copyright restrictions"]);
    }

    #[test]
    fn compose_legal_run_missing_author_omits_it_without_leading_gap() {
        let run = compose_legal_run(None, None, Some("Public domain"));
        assert_eq!(
            run,
            vec!["Public domain"],
            "an absent author must not leave an empty leading entry, got: {run:?}"
        );
    }

    #[test]
    fn compose_legal_run_all_absent_is_empty() {
        assert_eq!(compose_legal_run(None, None, None), Vec::<String>::new());
    }

    #[test]
    fn compose_legal_run_treats_empty_strings_as_absent() {
        let run = compose_legal_run(Some(""), Some(""), Some("Public domain"));
        assert_eq!(run, vec!["Public domain"]);
    }

    #[test]
    fn compose_legal_run_only_copyright_present() {
        let run = compose_legal_run(None, Some("No known copyright restrictions"), None);
        assert_eq!(run, vec!["No known copyright restrictions"]);
    }

    // image_dimensions

    fn repr_with_size(width: Option<i64>, height: Option<i64>) -> cpe_core::ir::Representation {
        cpe_core::ir::Representation {
            kind: RepresentationKind::Image,
            uri: "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg".to_owned(),
            width,
            height,
            alt: None,
            attribution: None,
            seq: 0,
        }
    }

    #[test]
    fn image_dimensions_formats_both_present() {
        let repr = repr_with_size(Some(690), Some(1024));
        assert_eq!(image_dimensions(&repr).as_deref(), Some("690 × 1024 px"));
    }

    #[test]
    fn image_dimensions_missing_width_returns_none() {
        let repr = repr_with_size(None, Some(1024));
        assert_eq!(image_dimensions(&repr), None);
    }

    #[test]
    fn image_dimensions_missing_height_returns_none() {
        let repr = repr_with_size(Some(690), None);
        assert_eq!(image_dimensions(&repr), None);
    }

    #[test]
    fn image_dimensions_both_missing_returns_none() {
        let repr = repr_with_size(None, None);
        assert_eq!(image_dimensions(&repr), None);
    }

    #[test]
    fn image_dimensions_non_positive_returns_none() {
        let repr = repr_with_size(Some(0), Some(0));
        assert_eq!(image_dimensions(&repr), None);
    }

    // render_media_viewer

    /// Same shape as [`image_with_text_props`] but for the picture's legal
    /// EAV triple, for `render_viewer_bar`'s markup-integration coverage.
    fn image_with_legal(id: &str, uri: &str, author: &str, copyright: &str, license: &str) -> Resource {
        let mut img = image(id, uri);
        for (name, text) in [("authorship", author), ("copyright", copyright), ("license", license)] {
            img.properties.push(cpe_core::ir::Property {
                name: name.to_owned(),
                values: vec![PropertyValue::Text { text: text.to_owned(), lang: None }],
            });
        }
        img
    }

    #[test]
    fn render_media_viewer_iiif_uri_mounts_image_viewer_element() {
        // The positive path: an IIIF-shaped URI (`iiif_info_json_url` returns
        // `Some`) must mount the crate's `<image-viewer>` — not just the
        // `.rv__stand` fallback every other test in this module exercises.
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/3d3n0f1C5FZ.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("<image-viewer"),
            "an IIIF-shaped URI must mount <image-viewer>, got: {out}"
        );
        assert!(
            out.contains(r#"id="rv-I_051""#),
            "the mounted element must carry the stable rv-<image-id> id, got: {out}"
        );
        assert!(
            out.contains(
                "tile-sources=\"[&quot;https://iiif.demo.dasch.swiss:443/0854/3d3n0f1C5FZ.jpx/info.json&quot;]\""
            ),
            "tile-sources must be the derived info.json URL, got: {out}"
        );
        assert!(
            out.contains(r#"chrome="false""#),
            "the crate's own stock chrome must be suppressed, got: {out}"
        );
        assert!(
            out.contains(r#"auth-mode="none""#),
            "auth must be None (CPE is auth-free), got: {out}"
        );
        assert!(
            out.contains(r#"label="An illustration""#),
            "the accessible label must come from the representation's alt, got: {out}"
        );
        assert!(
            out.contains("data-on:viewer-zoom"),
            "the canvas wrapper must listen for the crate's bubbling viewer-zoom event, got: {out}"
        );
        assert!(
            !out.contains("rv__stand"),
            "the IIIF-mounted branch must not also render the plain-plate fallback wrapper, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_non_iiif_uri_degrades_to_plain_stand_no_image_viewer() {
        // The degradation path: a non-IIIF URI must never mount
        // <image-viewer> at all (never .expect() a Some tile-source).
        let items = vec![image("I_999", "https://example.org/img.jpg")];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            !out.contains("<image-viewer"),
            "a non-IIIF URI must never mount <image-viewer>, got: {out}"
        );
        assert!(
            out.contains("rv__stand"),
            "a non-IIIF URI must degrade to the plain .rv__stand fallback, got: {out}"
        );
        assert!(out.contains("<img src=\"https://example.org/img.jpg\""), "got: {out}");
    }

    #[test]
    fn render_media_viewer_composes_legal_run_into_bar_markup() {
        // Markup integration: the unit tests cover `compose_legal_run` as
        // a pure function; this confirms `render_viewer_bar` actually reads
        // the real `authorship`/`copyright`/`license` property names and
        // joins the result into `.rv__legal .em-legal__run` with the ` · `
        // separator glyph — a property-name typo or a broken join in
        // `render_viewer_bar` itself would not be caught by the pure-function
        // tests alone.
        let items = vec![image_with_legal(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
            "John Tenniel",
            "No known copyright restrictions",
            "Public domain",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("em-legal__run"),
            "a non-empty legal run must render .em-legal__run, got: {out}"
        );
        assert!(out.contains("John Tenniel"), "got: {out}");
        assert!(out.contains("No known copyright restrictions"), "got: {out}");
        assert!(out.contains("Public domain"), "got: {out}");
        assert!(
            out.contains("em-legal__dot"),
            "three run entries must be joined by the separator glyph, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_copyright_equal_license_omits_copyright_from_bar() {
        // The real Alice case (CH_026/I_051): copyright == license == "Public
        // Domain" must render once, not twice, in the actual bar markup.
        let items = vec![image_with_legal(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
            "John Tenniel",
            "Public Domain",
            "Public Domain",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert_eq!(
            out.matches("Public Domain").count(),
            1,
            "copyright must be omitted when it duplicates the licence, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_restricted_shows_access_line_in_bar() {
        let items = vec![image_restricted(
            "I_093",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
            true,
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(out.contains("em-access"), "got: {out}");
        assert!(out.contains(r#"data-restricted="true""#), "got: {out}");
    }

    #[test]
    fn render_media_viewer_zoom_chip_wires_datastar_click_and_scale_readout() {
        // DataStar must drive the
        // crate's JS control surface (zoomIn) and read its viewer-zoom event
        // into the .rv__scale readout.
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("data-on:click=\"document.getElementById('rv-I_051').zoomIn()\""),
            "the zoom chip must call zoomIn() on the mounted viewer's stable id, got: {out}"
        );
        assert!(
            out.contains(r#"data-text="$scale""#),
            "the scale readout must be bound to the $scale signal, got: {out}"
        );
        assert!(
            out.contains(">FIT<"),
            "the scale readout's SSR/pre-upgrade text must be FIT, got: {out}"
        );
    }

    /// A resource id containing a single quote and a backslash must never
    /// break out of (or corrupt) the JS string literal
    /// `document.getElementById('rv-{id}')` is interpolated into. Every such
    /// interpolation site must escape via `escape_js_string_literal`, not
    /// merely assume the ids-are-alphanumeric convention holds.
    #[test]
    fn render_media_viewer_zoom_chip_escapes_hostile_resource_id() {
        let items = vec![image(
            r"I'05\1",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains(r"document.getElementById('rv-I\'05\\1').zoomIn()"),
            "the id's ' and \\ must be backslash-escaped in the JS string literal, got: {out}"
        );
        assert!(
            !out.contains(r"getElementById('rv-I'05\1')"),
            "an unescaped single quote must never appear inside the JS string literal \
             (it would close the string early), got: {out}"
        );
    }

    // render_media_viewer zoom cluster

    #[test]
    fn render_media_viewer_zoom_out_and_home_chips_wire_datastar_click() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("data-on:click=\"document.getElementById('rv-I_051').zoomOut()\""),
            "the zoom-out chip must call zoomOut() on the mounted viewer's stable id, got: {out}"
        );
        assert!(
            out.contains("data-on:click=\"document.getElementById('rv-I_051').goHome()\""),
            "the fit/home chip must call goHome() on the mounted viewer's stable id, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_zoom_chips_bind_disabled_state_to_zoom_signals() {
        // Three distinct edges — zoom-out disables at $atMin (the zoom-out
        // floor, 0.9× fit), fit/home at $atHome (at fit), zoom-in at $atMax (max
        // magnification, NOT $atNative — magnifying past screen-1:1 up to
        // maxZoomPixelRatio is allowed). Ties each disabled-state binding to its
        // OWN chip's `data-on:click` (not just an aggregate count) so a mapping
        // swap — e.g. zoom-in wrongly tied to $atHome — cannot pass this test.
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains(
                "data-on:click=\"document.getElementById('rv-I_051').zoomOut()\" \
                 data-attr=\"{'aria-disabled': $atMin ? 'true' : 'false'}\""
            ),
            "zoom-out must disable specifically at $atMin (the zoom-out floor), got: {out}"
        );
        assert!(
            out.contains(
                "data-on:click=\"document.getElementById('rv-I_051').goHome()\" \
                 data-attr=\"{'aria-disabled': $atHome ? 'true' : 'false'}\""
            ),
            "fit/home must disable specifically at $atHome (at fit), got: {out}"
        );
        assert!(
            out.contains(
                "data-on:click=\"document.getElementById('rv-I_051').zoomIn()\" \
                 data-attr=\"{'aria-disabled': $atMax ? 'true' : 'false'}\""
            ),
            "zoom-in must disable at $atMax (max magnification), not $atNative, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_zoom_chips_carry_title_and_aria_label() {
        // Every icon-only chip carries both `title` and `aria-label`
        // (the accessible name survives the container-query icon-only
        // collapse, which hides the chip's text `span` but never touches
        // `title`/`aria-label`).
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        for label in ["Zoom out", "Fit to view", "Zoom in"] {
            assert!(
                out.contains(&format!("title=\"{label}\" aria-label=\"{label}\"")),
                "chip labelled {label:?} must carry both title and aria-label, got: {out}"
            );
        }
    }

    #[test]
    fn render_media_viewer_degraded_canvas_omits_zoom_cluster_and_error_line() {
        // When render_viewer_canvas degrades to the plain .rv__stand
        // fallback (no mounted <image-viewer>), the zoom chips' data-on:click
        // targets a getElementById that resolves to null and their
        // `aria-disabled` bindings (object-form `data-attr`) read signals that were never
        // declared (falsy — rendering as enabled) — so the zoom cluster and
        // the viewer-error line, which can only ever be populated by that
        // missing element's events, must be entirely absent, not merely
        // inert-looking. `.rv__ctrls` itself still renders — it
        // hosts the Details toggle, which only needs `repr`/EAV data, not a
        // mounted viewer.
        let items = vec![image("I_999", "https://example.org/img.jpg")];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            !out.contains("<image-viewer"),
            "sanity: this is the non-IIIF degraded-canvas branch, got: {out}"
        );
        assert!(
            out.contains("rv__ctrls"),
            "a degraded canvas must still render .rv__ctrls for the Details toggle \
             (Fix 8), got: {out}"
        );
        assert!(
            !out.contains("rv__scale") && !out.contains("vgroup"),
            "a degraded canvas must not render the scale readout or zoom chip group, got: {out}"
        );
        assert!(
            !out.contains("zoomOut()") && !out.contains("zoomIn()") && !out.contains("goHome()"),
            "a degraded canvas must not wire any zoom control, got: {out}"
        );
        assert!(
            !out.contains("rv__err"),
            "a degraded canvas must not render the viewer-error line (it can never fire), got: {out}"
        );
        assert!(
            !out.contains("aria-disabled"),
            "a degraded canvas must not bind disabled-state to undeclared signals, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_declares_at_home_and_at_native_signal_defaults() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("data-signals=\"{atHome: true, atNative: false, atMin: true, atMax: false, viewerError: false, scale: 'FIT'}\""),
            "the canvas must declare atHome/atNative/atMin/viewerError/scale signal defaults \
             (atMin must default true — MIN_HOME_ZOOM_RATIO is 1.0, the zoom-out floor \
             IS fit, same as atHome; scale must default 'FIT' so the static readout text and \
             the reactive $scale signal agree before the first viewer-zoom event — owner \
             checkpoint round C), got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_viewer_error_line_is_wired_but_hidden_by_default() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("data-on:viewer-error=\"$viewerError = true\""),
            "the canvas must listen for the crate's viewer-error event, got: {out}"
        );
        // Pin the actual `.rv__err` markup as one contiguous
        // substring — the previous three-way `&&` (checking `"class=\"rv__err\""`,
        // `"hidden"`, and the `data-attr` binding separately) was trivially
        // satisfied by the two rails' unrelated `aria-hidden="..."` attributes
        // appearing elsewhere in the rendered output, so it didn't actually pin
        // the static-`hidden`-attribute fix on the `.rv__err` element itself.
        assert!(
            out.contains(
                "rv__err\" aria-live=\"polite\" aria-atomic=\"true\" hidden \
                 data-attr=\"{'hidden': !$viewerError}\""
            ),
            "the quiet error line must be present, hidden by default via a static \
             `hidden` attribute + reactive data-attr (never data-show — AC17), got: {out}"
        );
        assert!(
            out.contains("couldn't load"),
            "the quiet error line must carry the translated chrome string, got: {out}"
        );
    }

    // render_viewer_canvas drawers

    #[test]
    fn render_media_viewer_rail_signal_declared_on_rv_with_reactive_class_binding() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        // Drawer-open state is a reactive CLASS toggle, not a reactive `data-*`
        // attribute (DataStar owns the `data-*` namespace — writing into it
        // derails its binding pass; see `render_media_viewer`). No static
        // `data-rail` remains — drawers are transient client-side only.
        assert!(
            !out.contains(r#"data-rail="#),
            "no data-rail attribute must remain (superseded by the .rail-* class toggle), got: {out}"
        );
        assert!(
            out.contains(r#"data-class="{'rail-adjust': $rail === 'adjust', 'rail-details': $rail === 'details'}""#),
            "the drawer-open state must be a reactive class toggle on .rv, got: {out}"
        );
        assert!(
            out.contains(
                "data-signals=\"{rail:'closed', expand:false, plate:0, stripOpen:false, brightness:1, \
                 contrast:1, saturation:1, exposure:0, gamma:1, hue:0, invert:false}\""
            ),
            "the .rv-level data-signals must declare rail, expand, plate, stripOpen, plus all 7 adjust defaults, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_wider_toggle_is_a_maximize_chip_toggling_expand() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        // WIDER flips the `$expand` boolean; the article reacts via `data-class`
        // (resource_detail.rs). It is a plain reactive click, NOT a reactive
        // `data-*` attribute write (which would derail DataStar's binding pass).
        assert!(
            out.contains(r#"data-on:click="$expand = !$expand""#),
            "the WIDER chip must toggle the $expand signal on click, got: {out}"
        );
        // aria-pressed conveys the toggle state (constant label + reactive state,
        // mirroring the Details/Adjust toggles).
        assert!(
            out.contains(r#"data-attr="{'aria-pressed': $expand ? 'true' : 'false'}""#),
            "the WIDER chip must bind aria-pressed to $expand, got: {out}"
        );
        // Paired with the maximize glyph + an accessible name (icon-only chip).
        assert!(
            out.contains("id=\"rv-I_051-wider-toggle\""),
            "the WIDER chip must carry its stable id, got: {out}"
        );
        assert!(
            out.contains("M6 2.6H2.6V6M10 2.6h3.4V6M13.4 10v3.4H10M6 13.4H2.6V10"),
            "the WIDER chip must render the maximize glyph, got: {out}"
        );
        assert!(
            out.contains(r#"title="Wider view" aria-label="Wider view""#),
            "the WIDER chip must carry a title + aria-label accessible name, got: {out}"
        );
    }

    // sequence paging

    fn three_iiif_images() -> Vec<Resource> {
        vec![
            image("I_001", "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg"),
            image("I_002", "https://iiif.demo.dasch.swiss:443/0854/b.jpx/full/max/0/default.jpg"),
            image("I_003", "https://iiif.demo.dasch.swiss:443/0854/c.jpx/full/max/0/default.jpg"),
        ]
    }

    #[test]
    fn render_media_viewer_multi_image_mounts_one_sequence_viewer_with_pager_and_strip() {
        let items = three_iiif_images();
        let out = render_media_viewer(&items, "Alice", 2, "/p/CH_001", None, false, &lang()).into_string();
        // ONE multi-source `<image-viewer>` for the whole sequence — not one per
        // image (paging swaps the page WITHIN the element via goToPage).
        assert_eq!(
            out.matches("<image-viewer").count(),
            1,
            "exactly one multi-source viewer must mount, got: {out}"
        );
        // The viewer's own pager + strip render for a multi-image host.
        assert!(out.contains("class=\"pg\""), "the .pg pager must render, got: {out}");
        assert!(out.contains("em-col__strip"), "the thumb strip must render, got: {out}");
    }

    #[test]
    fn render_media_viewer_sequence_pages_client_side_never_url_or_fragment() {
        let items = three_iiif_images();
        let out = render_media_viewer(&items, "Alice", 1, "/p/CH_001", None, false, &lang()).into_string();
        // Pager/strip set `$plate` and call `goToPage` on the element…
        assert!(out.contains("$plate = "), "must set the $plate signal, got: {out}");
        assert!(out.contains(").goToPage("), "must page the element via goToPage, got: {out}");
        // …and NEVER `@get` a `_plate` fragment endpoint, and
        // NEVER write the URL / history (`dsp-incubator
        // cpe/docs/adr/0007-fragment-swaps-are-transient.md`).
        assert!(
            !out.contains("_plate"),
            "must not use the _plate @get fragment endpoint, got: {out}"
        );
        assert!(
            !out.contains("pushState") && !out.contains("replaceState"),
            "must not write the URL/history, got: {out}"
        );
        // The zero-JS fallback is still a real `?img=N` full-reload href.
        assert!(
            out.contains("?img=2#record-figure"),
            "the pager must carry a real ?img= zero-JS href, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_sequence_pre_renders_toggled_per_image_blocks() {
        let items = three_iiif_images();
        // selected = 2 (1-based) → sel0 = 1.
        let out = render_media_viewer(&items, "Alice", 2, "/p/CH_001", None, false, &lang()).into_string();
        // Every image's legal/Details/caption block is pre-rendered and toggled
        // by `$plate` via the object-form `data-attr` — never the
        // hyphen-suffix `data-attr-hidden` (silently ignored) or `data-show`.
        for i in 0..3 {
            assert!(
                out.contains(&format!("data-attr=\"{{'hidden': $plate != {i}}}\"")),
                "block {i} must carry the reactive plate toggle, got: {out}"
            );
        }
        assert!(
            !out.contains("data-attr-hidden") && !out.contains("data-show"),
            "must use object-form data-attr, not the ignored hyphen/`data-show` forms, got: {out}"
        );
        // Zero-JS: the NON-selected blocks carry a static `hidden`; the selected
        // one (sel0 = 1) does not — so with JS off only image 2 shows.
        assert!(
            out.contains("plate 1 of 3") && out.contains("plate 2 of 3") && out.contains("plate 3 of 3"),
            "each image's Details must carry its `plate i of n` identifier, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_single_image_has_no_sequence_wiring() {
        let items = vec![image(
            "I_001",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/p/CH_001", None, false, &lang()).into_string();
        assert!(
            !out.contains("class=\"pg\"") && !out.contains("em-col__strip"),
            "a single image must carry no pager/strip, got: {out}"
        );
        assert!(
            !out.contains("$plate != "),
            "a single image must carry no per-image plate toggle, got: {out}"
        );
        assert!(
            !out.contains("plate 1 of 1"),
            "a lone image must not carry a `plate i of n` suffix, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_strip_toggle_is_client_side_stripopen_signal() {
        let items = three_iiif_images();
        let out = render_media_viewer(&items, "Alice", 1, "/p/CH_001", None, false, &lang()).into_string();
        // The strip fold is a client-side `$stripOpen` toggle (zero-JS `?strip=`
        // href fallback), never a `_plate`/`@get` reload.
        assert!(
            out.contains("$stripOpen = !$stripOpen"),
            "the strip toggle must flip the $stripOpen signal, got: {out}"
        );
        assert!(
            out.contains(r#"data-attr="{'hidden': !$stripOpen}""#),
            "the strip list visibility must bind to $stripOpen, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_details_and_adjust_toggles_write_the_shared_rail_signal() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("$rail = open ? 'details' : 'closed'"),
            "the Details toggle must flip $rail via a toggle-or-close ternary \
             (plus a focus-restore setTimeout when opening, Fix 4), got: {out}"
        );
        assert!(
            out.contains("$rail = open ? 'adjust' : 'closed'"),
            "the Adjust toggle must flip $rail via the mirror expression — mutual exclusion \
             follows from the single-valued signal, not markup coordination, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_details_drawer_renders_fields_and_never_bar_only_legal_fields() {
        let mut img = image_with_legal(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
            "John Tenniel",
            "No known copyright restrictions",
            "Public domain",
        );
        img.properties.push(cpe_core::ir::Property {
            name: "description".to_owned(),
            values: vec![PropertyValue::Text { text: "A fearsome creature.".to_owned(), lang: None }],
        });
        let items = vec![img];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();

        let details_start = out
            .find("rv__rail--details")
            .unwrap_or_else(|| panic!("the Details rail must render, got: {out}"));
        let details_end = out
            .find("rv__rail--adjust")
            .unwrap_or_else(|| panic!("the Adjust rail must render after Details, got: {out}"));
        let details_html = &out[details_start..details_end];

        assert!(
            !details_html.contains("A fearsome creature."),
            "the Details drawer must NOT duplicate the description (Fix 9) — the plate \
             caption below the frame already shows it, got: {details_html}"
        );
        assert!(
            details_html.contains("I_051"),
            "the IDENTIFIER row must show the image id, got: {details_html}"
        );
        assert!(
            details_html.contains("1000 × 1200 px"),
            "the DIMENSIONS row must render the representation's dimensions, got: {details_html}"
        );
        assert!(
            details_html.contains("a.jpx"),
            "the IIIF SOURCE row must render the representation's raw uri, got: {details_html}"
        );
        assert!(
            !details_html.contains("John Tenniel")
                && !details_html.contains("No known copyright restrictions")
                && !details_html.contains("Public domain"),
            "the Details drawer must never duplicate the bar's author/copyright/licence \
             fields (I4), got: {details_html}"
        );
    }

    #[test]
    fn render_media_viewer_details_drawer_shows_restricted_access_prose() {
        let items = vec![image_restricted(
            "I_093",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
            true,
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("rf__row--access"),
            "a restricted image's Details drawer must render the ACCESS row, got: {out}"
        );
        assert!(
            out.contains("reduced resolution"),
            "the ACCESS row must carry the longer restricted-view prose \
             (RvAccessRestrictedLong), got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_adjust_numeric_rows_wire_units_bridged_setters() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(out.contains("setBrightness($brightness * 100)"), "got: {out}");
        assert!(out.contains("setContrast($contrast * 100)"), "got: {out}");
        assert!(out.contains("setSaturation($saturation * 100)"), "got: {out}");
        assert!(
            out.contains("setExposure($exposure)") && !out.contains("setExposure($exposure * 100)"),
            "exposure must be pass-through, not percent-bridged, got: {out}"
        );
        assert!(
            out.contains("setGamma($gamma)") && !out.contains("setGamma($gamma * 100)"),
            "gamma must be pass-through, not percent-bridged, got: {out}"
        );
        assert!(
            out.contains("setHue($hue)") && !out.contains("setHue($hue * 100)"),
            "hue must be pass-through, not percent-bridged, got: {out}"
        );
        assert!(
            out.contains("data-bind=\"brightness\"")
                && out.contains("data-bind=\"contrast\"")
                && out.contains("data-bind=\"saturation\"")
                && out.contains("data-bind=\"exposure\"")
                && out.contains("data-bind=\"gamma\"")
                && out.contains("data-bind=\"hue\""),
            "each numeric row's <input> must two-way-bind its own signal, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_invert_is_a_chip_not_a_track() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("adj--flag"),
            "invert must render inside the flag-row wrapper, got: {out}"
        );
        assert!(
            out.contains("$invert = !$invert; document.getElementById('rv-I_051').setInvert($invert)"),
            "invert's chip click must toggle the signal and call setInvert, got: {out}"
        );
        let invert_row_idx = out.find("adj--flag").expect("invert row must render");
        let invert_row_tail = &out[invert_row_idx..];
        let note_offset = invert_row_tail.find("rv__rail-note").unwrap_or(invert_row_tail.len());
        assert!(
            !invert_row_tail[..note_offset].contains("adj__in"),
            "invert must not render a range track, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_reset_all_resets_every_signal_and_calls_reset_adjust() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains(
                "$brightness = 1; $contrast = 1; $saturation = 1; $exposure = 0; \
                 $gamma = 1; $hue = 0; $invert = false; \
                 document.getElementById('rv-I_051').resetAdjust()"
            ),
            "RESET ALL must reset all 7 signals AND call resetAdjust(), got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_adjust_note_renders_in_rail_note_after_the_rows() {
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("rv__rail-note"),
            "the transient note must render in .rv__rail-note, got: {out}"
        );
        assert!(
            out.contains("carry across plates"),
            "the note text must be the transient-adjustments sentence, got: {out}"
        );
        let hue_idx = out.find("setHue").expect("hue row must render");
        let note_idx = out.find("rv__rail-note").expect("note must render");
        assert!(
            hue_idx < note_idx,
            "the note must render after the adjust rows, not before, got: {out}"
        );
    }

    // Details toggle, dimming, closed rails, accessible names, range bounds

    #[test]
    fn render_media_viewer_details_toggle_has_no_glyph_adjust_toggle_does() {
        // "The bar's Details toggle carries no glyph (mono label — the
        // six-glyph set is scoped to controls that act on the picture)."
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        let details_idx = out
            .find("$rail = open ? 'details' : 'closed'")
            .expect("Details toggle must render");
        let adjust_idx = out
            .find("$rail = open ? 'adjust' : 'closed'")
            .expect("Adjust toggle must render");
        assert!(
            details_idx < adjust_idx,
            "expected the Details toggle to render before the Adjust toggle, got: {out}"
        );
        let details_to_adjust = &out[details_idx..adjust_idx];
        assert!(
            !details_to_adjust.contains("class=\"ico\""),
            "the Details toggle must carry no glyph (mono label only, AC5), got: {details_to_adjust}"
        );
        let after_adjust = &out[adjust_idx..];
        let adjust_button_end = after_adjust.find("</button>").unwrap_or(after_adjust.len());
        assert!(
            after_adjust[..adjust_button_end].contains("class=\"ico\""),
            "the Adjust toggle must carry the sliders glyph, got: {after_adjust}"
        );
    }

    #[test]
    fn render_media_viewer_adjust_rows_bind_untouched_class_to_their_own_default() {
        // "data-touched dimming" — each of the seven rows dims when at its
        // default, via a reactive `.untouched` CLASS toggle (NOT a reactive
        // `data-*` attribute, which would derail DataStar's binding pass — see
        // `render_media_viewer`). Each must test its OWN default (a shared/
        // incorrect default would silently mis-dim every row except matches).
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        for expr in [
            "data-class=\"{untouched: $brightness === 1}\"",
            "data-class=\"{untouched: $contrast === 1}\"",
            "data-class=\"{untouched: $saturation === 1}\"",
            "data-class=\"{untouched: $exposure === 0}\"",
            "data-class=\"{untouched: $gamma === 1}\"",
            "data-class=\"{untouched: $hue === 0}\"",
            "data-class=\"{untouched: $invert === false}\"",
        ] {
            assert!(
                out.contains(expr),
                "expected {expr:?} to dim the row via .untouched at its own default, got: {out}"
            );
        }
    }

    #[test]
    fn render_media_viewer_closed_rails_carry_aria_hidden_binding() {
        // "aria-hidden when closed".
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("data-attr=\"{'aria-hidden': $rail !== 'details' ? 'true' : 'false'}\""),
            "the Details rail must bind aria-hidden to $rail, got: {out}"
        );
        assert!(
            out.contains("data-attr=\"{'aria-hidden': $rail !== 'adjust' ? 'true' : 'false'}\""),
            "the Adjust rail must bind aria-hidden to $rail, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_toggles_and_invert_chip_bind_aria_pressed() {
        // aria-pressed on every toggle, including the invert chip.
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        assert!(
            out.contains("data-attr=\"{'aria-pressed': $rail === 'details' ? 'true' : 'false'}\""),
            "the Details toggle must bind aria-pressed, got: {out}"
        );
        assert!(
            out.contains("data-attr=\"{'aria-pressed': $rail === 'adjust' ? 'true' : 'false'}\""),
            "the Adjust toggle must bind aria-pressed, got: {out}"
        );
        assert!(
            out.contains("data-attr=\"{'aria-pressed': $invert ? 'true' : 'false'}\""),
            "the invert chip must bind aria-pressed to $invert, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_rail_close_buttons_carry_title_and_aria_label() {
        // Icon-only chips carry title + aria-label — the two rails'
        // `x` close controls are icon-only.
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        let close_label = ChromeMsg::RvDrawerCloseLabel.text(&lang());
        assert_eq!(
            out.matches(&format!("title=\"{close_label}\" aria-label=\"{close_label}\""))
                .count(),
            2,
            "both rails' close buttons must carry title+aria-label, got: {out}"
        );
    }

    #[test]
    fn render_media_viewer_adjust_rows_carry_design_pinned_range_bounds() {
        // The slider min/max/step per operator — a typo'd
        // bound is invisible to the units-bridge setter-call assertions
        // alone.
        let items = vec![image(
            "I_051",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        for bounds in [
            r#"type="range" min="0.4" max="1.6" step="0.01""#, // brightness (first match)
            r#"type="range" min="0" max="2" step="0.01""#,     // saturation
            r#"type="range" min="-1" max="1" step="0.01""#,    // exposure
            r#"type="range" min="0.5" max="2" step="0.01""#,   // gamma
            r#"type="range" min="-180" max="180" step="1""#,   // hue
        ] {
            assert!(out.contains(bounds), "expected an <input> with {bounds:?}, got: {out}");
        }
        // brightness AND contrast share the same 0.4–1.6/0.01 bounds — assert
        // there are two such inputs (not just one accidentally reused twice).
        assert_eq!(
            out.matches(r#"type="range" min="0.4" max="1.6" step="0.01""#).count(),
            2,
            "brightness and contrast must both carry the 0.4–1.6 bounds, got: {out}"
        );
    }

    // Snapshot matrix
    //
    // A representative matrix, not a full cross-product: open
    // vs restricted, single vs sequence (CH_001-shaped — several images incl.
    // one `limited-view`, mirroring the real Alice demo's `?img=56` plate),
    // EN vs DE. Drawer/expand/zoom states are client-side signals that don't
    // change the initial server-rendered markup, so they're exercised only by
    // the unit tests above, not chased here. `data-appearance` (dark mode)
    // never appears in this module's own output — it's emitted once on
    // `<html>` by `render_page` — so the "one dark" axis is covered by a
    // full-page snapshot in `dsp-incubator cpe/web`'s `page` module instead.

    fn lang_de() -> Lang {
        let site = SiteConfig {
            title: "Test".to_owned(),
            description: None,
            cover_image: None,
            image_credit: None,
            eyebrow: None,
            languages: vec!["en".to_owned(), "de".to_owned()],
            default_language: "en".to_owned(),
            nav: vec![],
            chrome: vec![],
            property_labels: vec![],
            class_labels: vec![],
            theme: cpe_core::theme::ThemeKnob::default(),
            date_precision_word: false,
            footer_attribution: None,
            not_found_browse: vec![],
            narrative_companion: Default::default(),
            narrative_side: Default::default(),
            text_align: Default::default(),
        };
        cpe_core::lang::resolve(&["de".to_owned()], &site)
    }

    /// A small CH_001-shaped sequence: several IIIF-viewable images, one of
    /// them `limited-view` (mirroring the real demo's restricted plate) — big
    /// enough to exercise sequence/pager/strip wiring without a 57-image
    /// snapshot.
    fn sequence_items() -> Vec<Resource> {
        vec![
            image("I_051", "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg"),
            image("I_052", "https://iiif.demo.dasch.swiss:443/0854/b.jpx/full/max/0/default.jpg"),
            image_restricted(
                "I_056",
                "https://iiif.demo.dasch.swiss:443/0854/c.jpx/full/max/0/default.jpg",
                true,
            ),
        ]
    }

    #[test]
    fn snapshot_render_media_viewer_single_open_en() {
        let items = vec![image(
            "I_001",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_media_viewer_single_open_de() {
        let items = vec![image(
            "I_001",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang_de()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_media_viewer_single_restricted_en() {
        let items = vec![image_restricted(
            "I_056",
            "https://iiif.demo.dasch.swiss:443/0854/c.jpx/full/max/0/default.jpg",
            true,
        )];
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, false, &lang()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_media_viewer_sequence_en() {
        let items = sequence_items();
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, true, &lang()).into_string();
        insta::assert_snapshot!(out);
    }

    #[test]
    fn snapshot_render_media_viewer_sequence_de() {
        let items = sequence_items();
        let out = render_media_viewer(&items, "Alice", 1, "/host", None, true, &lang_de()).into_string();
        insta::assert_snapshot!(out);
    }

    /// A non-IIIF (but still safe `https`) image representation — one whose
    /// URI has no `/full/max/` segment, so [`iiif_info_json_url`] returns
    /// `None` for it (see `iiif.rs`'s `iiif_info_json_url_non_iiif_passthrough_returns_none`).
    /// Used to exercise `has_sequence_pager`'s "every item IIIF-sequenceable"
    /// condition against a mixed set.
    fn non_iiif_image(id: &str, uri: &str) -> Resource {
        image(id, uri)
    }

    #[test]
    fn has_sequence_pager_false_for_zero_images() {
        assert!(!has_sequence_pager(&[]));
    }

    #[test]
    fn has_sequence_pager_false_for_one_image() {
        let items = vec![image(
            "I_001",
            "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg",
        )];
        assert!(!has_sequence_pager(&items));
    }

    #[test]
    fn has_sequence_pager_true_for_multiple_all_iiif_images() {
        let items = vec![
            image("I_001", "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg"),
            image("I_002", "https://iiif.demo.dasch.swiss:443/0854/b.jpx/full/max/0/default.jpg"),
            image("I_003", "https://iiif.demo.dasch.swiss:443/0854/c.jpx/full/max/0/default.jpg"),
        ];
        assert!(has_sequence_pager(&items));
    }

    #[test]
    fn has_sequence_pager_false_for_mixed_iiif_and_non_iiif_images() {
        let items = vec![
            image("I_001", "https://iiif.demo.dasch.swiss:443/0854/a.jpx/full/max/0/default.jpg"),
            non_iiif_image("I_002", "https://example.org/img/plain.jpg"),
        ];
        assert!(
            !has_sequence_pager(&items),
            "a set with any non-IIIF-shaped image must not offer the sequence pager"
        );
    }
}
