use maud::{html, Markup};

use crate::builder_common::ComponentBuilder;

const BASE_CLASSES: &str =
    "text-indigo-900 hover:text-indigo-600 visited:text-indigo-600 font-medium cursor-pointer no-underline dark:text-indigo-400 dark:hover:text-indigo-300 dark:visited:text-indigo-300";

#[derive(Debug, Clone)]
pub enum LinkTarget {
    SelfTarget,
    Blank,
    Parent,
    Top,
}

impl LinkTarget {
    fn target(&self) -> &'static str {
        match self {
            LinkTarget::SelfTarget => "_self",
            LinkTarget::Blank => "_blank",
            LinkTarget::Parent => "_parent",
            LinkTarget::Top => "_top",
        }
    }

    fn rel(&self) -> Option<&'static str> {
        match self {
            // Security: prevent window.opener access and tabnabbing attacks
            LinkTarget::Blank => Some("noopener noreferrer"),
            _ => None,
        }
    }
}

/// Builder for creating link components with flexible configuration
///
/// # Examples
///
/// ```rust
/// use components::{link::link, LinkTarget, ComponentBuilder};
///
/// // Simple link
/// let simple = link("Click here", "/path").build();
///
/// // External link (opens in new tab)
/// let external = link("External", "https://example.com")
///     .target(LinkTarget::Blank)
///     .build();
///
/// // Link with ID and test ID
/// let customized = link("Customized", "/path")
///     .with_id("my-link")
///     .with_test_id("link-test")
///     .build();
/// ```
pub struct LinkBuilder {
    text: String,
    url: String,
    target: LinkTarget,
    id: Option<String>,
    test_id: Option<String>,
}

impl ComponentBuilder for LinkBuilder {
    fn id_mut(&mut self) -> &mut Option<String> {
        &mut self.id
    }

    fn test_id_mut(&mut self) -> &mut Option<String> {
        &mut self.test_id
    }

    fn build(self) -> Markup {
        let test_id = self.test_id.as_deref().unwrap_or("link");
        let rel_attr = self.target.rel();

        html! {
            @if let Some(rel) = rel_attr {
                a href=(self.url)
                  target=(self.target.target())
                  rel=(rel)
                  class=(BASE_CLASSES)
                  data-testid=(test_id)
                  id=[self.id] {
                    (self.text)
                }
            } @else {
                a href=(self.url)
                  target=(self.target.target())
                  class=(BASE_CLASSES)
                  data-testid=(test_id)
                  id=[self.id] {
                    (self.text)
                }
            }
        }
    }
}

impl LinkBuilder {
    /// Creates a new link builder with the specified text and URL
    ///
    /// Default target is `LinkTarget::SelfTarget` (same window)
    fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            url: url.into(),
            target: LinkTarget::SelfTarget,
            id: None,
            test_id: None,
        }
    }

    /// Sets the link target (where the link opens)
    ///
    /// Use `LinkTarget::Blank` for external links to open in new tab
    #[must_use = "builder does nothing unless you call .build()"]
    pub fn target(mut self, target: LinkTarget) -> Self {
        self.target = target;
        self
    }
}

/// Creates a new link with the specified text and URL
///
/// Returns a `LinkBuilder` for further customization
///
/// # Examples
///
/// ```rust
/// use components::{link::link, LinkTarget, ComponentBuilder};
///
/// // Simple link
/// let simple = link("Home", "/").build();
///
/// // With customization
/// let external = link("External", "https://example.com")
///     .target(LinkTarget::Blank)
///     .with_id("external-link")
///     .build();
/// ```
#[must_use = "call .build() to render the component"]
pub fn link(text: impl Into<String>, url: impl Into<String>) -> LinkBuilder {
    LinkBuilder::new(text, url)
}

/// Convenience function for creating an external link (opens in new tab)
///
/// Equivalent to `link(text, url).target(LinkTarget::Blank).build()`
pub fn link_external(text: impl Into<String>, url: impl Into<String>) -> Markup {
    link(text, url).target(LinkTarget::Blank).build()
}
