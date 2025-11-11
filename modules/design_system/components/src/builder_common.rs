use maud::Markup;

/// Common builder functionality for UI components
///
/// This trait provides default implementations for standard builder methods
/// that are common across most UI components:
///
/// - `with_id()` - Sets the HTML `id` attribute
/// - `with_test_id()` - Sets the `data-testid` attribute for testing
/// - `build()` - Consumes the builder and returns the rendered markup
///
/// # Implementation
///
/// To implement this trait for your builder:
///
/// 1. Add `id: Option<String>` and `test_id: Option<String>` fields to your builder struct
/// 2. Implement `id_mut()` and `test_id_mut()` to return mutable references to these fields
/// 3. Implement `build()` with your component's rendering logic
///
/// Once implemented, your builder automatically inherits `with_id()` and `with_test_id()` methods
/// with no additional code needed.
///
/// # Example
///
/// ```rust
/// use components::builder_common::ComponentBuilder;
/// use maud::{html, Markup};
///
/// pub struct ButtonBuilder {
///     text: String,
///     id: Option<String>,
///     test_id: Option<String>,
/// }
///
/// impl ComponentBuilder for ButtonBuilder {
///     fn id_mut(&mut self) -> &mut Option<String> {
///         &mut self.id
///     }
///
///     fn test_id_mut(&mut self) -> &mut Option<String> {
///         &mut self.test_id
///     }
///
///     fn build(self) -> Markup {
///         html! {
///             button id=[self.id] data-testid=[self.test_id] {
///                 (self.text)
///             }
///         }
///     }
/// }
///
/// // Now ButtonBuilder automatically has with_id() and with_test_id()!
/// let button = ButtonBuilder { text: "Click".into(), id: None, test_id: None }
///     .with_id("my-button")
///     .with_test_id("test-button")
///     .build();
/// ```
pub trait ComponentBuilder: Sized {
    /// Returns a mutable reference to the optional ID field
    ///
    /// This is used by the default `with_id()` implementation.
    fn id_mut(&mut self) -> &mut Option<String>;

    /// Returns a mutable reference to the optional test ID field
    ///
    /// This is used by the default `with_test_id()` implementation.
    fn test_id_mut(&mut self) -> &mut Option<String>;

    /// Builds and returns the final component markup
    ///
    /// This method must be implemented by each builder with component-specific
    /// rendering logic. It consumes the builder and returns `maud::Markup`.
    fn build(self) -> Markup;

    /// Sets the HTML id attribute for the component
    ///
    /// # Example
    /// ```rust
    /// use components::{button::button, ComponentBuilder};
    ///
    /// let btn = button("Click me")
    ///     .with_id("submit-button")
    ///     .build();
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    fn with_id(mut self, id: impl Into<String>) -> Self {
        *self.id_mut() = Some(id.into());
        self
    }

    /// Sets the data-testid attribute for testing
    ///
    /// This attribute is used by testing frameworks to locate elements.
    ///
    /// # Example
    /// ```rust
    /// use components::{button::button, ComponentBuilder};
    ///
    /// let btn = button("Click me")
    ///     .with_test_id("submit-btn")
    ///     .build();
    /// ```
    #[must_use = "builder does nothing unless you call .build()"]
    fn with_test_id(mut self, test_id: impl Into<String>) -> Self {
        *self.test_id_mut() = Some(test_id.into());
        self
    }
}
