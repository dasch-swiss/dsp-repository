//! Behavioural tests for the Maud rendering + asset lookup surface.
//!
//! The viewer's *runtime* behaviour lives in `assets/image-viewer.js` and is not
//! exercised here — these tests pin the Rust contract: attributes are rendered
//! as expected, source URLs are safely encoded, and the asset lookup is present
//! and traversal-proof.

use cpe_image_viewer::{asset, head_tag, Auth, Viewer, ASSET_PREFIX};
use maud::{html, Render};

fn render(v: &Viewer) -> String {
    v.render().into_string()
}

#[test]
fn single_source_renders_element_with_id_and_sources() {
    let html = render(&Viewer::new(
        "page-viewer",
        vec!["https://iiif.example/abcd/info.json".to_owned()],
    ));
    assert!(html.contains("<image-viewer"));
    assert!(html.contains(r#"id="page-viewer""#));
    // The sources land as a JSON array, HTML-escaped as an attribute value.
    assert!(html.contains("tile-sources="));
    assert!(html.contains("https://iiif.example/abcd/info.json"));
    assert!(html.contains(r#"auth-mode="none""#));
}

#[test]
fn multiple_sources_are_json_encoded_in_order() {
    let html = render(&Viewer::new(
        "v",
        vec![
            "https://iiif.example/a/info.json".to_owned(),
            "https://iiif.example/b/info.json".to_owned(),
        ],
    ));
    // Attribute value is HTML-escaped, so quotes appear as &quot;.
    assert!(
        html.contains("[&quot;https://iiif.example/a/info.json&quot;,&quot;https://iiif.example/b/info.json&quot;]")
    );
}

// ── Engine seam ─────────────────────────────────────────────────────────────
// The point of these two is the *non-breaking* guarantee: a consumer that
// never mentions an engine keeps getting a working viewer, and the engine slug
// is spelled identically in the Rust markup and the JS.

#[test]
fn default_engine_is_rendered_without_being_asked_for() {
    // CPE calls `Viewer::new(...)` and nothing else. It must still name the
    // engine in the markup.
    let html = render(&Viewer::new("v", vec!["x".to_owned()]));
    assert!(html.contains(r#"engine="osd-wc""#));
}

#[test]
fn the_js_dispatches_on_the_same_slug_the_rust_emits() {
    // The slug is one vocabulary across the HTML attribute and the JS module.
    // This pins the Rust↔JS pair: if someone renames the builtin engine on one
    // side only, this fails.
    let js = std::str::from_utf8(asset("image-viewer.js").expect("module embedded").bytes).expect("module is utf-8");
    assert!(js.contains(r#"BUILTIN_ENGINE = "osd-wc""#));
    let html = render(&Viewer::new("v", vec!["x".to_owned()]));
    assert!(html.contains(r#"engine="osd-wc""#));
}

#[test]
fn cookie_auth_sets_attr() {
    let html = render(&Viewer::new("v", vec!["x".to_owned()]).auth(Auth::Cookie));
    assert!(html.contains(r#"auth-mode="cookie""#));
}

#[test]
fn navigator_is_a_boolean_attr() {
    let on = render(&Viewer::new("v", vec!["x".to_owned()]).navigator(true));
    assert!(on.contains("navigator"));
    let off = render(&Viewer::new("v", vec!["x".to_owned()]).navigator(false));
    assert!(!off.contains("navigator"));
}

#[test]
fn adjust_is_a_boolean_attr() {
    let on = render(&Viewer::new("v", vec!["x".to_owned()]).adjust(true));
    assert!(on.contains("adjust"));
    let off = render(&Viewer::new("v", vec!["x".to_owned()]));
    assert!(!off.contains("adjust"));
}

#[test]
fn sequence_attr_omitted_unless_forced() {
    let auto = render(&Viewer::new("v", vec!["x".to_owned()]));
    assert!(!auto.contains("sequence="));
    let forced = render(&Viewer::new("v", vec!["x".to_owned()]).sequence(true));
    assert!(forced.contains(r#"sequence="true""#));
}

#[test]
fn fallback_is_projected_as_light_dom_child() {
    let html = render(&Viewer::new("v", vec!["x".to_owned()]).fallback(html! {
        img src="/thumb.jpg" alt="preview";
    }));
    assert!(html.contains(r#"<img src="/thumb.jpg" alt="preview">"#));
}

#[test]
fn overlay_markup_in_the_fallback_passes_through_unchanged() {
    // The JS adopts the overlay from the element's light-DOM children, so the
    // builder must hand the host's overlay markup through byte for byte.
    let overlay = html! {
        svg data-iv-overlay viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true" {
            g data-iv-shape="a1" style="--iv-c:#ff3333;--iv-on:#000" {
                rect data-iv-part="halo" x="17.000" y="15.000" width="40.000" height="30.000" {}
                rect data-iv-part="line" x="17.000" y="15.000" width="40.000" height="30.000" {}
            }
            g data-iv-shape="a2" style="--iv-c:#3333ff;--iv-on:#fff" {
                polygon data-iv-part="line" points="47.000,14.000 60.000,30.000 40.000,30.000" {}
            }
        }
        span
            data-iv-tag="a1"
            data-iv-x="17.000"
            data-iv-y="15.000"
            data-iv-align="start"
            style="--iv-c:#ff3333;--iv-on:#000"
            aria-hidden="true"
        { "1" }
    };
    let expected = overlay.clone().into_string();
    let html = render(&Viewer::new("v", vec!["x".to_owned()]).fallback(html! {
        img src="/thumb.jpg" alt="";
        (overlay)
    }));
    assert!(html.contains(&expected), "overlay markup missing or altered: {html}");
}

#[test]
fn quotes_in_a_source_cannot_break_out_of_the_attribute() {
    // A crafted URL with a double quote must not close the attribute.
    let html = render(&Viewer::new("v", vec![r#"https://evil/"><script>x</script>"#.to_owned()]));
    // The raw sequence `"><script>` must NOT appear verbatim; the quote is
    // escaped to &quot; and `<`/`>` to &lt;/&gt; inside the attribute.
    assert!(!html.contains(r#""><script>"#));
    assert!(html.contains("&quot;"));
}

// ── Host-rendered chrome control surface ────────────────────────────────────

#[test]
fn chrome_is_on_by_default_and_omitted_from_markup() {
    // The default renders no `chrome` attribute — the
    // attribute is only ever rendered when opting OUT.
    let html = render(&Viewer::new("v", vec!["x".to_owned()]));
    assert!(!html.contains("chrome="));
}

#[test]
fn chrome_false_renders_the_opt_out_attribute() {
    let html = render(&Viewer::new("v", vec!["x".to_owned()]).chrome(false));
    assert!(html.contains(r#"chrome="false""#));
}

#[test]
fn chrome_true_explicitly_still_omits_the_attribute() {
    // Symmetric with the other boolean-ish builders: naming the default value
    // explicitly must not change the rendered markup.
    let html = render(&Viewer::new("v", vec!["x".to_owned()]).chrome(true));
    assert!(!html.contains("chrome="));
}

#[test]
fn page_is_absent_by_default() {
    let html = render(&Viewer::new("v", vec!["x".to_owned()]));
    assert!(!html.contains("page="));
}

#[test]
fn page_renders_the_1_based_initial_page_attribute() {
    let html = render(&Viewer::new("v", vec!["x".to_owned()]).page(3));
    assert!(html.contains(r#"page="3""#));
}

#[test]
fn page_is_not_in_the_js_observed_attributes_list() {
    // Load-bearing: `page` (and `chrome`) must never trigger
    // attributeChangedCallback's teardown+reinit — that's the whole point of
    // an initial-page mechanism that survives a DataStar fragment swap.
    let js = std::str::from_utf8(asset("image-viewer.js").expect("module embedded").bytes).expect("module is utf-8");
    let observed_block = js
        .split("static get observedAttributes()")
        .nth(1)
        .and_then(|s| s.split("];").next())
        .expect("observedAttributes getter present");
    // The slice is the list: it holds an attribute that is observed.
    assert!(observed_block.contains("\"tile-sources\""));
    assert!(!observed_block.contains("\"page\""));
    assert!(!observed_block.contains("\"chrome\""));
}

#[test]
fn js_module_exposes_the_documented_public_control_methods() {
    // One-vocabulary guarantee for the host-rendered-chrome control surface:
    // every method this crate's README documents must actually be defined on
    // the element, so a doc/JS drift fails here instead of at runtime in CPE.
    let js = std::str::from_utf8(asset("image-viewer.js").expect("module embedded").bytes).expect("module is utf-8");
    for method in [
        "zoomIn(",
        "zoomOut(",
        "goHome(",
        "goToPage(",
        "setBrightness(",
        "setContrast(",
        "setExposure(",
        "setSaturation(",
        "setGamma(",
        "setHue(",
        "setInvert(",
        "resetAdjust(",
        "setOverlayState(",
        "zoomToShape(",
    ] {
        assert!(js.contains(method), "image-viewer.js should define {method}");
    }
    assert!(js.contains("viewer-zoom"));
}

#[test]
fn exposure_and_gamma_approximation_is_labelled_in_the_osd_js() {
    // Load-bearing: the
    // osd-wc engine's CSS-filter exposure/gamma are an APPROXIMATION, and the
    // crate must label that fact in code rather than letting this be mistaken
    // for a faithful implementation.
    let js = std::str::from_utf8(asset("image-viewer.js").expect("module embedded").bytes).expect("module is utf-8");
    let filter_fn = js
        .split("_applyFilter() {")
        .nth(1)
        .and_then(|s| s.split("_buildAdjustPanel").next())
        .expect("_applyFilter method present");
    assert!(
        filter_fn.contains("APPROXIMATION"),
        "_applyFilter must label the exposure/gamma approximation"
    );
}

#[test]
fn osd_exposure_and_gamma_defaults_and_ranges_match_the_design_table() {
    // The osd-wc engine's exposure/gamma defaults/ranges are pinned:
    // exposure min -1 max 1 default 0; gamma min 0.5
    // max 2 default 1.
    let js = std::str::from_utf8(asset("image-viewer.js").expect("module embedded").bytes).expect("module is utf-8");
    // Defaults, in ADJUST_DEFAULTS.
    assert!(js.contains("exposure: 0,"), "ADJUST_DEFAULTS.exposure must default to 0");
    assert!(js.contains("gamma: 1,"), "ADJUST_DEFAULTS.gamma must default to 1");
    // setExposure/setGamma delegate to _setNumericAdjust with those ranges.
    let set_exposure = js
        .split("setExposure(v) {")
        .nth(1)
        .and_then(|s| s.split("setGamma(v) {").next())
        .expect("setExposure method present");
    assert!(
        set_exposure.contains(r#"_setNumericAdjust("exposure", v, -1, 1)"#),
        "setExposure must delegate to _setNumericAdjust with [-1, 1]"
    );
    let set_gamma = js
        .split("setGamma(v) {")
        .nth(1)
        .and_then(|s| s.split("resetAdjust()").next())
        .expect("setGamma method present");
    assert!(
        set_gamma.contains(r#"_setNumericAdjust("gamma", v, 0.5, 2)"#),
        "setGamma must delegate to _setNumericAdjust with [0.5, 2]"
    );
    // The built-in adjust panel's own sliders carry the same ranges.
    assert!(
        js.contains(r#"["exposure", "Exposure", -1, 1, 0.01]"#),
        "built-in Adjust panel's exposure row must match the design range"
    );
    assert!(
        js.contains(r#"["gamma", "Gamma", 0.5, 2, 0.01]"#),
        "built-in Adjust panel's gamma row must match the design range"
    );
}

#[test]
fn set_numeric_adjust_falls_back_to_neutral_default_on_non_finite_input() {
    // Pins the non-finite safety: every numeric adjust setter
    // routes through `_setNumericAdjust`, which falls back to each
    // operator's own NEUTRAL default (via `ADJUST_DEFAULTS[key]`) rather
    // than `clamp()`'s raw `min`, on a non-finite input. A future refactor
    // that inlines this logic back into the individual setters (and drops
    // the neutral-default fallback) must fail this test.
    let js = std::str::from_utf8(asset("image-viewer.js").expect("module embedded").bytes).expect("module is utf-8");
    assert!(
        js.contains("Number.isFinite(n) ? clamp(n, min, max) : ADJUST_DEFAULTS[key]"),
        "_setNumericAdjust must fall back to ADJUST_DEFAULTS[key] on non-finite input"
    );
    // The non-finite-safety fallback above is only load-bearing if each of the
    // four numeric adjust setters actually delegates to `_setNumericAdjust`
    // rather than calling `clamp()` directly. Pin that delegation here too —
    // otherwise a regression that inlines e.g. `setBrightness` back to raw
    // `clamp()` would still pass this test.
    let set_brightness = js
        .split("setBrightness(v) {")
        .nth(1)
        .and_then(|s| s.split("setContrast(v) {").next())
        .expect("setBrightness method present");
    assert!(
        set_brightness.contains(r#"_setNumericAdjust("brightness", v, 0, 200)"#),
        "setBrightness must delegate to _setNumericAdjust with [0, 200]"
    );
    let set_contrast = js
        .split("setContrast(v) {")
        .nth(1)
        .and_then(|s| s.split("setSaturation(v) {").next())
        .expect("setContrast method present");
    assert!(
        set_contrast.contains(r#"_setNumericAdjust("contrast", v, 0, 200)"#),
        "setContrast must delegate to _setNumericAdjust with [0, 200]"
    );
    let set_saturation = js
        .split("setSaturation(v) {")
        .nth(1)
        .and_then(|s| s.split("setHue(v) {").next())
        .expect("setSaturation method present");
    assert!(
        set_saturation.contains(r#"_setNumericAdjust("saturate", v, 0, 200)"#),
        "setSaturation must delegate to _setNumericAdjust with [0, 200]"
    );
    let set_hue = js
        .split("setHue(v) {")
        .nth(1)
        .and_then(|s| s.split("setInvert(on) {").next())
        .expect("setHue method present");
    assert!(
        set_hue.contains(r#"_setNumericAdjust("hue", v, -180, 180)"#),
        "setHue must delegate to _setNumericAdjust with [-180, 180]"
    );
}

#[test]
fn head_tag_points_at_the_module() {
    let tag = head_tag(ASSET_PREFIX).into_string();
    assert_eq!(
        tag,
        r#"<script type="module" src="/_static/image-viewer/image-viewer.js"></script>"#
    );
}

#[test]
fn head_tag_trims_trailing_slash() {
    let tag = head_tag("/assets/osd/").into_string();
    assert!(tag.contains(r#"src="/assets/osd/image-viewer.js""#));
}

#[test]
fn assets_are_embedded_and_servable() {
    let module = asset("image-viewer.js").expect("module embedded");
    assert!(module.content_type.starts_with("text/javascript"));
    assert!(!module.bytes.is_empty());

    let osd = asset("vendor/openseadragon.min.js").expect("osd embedded");
    assert!(osd.bytes.windows(13).any(|w| w == b"OpenSeadragon"));

    let sprite = asset("vendor/images/zoomin_rest.png").expect("sprite embedded");
    assert_eq!(sprite.content_type, "image/png");
    // PNG magic number.
    assert_eq!(&sprite.bytes[..4], &[0x89, b'P', b'N', b'G']);

    // Text, so that a browser shows them and never runs or downloads them.
    let lock = asset("vendor/VENDOR.lock").expect("lock embedded");
    assert_eq!(lock.content_type, "text/plain; charset=utf-8");
    let licence = asset("LICENSE-openseadragon.txt").expect("licence embedded");
    assert_eq!(licence.content_type, "text/plain; charset=utf-8");
}

#[test]
fn asset_lookup_rejects_traversal_and_unknowns() {
    assert!(asset("../Cargo.toml").is_none());
    assert!(asset("vendor/../../secret").is_none());
    assert!(asset("").is_none());
    assert!(asset("does-not-exist.js").is_none());
}

fn collect(dir: &std::path::Path, prefix: &str, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("assets directory is readable") {
        let entry = entry.expect("entry");
        let name = entry.file_name().into_string().expect("utf-8 file name");
        if name.starts_with('.') {
            continue;
        }
        let path = format!("{prefix}{name}");
        if entry.file_type().expect("file type").is_dir() {
            collect(&entry.path(), &format!("{path}/"), out);
        } else {
            out.push(path);
        }
    }
}

#[test]
fn asset_returns_every_file_under_assets() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mut found = Vec::new();
    collect(&root, "", &mut found);
    assert!(!found.is_empty());
    for path in found {
        let on_disk = std::fs::read(root.join(&path)).expect("file is readable");
        let served = asset(&path).unwrap_or_else(|| panic!("{path} is not in ASSETS"));
        assert_eq!(served.bytes, on_disk.as_slice(), "{path} differs from disk");
    }
}
