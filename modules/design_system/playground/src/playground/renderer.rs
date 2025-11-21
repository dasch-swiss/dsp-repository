use maud::Markup;

use crate::playground::components as component_store;
use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::showcases::*;

/// Represents a single example within a component showcase
///
/// This structure captures both the rendered component and the Rust code
/// that generates it, enabling a code-view toggle feature in the playground.
#[derive(Debug, Clone)]
pub struct ComponentExample {
    /// Unique identifier for this example (for future use with anchor links)
    #[allow(dead_code)]
    pub id: &'static str,
    /// Display name for this example
    #[allow(dead_code)]
    pub name: &'static str,
    /// Optional description explaining when/how to use this variant
    #[allow(dead_code)]
    pub description: Option<&'static str>,
    /// The Rust code as a string (captured via stringify!)
    pub code: &'static str,
    /// The rendered Markup
    pub markup: Markup,
}

/// Represents a section of related examples in a component showcase
#[derive(Debug, Clone)]
pub struct ComponentSection {
    /// Section title (e.g., "Button Variants", "Icon Buttons")
    pub title: &'static str,
    /// Optional section description
    pub description: Option<&'static str>,
    /// Examples within this section
    pub examples: Vec<ComponentExample>,
}

/// Helper function for creating component examples with code display
///
/// Creates a ComponentExample with description.
pub fn example_with_description(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    code: &'static str,
    markup: Markup,
) -> ComponentExample {
    ComponentExample { id, name, description: Some(description), code, markup }
}

/// Helper function for creating component examples with code display
///
/// Creates a ComponentExample without description.
pub fn example(id: &'static str, name: &'static str, code: &'static str, markup: Markup) -> ComponentExample {
    ComponentExample { id, name, description: None, code, markup }
}

/// Trait for rendering components with different variants
///
/// Each component renderer implements this trait to provide its showcase
/// examples in the playground.
pub trait ComponentRenderer {
    /// Render a component with the specified variant and parameters
    ///
    /// Default implementation returns a placeholder message. Override this method
    /// for Component Store renderers or showcases without code-view support.
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        use maud::html;
        Ok(html! {
            div class="p-8" {
                p class="text-gray-600 dark:text-gray-400" {
                    "This component uses code-view rendering. Please implement render_variant_with_code()."
                }
            }
        })
    }

    /// Render component with code-view support (optional)
    ///
    /// Returns structured sections with examples that include both rendered
    /// markup and source code. This enables toggle between rendered and code views.
    ///
    /// Default implementation returns None, indicating no code-view support.
    fn render_variant_with_code(
        &self,
        _variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        Ok(None)
    }
}

/// Registry for all component renderers (Examples and Variants tab)
///
/// Maps component names to their renderer implementations for the Examples and Variants tab.
pub struct ComponentRendererRegistry;

impl ComponentRendererRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            "button" => Some(Box::new(ButtonRenderer)),
            "dropdown" => Some(Box::new(DropdownRenderer)),
            "footer" => Some(Box::new(FooterRenderer)),
            "header" => Some(Box::new(HeaderRenderer)),
            "hero" => Some(Box::new(HeroRenderer)),
            "icon" => Some(Box::new(IconRenderer)),
            "link" => Some(Box::new(LinkRenderer)),
            "logo-cloud" => Some(Box::new(LogoCloudRenderer)),
            "menu" => Some(Box::new(MenuRenderer)),
            "menu-item" => Some(Box::new(MenuItemRenderer)),
            "shell" => Some(Box::new(ShellRenderer)),
            _ => None,
        }
    }
}

/// Registry for component isolation renderers (Component Store tab)
///
/// Maps component names to their renderer implementations for the Component Store tab.
/// These renderers display isolated component variants for testing and visual regression.
pub struct ComponentIsolationRegistry;

impl ComponentIsolationRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            "button" => Some(Box::new(component_store::ButtonComponentRenderer)),
            "dropdown" => Some(Box::new(component_store::DropdownComponentRenderer)),
            "footer" => Some(Box::new(component_store::FooterComponentRenderer)),
            "header" => Some(Box::new(component_store::HeaderComponentRenderer)),
            "hero" => Some(Box::new(component_store::HeroComponentRenderer)),
            "icon" => Some(Box::new(component_store::IconComponentRenderer)),
            "link" => Some(Box::new(component_store::LinkComponentRenderer)),
            "logo-cloud" => Some(Box::new(component_store::LogoCloudComponentRenderer)),
            "menu" => Some(Box::new(component_store::MenuComponentRenderer)),
            "menu-item" => Some(Box::new(component_store::MenuItemComponentRenderer)),
            "shell" => Some(Box::new(component_store::ShellComponentRenderer)),
            _ => None,
        }
    }
}

/// Render component sections with code-view toggle support
///
/// Creates a showcase with toggle buttons that allow switching between
/// rendered view and code view for each example.
pub fn render_sections_with_code_view(sections: Vec<ComponentSection>) -> Markup {
    use maud::html;

    html! {
        div class="flex flex-col gap-6 p-8" {
            div class="mb-4 p-4 bg-blue-50 dark:bg-blue-950 rounded-md" {
                p class="text-sm text-blue-900 dark:text-blue-100" {
                    "💡 Toggle between rendered view and code view using the buttons. All buttons include DataStar onclick handlers."
                }
            }

            @for section in sections {
                @let checkbox_id = format!("toggle-{}", section.title.to_lowercase().replace(' ', "-"));

                // Card wrapper for entire section
                div class="border border-gray-200 dark:border-gray-700 rounded-lg" {
                    // Hidden checkbox for CSS-only toggle
                    input type="checkbox" id=(checkbox_id) class="peer hidden" onchange="if(this.checked && typeof Prism !== 'undefined') { setTimeout(() => Prism.highlightAll(), 10); }";

                    // Section header with toggle
                    div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between" {
                        div {
                            h3 class="text-lg font-semibold" { (section.title) }
                            @if let Some(desc) = section.description {
                                p class="text-sm text-gray-600 dark:text-gray-400 mt-1" { (desc) }
                            }
                        }
                        label
                            for=(checkbox_id)
                            class="px-3 py-1 text-xs font-medium rounded-md border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800 cursor-pointer"
                            {
                            span class="peer-checked:hidden" { "Show Code" }
                            span class="hidden peer-checked:inline" { "Show Rendered" }
                        }
                    }

                    // Rendered view (all examples, hidden when checkbox is checked)
                    div class="p-6 peer-checked:hidden flex flex-col gap-6 items-start" {
                        @for example in &section.examples {
                            (example.markup)
                        }
                    }

                    // Code view (all code examples with dividers, hidden by default)
                    div class="p-4 hidden peer-checked:block" {
                        @for (idx, example) in section.examples.iter().enumerate() {
                            @if idx > 0 {
                                div class="border-t border-gray-200 dark:border-gray-700 my-4" {}
                            }
                            pre class="line-numbers" {
                                code class="language-rust" {
                                    (example.code)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
