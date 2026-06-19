//! Mosaic tiles: small, reusable server-rendered UI primitives.
//!
//! Each tile is a `fn -> maud::Markup`. Variant enums expose `css_class()`
//! returning complete, literal class strings so Tailwind's source scanner sees
//! every class. Design tokens live in
//! `src/components/theme_provider/tokens.css`, imported by the consuming app's
//! Tailwind entry.

mod components;

pub use components::*;
