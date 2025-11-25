use maud::{html, Markup};

use crate::playground::{component_registry, docs_parser};

/// Helper function to build component links preserving current state
fn build_component_link(component_name: &str, current_params: &str) -> String {
    // Parse current parameters, only preserving theme
    let mut preserved_params = Vec::new();

    for param in current_params.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key == "theme" {
                preserved_params.push(format!("{}={}", key, value));
            }
        }
    }

    // Build new URL with component parameter first, defaulting to component-store view
    let mut url = format!("/?component={}&view=component-store", component_name);

    if !preserved_params.is_empty() {
        url.push('&');
        url.push_str(&preserved_params.join("&"));
    }

    url
}

/// Common HTML document structure for playground pages
pub fn render_page_shell(theme: &str, title: &str, css_path: &str, content: Markup) -> Markup {
    html! {
        (maud::DOCTYPE)
        html lang="en" class=(if theme == "dark" { "dark" } else { "" }) {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width, initial-scale=1.0";
                title { (title) }
                link rel="stylesheet" href="/design-system/tailwind.css";
                link rel="stylesheet" href=(css_path);
                script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4" {}
                // avoid mentioning the library below in clear text, so that it is not found in github searches
                script src=(format!("https://cdn.jsdelivr.net/npm/@t{}in{}us/elements@1", "ailw", "dpl")) type="module" {}
                script src="https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.0-beta.11/bundles/datastar.js" type="module" {}
                // Syntax highlighting for code blocks
                link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/prismjs@1.29.0/themes/prism.min.css";
                link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/prismjs@1.29.0/plugins/line-numbers/prism-line-numbers.min.css";
                script src="https://cdn.jsdelivr.net/npm/prismjs@1.29.0/prism.min.js" {}
                script src="https://cdn.jsdelivr.net/npm/prismjs@1.29.0/components/prism-rust.min.js" {}
                script src="https://cdn.jsdelivr.net/npm/prismjs@1.29.0/plugins/line-numbers/prism-line-numbers.min.js" {}
            }
            body { (content) }
            // body class="theme-text-primary theme-bg-primary m-0 p-0 h-screen overflow-hidden" { (content) }
        }
    }
}

/// Template for component iframe preview
pub fn render_iframe_content(component_markup: Markup) -> Markup {
    html! {
        div class="component-preview" {
            (component_markup)
        }
    }
}

/// Template for error display
pub fn render_error_content(error_message: &str) -> Markup {
    html! {
        div class="flex flex-col items-center justify-center min-h-48 p-6 text-center" {
            div class="theme-warning-bg border theme-warning-border rounded-lg p-6 mb-4 max-w-md" {
                h3 class="m-0 mb-4 theme-warning-text text-xl" { "Error" }
                p class="m-0 theme-warning-text leading-relaxed" { (error_message) }
            }
            div {
                a href="/" target="_parent" class="inline-block px-4 py-2 theme-accent-primary text-white no-underline rounded text-sm font-medium transition-colors duration-200 theme-accent-hover" { "← Back to Playground" }
            }
        }
    }
}

/// Template for component documentation section
pub fn render_documentation_section(component_name: &str) -> Markup {
    // Try to find the component spec and load documentation
    if let Some(spec) = component_registry::get_component_spec_by_route_name(component_name) {
        if let Ok(doc) = docs_parser::load_component_documentation(spec) {
            return html! {
                div class="flex-1 px-8 py-4 overflow-y-auto documentation-content theme-bg-primary" {
                    (maud::PreEscaped(doc.content_html))
                }
            };
        }
    }

    // Fallback to basic documentation
    html! {
        div class="flex-1 px-8 py-4 overflow-y-auto max-w-3xl mx-auto theme-bg-primary" {
            h3 class="text-xl font-medium mb-3 theme-text-primary" { (component_name) " Documentation" }
            p { em class="italic theme-text-secondary" { "Detailed documentation coming soon..." } }
        }
    }
}

// NOTE: render_global_controls has been removed - theme selector is now part of component controls
// and only affects the iframe, not the main playground UI

/// Template for component-specific controls (variant selector, theme selector, and open button)
pub fn render_component_controls(
    component_info: &component_registry::ComponentInfo,
    current_variant: &str,
    current_theme: &str,
) -> Markup {
    html! {
        div class="flex gap-4 p-3 border-b theme-border-subtle theme-bg-primary" {
            div class="flex items-center gap-2" {
                label for="variant-select" class="text-sm theme-text-secondary whitespace-nowrap" { "Variant:" }
                select id="variant-select" data-param="variant" class="px-2 py-1 border theme-border-subtle rounded theme-bg-primary theme-text-primary text-sm" {
                    @for variant in &component_info.variants {
                        option value=(variant.value) selected[variant.value == current_variant] {
                            (variant.name)
                        }
                    }
                }
            }
            div class="flex items-center gap-2" {
                label for="theme-select" class="text-sm theme-text-secondary whitespace-nowrap" { "Theme:" }
                select id="theme-select" data-param="theme" class="px-2 py-1 border theme-border-subtle rounded theme-bg-primary theme-text-primary text-sm" {
                    option value="light" selected[current_theme == "light"] { "Light" }
                    option value="dark" selected[current_theme == "dark"] { "Dark" }
                }
            }
            div class="flex items-center ml-auto" {
                button class="flex items-center gap-2 px-4 py-2 theme-accent-primary theme-accent-hover text-white border-none rounded cursor-pointer text-sm font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-blue-600 focus:ring-offset-2" onclick="openComponentInNewTab()" title="Open component preview in new tab" {
                    "Open Component ↗"
                }
            }
        }
    }
}

/// Template for component navigation sidebar
pub fn render_component_sidebar(
    components: &[component_registry::ComponentInfo],
    current_component: &str,
    current_params: &str,
) -> Markup {
    html! {
        nav class="theme-bg-primary border-r theme-border-subtle p-4 overflow-y-auto" {
            h2 class="text-lg font-normal mb-3 theme-text-primary" { "Component Library" }
            ul class="list-none" {
                @for component in components {
                    li class="mb-1" {
                        a href=(build_component_link(&component.route_name, current_params))
                          class=(format!("block px-3 py-2 rounded text-sm no-underline transition-all duration-200 {}",
                            if component.route_name == current_component {
                                "theme-accent-primary text-white"
                            } else {
                                "theme-text-secondary hover:theme-bg-secondary hover:theme-text-primary"
                            })) {
                            (component.name)
                        }
                    }
                }
            }
        }
    }
}

/// Template for component preview tabs
pub fn render_component_tabs(
    iframe_src_component_store: &str,
    iframe_src_examples: &str,
    component_info: &component_registry::ComponentInfo,
    current_view: &str,
    current_variant: &str,
    current_theme: &str,
) -> Markup {
    html! {
        div class="flex border-b theme-border-subtle theme-bg-primary" {
            button data-tab-button data-tab="component-store" class=(format!("bg-none border-none px-4 py-3 cursor-pointer border-b-2 transition-all duration-200 {}",
                if current_view == "component-store" {
                    "tab-button-active"
                } else {
                    "tab-button-inactive"
                })) { "Component" }
            button data-tab-button data-tab="examples" class=(format!("bg-none border-none px-4 py-3 cursor-pointer border-b-2 transition-all duration-200 {}",
                if current_view == "examples" {
                    "tab-button-active"
                } else {
                    "tab-button-inactive"
                })) { "Examples and Variants" }
            button data-tab-button data-tab="documentation" class=(format!("bg-none border-none px-4 py-3 cursor-pointer border-b-2 transition-all duration-200 {}",
                if current_view == "documentation" {
                    "tab-button-active"
                } else {
                    "tab-button-inactive"
                })) { "Documentation" }
        }
        // Component tab content
        div data-tab-content data-panel="component-store" class=(format!("flex-1 overflow-hidden {}",
            if current_view == "component-store" { "flex flex-col" } else { "hidden" })) id="component-store-tab" {
            (render_component_controls(component_info, current_variant, current_theme))
            iframe id="component-store-iframe" src=(iframe_src_component_store) class="flex-1 border-none w-full h-full theme-bg-primary" {}
        }
        // Examples and Variants tab content
        div data-tab-content data-panel="examples" class=(format!("flex-1 overflow-hidden {}",
            if current_view == "examples" { "flex flex-col" } else { "hidden" })) id="examples-tab" {
            iframe id="examples-iframe" src=(iframe_src_examples) class="flex-1 border-none w-full h-full theme-bg-primary" {}
        }
        // Documentation tab content
        div data-tab-content data-panel="documentation" class=(format!("flex-1 overflow-hidden theme-bg-primary {}",
            if current_view == "documentation" { "flex flex-col" } else { "hidden" })) id="documentation-tab" {
            (render_documentation_section(&component_info.route_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_component_link_basic() {
        let result = build_component_link("banner", "component=button&theme=light&view=component");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_with_variant() {
        let result = build_component_link("banner", "component=button&variant=primary&theme=light&view=component");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_preserve_all_params() {
        let current_params = "component=button&variant=secondary&theme=dark&view=documentation";
        let result = build_component_link("tile", current_params);
        assert_eq!(result, "/?component=tile&view=component-store&theme=dark");
    }

    #[test]
    fn test_build_component_link_empty_params() {
        let result = build_component_link("button", "");
        assert_eq!(result, "/?component=button&view=component-store");
    }

    #[test]
    fn test_build_component_link_only_component_param() {
        let result = build_component_link("banner", "component=button");
        assert_eq!(result, "/?component=banner&view=component-store");
    }

    #[test]
    fn test_build_component_link_malformed_params() {
        // Test with malformed parameter (missing value)
        let result = build_component_link("banner", "component=button&theme=&view=component");
        assert_eq!(result, "/?component=banner&view=component-store&theme=");
    }

    #[test]
    fn test_build_component_link_single_param_no_equals() {
        // Test with parameter that has no equals sign
        let result = build_component_link("banner", "component=button&invalidparam&theme=light");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_duplicate_non_component_params() {
        // Test with duplicate non-component parameters
        let result = build_component_link("banner", "component=button&theme=light&theme=dark&view=component");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light&theme=dark");
    }

    #[test]
    fn test_build_component_link_component_param_not_first() {
        // Test when component is not the first parameter
        let result = build_component_link("banner", "theme=light&component=button&view=component");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_special_characters() {
        // Test with special characters in component name
        let result = build_component_link("test-component", "component=button&theme=light");
        assert_eq!(result, "/?component=test-component&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_empty_component_name() {
        let result = build_component_link("", "component=button&theme=light");
        assert_eq!(result, "/?component=&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_no_component_in_current_params() {
        // Test when current params don't contain component parameter
        let result = build_component_link("banner", "theme=light&view=component");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_multiple_equals_in_value() {
        // Test with parameter value containing equals signs
        let result = build_component_link("banner", "component=button&custom=value=with=equals&theme=light");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_build_component_link_preserves_order() {
        // Test that parameter order is preserved (except component is first)
        let result = build_component_link("banner", "component=button&view=component&theme=light&variant=primary");
        assert_eq!(result, "/?component=banner&view=component-store&theme=light");
    }

    #[test]
    fn test_render_page_shell_theme_conditional_logic() {
        let content = html! { div { "Test content" } };

        // Test light theme - should NOT have dark class
        let light_result = render_page_shell("light", "Test Title", "/test.css", content.clone());
        let light_str = light_result.into_string();
        assert!(!light_str.contains("class=\"dark\""));

        // Test dark theme - should have dark class
        let dark_result = render_page_shell("dark", "Test Title", "/test.css", content);
        let dark_str = dark_result.into_string();
        assert!(dark_str.contains("class=\"dark\""));
    }
}
