use components::button::{self, ButtonVariant};
use components::header::{HeaderConfig, NavElement, NavItem, NavMenu, NavMenuItem};
use components::{footer, header, hero, shell};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;

/// Trait for rendering components with different variants
pub trait ComponentRenderer {
    /// Render a component with the specified variant and parameters
    fn render_variant(&self, variant: &str, params: &PlaygroundParams) -> PlaygroundResult<Markup>;

    /// Get the default variant for this component
    fn default_variant(&self) -> &'static str;

    /// Get all supported variants for this component
    #[allow(dead_code)]
    fn supported_variants(&self) -> Vec<&'static str>;
}

/// Button component renderer
pub struct ButtonRenderer;

impl ComponentRenderer for ButtonRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let button_variant = match variant {
            "secondary" => ButtonVariant::Secondary,
            "primary" => ButtonVariant::Primary,
            other => {
                return Err(PlaygroundError::InvalidVariant {
                    component: "button".to_string(),
                    variant: other.to_string(),
                })
            }
        };

        let markup = button::button_with_variant("Sample Button", button_variant, false);
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "primary"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["primary", "secondary"]
    }
}

/// Shell component renderer
pub struct ShellRenderer;

impl ComponentRenderer for ShellRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // Create sample navigation with both items and menus for playground demonstration
        let header_nav_elements = vec![
            NavElement::Item(NavItem { label: "Home", href: "/" }),
            NavElement::Item(NavItem { label: "Projects", href: "/projects" }),
            NavElement::Menu(NavMenu {
                label: "Resources",
                items: vec![
                    NavMenuItem { label: "Documentation", href: "/docs" },
                    NavMenuItem { label: "Tutorials", href: "/tutorials" },
                    NavMenuItem { label: "API Reference", href: "/api" },
                ],
            }),
            NavElement::Item(NavItem { label: "Contact", href: "/contact" }),
        ];

        // Create header configuration
        let header_config = HeaderConfig {
            company_name: "DaSCH Service Platform",
            logo_light_url: "/assets/logo.png",
            logo_dark_url: "/assets/logo.png",
            login_href: "/login",
        };

        // Create sample content for demonstration
        let sample_content = html! {
            section {
                h1 { "Welcome to the Application Shell" }
                p  {
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris."
                }
                p  {
                    "Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum."
                }
            }
        };

        // Create footer configuration
        let footer_config = footer::FooterConfig {
            company_name: "DaSCH",
            description: "Digital infrastructure for humanities research data preservation and discovery.",
            copyright_text: "© 2024 DaSCH, University of Basel. All rights reserved.",
            logo_light_url: "assets/logo.png",
            logo_dark_url: "assets/logo.png",
        };

        let markup = match variant {
            "header-only" => {
                // Shell with header navigation only
                shell::shell(header_nav_elements, header_config, footer_config)
                    .with_content(sample_content)
                    .build()
            }
            _ => {
                // Default to header-only variant
                shell::shell(header_nav_elements, header_config, footer_config)
                    .with_content(sample_content)
                    .build()
            }
        };
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "header-only"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["header-only"]
    }
}

/// Header component renderer
pub struct HeaderRenderer;

impl ComponentRenderer for HeaderRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let nav_elements = vec![
            header::NavElement::Item(header::NavItem { label: "Projects", href: "#" }),
            header::NavElement::Item(header::NavItem { label: "Services", href: "#" }),
            header::NavElement::Menu(header::NavMenu {
                label: "How to",
                items: vec![
                    header::NavMenuItem { label: "Docs", href: "#" },
                    header::NavMenuItem { label: "Knowledge center", href: "#" },
                    header::NavMenuItem { label: "Documents", href: "#" },
                ],
            }),
            header::NavElement::Item(header::NavItem { label: "About us", href: "#" }),
        ];

        let config = header::HeaderConfig {
            company_name: "DaSCH Service Platform",
            logo_light_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600",
            logo_dark_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500",
            login_href: "#",
        };

        let markup = header::header(nav_elements, &config);
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}

/// Hero component renderer
pub struct HeroRenderer;

impl ComponentRenderer for HeroRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let config = hero::HeroConfig {
            headline: "DaSCH Service Platform",
            description: "Long-term archive for humanities research data with discovery and presentation tools for researchers.",
            announcement_text: "New features available for data management.",
            announcement_link_text: "Read more",
            announcement_href: "#",
            primary_button_text: "Get started",
            primary_button_href: "#",
            secondary_button_text: "Learn more",
            secondary_button_href: "#",
            image_url: "https://images.unsplash.com/photo-1483389127117-b6a2102724ae?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1587&q=80",
            image_alt: "Research data visualization",
        };

        let markup = hero::hero(&config);
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}

/// Footer component renderer
pub struct FooterRenderer;

impl ComponentRenderer for FooterRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let config = footer::FooterConfig {
            company_name: "DaSCH",
            description: "Digital infrastructure for humanities research data preservation and discovery.",
            copyright_text: "© 2024 DaSCH, University of Basel. All rights reserved.",
            logo_light_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600",
            logo_dark_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500",
        };

        let markup = footer::footer(&config);
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}

/// Registry for all component renderers
pub struct ComponentRendererRegistry;

impl ComponentRendererRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            "button" => Some(Box::new(ButtonRenderer)),
            "footer" => Some(Box::new(FooterRenderer)),
            "header" => Some(Box::new(HeaderRenderer)),
            "hero" => Some(Box::new(HeroRenderer)),
            "shell" => Some(Box::new(ShellRenderer)),
            _ => None,
        }
    }
}
