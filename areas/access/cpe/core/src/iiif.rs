//! IIIF Image API URL helpers.
//!
//! `translate.py` (and every project's `translate.*`) stores a Representation's
//! `uri` as the canonical full-resolution IIIF Image API request:
//! `{base}/{identifier}/full/max/0/default.jpg`, e.g.
//! `https://iiif.demo.dasch.swiss:443/0854/6IQRqvpLkz5-XJnRjN1tjD9.jpx/full/max/0/default.jpg`
//! (verified against `cpe/projects/alice-in-daschland/data.sql`'s `representations`
//! table). Every helper here rewrites or derives from that `/full/max/` size
//! segment; a URI that doesn't contain it (a non-IIIF URL, an already-sized
//! URL, a placeholder) degrades gracefully — see each function's doc comment —
//! rather than panicking or producing a malformed URL.
//!
//! This module is the single home for IIIF URL derivation; don't duplicate
//! the `/full/max/` parsing elsewhere.

use std::borrow::Cow;

/// Width (px) requested for list thumbnails via the IIIF Image API `size`
/// parameter. The display box is 6rem (96px, see `.rl-annot__thumb` in
/// grid.css); 384 is 4x that, so the image stays crisp up to 3x-DPI screens
/// while still downloading a few KB instead of the multi-MB full image.
/// At ≤640px the phone row layout widens the box to 7.5rem (120px); 384 is
/// still 3.2x that.
///
/// Keep this in step with `.rl-annot__thumb`'s width — the box was enlarged
/// twice during the owner's responsive pass and a stale value here shows up
/// as soft thumbnails on HiDPI, which is easy to miss on a 1x display.
pub(crate) const THUMB_WIDTH: u32 = 384;

/// Width (px) requested, via the IIIF Image API `size` parameter, for a
/// viewer's zero-JS fallback image and for the compound reader's member scan.
///
/// Its CSS boxes (see the "display-size CSS box needs a paired fetch-size
/// constant" Non-obvious constraint in `cpe/CLAUDE.md`):
/// - the book reader's stage (`--stage-h`, `cpe/static/cpe/record.css`). A book page is portrait,
///   so the stage's height binds it: 900 covers a page at 1× up to the stage's 92rem ceiling, and
///   no further;
/// - the record viewer's stand (`component::media`), where a landscape image is bound by the
///   column's width instead;
/// - the member scan (`compound_viewer::render_member_scan`, `.cv-member__scan img` in
///   `record.css`: 30rem tall, column-wide).
pub(crate) const PLATE_WIDTH: u32 = 900;

/// Width (px) requested for embedded-media strip thumbnails via the IIIF
/// Image API `size` parameter.
///
/// Paired with the strip-thumb CSS box in `cpe/static/cpe/media.css`, which
/// renders at 52px — 208 is 4x that, deliberately NOT the reference design's
/// `96`, so the thumbnail stays crisp up to Retina/3x-DPI screens (mirrors
/// `THUMB_WIDTH`'s rationale above; the reference design's `96` was sized
/// for a 24px display box, which does not match this port's 52px strip-thumb
/// box).
pub(crate) const STRIP_THUMB_WIDTH: u32 = 208;

/// Width (px) requested for grid-mode tile images (`.gm__box`) via the IIIF
/// Image API `size` parameter.
///
/// Paired with the `.gm__box` CSS box in `cpe/static/cpe/grid.css`: change
/// both together. The box's `minmax(--tile-min, 1fr)` track has a floor of at
/// most 16rem (256px) and stretches a tile until one more column fits. With
/// three or more columns and the 24px `--grid-gap` the widest tile is
/// `(4 × 256 + 24) / 3`, about 349px; a phone's two columns are about 167px.
/// 512 is 2× at the floor and at least 1.47× at the widest tile. It assumes a
/// device pixel ratio up to 2 and no `srcset`.
///
/// Its own constant, not [`THUMB_WIDTH`]: each fetch size pairs with exactly
/// one CSS box, and `.gm__box` is not `.rl-annot__thumb`.
pub(crate) const GRID_TILE_WIDTH: u32 = 512;

/// Largest side (px) requested for a tall grid tile, paired with the tall box
/// (`.gm[data-tile="tall"] .gm__box`, `cpe/static/cpe/grid.css`), whose height
/// is capped at 70svh: 1× on a 1440px-tall window, 1.6× on a 900px one.
pub(crate) const GRID_TALL_FETCH_PX: i64 = 1000;

/// Clamp a candidate IIIF request width to the target image's own stored
/// pixel width: `min(wanted, stored)` when `stored` is a known positive
/// width, else `wanted` unclamped. Shared by `iiif_thumb_url_clamped`/
/// `iiif_grid_tile_url_clamped`/`iiif_plate_url`.
fn clamp_width(wanted: u32, stored: Option<i64>) -> u32 {
    match stored {
        Some(stored) if stored > 0 => {
            let stored = u32::try_from(stored).unwrap_or(wanted);
            wanted.min(stored)
        }
        _ => wanted,
    }
}

/// Rewrite a IIIF Image API URL to request a browse-row thumbnail, clamped to
/// the target image's own stored pixel width: `/full/<min(THUMB_WIDTH,
/// w)>,/` when `w` is known and positive, else the unclamped `THUMB_WIDTH`.
/// `translate.py` emits canonical `…/full/max/0/default.jpg` URLs; this
/// swaps the `/full/max/` size segment, e.g. `…/full/max/0/default.jpg` →
/// `…/full/384,/0/default.jpg`. IIIF v3 (Sipi) rejects a size request wider
/// than the image's own native width (400), so a thumbnail fetch must never
/// ask for more than the image actually has — see `crate::ir::MediaThumb`'s
/// doc comment.
///
/// Any URL that does not contain the `/full/max/` segment (non-IIIF URLs,
/// placeholders, already-sized URLs) is returned unchanged, so the rewrite is
/// always safe (`None`-returning `iiif_plate_url`/`iiif_strip_thumb_url` are
/// a different shape, used by different callers).
pub fn iiif_thumb_url_clamped(uri: &str, w: Option<i64>) -> Cow<'_, str> {
    width_url_clamped(uri, THUMB_WIDTH, w)
}

/// Rewrite a IIIF Image API URL to request the grid-mode tile size, clamped
/// to the target image's own stored pixel width. Mirrors
/// `iiif_thumb_url_clamped`'s clamp exactly with `GRID_TILE_WIDTH` as the
/// unclamped default.
pub fn iiif_grid_tile_url_clamped(uri: &str, w: Option<i64>) -> Cow<'_, str> {
    width_url_clamped(uri, GRID_TILE_WIDTH, w)
}

/// The image request of a tall grid tile: the whole image fitted into
/// [`GRID_TALL_FETCH_PX`] and clamped to its own pixels
/// ([`iiif_confined_url`]). A tall image is bound by its height, which a
/// width-sized request cannot clamp. With either stored side unknown it is the
/// width-clamped request of an ordinary tile.
pub fn iiif_tall_tile_url(uri: &str, w: Option<i64>, h: Option<i64>) -> Cow<'_, str> {
    w.zip(h)
        .and_then(|(w, h)| iiif_confined_url(uri, w, h, GRID_TALL_FETCH_PX))
        .map_or_else(|| iiif_grid_tile_url_clamped(uri, w), Cow::Owned)
}

/// Width (px) requested for a `strip-row` tile, paired with the
/// `.strip__bed` box (`static/cpe/incunabula.css`), 2× its widest tile:
/// change both together.
pub(crate) const STRIP_TILE_FETCH_PX: u32 = 192;

/// Rewrite a IIIF Image API URL to request a `strip-row` tile, clamped like
/// [`iiif_grid_tile_url_clamped`] with `STRIP_TILE_FETCH_PX`.
pub fn iiif_strip_tile_url_clamped(uri: &str, w: Option<i64>) -> Cow<'_, str> {
    width_url_clamped(uri, STRIP_TILE_FETCH_PX, w)
}

/// The shared body of the width-sized helpers above: `/full/max/` →
/// `/full/<clamp_width(wanted, w)>,/`, any other URL unchanged.
fn width_url_clamped(uri: &str, wanted: u32, w: Option<i64>) -> Cow<'_, str> {
    match uri.split_once("/full/max/") {
        Some((head, tail)) => {
            let width = clamp_width(wanted, w);
            Cow::Owned(format!("{head}/full/{width},/{tail}"))
        }
        None => Cow::Borrowed(uri),
    }
}

/// Rewrite a IIIF Image API URL to request the embedded-media plate-stage
/// size: `/full/<min(PLATE_WIDTH, w)>,/` — clamped to the image's own stored
/// pixel width `w` so the request never asks for an upscale. IIIF v3 rejects
/// a size request larger than the image's native width without the `^`
/// upscale flag (returns 400), so `w` (when known) always wins the `min`.
///
/// `w` is `None` when the Representation's stored width is unknown (the
/// `representations.width` column is nullable) — in that case `PLATE_WIDTH`
/// is used unclamped (the common case; every real Alice image row has a
/// stored width, so this arm is a defensive fallback, not the expected
/// path).
///
/// Returns `None` (rather than the original URL) when the stored URI does
/// not match the canonical `/full/max/` IIIF shape — the plate stage has no
/// well-defined size request to make for a non-IIIF URI, so the caller must
/// decide its own non-IIIF fallback (e.g. render the URI verbatim) instead
/// of silently sizing an arbitrary external URL.
pub fn iiif_plate_url(uri: &str, w: Option<i64>) -> Option<String> {
    let (head, tail) = uri.split_once("/full/max/")?;
    let width = clamp_width(PLATE_WIDTH, w);
    Some(format!("{head}/full/{width},/{tail}"))
}

/// Rewrite a IIIF Image API URL to request the embedded-media strip-thumb
/// size: `/full/{STRIP_THUMB_WIDTH},/`. Mirrors `iiif_thumb_url_clamped`'s
/// rewrite shape with the strip-specific constant.
///
/// Returns `None` for a non-IIIF URI (no `/full/max/` segment) — see
/// `iiif_plate_url`'s doc comment for the same graceful-degradation
/// rationale.
pub fn iiif_strip_thumb_url(uri: &str) -> Option<String> {
    let (head, tail) = uri.split_once("/full/max/")?;
    Some(format!("{head}/full/{STRIP_THUMB_WIDTH},/{tail}"))
}

/// Derive the IIIF `info.json` URL (the IIIF Image API base) from a stored
/// `/full/max/…` Representation URI — what an OpenSeadragon-style viewer's
/// `tile-sources` (or an IIIF SOURCE rights-row) needs.
///
/// The stored URI shape is `{base}/{identifier}/full/max/0/default.jpg`; the
/// info.json URL is `{base}/{identifier}/info.json` — i.e. everything before
/// the `/full/max/` size segment, plus `/info.json`.
///
/// Returns `None` for a non-IIIF URI (no `/full/max/` segment) — see
/// `iiif_plate_url`'s doc comment for the same graceful-degradation
/// rationale.
///
/// Its first production caller was the removed zoom-state surface
/// (`component::zoom`, rejected on owner review) — unreferenced outside
/// tests since, until record-viewer plan step 4's `component::media::
/// render_viewer_canvas`, which builds the `<image-viewer>`'s `tile-sources`
/// from exactly this URL (dropping the `#[allow(dead_code)]` that covered
/// the gap between the two callers, per the plan's round-1 review Critical
/// C1).
pub fn iiif_info_json_url(uri: &str) -> Option<String> {
    let (head, _tail) = uri.split_once("/full/max/")?;
    Some(format!("{head}/info.json"))
}

/// Largest side (px) requested for a woodcut crop, paired with the 24rem
/// crop bed (`.case__bed`, `static/cpe/incunabula.css`), ≥ 2× at desk. The
/// request is further clamped to the region's own pixel size
/// ([`iiif_region_url`]).
pub const CROP_FETCH_PX: i64 = 800;

/// Largest side (px) requested for a detail crop, paired with the 4.5rem
/// detail bed (`.case__detail .case__bed`, `static/cpe/incunabula.css`): a
/// 2.5:1 strip 4.5rem tall is about 180 × 72 CSS px, so 400 covers 2×.
pub const DETAIL_FETCH_PX: i64 = 400;

/// Largest side (px) requested for a whole page in a page case, paired with
/// the 15rem whole-page bed (`.case--page .case__crop`,
/// `static/cpe/incunabula.css`). Clamped to the image's own size
/// ([`iiif_confined_url`]).
pub const PAGE_FETCH_PX: i64 = 480;

/// Largest side (px) requested for an edition card's cover plate, paired with
/// the 15rem plate (`.edition__plate`, `static/cpe/incunabula.css`), 2× its
/// 240 CSS px. Its own constant although it equals [`PAGE_FETCH_PX`]: each
/// fetch size pairs with exactly one CSS box. Clamped to the image's own size
/// ([`iiif_confined_url`]).
pub const PLATE_CARD_FETCH_PX: i64 = 480;

/// The margin (% of the image) a crop adds on every side of an annotation's
/// box.
pub const CROP_MARGIN_PCT: f64 = 1.5;

/// A IIIF `pct:` region in hundredths of a percent, the two-decimal
/// precision the URL carries; `x + w` and `y + h` never exceed 100 %.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionPct {
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

impl RegionPct {
    /// The region's pixel size on a `img_w` × `img_h` image, floored.
    fn pixels(self, img_w: i64, img_h: i64) -> (i64, i64) {
        (self.w * img_w / 10_000, self.h * img_h / 10_000)
    }

    /// The region's pixel aspect (width / height) on a `img_w` × `img_h`
    /// image, from the same rounded percentages the URL carries. `None` for
    /// an unsized image or an empty region.
    pub fn pixel_aspect(self, img_w: i64, img_h: i64) -> Option<f64> {
        let (num, den) = (self.w * img_w, self.h * img_h);
        // Region and image sizes are far below 2^52, so both are exact f64s.
        (img_w > 0 && img_h > 0 && den > 0).then(|| num as f64 / den as f64)
    }
}

/// Percent → hundredths of a percent, rounded half away from zero.
fn hundredths(pct: f64) -> i64 {
    // Clamped to 0..=100 % first, so the cast cannot overflow.
    (pct.clamp(0.0, 100.0) * 100.0).round() as i64
}

/// The crop region of an annotation's 0..1 box (`x`, `y`, `w`, `h`), padded
/// by `margin_pct` on every side and clamped to the image:
/// `X = max(0, 100x − m)`, `W = min(100 − X, 100w + 2m)`, likewise `Y`/`H`.
/// A polygon or circle passes its bounding box.
pub fn crop_region(x: f64, y: f64, w: f64, h: f64, margin_pct: f64) -> RegionPct {
    let left = (100.0 * x - margin_pct).max(0.0);
    let top = (100.0 * y - margin_pct).max(0.0);
    let width = (100.0 - left).min(100.0 * w + 2.0 * margin_pct);
    let height = (100.0 - top).min(100.0 * h + 2.0 * margin_pct);
    let (x, y) = (hundredths(left), hundredths(top));
    RegionPct {
        x,
        y,
        w: hundredths(width).min(10_000 - x),
        h: hundredths(height).min(10_000 - y),
    }
}

/// `n` hundredths as a two-decimal percentage (`1701` → `17.01`).
fn pct_text(n: i64) -> String {
    format!("{}.{:02}", n / 100, n % 100)
}

/// Rewrite a IIIF Image API URL to request `region` of the image, fitted
/// into `!a,b` where each side is `max_px` or the region's own pixel size on
/// the stored `img_w` × `img_h` image, whichever is smaller: a `!w,h` box
/// larger than the region in both sides asks Sipi for an upscale, which it
/// answers with a 500. `None` for a non-IIIF URI (no `/full/max/` segment)
/// or an unsized image.
pub fn iiif_region_url(uri: &str, region: RegionPct, img_w: i64, img_h: i64, max_px: i64) -> Option<String> {
    if img_w <= 0 || img_h <= 0 {
        return None;
    }
    let (head, tail) = uri.split_once("/full/max/")?;
    let (px_w, px_h) = region.pixels(img_w, img_h);
    let a = px_w.clamp(1, max_px);
    let b = px_h.clamp(1, max_px);
    Some(format!(
        "{head}/pct:{},{},{},{}/!{a},{b}/{tail}",
        pct_text(region.x),
        pct_text(region.y),
        pct_text(region.w),
        pct_text(region.h)
    ))
}

/// Rewrite a IIIF Image API URL to request the whole image fitted into
/// `/full/!a,b/`, each side `max_px` or the image's own stored size,
/// whichever is smaller, so the fit never asks for an upscale (as in
/// [`iiif_region_url`]). `None` for a non-IIIF URI or an unsized image.
pub fn iiif_confined_url(uri: &str, img_w: i64, img_h: i64, max_px: i64) -> Option<String> {
    if img_w <= 0 || img_h <= 0 {
        return None;
    }
    let (head, tail) = uri.split_once("/full/max/")?;
    Some(format!("{head}/full/!{},{}/{tail}", img_w.min(max_px), img_h.min(max_px)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── crop_region / iiif_region_url ────────────────────────────────────────

    const C1V: &str = "https://iiif.dasch.swiss/0803/IRyggQzjPMH-B93SNJ70OHN.jp2/full/max/0/default.jpg";

    fn c1v_region() -> RegionPct {
        // The stored box of dt. c1v's woodcut (`bsX4cDKkW3GMC8_Ms2tOpw`).
        crop_region(
            0.18505338078291814,
            0.16203703703703703,
            0.6334519572953737,
            0.5578703703703705,
            CROP_MARGIN_PCT,
        )
    }

    #[test]
    fn c1v_crop_url_has_the_verified_region_and_a_capped_size() {
        assert_eq!(
            iiif_region_url(C1V, c1v_region(), 3322, 5113, CROP_FETCH_PX).as_deref(),
            Some(
                "https://iiif.dasch.swiss/0803/IRyggQzjPMH-B93SNJ70OHN.jp2/\
                 pct:17.01,14.70,66.35,58.79/!800,800/0/default.jpg"
            )
        );
    }

    #[test]
    fn a_small_region_requests_its_own_pixel_size() {
        // pct:20,20,10,10 of 3564 × 5304 is 356.4 × 530.4 px: `!800,800`
        // would ask Sipi to upscale (500), `!356,530` does not.
        let region = crop_region(0.215, 0.215, 0.07, 0.07, CROP_MARGIN_PCT);
        assert_eq!(region, RegionPct { x: 2000, y: 2000, w: 1000, h: 1000 });
        let url = iiif_region_url(C1V, region, 3564, 5304, CROP_FETCH_PX).expect("iiif");
        assert!(url.ends_with("/pct:20.00,20.00,10.00,10.00/!356,530/0/default.jpg"), "{url}");
    }

    #[test]
    fn the_size_comes_from_the_rounded_percentages() {
        // W = 10.006 % rounds to 10.01 %: 10.01 % of 10000 px is 1001 px,
        // where the unrounded 10.006 % would give 1000.
        let region = crop_region(0.5, 0.5, 0.07006, 0.07006, CROP_MARGIN_PCT);
        let url = iiif_region_url(C1V, region, 10_000, 10_000, 5000).expect("iiif");
        assert!(url.contains("/pct:48.50,48.50,10.01,10.01/!1001,1001/"), "{url}");
    }

    #[test]
    fn the_region_is_clamped_to_the_image() {
        // A box at the top-left corner and one running off the bottom-right.
        assert_eq!(
            crop_region(0.01, 0.0, 0.2, 0.3, CROP_MARGIN_PCT),
            RegionPct { x: 0, y: 0, w: 2300, h: 3300 }
        );
        let edge = crop_region(0.9, 0.95, 0.1, 0.05, CROP_MARGIN_PCT);
        assert_eq!(edge, RegionPct { x: 8850, y: 9350, w: 1150, h: 650 });
        // Half-way roundings of X and W never sum past 100 %.
        let half = crop_region(0.18505, 0.0, 0.9, 0.5, CROP_MARGIN_PCT);
        assert_eq!(half.x + half.w, 10_000);
    }

    #[test]
    fn a_whole_page_is_fitted_into_the_page_box_and_never_upscaled() {
        assert_eq!(
            iiif_confined_url(C1V, 3322, 5113, PAGE_FETCH_PX).as_deref(),
            Some(
                "https://iiif.dasch.swiss/0803/IRyggQzjPMH-B93SNJ70OHN.jp2/\
                 full/!480,480/0/default.jpg"
            )
        );
        // A scan narrower than the box asks for its own width on that side.
        let small = iiif_confined_url(C1V, 400, 600, PAGE_FETCH_PX).expect("iiif");
        assert!(small.contains("/full/!400,480/"), "{small}");
        assert_eq!(iiif_confined_url("https://example.org/scan.jpg", 400, 600, PAGE_FETCH_PX), None);
        assert_eq!(iiif_confined_url(C1V, 400, 0, PAGE_FETCH_PX), None);
    }

    #[test]
    fn region_text_always_has_two_decimals() {
        assert_eq!(pct_text(1470), "14.70");
        assert_eq!(pct_text(5), "0.05");
        assert_eq!(pct_text(10_000), "100.00");
    }

    #[test]
    fn a_non_iiif_or_unsized_image_has_no_crop_url() {
        let plain = "https://example.org/scan.jpg";
        assert_eq!(iiif_region_url(plain, c1v_region(), 3322, 5113, CROP_FETCH_PX), None);
        assert_eq!(iiif_region_url(C1V, c1v_region(), 0, 5113, CROP_FETCH_PX), None);
    }

    #[test]
    fn the_pixel_aspect_uses_the_image_size() {
        let ar = c1v_region().pixel_aspect(3322, 5113).expect("sized");
        // (66.35 % × 3322) / (58.79 % × 5113)
        assert!((ar - 0.733_266).abs() < 1e-5, "{ar}");
        assert_eq!(c1v_region().pixel_aspect(0, 5113), None);
    }

    // ── iiif_thumb_url_clamped ───────────────────────────────────────────────

    #[test]
    fn iiif_thumb_url_clamped_clamps_to_narrower_stored_width() {
        // Stored width (288) is narrower than THUMB_WIDTH (384) — Incunabula
        // BorderStrips (288-360px) are exactly this case.
        let uri = "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg";
        assert_eq!(
            iiif_thumb_url_clamped(uri, Some(288)).as_ref(),
            "https://iiif.dasch.swiss/0803/abc.jpx/full/288,/0/default.jpg",
            "must clamp to the narrower stored width, never request THUMB_WIDTH unclamped"
        );
    }

    #[test]
    fn iiif_thumb_url_clamped_uses_thumb_width_when_stored_width_is_wider() {
        let uri = "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg";
        assert_eq!(
            iiif_thumb_url_clamped(uri, Some(4000)).as_ref(),
            "https://iiif.dasch.swiss/0803/abc.jpx/full/384,/0/default.jpg",
            "must cap at THUMB_WIDTH when the image is wider"
        );
    }

    #[test]
    fn iiif_thumb_url_clamped_uses_thumb_width_when_stored_width_unknown() {
        let uri = "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg";
        assert_eq!(
            iiif_thumb_url_clamped(uri, None).as_ref(),
            "https://iiif.dasch.swiss/0803/abc.jpx/full/384,/0/default.jpg",
            "must fall back to unclamped THUMB_WIDTH when width is unknown"
        );
    }

    #[test]
    fn iiif_thumb_url_clamped_leaves_non_iiif_urls_unchanged() {
        let plain = "https://example.org/img/thumb.png";
        assert_eq!(iiif_thumb_url_clamped(plain, Some(100)).as_ref(), plain);
    }

    // ── iiif_grid_tile_url_clamped ───────────────────────────────────────────

    #[test]
    fn iiif_grid_tile_url_clamped_clamps_to_narrower_stored_width() {
        let uri = "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg";
        assert_eq!(
            iiif_grid_tile_url_clamped(uri, Some(288)).as_ref(),
            "https://iiif.dasch.swiss/0803/abc.jpx/full/288,/0/default.jpg",
        );
    }

    #[test]
    fn iiif_grid_tile_url_clamped_uses_grid_tile_width_when_stored_width_unknown() {
        let uri = "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg";
        assert_eq!(
            iiif_grid_tile_url_clamped(uri, None).as_ref(),
            "https://iiif.dasch.swiss/0803/abc.jpx/full/512,/0/default.jpg",
        );
    }

    /// The value `.gm__box`'s comment in grid.css states, and the clamp on
    /// both sides of it.
    #[test]
    fn grid_tile_width_is_512_and_a_narrower_image_wins() {
        assert_eq!(GRID_TILE_WIDTH, 512);
        let uri = "https://iiif.dasch.swiss/0803/abc.jpx/full/max/0/default.jpg";
        assert!(iiif_grid_tile_url_clamped(uri, Some(511)).contains("/full/511,/"));
        assert!(iiif_grid_tile_url_clamped(uri, Some(3322)).contains("/full/512,/"));
    }

    // ── iiif_tall_tile_url ────────────────────────────────────────────────────

    const STRIP: &str = "https://iiif.dasch.swiss/0803/l10.jp2/full/max/0/default.jpg";

    #[test]
    fn a_tall_tile_is_fitted_and_clamped_to_the_images_own_pixels() {
        // l-10 is 360 × 4264: the width stays its own, the height is capped.
        assert_eq!(
            iiif_tall_tile_url(STRIP, Some(360), Some(4264)).as_ref(),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/!360,1000/0/default.jpg",
        );
        // Smaller than the cap on both sides: its own size, never an upscale.
        assert_eq!(
            iiif_tall_tile_url(STRIP, Some(90), Some(800)).as_ref(),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/!90,800/0/default.jpg",
        );
        // Each side is clamped on its own, whichever way the image lies.
        assert_eq!(
            iiif_tall_tile_url(STRIP, Some(3000), Some(800)).as_ref(),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/!1000,800/0/default.jpg",
        );
        // A URI that is not IIIF is left alone.
        assert_eq!(
            iiif_tall_tile_url("https://example.org/strip.jpg", Some(360), Some(4264)).as_ref(),
            "https://example.org/strip.jpg",
        );
    }

    #[test]
    fn a_tall_tile_with_an_unknown_side_takes_the_width_clamped_request() {
        assert_eq!(
            iiif_tall_tile_url(STRIP, Some(360), None).as_ref(),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/360,/0/default.jpg",
        );
        assert_eq!(
            iiif_tall_tile_url(STRIP, None, Some(4264)).as_ref(),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/512,/0/default.jpg",
        );
        assert_eq!(
            iiif_tall_tile_url(STRIP, Some(0), Some(4264)).as_ref(),
            "https://iiif.dasch.swiss/0803/l10.jp2/full/512,/0/default.jpg",
            "a zero width is an unknown one"
        );
    }

    // ── iiif_plate_url ────────────────────────────────────────────────────────

    #[test]
    fn iiif_plate_url_clamps_to_narrower_stored_width() {
        // Stored width (562) is narrower than PLATE_WIDTH (900) — the
        // clamp must request 562, never upscale past the image's own size.
        let uri = "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg";
        let out = iiif_plate_url(uri, Some(562)).expect("iiif url should parse");
        assert_eq!(
            out, "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/562,/0/default.jpg",
            "must clamp to the narrower stored width, got: {out}"
        );
    }

    #[test]
    fn iiif_plate_url_uses_plate_width_when_stored_width_is_wider() {
        let uri = "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg";
        let out = iiif_plate_url(uri, Some(4000)).expect("iiif url should parse");
        assert_eq!(
            out, "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/900,/0/default.jpg",
            "must cap at PLATE_WIDTH when the image is wider, got: {out}"
        );
    }

    #[test]
    fn iiif_plate_url_uses_plate_width_when_stored_width_unknown() {
        let uri = "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg";
        let out = iiif_plate_url(uri, None).expect("iiif url should parse");
        assert_eq!(
            out, "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/900,/0/default.jpg",
            "must fall back to PLATE_WIDTH when width is unknown, got: {out}"
        );
    }

    #[test]
    fn iiif_plate_url_non_iiif_passthrough_returns_none() {
        let plain = "https://example.org/img/full-res.png";
        assert_eq!(iiif_plate_url(plain, Some(1000)), None);
    }

    // ── iiif_strip_thumb_url ─────────────────────────────────────────────────

    #[test]
    fn iiif_strip_thumb_url_rewrites_size_segment() {
        let uri = "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/max/0/default.jpg";
        let out = iiif_strip_thumb_url(uri).expect("iiif url should parse");
        assert_eq!(out, "https://iiif.demo.dasch.swiss:443/0854/abc.jpx/full/208,/0/default.jpg",);
    }

    #[test]
    fn iiif_strip_thumb_url_non_iiif_passthrough_returns_none() {
        let plain = "https://example.org/img/thumb.png";
        assert_eq!(iiif_strip_thumb_url(plain), None);
    }

    // ── iiif_info_json_url ───────────────────────────────────────────────────

    #[test]
    fn iiif_info_json_url_derives_base_plus_info_json() {
        let uri = "https://iiif.demo.dasch.swiss:443/0854/6IQRqvpLkz5-XJnRjN1tjD9.jpx/full/max/0/default.jpg";
        let out = iiif_info_json_url(uri).expect("iiif url should parse");
        assert_eq!(
            out,
            "https://iiif.demo.dasch.swiss:443/0854/6IQRqvpLkz5-XJnRjN1tjD9.jpx/info.json"
        );
    }

    #[test]
    fn iiif_info_json_url_non_iiif_passthrough_returns_none() {
        let plain = "https://example.org/img/manifest.json";
        assert_eq!(iiif_info_json_url(plain), None);
    }
}
