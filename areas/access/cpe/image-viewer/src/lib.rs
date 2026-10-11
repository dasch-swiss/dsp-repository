//! `cpe-image-viewer` — an IIIF deep-zoom image viewer, packaged as an
//! `<image-viewer>` web component with a [`maud`] helper. The deep-zoom engine
//! is a vendored OpenSeadragon, the only one: the element always renders with
//! it. CPE's component library is the one consumer.
//!
//! The crate is "Layer 1" of
//! dsp-incubator cpe/docs/adr/0010-openseadragon-integration.md: a thin
//! Rust wrapper around a self-contained JS/asset bundle ("Layer 0"). The crate
//! embeds the bundle and exposes three things a Rust/Maud MPA host needs:
//!
//! 1. [`asset`] — look up an embedded asset (the module, the vendored OpenSeadragon, its control
//!    sprites) so the host can serve it from one route under [`ASSET_PREFIX`].
//! 2. [`head_tag`] — the `<script type="module">` tag that registers the custom element; goes in
//!    `<head>`.
//! 3. [`Viewer`] — a builder that renders an `<image-viewer>` element. It implements
//!    [`maud::Render`], so it drops straight into `html! { (viewer) }`.
//!
//! The Rust side carries **no viewer logic** — it only renders attributes and
//! serves bytes. All behaviour lives in the bundled `image-viewer.js`. Keeping
//! that boundary clean is what makes the bundle extractable to a non-Rust
//! consumer later
//! (dsp-incubator cpe/docs/adr/0010-openseadragon-integration.md, "reversibility").
//!
//! # Example
//! ```
//! use cpe_image_viewer::{Viewer, Auth};
//! use maud::html;
//!
//! let viewer = Viewer::new("page-viewer", vec![
//!     "https://iiif.example/abcd/info.json".to_owned(),
//! ])
//! .auth(Auth::Cookie)
//! .navigator(true)
//! .label("Page 1 recto");
//!
//! let _markup = html! { (viewer) };
//! ```

use maud::{html, Markup, Render};

/// The embedded bundle: `image-viewer.js`, `vendor/**` (OpenSeadragon, its control
/// sprites, the `VENDOR.lock` manifest) and the OpenSeadragon licence text. Every
/// file under `assets/` has a row: `tests/render.rs` fails for one that has none.
static ASSETS: &[(&str, &[u8])] = &[
    (
        "LICENSE-openseadragon.txt",
        include_bytes!("../assets/LICENSE-openseadragon.txt"),
    ),
    ("image-viewer.js", include_bytes!("../assets/image-viewer.js")),
    ("vendor/VENDOR.lock", include_bytes!("../assets/vendor/VENDOR.lock")),
    (
        "vendor/images/button_grouphover.png",
        include_bytes!("../assets/vendor/images/button_grouphover.png"),
    ),
    (
        "vendor/images/button_hover.png",
        include_bytes!("../assets/vendor/images/button_hover.png"),
    ),
    (
        "vendor/images/button_pressed.png",
        include_bytes!("../assets/vendor/images/button_pressed.png"),
    ),
    (
        "vendor/images/button_rest.png",
        include_bytes!("../assets/vendor/images/button_rest.png"),
    ),
    (
        "vendor/images/flip_grouphover.png",
        include_bytes!("../assets/vendor/images/flip_grouphover.png"),
    ),
    (
        "vendor/images/flip_hover.png",
        include_bytes!("../assets/vendor/images/flip_hover.png"),
    ),
    (
        "vendor/images/flip_pressed.png",
        include_bytes!("../assets/vendor/images/flip_pressed.png"),
    ),
    (
        "vendor/images/flip_rest.png",
        include_bytes!("../assets/vendor/images/flip_rest.png"),
    ),
    (
        "vendor/images/fullpage_grouphover.png",
        include_bytes!("../assets/vendor/images/fullpage_grouphover.png"),
    ),
    (
        "vendor/images/fullpage_hover.png",
        include_bytes!("../assets/vendor/images/fullpage_hover.png"),
    ),
    (
        "vendor/images/fullpage_pressed.png",
        include_bytes!("../assets/vendor/images/fullpage_pressed.png"),
    ),
    (
        "vendor/images/fullpage_rest.png",
        include_bytes!("../assets/vendor/images/fullpage_rest.png"),
    ),
    (
        "vendor/images/home_grouphover.png",
        include_bytes!("../assets/vendor/images/home_grouphover.png"),
    ),
    (
        "vendor/images/home_hover.png",
        include_bytes!("../assets/vendor/images/home_hover.png"),
    ),
    (
        "vendor/images/home_pressed.png",
        include_bytes!("../assets/vendor/images/home_pressed.png"),
    ),
    (
        "vendor/images/home_rest.png",
        include_bytes!("../assets/vendor/images/home_rest.png"),
    ),
    (
        "vendor/images/next_grouphover.png",
        include_bytes!("../assets/vendor/images/next_grouphover.png"),
    ),
    (
        "vendor/images/next_hover.png",
        include_bytes!("../assets/vendor/images/next_hover.png"),
    ),
    (
        "vendor/images/next_pressed.png",
        include_bytes!("../assets/vendor/images/next_pressed.png"),
    ),
    (
        "vendor/images/next_rest.png",
        include_bytes!("../assets/vendor/images/next_rest.png"),
    ),
    (
        "vendor/images/previous_grouphover.png",
        include_bytes!("../assets/vendor/images/previous_grouphover.png"),
    ),
    (
        "vendor/images/previous_hover.png",
        include_bytes!("../assets/vendor/images/previous_hover.png"),
    ),
    (
        "vendor/images/previous_pressed.png",
        include_bytes!("../assets/vendor/images/previous_pressed.png"),
    ),
    (
        "vendor/images/previous_rest.png",
        include_bytes!("../assets/vendor/images/previous_rest.png"),
    ),
    (
        "vendor/images/rotateleft_grouphover.png",
        include_bytes!("../assets/vendor/images/rotateleft_grouphover.png"),
    ),
    (
        "vendor/images/rotateleft_hover.png",
        include_bytes!("../assets/vendor/images/rotateleft_hover.png"),
    ),
    (
        "vendor/images/rotateleft_pressed.png",
        include_bytes!("../assets/vendor/images/rotateleft_pressed.png"),
    ),
    (
        "vendor/images/rotateleft_rest.png",
        include_bytes!("../assets/vendor/images/rotateleft_rest.png"),
    ),
    (
        "vendor/images/rotateright_grouphover.png",
        include_bytes!("../assets/vendor/images/rotateright_grouphover.png"),
    ),
    (
        "vendor/images/rotateright_hover.png",
        include_bytes!("../assets/vendor/images/rotateright_hover.png"),
    ),
    (
        "vendor/images/rotateright_pressed.png",
        include_bytes!("../assets/vendor/images/rotateright_pressed.png"),
    ),
    (
        "vendor/images/rotateright_rest.png",
        include_bytes!("../assets/vendor/images/rotateright_rest.png"),
    ),
    (
        "vendor/images/zoomin_grouphover.png",
        include_bytes!("../assets/vendor/images/zoomin_grouphover.png"),
    ),
    (
        "vendor/images/zoomin_hover.png",
        include_bytes!("../assets/vendor/images/zoomin_hover.png"),
    ),
    (
        "vendor/images/zoomin_pressed.png",
        include_bytes!("../assets/vendor/images/zoomin_pressed.png"),
    ),
    (
        "vendor/images/zoomin_rest.png",
        include_bytes!("../assets/vendor/images/zoomin_rest.png"),
    ),
    (
        "vendor/images/zoomout_grouphover.png",
        include_bytes!("../assets/vendor/images/zoomout_grouphover.png"),
    ),
    (
        "vendor/images/zoomout_hover.png",
        include_bytes!("../assets/vendor/images/zoomout_hover.png"),
    ),
    (
        "vendor/images/zoomout_pressed.png",
        include_bytes!("../assets/vendor/images/zoomout_pressed.png"),
    ),
    (
        "vendor/images/zoomout_rest.png",
        include_bytes!("../assets/vendor/images/zoomout_rest.png"),
    ),
    (
        "vendor/openseadragon.min.js",
        include_bytes!("../assets/vendor/openseadragon.min.js"),
    ),
];

/// The conventional mount prefix for this crate's assets. A host is free to
/// mount elsewhere — the JS self-locates the vendored OpenSeadragon and control
/// sprites relative to its own module URL, so nothing here hardcodes the path.
pub const ASSET_PREFIX: &str = "/_static/image-viewer";

/// An embedded asset: raw bytes plus a `Content-Type` for the HTTP response.
#[derive(Debug, Clone, Copy)]
pub struct Asset {
    pub bytes: &'static [u8],
    pub content_type: &'static str,
}

/// Look up an embedded asset by path relative to the crate's asset root, e.g.
/// `"image-viewer.js"` or `"vendor/images/zoomin_rest.png"`.
///
/// Rejects any path with an empty, dot, or parent (`..`) segment, so a host can
/// forward an untrusted request tail straight in without a traversal check.
/// Returns `None` for unknown or rejected paths.
///
/// ```
/// assert!(cpe_image_viewer::asset("image-viewer.js").is_some());
/// assert!(cpe_image_viewer::asset("vendor/openseadragon.min.js").is_some());
/// assert!(cpe_image_viewer::asset("../Cargo.toml").is_none());
/// assert!(cpe_image_viewer::asset("nope.js").is_none());
/// ```
pub fn asset(path: &str) -> Option<Asset> {
    let unsafe_segment = path.split('/').any(|seg| seg.is_empty() || seg == "." || seg == "..");
    if unsafe_segment {
        return None;
    }
    let (_, bytes) = ASSETS.iter().find(|(name, _)| *name == path)?;
    Some(Asset { bytes, content_type: content_type_for(path) })
}

fn content_type_for(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("js") => "text/javascript; charset=utf-8",
        Some("png") => "image/png",
        Some("txt" | "lock") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// The `<script type="module">` tag that registers `<image-viewer>`. Put it in
/// `<head>` once per page. `base` is where the host mounted the assets — pass
/// [`ASSET_PREFIX`] unless you mounted them elsewhere.
///
/// The module is tiny; the heavy OpenSeadragon bundle is lazy-loaded by the
/// element only on pages that actually contain a viewer.
pub fn head_tag(base: &str) -> Markup {
    let src = format!("{}/image-viewer.js", base.trim_end_matches('/'));
    html! {
        script type="module" src=(src) {}
    }
}

/// How the viewer authenticates its IIIF requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Auth {
    /// Public images — no credentials sent. The default.
    #[default]
    None,
    /// Send credentials (cookies) with the `info.json` fetch and every tile
    /// request — for cookie-gated IIIF such as SIPI. Requires the image server
    /// to allow credentialed CORS.
    ///
    /// Bearer-token auth is intentionally not offered: a token rendered into an
    /// HTML attribute leaks into the DOM. See the crate README, "Auth".
    Cookie,
}

impl Auth {
    fn as_attr(self) -> &'static str {
        match self {
            Auth::None => "none",
            Auth::Cookie => "cookie",
        }
    }
}

/// A builder for an `<image-viewer>` element. Implements [`maud::Render`].
///
/// The element always renders with the OpenSeadragon engine.
#[derive(Debug, Clone)]
pub struct Viewer {
    id: String,
    sources: Vec<String>,
    auth: Auth,
    navigator: bool,
    sequence: Option<bool>,
    adjust: bool,
    label: Option<String>,
    fallback: Option<Markup>,
    /// Stock-chrome opt-out (default `true` — OSD's own controls stay on).
    /// See [`Viewer::chrome`].
    chrome: bool,
    /// Initial page (1-based) to open a multi-source sequence on. See
    /// [`Viewer::page`].
    initial_page: Option<u32>,
}

impl Viewer {
    /// A viewer with a stable DOM `id` and one or more IIIF `info.json` URLs.
    ///
    /// The `id` must be **stable across re-renders**: idiomorph keys on it, so a
    /// stable id lets a surrounding DataStar fragment swap preserve a running
    /// viewer instead of tearing it down and rebuilding.
    ///
    /// One source → a single image. Multiple → a paged sequence (sequence mode
    /// auto-enables; override with [`Viewer::sequence`]).
    pub fn new(id: impl Into<String>, sources: Vec<String>) -> Self {
        Self {
            id: id.into(),
            sources,
            auth: Auth::None,
            navigator: false,
            sequence: None,
            adjust: false,
            label: None,
            fallback: None,
            chrome: true,
            initial_page: None,
        }
    }

    /// Set the authentication mode (default [`Auth::None`]).
    pub fn auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    /// Show the overview-navigator thumbnail (default off).
    pub fn navigator(mut self, on: bool) -> Self {
        self.navigator = on;
        self
    }

    /// Force sequence mode on/off. When unset it auto-enables for >1 source.
    pub fn sequence(mut self, on: bool) -> Self {
        self.sequence = Some(on);
        self
    }

    /// Offer image-adjustment filters — useful for reading manuscripts (the
    /// hue rotation shifts the colour cast of discoloured parchment). Adds an **"Adjust" toggle**
    /// in the viewer's bottom-left corner; the slider panel is **collapsed by default**
    /// (filters off until the user opens it) and the toggle expands/collapses it. How the
    /// filters render: brightness / contrast / exposure / saturation / gamma / hue /
    /// invert (all 7 operators) as CSS filters on the tiled-image canvas —
    /// GPU-composited, no per-tile cost. `exposure`/`gamma` have no native CSS
    /// filter equivalent, so they are **APPROXIMATED** (folded into the
    /// `brightness()` filter term) — see `assets/image-viewer.js`'s
    /// `_applyFilter` for the labelled approximation.
    pub fn adjust(mut self, on: bool) -> Self {
        self.adjust = on;
        self
    }

    /// An accessible label for the viewer region.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Light-DOM fallback shown until the viewer opens — typically a plain IIIF
    /// thumbnail `<img>` for the no-JS and pre-upgrade paths. Progressive
    /// enhancement. It is NOT shown again when the viewer fails to open: the
    /// element hides it, shows an error message (`role="alert"`) and emits
    /// `viewer-error`, so that a still image is not taken for a working viewer.
    pub fn fallback(mut self, markup: Markup) -> Self {
        self.fallback = Some(markup);
        self
    }

    /// Opt **out** of OSD's own stock chrome — its navigation (prev/next),
    /// sequence, and rotation controls (`showNavigationControl` /
    /// `showSequenceControl` / `showRotationControl`, all default **on**).
    ///
    /// Set this when the host renders its **own** paging/zoom chrome (e.g. CPE's
    /// `.pg` pager + zoom chips calling [`goToPage`](#public-js-control-surface)
    /// and friends) — without it a multi-source viewer draws a second,
    /// out-of-band pager that pages the canvas without going through the host's
    /// swap, desyncing any surrounding UI from the viewer's actual page/zoom
    /// state. Default `true` (OSD's own chrome shows).
    pub fn chrome(mut self, on: bool) -> Self {
        self.chrome = on;
        self
    }

    /// The **1-based** page to mount a multi-source sequence on, instead of
    /// always opening on page 1 (e.g. a takeover reopening on the image the
    /// visitor clicked). Rendered as a plain, **non-reactive** HTML attribute
    /// read once at element init — it is deliberately excluded from the
    /// element's `observedAttributes`, so setting/patching it (e.g. across a
    /// DataStar fragment swap) never triggers the teardown+reinit that mutating
    /// an *observed* attribute like `tile-sources` would. See the crate README,
    /// "Public JS control surface", for the 1-based convention rationale and the
    /// runtime [`goToPage`](#public-js-control-surface) sibling.
    pub fn page(mut self, n: u32) -> Self {
        self.initial_page = Some(n);
        self
    }
}

impl Render for Viewer {
    fn render(&self) -> Markup {
        let sources_json = json_string_array(&self.sources);
        let sequence_attr = self.sequence.map(|on| if on { "true" } else { "false" });
        // Both omitted at their defaults, so a consumer that never calls
        // `.chrome()`/`.page()` gets no `chrome` or `page` attribute.
        let chrome_attr = if self.chrome { None } else { Some("false") };
        let page_str = self.initial_page.map(|n| n.to_string());
        html! {
            image-viewer
                id=(self.id)
                engine="osd-wc"
                tile-sources=(sources_json)
                auth-mode=(self.auth.as_attr())
                navigator[self.navigator]
                sequence=[sequence_attr]
                adjust[self.adjust]
                label=[self.label.as_deref()]
                chrome=[chrome_attr]
                page=[page_str.as_deref()]
            {
                @if let Some(fallback) = &self.fallback { (fallback) }
            }
        }
    }
}

/// Serialise `items` as a compact JSON array of strings, e.g.
/// `["a","b"]`. Hand-rolled to avoid a serde dependency; only `"` `\` and the
/// C0 control chars need escaping inside a JSON string.
///
/// Returns a plain `String` **on purpose**: maud HTML-escapes it as an
/// attribute value (`"` → `&quot;`), which the browser decodes back to `"`
/// before `JSON.parse` sees it. Wrapping it in `PreEscaped` would emit raw `"`
/// and break out of the attribute — an HTML-injection hole. Do not "optimise"
/// this into `PreEscaped`.
fn json_string_array(items: &[String]) -> String {
    let mut out = String::from("[");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_json_string(&mut out, item);
    }
    out.push(']');
    out
}

/// Append `s` as a quoted, escaped JSON string to `out`. The one place the
/// JSON-string escaping rules live, used by [`json_string_array`]. Only `"` `\`
/// and the C0 control chars need escaping.
fn push_json_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::ASSETS;

    #[test]
    fn asset_names_are_unique() {
        // `asset` returns the first row with a name, so a second row with the
        // same name would never be served.
        let mut names: Vec<&str> = ASSETS.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "ASSETS has two rows with the same name");
    }
}
