use maud::{html, Markup};

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
                a href="/playground" target="_parent" { "← Back to Playground" }
            }
        }
    }
}

/// Template for component documentation section
pub fn render_documentation_section(component_name: &str, description: &str) -> Markup {
    html! {
        div class="documentation-content" {
            h3 { (component_name) " Documentation" }
            p { (description) }
            p { em { "Detailed documentation coming soon..." } }
        }
    }
}

/// Template for component controls section
pub fn render_component_controls(
    component_info: &crate::playground::components::ComponentInfo,
    current_variant: &str,
    current_theme: &str,
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
            @if component_info.supports_theme {
                div class="parameter-group" {
                    label for="theme-select" { "Theme:" }
                    select id="theme-select" data-param="theme" {
                        option value="light" selected[current_theme == "light"] { "Light" }
                        option value="dark" selected[current_theme == "dark"] { "Dark" }
                    }
                }
            }
        }
    }
}

/// Template for component navigation sidebar
pub fn render_component_sidebar(
    components: &[crate::playground::components::ComponentInfo],
    current_component: &str,
) -> Markup {
    html! {
        nav class="playground-sidebar" {
            h2 { "Components" }
            ul class="component-list" {
                @for component in components {
                    li {
                        a href=(format!("/?component={}", component.route_name))
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
    component_info: &crate::playground::components::ComponentInfo,
) -> Markup {
    html! {
        div class="playground-tabs" {
            button class="tab-button active" data-tab="component" { "Component" }
            button class="tab-button" data-tab="documentation" { "Documentation" }
        }
        div class="tab-content active" id="component-tab" {
            iframe id="component-iframe" src=(iframe_src) {}
        }
        div class="tab-content" id="documentation-tab" {
            (render_documentation_section(&component_info.name, &component_info.description))
        }
    }
}
