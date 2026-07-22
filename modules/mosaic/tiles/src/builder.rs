//! Shared builder machinery for complex tiles.
//!
//! Tiles with several independent optional axes are built with a builder rather
//! than a `*Props` struct (see `docs/src/mosaic/component-api-conventions.md`).
//! `ComponentBuilder` supplies the options every tile shares — `with_id` and
//! `with_test_id` — plus `build`, so each builder only writes its own axes.
//!
//! Every builder also implements [`maud::Render`], so it can be spliced into an
//! `html!` template directly without calling `.build()`:
//!
//! ```ignore
//! html! { (button("Save").variant(ButtonVariant::Secondary)) }
//! ```
//!
//! Builder structs are marked `#[must_use]`, so one that is constructed but
//! never spliced or built is a compile-time warning.

use maud::Markup;

/// Options and finaliser shared by every complex-tile builder.
///
/// Implementors provide `id_mut`, `test_id_mut`, and `build`; the `with_*`
/// setters come for free. Bring this trait into scope (`use mosaic_tiles::ComponentBuilder`)
/// to call `with_id` / `with_test_id` / `build`. Splicing a builder into `html!`
/// needs no import — Maud calls [`maud::Render`] directly.
pub trait ComponentBuilder: Sized {
    /// Mutable access to the builder's optional `id` field.
    fn id_mut(&mut self) -> &mut Option<String>;

    /// Mutable access to the builder's optional `data-testid` field.
    fn test_id_mut(&mut self) -> &mut Option<String>;

    /// Consume the builder and render it to `Markup`.
    ///
    /// Only needed outside an `html!` template (e.g. returning a bare `Markup`
    /// or in a test); inside a template, splice the builder directly.
    fn build(self) -> Markup;

    /// Set the HTML `id` attribute.
    #[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
    fn with_id(mut self, id: impl Into<String>) -> Self {
        *self.id_mut() = Some(id.into());
        self
    }

    /// Set the `data-testid` attribute (stable selector for the test suites).
    #[must_use = "a builder renders nothing unless it is spliced into `html!` or `.build()` is called"]
    fn with_test_id(mut self, test_id: impl Into<String>) -> Self {
        *self.test_id_mut() = Some(test_id.into());
        self
    }
}
