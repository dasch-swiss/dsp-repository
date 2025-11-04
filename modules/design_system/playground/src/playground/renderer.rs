use components::button::{self, ButtonVariant};
use components::header::{HeaderConfig, NavElement, NavItem, NavMenu, NavMenuItem};
use components::{footer, header, hero, link, menu, menu_item, shell, LinkTarget};
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
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "button".to_string(),
                variant: variant.to_string(),
            });
        }

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Variants" }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Primary - Main call-to-action" }
                            (button::button_with_variant("Primary Button", ButtonVariant::Primary, false))
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary - Alternative action" }
                            (button::button_with_variant("Secondary Button", ButtonVariant::Secondary, false))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Disabled States" }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Primary Disabled" }
                            (button::button_with_variant("Disabled Primary", ButtonVariant::Primary, true))
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary Disabled" }
                            (button::button_with_variant("Disabled Secondary", ButtonVariant::Secondary, true))
                        }
                    }
                }
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}

/// Link component renderer
pub struct LinkRenderer;

impl ComponentRenderer for LinkRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "link".to_string(),
                variant: variant.to_string(),
            });
        }

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Links" }
                    div class="flex flex-col gap-3" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "link::link() - Default link (opens in the iFrame here)" }
                            (link::link("Go to homepage (Opens in the iframe as we are in the iframe)", "/"))
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "LinkTarget::Parent - Opens in parent frame" }
                            (link::link_with_target("Parent frame", "/?component=button&theme=light&view=component", LinkTarget::Parent))
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "LinkTarget::Top - Opens in same window/tab" }
                            (link::link_with_target("Button", "/?component=button&theme=light&view=component", LinkTarget::Top))
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "link::link_external() - Opens in new tab with security" }
                            (link::link_external("Visit GitHub", "https://github.com"))
                        }
                    }
                }
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
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

/// Menu Item component renderer
pub struct MenuItemRenderer;

impl ComponentRenderer for MenuItemRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "menu-item".to_string(),
                variant: variant.to_string(),
            });
        }

        // Create sample SVG icons for demonstration
        let star_icon = html! {
            svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class=(menu_item::icon_classes()) {
                path d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401Z" clip-rule="evenodd" fill-rule="evenodd";
            }
        };

        let code_icon = html! {
            svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class=(menu_item::icon_classes()) {
                path d="M6.28 5.22a.75.75 0 0 1 0 1.06L2.56 10l3.72 3.72a.75.75 0 0 1-1.06 1.06L.97 10.53a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 0Zm7.44 0a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L17.44 10l-3.72-3.72a.75.75 0 0 1 0-1.06ZM11.377 2.011a.75.75 0 0 1 .612.867l-2.5 14.5a.75.75 0 0 1-1.478-.255l2.5-14.5a.75.75 0 0 1 .866-.612Z" clip-rule="evenodd" fill-rule="evenodd";
            }
        };

        let flag_icon = html! {
            svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class=(menu_item::icon_classes()) {
                path d="M3.5 2.75a.75.75 0 0 0-1.5 0v14.5a.75.75 0 0 0 1.5 0v-4.392l1.657-.348a6.449 6.449 0 0 1 4.271.572 7.948 7.948 0 0 0 5.965.524l2.078-.64A.75.75 0 0 0 18 12.25v-8.5a.75.75 0 0 0-.904-.734l-2.38.501a7.25 7.25 0 0 1-4.186-.363l-.502-.2a8.75 8.75 0 0 0-5.053-.439l-1.475.31V2.75Z";
            }
        };

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Link Menu Items" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item("View Profile", "#"))
                            (menu_item::link_menu_item("Settings", "#"))
                            (menu_item::link_menu_item("Help Center", "#"))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Link Menu Items with Icons" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item_with_icon("Add to favorites", "#", star_icon.clone()))
                            (menu_item::link_menu_item_with_icon("Embed", "#", code_icon.clone()))
                            (menu_item::link_menu_item_with_icon("Report content", "#", flag_icon.clone()))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Menu Items" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::button_menu_item("Delete"))
                            (menu_item::button_menu_item("Archive"))
                            (menu_item::button_menu_item("Mark as read"))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Menu Items with Icons" }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                            (menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                            (menu_item::button_menu_item_with_icon("Copy link", flag_icon.clone()))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu Item Divider" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "Use dividers to separate groups of menu items"
                    }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item("Item 1", "#"))
                            (menu_item::link_menu_item("Item 2", "#"))
                            (menu_item::menu_item_divider())
                            (menu_item::link_menu_item("Item 3", "#"))
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Mixed Menu (Links and Buttons)" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "A typical menu combining navigation links and action buttons"
                    }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item_with_icon("View Details", "/details", star_icon.clone()))
                            (menu_item::link_menu_item_with_icon("Edit", "/edit", code_icon.clone()))
                            (menu_item::menu_item_divider())
                            (menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                            (menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                            (menu_item::menu_item_divider())
                            (menu_item::button_menu_item_with_icon("Delete", flag_icon))
                        }
                    }
                }
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "default"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}

/// Menu component renderer
pub struct MenuRenderer;

impl ComponentRenderer for MenuRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "menu".to_string(),
                variant: variant.to_string(),
            });
        }

        // Create sample SVG icons for demonstration
        let star_icon = html! {
            svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class=(menu_item::icon_classes()) {
                path d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401Z" clip-rule="evenodd" fill-rule="evenodd";
            }
        };

        let code_icon = html! {
            svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class=(menu_item::icon_classes()) {
                path d="M6.28 5.22a.75.75 0 0 1 0 1.06L2.56 10l3.72 3.72a.75.75 0 0 1-1.06 1.06L.97 10.53a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 0Zm7.44 0a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L17.44 10l-3.72-3.72a.75.75 0 0 1 0-1.06ZM11.377 2.011a.75.75 0 0 1 .612.867l-2.5 14.5a.75.75 0 0 1-1.478-.255l2.5-14.5a.75.75 0 0 1 .866-.612Z" clip-rule="evenodd" fill-rule="evenodd";
            }
        };

        let flag_icon = html! {
            svg viewBox="0 0 20 20" fill="currentColor" data-slot="icon" aria-hidden="true" class=(menu_item::icon_classes()) {
                path d="M3.5 2.75a.75.75 0 0 0-1.5 0v14.5a.75.75 0 0 0 1.5 0v-4.392l1.657-.348a6.449 6.449 0 0 1 4.271.572 7.948 7.948 0 0 0 5.965.524l2.078-.64A.75.75 0 0 0 18 12.25v-8.5a.75.75 0 0 0-.904-.734l-2.38.501a7.25 7.25 0 0 1-4.186-.363l-.502-.2a8.75 8.75 0 0 0-5.053-.439l-1.475.31V2.75Z";
            }
        };

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Comprehensive Menu Example" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "A complete menu showcasing links, buttons, icons, and dividers. The menu automatically positions itself optimally based on available screen space."
                    }
                    div class="relative inline-block" {
                        button popovertarget="demo-menu" class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500" {
                            "Open Menu"
                        }
                        (menu::menu()
                            .with_id("demo-menu")
                            .with_item(menu_item::link_menu_item("Profile", "/profile"))
                            .with_item(menu_item::link_menu_item("Settings", "/settings"))
                            .with_item(menu_item::menu_item_divider())
                            .with_item(menu_item::link_menu_item_with_icon("Add to favorites", "/favorites", star_icon.clone()))
                            .with_item(menu_item::link_menu_item_with_icon("View source", "/source", code_icon.clone()))
                            .with_item(menu_item::menu_item_divider())
                            .with_item(menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                            .with_item(menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                            .with_item(menu_item::menu_item_divider())
                            .with_item(menu_item::button_menu_item_with_icon("Delete", flag_icon))
                            .build())
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Conditional Menu Building" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "The builder pattern allows flexible, conditional menu construction"
                    }
                    div class="relative inline-block" {
                        button popovertarget="conditional-menu" class="rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-indigo-500" {
                            "Dynamic Menu"
                        }
                        @let is_admin = true;
                        @let builder = menu::menu().with_id("conditional-menu");
                        @let builder = builder.with_item(menu_item::link_menu_item("Dashboard", "/dashboard"));
                        @let builder = builder.with_item(menu_item::link_menu_item("Profile", "/profile"));
                        @let builder = if is_admin {
                            builder
                                .with_item(menu_item::menu_item_divider())
                                .with_item(menu_item::link_menu_item("Admin Panel", "/admin"))
                        } else {
                            builder
                        };
                        (builder.build())
                    }
                }
            }
        };

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
            "link" => Some(Box::new(LinkRenderer)),
            "menu" => Some(Box::new(MenuRenderer)),
            "menu-item" => Some(Box::new(MenuItemRenderer)),
            "shell" => Some(Box::new(ShellRenderer)),
            _ => None,
        }
    }
}
