//! Presentation-layer appearance and per-project theme knob types for CPE.
//!
//! `Appearance` is the resolved colour scheme for a request (default
//! `Light`). It is threaded from the HTTP request through `RenderContext` to
//! `render_page`, which emits `data-appearance="dark"` on `<html>` only when
//! the scheme is dark; light emits no attribute (it is the CSS baseline). CPE
//! deliberately does not follow the OS `prefers-color-scheme` — light is always
//! the default and the header toggle is the only switch.
//!
//! `ThemeKnob`, `Accent`, `Background`, and `Typeface` form the per-project
//! visual design knob. The knob is declared in `config.kdl` under `site { theme {
//! … } }` and lowered by the validator into a typed `ThemeKnob`. The renderer
//! emits it as `data-accent`, `data-bg`, and `data-pick` attributes on `<html>`.
//! Absent `theme {}` block ⇒ `ThemeKnob::default()` (Ochre accent + Tint
//! background + Newsreader typeface). The allowed values are authoritative from
//! `areas/access/cpe/components/style/tokens.css` (`data-accent` ∈ {ochre, green,
//! teal, red}; `data-bg` ∈ {neutral, tint}; `data-pick` ∈ the 11 faces in
//! `Typeface::all()`).

// Per-project theme knob

/// The accent colour for the project. Controls the `data-accent` attribute on
/// `<html>`. `areas/access/cpe/components/style/tokens.css` defines colour-token overrides for
/// each value under `[data-accent="<value>"]`.
///
/// `Ochre` is the default — the "warm scholarly" option most suited to
/// archival-humanities projects. The other values give projects a distinct
/// identity without leaving the CPE design language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Accent {
    /// Warm amber-gold — the default accent for most projects.
    #[default]
    Ochre,
    /// Cool botanical green.
    Green,
    /// Quiet sea-teal.
    Teal,
    /// Vivid archival red.
    Red,
}

impl Accent {
    /// Parse the accent value from a `config.kdl` string.
    ///
    /// Mirrors `FacetKind::parse_config_str` — named `parse_config_str` (not
    /// `from_str`) to avoid confusion with `std::str::FromStr`. Returns `None`
    /// for any unrecognised value; the validator emits `UnknownThemeValue`.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "ochre" => Some(Self::Ochre),
            "green" => Some(Self::Green),
            "teal" => Some(Self::Teal),
            "red" => Some(Self::Red),
            _ => None,
        }
    }

    /// Return the value for the `data-accent` HTML attribute.
    ///
    /// Round-trips with `parse_config_str`:
    /// `parse_config_str(a.as_data_attr()) == Some(a)`.
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Ochre => "ochre",
            Self::Green => "green",
            Self::Teal => "teal",
            Self::Red => "red",
        }
    }
}

/// The background tint for the project. Controls the `data-bg` attribute on
/// `<html>`. `Tint` applies a faint warm wash; `Neutral` leaves the surface
/// fully neutral. Together with the accent, it sets how warm the surface reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Background {
    /// Neutral surface — no tint applied.
    Neutral,
    /// Faint warm tint — the default, suits most archival projects.
    #[default]
    Tint,
}

impl Background {
    /// Parse the background value from a `config.kdl` string.
    ///
    /// Returns `None` for any unrecognised value; the validator emits
    /// `UnknownThemeValue`.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "neutral" => Some(Self::Neutral),
            "tint" => Some(Self::Tint),
            _ => None,
        }
    }

    /// Return the value for the `data-bg` HTML attribute.
    ///
    /// Round-trips with `parse_config_str`:
    /// `parse_config_str(b.as_data_attr()) == Some(b)`.
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Tint => "tint",
        }
    }
}

/// The typeface for the project. Controls the `data-pick` attribute on
/// `<html>`. `areas/access/cpe/components/style/tokens.css` defines a font-token override for
/// each value under `[data-pick="<value>"]`.
///
/// The enum value *is* the `data-pick` face name, so it round-trips exactly like
/// `Accent`/`Background` (`parse_config_str(t.as_data_attr()) == Some(t)`). All
/// eleven faces `tokens.css` defines are exposed: serif-led faces, two
/// serif-title / sans-body pairings, and sans-led faces.
///
/// `Newsreader` is the default — it equals the `data-direction="edition"` font,
/// so the default emits `data-pick="newsreader"` as a visual no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Typeface {
    // Bookish (serif-led, literary)
    /// Newsreader — the edition default.
    #[default]
    Newsreader,
    /// EB Garamond.
    Garamond,
    /// Crimson Pro.
    Crimson,
    // Balanced (middle road, incl. serif-title / sans-body pairings)
    /// Spectral.
    Spectral,
    /// Lora.
    Lora,
    /// Literata.
    Literata,
    /// Spectral title + Figtree body (pairing).
    SpectralFigtree,
    /// Newsreader title + Figtree body (pairing).
    NewsreaderFigtree,
    // Casual (sans-led, approachable)
    /// Figtree.
    Figtree,
    /// Hanken Grotesk.
    Hanken,
    /// Schibsted Grotesk.
    Schibsted,
}

impl Typeface {
    /// Parse the typeface value from a `config.kdl` string (the `data-pick`
    /// face name). Returns `None` for any unrecognised value; the validator
    /// emits `UnknownThemeValue`.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "newsreader" => Some(Self::Newsreader),
            "garamond" => Some(Self::Garamond),
            "crimson" => Some(Self::Crimson),
            "spectral" => Some(Self::Spectral),
            "lora" => Some(Self::Lora),
            "literata" => Some(Self::Literata),
            "spectral-figtree" => Some(Self::SpectralFigtree),
            "newsreader-figtree" => Some(Self::NewsreaderFigtree),
            "figtree" => Some(Self::Figtree),
            "hanken" => Some(Self::Hanken),
            "schibsted" => Some(Self::Schibsted),
            _ => None,
        }
    }

    /// Return the value for the `data-pick` HTML attribute.
    ///
    /// Round-trips with `parse_config_str`:
    /// `parse_config_str(t.as_data_attr()) == Some(t)`.
    pub fn as_data_attr(self) -> &'static str {
        match self {
            Self::Newsreader => "newsreader",
            Self::Garamond => "garamond",
            Self::Crimson => "crimson",
            Self::Spectral => "spectral",
            Self::Lora => "lora",
            Self::Literata => "literata",
            Self::SpectralFigtree => "spectral-figtree",
            Self::NewsreaderFigtree => "newsreader-figtree",
            Self::Figtree => "figtree",
            Self::Hanken => "hanken",
            Self::Schibsted => "schibsted",
        }
    }

    /// The Google Fonts `family=` specs this face needs: the families its
    /// `tokens.css` `[data-pick]` rule names (a pairing lists both). Serifs
    /// request italics; sans faces request weights only.
    #[doc(hidden)]
    pub fn font_families(self) -> &'static [&'static str] {
        match self {
            Self::Newsreader => &[NEWSREADER_FAMILY],
            Self::Garamond => &["EB+Garamond:ital,wght@0,400;0,500;0,600;1,400"],
            Self::Crimson => &["Crimson+Pro:ital,wght@0,400;0,500;0,600;1,400"],
            Self::Spectral => &[SPECTRAL_FAMILY],
            Self::Lora => &[LORA_FAMILY],
            Self::Literata => &["Literata:ital,wght@0,400;0,500;0,600;1,400"],
            Self::SpectralFigtree => &[SPECTRAL_FAMILY, FIGTREE_FAMILY],
            Self::NewsreaderFigtree => &[NEWSREADER_FAMILY, FIGTREE_FAMILY],
            Self::Figtree => &[FIGTREE_FAMILY],
            Self::Hanken => &["Hanken+Grotesk:wght@400;500;600"],
            Self::Schibsted => &["Schibsted+Grotesk:wght@400;500;600"],
        }
    }

    /// All faces, in declaration order. Used by tests and the validator's
    /// "allowed values" message.
    pub fn all() -> &'static [Typeface] {
        &[
            Self::Newsreader,
            Self::Garamond,
            Self::Crimson,
            Self::Spectral,
            Self::Lora,
            Self::Literata,
            Self::SpectralFigtree,
            Self::NewsreaderFigtree,
            Self::Figtree,
            Self::Hanken,
            Self::Schibsted,
        ]
    }
}

const NEWSREADER_FAMILY: &str =
    "Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,500;0,6..72,600;1,6..72,400;1,6..72,500;1,6..72,600";
const SPECTRAL_FAMILY: &str = "Spectral:ital,wght@0,400;0,500;0,600;1,400;1,500;1,600";
const FIGTREE_FAMILY: &str = "Figtree:wght@400;500;600";
/// Lora, also the fixed face of `.dasch-link`
/// (`areas/access/cpe/components/style/shell.css`), which `component::document_head` loads on every
/// project page.
#[doc(hidden)]
pub const LORA_FAMILY: &str = "Lora:ital,wght@0,400;0,500;0,600;1,400";

/// The per-project visual design knob. Declared in `config.kdl` under
/// `site { theme { accent "…"; background "…"; typeface "…" } }` and emitted as
/// `data-accent`, `data-bg`, and `data-pick` attributes on `<html>`.
///
/// All fields have meaningful defaults (`Accent::Ochre` + `Background::Tint` +
/// `Typeface::Newsreader`), so `ThemeKnob::default()` is valid and correct when
/// the `theme {}` block is absent. There is no `Option<ThemeKnob>` — absent is
/// identical to default.
///
/// The enumerated knob can later grow a token-override layer without breaking
/// the `data-accent`/`data-bg`/`data-pick` data-attr contract.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThemeKnob {
    /// The accent colour emitted as `data-accent`.
    pub accent: Accent,
    /// The background tint emitted as `data-bg`.
    pub background: Background,
    /// The typeface emitted as `data-pick`.
    pub typeface: Typeface,
}

// Appearance

/// The resolved colour scheme for a single HTTP request.
///
/// CPE defaults to [`Appearance::Light`] and deliberately ignores the OS
/// `prefers-color-scheme` (see the comment in `tokens.css`). `Dark` is an
/// explicit user choice, stored in the `appearance` cookie and emitted as
/// `data-appearance="dark"` on `<html>`; light emits no attribute (it is the
/// CSS baseline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
}

impl Appearance {
    /// Parse an appearance value from a config or cookie/query string.
    ///
    /// Returns `Some(Appearance)` for the two recognised values (`"light"` /
    /// `"dark"`); `None` for any other (unknown or empty) input, which the
    /// caller treats as "no recognised signal" and sets no cookie.
    pub fn parse_config_str(s: &str) -> Option<Self> {
        match s {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// Return the canonical string representation.
    ///
    /// Round-trips with `parse_config_str`: `parse_config_str(a.as_str()) == Some(a)`.
    /// Used both as the cookie value and as the `data-appearance` attribute value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Accent

    #[test]
    fn accent_parse_ochre() {
        assert_eq!(Accent::parse_config_str("ochre"), Some(Accent::Ochre));
    }

    #[test]
    fn accent_parse_green() {
        assert_eq!(Accent::parse_config_str("green"), Some(Accent::Green));
    }

    #[test]
    fn accent_parse_teal() {
        assert_eq!(Accent::parse_config_str("teal"), Some(Accent::Teal));
    }

    #[test]
    fn accent_parse_red() {
        assert_eq!(Accent::parse_config_str("red"), Some(Accent::Red));
    }

    #[test]
    fn accent_parse_unknown_returns_none() {
        assert_eq!(Accent::parse_config_str("purple"), None);
        assert_eq!(Accent::parse_config_str(""), None);
        assert_eq!(Accent::parse_config_str("Ochre"), None);
    }

    #[test]
    fn accent_as_data_attr_round_trips() {
        for a in [Accent::Ochre, Accent::Green, Accent::Teal, Accent::Red] {
            assert_eq!(
                Accent::parse_config_str(a.as_data_attr()),
                Some(a),
                "round-trip failed for {a:?}"
            );
        }
    }

    #[test]
    fn accent_data_attr_values() {
        assert_eq!(Accent::Ochre.as_data_attr(), "ochre");
        assert_eq!(Accent::Green.as_data_attr(), "green");
        assert_eq!(Accent::Teal.as_data_attr(), "teal");
        assert_eq!(Accent::Red.as_data_attr(), "red");
    }

    // Background

    #[test]
    fn background_parse_neutral() {
        assert_eq!(Background::parse_config_str("neutral"), Some(Background::Neutral));
    }

    #[test]
    fn background_parse_tint() {
        assert_eq!(Background::parse_config_str("tint"), Some(Background::Tint));
    }

    #[test]
    fn background_parse_unknown_returns_none() {
        assert_eq!(Background::parse_config_str("warm"), None);
        assert_eq!(Background::parse_config_str(""), None);
        assert_eq!(Background::parse_config_str("Tint"), None);
    }

    #[test]
    fn background_as_data_attr_round_trips() {
        for b in [Background::Neutral, Background::Tint] {
            assert_eq!(
                Background::parse_config_str(b.as_data_attr()),
                Some(b),
                "round-trip failed for {b:?}"
            );
        }
    }

    #[test]
    fn background_data_attr_values() {
        assert_eq!(Background::Neutral.as_data_attr(), "neutral");
        assert_eq!(Background::Tint.as_data_attr(), "tint");
    }

    // Typeface

    #[test]
    fn typeface_parse_known_values() {
        assert_eq!(Typeface::parse_config_str("newsreader"), Some(Typeface::Newsreader));
        assert_eq!(Typeface::parse_config_str("spectral"), Some(Typeface::Spectral));
        assert_eq!(Typeface::parse_config_str("figtree"), Some(Typeface::Figtree));
        // The two serif-title / sans-body pairings (hyphenated values).
        assert_eq!(Typeface::parse_config_str("spectral-figtree"), Some(Typeface::SpectralFigtree));
        assert_eq!(
            Typeface::parse_config_str("newsreader-figtree"),
            Some(Typeface::NewsreaderFigtree)
        );
        assert_eq!(Typeface::parse_config_str("garamond"), Some(Typeface::Garamond));
    }

    #[test]
    fn typeface_parse_unknown_returns_none() {
        assert_eq!(Typeface::parse_config_str("comic-sans"), None);
        assert_eq!(Typeface::parse_config_str(""), None);
        // Case-sensitive, like Accent/Background.
        assert_eq!(Typeface::parse_config_str("Newsreader"), None);
        // A grouping name such as "bookish" is not a face.
        assert_eq!(Typeface::parse_config_str("bookish"), None);
    }

    #[test]
    fn typeface_as_data_attr_round_trips() {
        for &t in Typeface::all() {
            assert_eq!(
                Typeface::parse_config_str(t.as_data_attr()),
                Some(t),
                "round-trip failed for {t:?}"
            );
        }
    }

    #[test]
    fn typeface_all_has_eleven_faces() {
        assert_eq!(Typeface::all().len(), 11);
    }

    #[test]
    fn typeface_data_attr_values() {
        assert_eq!(Typeface::Newsreader.as_data_attr(), "newsreader");
        assert_eq!(Typeface::Spectral.as_data_attr(), "spectral");
        assert_eq!(Typeface::Figtree.as_data_attr(), "figtree");
        assert_eq!(Typeface::SpectralFigtree.as_data_attr(), "spectral-figtree");
    }

    #[test]
    fn typeface_default_is_newsreader() {
        assert_eq!(Typeface::default(), Typeface::Newsreader);
    }

    // ThemeKnob

    #[test]
    fn theme_knob_default_is_ochre_tint_newsreader() {
        let knob = ThemeKnob::default();
        assert_eq!(knob.accent, Accent::Ochre, "default accent must be Ochre");
        assert_eq!(knob.background, Background::Tint, "default background must be Tint");
        assert_eq!(knob.typeface, Typeface::Newsreader, "default typeface must be Newsreader");
    }

    // parse_config_str

    #[test]
    fn parse_light_returns_light() {
        assert_eq!(Appearance::parse_config_str("light"), Some(Appearance::Light));
    }

    #[test]
    fn parse_dark_returns_dark() {
        assert_eq!(Appearance::parse_config_str("dark"), Some(Appearance::Dark));
    }

    #[test]
    fn parse_unknown_returns_none() {
        assert_eq!(Appearance::parse_config_str("solarized"), None);
    }

    #[test]
    fn parse_empty_returns_none() {
        assert_eq!(Appearance::parse_config_str(""), None);
    }

    #[test]
    fn parse_uppercase_returns_none() {
        // Parsing is case-sensitive; "Dark" is not a valid value.
        assert_eq!(Appearance::parse_config_str("Dark"), None);
        assert_eq!(Appearance::parse_config_str("LIGHT"), None);
    }

    // as_str

    #[test]
    fn as_str_light_is_light() {
        assert_eq!(Appearance::Light.as_str(), "light");
    }

    #[test]
    fn as_str_dark_is_dark() {
        assert_eq!(Appearance::Dark.as_str(), "dark");
    }

    // round-trip

    #[test]
    fn round_trip_light() {
        let a = Appearance::Light;
        assert_eq!(Appearance::parse_config_str(a.as_str()), Some(a));
    }

    #[test]
    fn round_trip_dark() {
        let a = Appearance::Dark;
        assert_eq!(Appearance::parse_config_str(a.as_str()), Some(a));
    }
}
