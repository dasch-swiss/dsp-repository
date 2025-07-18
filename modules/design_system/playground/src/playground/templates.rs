use maud::{html, Markup};

use crate::playground::{components, docs_parser};

/// Helper function to build component links preserving current state
fn build_component_link(component_name: &str, current_params: &str) -> String {
    // Parse current parameters
    let mut preserved_params = Vec::new();

    for param in current_params.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            if key != "component" {
                preserved_params.push(format!("{}={}", key, value));
            }
        }
    }

    // Build new URL with component parameter first
    let mut url = format!("/?component={}", component_name);

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
                link rel="stylesheet" href=(css_path);
            }
            body { (content) }
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
        div .error-container {
            div .error-message {
                h3 { "Error" }
                p { (error_message) }
            }
            div .error-actions {
                a href="/" target="_parent" { "← Back to Playground" }
            }
        }
    }
}

/// Template for component documentation section
pub fn render_documentation_section(component_name: &str) -> Markup {
    // Try to find the component spec and load documentation
    if let Some(spec) = components::get_component_spec_by_route_name(component_name) {
        if let Ok(doc) = docs_parser::load_component_documentation(spec) {
            return html! {
                div class="documentation-content" {
                    (maud::PreEscaped(doc.content_html))
                }
            };
        }
    }

    // Fallback to basic documentation
    html! {
        div class="documentation-content" {
            h3 { (component_name) " Documentation" }
            p { em { "Detailed documentation coming soon..." } }
        }
    }
}

/// Template for component controls section
pub fn render_component_controls(
    component_info: &components::ComponentInfo,
    current_variant: &str,
    current_theme: &str,
    iframe_src: &str,
) -> Markup {
    html! {
        div class="playground-controls" {
            div class="parameter-group" {
                label for="variant-select" { "Variant:" }
                select id="variant-select" data-param="variant" {
                    @for variant in &component_info.variants {
                        option value=(variant.value) selected[variant.value == current_variant] {
                            (variant.name)
                        }
                    }
                }
            }
            div class="parameter-group" {
                label for="theme-select" { "Theme:" }
                select id="theme-select" data-param="theme" {
                    option value="light" selected[current_theme == "light"] { "Light" }
                    option value="dark" selected[current_theme == "dark"] { "Dark" }
                }
            }
            div class="parameter-group" {
                button class="open-in-new-tab-btn" onclick=(format!("window.open('{}', '_blank')", iframe_src)) title="Open component preview in new tab" {
                    "Open Component"
                }
            }
        }
    }
}

/// Template for component navigation sidebar
pub fn render_component_sidebar(
    components: &[components::ComponentInfo],
    current_component: &str,
    current_params: &str,
) -> Markup {
    html! {
        nav class="playground-sidebar" {
            h2 { "Components" }
            ul class="component-list" {
                @for component in components {
                    li {
                        a href=(build_component_link(&component.route_name, current_params))
                          class=(format!("component-link{}", if component.route_name == current_component { " active" } else { "" })) {
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
    iframe_src: &str,
    component_info: &components::ComponentInfo,
    current_view: &str,
) -> Markup {
    html! {
        div class="playground-tabs" {
            button class="tab-button" class=(if current_view == "component" { "active" } else { "" }) data-tab="component" { "Component" }
            button class="tab-button" class=(if current_view == "documentation" { "active" } else { "" }) data-tab="documentation" { "Documentation" }
        }
        div class="tab-content" class=(if current_view == "component" { "active" } else { "" }) id="component-tab" {
            iframe id="component-iframe" src=(iframe_src) {}
        }
        div class="tab-content" class=(if current_view == "documentation" { "active" } else { "" }) id="documentation-tab" {
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
        assert_eq!(result, "/?component=banner&theme=light&view=component");
    }

    #[test]
    fn test_build_component_link_with_variant() {
        let result = build_component_link("banner", "component=button&variant=primary&theme=light&view=component");
        assert_eq!(result, "/?component=banner&variant=primary&theme=light&view=component");
    }

    #[test]
    fn test_build_component_link_preserve_all_params() {
        let current_params = "component=button&variant=secondary&theme=dark&view=documentation";
        let result = build_component_link("tile", current_params);
        assert_eq!(result, "/?component=tile&variant=secondary&theme=dark&view=documentation");
    }

    #[test]
    fn test_build_component_link_empty_params() {
        let result = build_component_link("button", "");
        assert_eq!(result, "/?component=button");
    }

    #[test]
    fn test_build_component_link_only_component_param() {
        let result = build_component_link("banner", "component=button");
        assert_eq!(result, "/?component=banner");
    }

    #[test]
    fn test_build_component_link_malformed_params() {
        // Test with malformed parameter (missing value)
        let result = build_component_link("banner", "component=button&theme=&view=component");
        assert_eq!(result, "/?component=banner&theme=&view=component");
    }

    #[test]
    fn test_build_component_link_single_param_no_equals() {
        // Test with parameter that has no equals sign
        let result = build_component_link("banner", "component=button&invalidparam&theme=light");
        assert_eq!(result, "/?component=banner&theme=light");
    }

    #[test]
    fn test_build_component_link_duplicate_non_component_params() {
        // Test with duplicate non-component parameters
        let result = build_component_link("banner", "component=button&theme=light&theme=dark&view=component");
        assert_eq!(result, "/?component=banner&theme=light&theme=dark&view=component");
    }

    #[test]
    fn test_build_component_link_component_param_not_first() {
        // Test when component is not the first parameter
        let result = build_component_link("banner", "theme=light&component=button&view=component");
        assert_eq!(result, "/?component=banner&theme=light&view=component");
    }

    #[test]
    fn test_build_component_link_special_characters() {
        // Test with special characters in component name
        let result = build_component_link("test-component", "component=button&theme=light");
        assert_eq!(result, "/?component=test-component&theme=light");
    }

    #[test]
    fn test_build_component_link_empty_component_name() {
        let result = build_component_link("", "component=button&theme=light");
        assert_eq!(result, "/?component=&theme=light");
    }

    #[test]
    fn test_build_component_link_no_component_in_current_params() {
        // Test when current params don't contain component parameter
        let result = build_component_link("banner", "theme=light&view=component");
        assert_eq!(result, "/?component=banner&theme=light&view=component");
    }

    #[test]
    fn test_build_component_link_multiple_equals_in_value() {
        // Test with parameter value containing equals signs
        let result = build_component_link("banner", "component=button&custom=value=with=equals&theme=light");
        assert_eq!(result, "/?component=banner&custom=value=with=equals&theme=light");
    }

    #[test]
    fn test_build_component_link_preserves_order() {
        // Test that parameter order is preserved (except component is first)
        let result = build_component_link("banner", "component=button&view=component&theme=light&variant=primary");
        assert_eq!(result, "/?component=banner&view=component&theme=light&variant=primary");
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
