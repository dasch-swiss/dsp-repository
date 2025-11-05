use maud::Markup;

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::showcases::*;

/// Represents a single example within a component showcase
///
/// This structure captures both the rendered component and the Rust code
/// that generates it, enabling a code-view toggle feature in the playground.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ComponentExample {
    /// Unique identifier for this example
    pub id: &'static str,
    /// Display name for this example
    pub name: &'static str,
    /// Optional description explaining when/how to use this variant
    pub description: Option<&'static str>,
    /// The Rust code as a string (captured via stringify!)
    pub code: &'static str,
    /// The rendered Markup
    pub markup: Markup,
}

/// Represents a section of related examples in a component showcase
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ComponentSection {
    /// Section title (e.g., "Button Variants", "Icon Buttons")
    pub title: &'static str,
    /// Optional section description
    pub description: Option<&'static str>,
    /// Examples within this section
    pub examples: Vec<ComponentExample>,
}

/// Macro for creating component examples with automatic code capture
///
/// This macro captures the code that generates a component as a string
/// while also executing it to produce the rendered markup.
///
/// # Usage
///
/// ```rust
/// use crate::example;
///
/// let ex = example!{
///     id: "primary-button",
///     name: "Primary Button",
///     description: "Main call-to-action button",
///     code: {
///         button::button("Click Me")
///             .variant(ButtonVariant::Primary)
///             .onclick("console.log('Clicked!')")
///             .build()
///     }
/// };
/// ```
#[macro_export]
macro_rules! example {
    (
        id: $id:expr,
        name: $name:expr,
        description: $description:expr,
        code: $code:block
    ) => {
        $crate::playground::renderer::ComponentExample {
            id: $id,
            name: $name,
            description: Some($description),
            code: stringify!($code),
            markup: $code,
        }
    };
    (
        id: $id:expr,
        name: $name:expr,
        code: $code:block
    ) => {
        $crate::playground::renderer::ComponentExample {
            id: $id,
            name: $name,
            description: None,
            code: stringify!($code),
            markup: $code,
        }
    };
}

/// Trait for rendering components with different variants
///
/// Each component renderer implements this trait to provide its showcase
/// examples in the playground.
pub trait ComponentRenderer {
    /// Render a component with the specified variant and parameters
    ///
    /// Returns the rendered markup. In the future, this may be extended
    /// to return Vec<ComponentSection> for full code-view support.
    fn render_variant(&self, variant: &str, params: &PlaygroundParams) -> PlaygroundResult<Markup>;

    /// Get the default variant for this component
    fn default_variant(&self) -> &'static str;

    /// Get all supported variants for this component
    #[allow(dead_code)]
    fn supported_variants(&self) -> Vec<&'static str>;
}

/// Registry for all component renderers
///
/// Maps component names to their renderer implementations.
pub struct ComponentRendererRegistry;

impl ComponentRendererRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            "button" => Some(Box::new(ButtonRenderer)),
            "footer" => Some(Box::new(FooterRenderer)),
            "header" => Some(Box::new(HeaderRenderer)),
            "hero" => Some(Box::new(HeroRenderer)),
            "icon" => Some(Box::new(IconRenderer)),
            "link" => Some(Box::new(LinkRenderer)),
            "menu" => Some(Box::new(MenuRenderer)),
            "menu-item" => Some(Box::new(MenuItemRenderer)),
            "shell" => Some(Box::new(ShellRenderer)),
            _ => None,
        }
    }
}
