use components::header::{HeaderConfig, NavElement, NavItem, NavMenu, NavMenuItem};
use components::{footer, shell};
use maud::{html, Markup};

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Shell component renderer for Component Store
pub struct ShellComponentRenderer;

impl ComponentRenderer for ShellComponentRenderer {
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
}
