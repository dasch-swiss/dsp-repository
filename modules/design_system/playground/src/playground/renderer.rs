use components::button::{self, ButtonVariant};
use components::header::{HeaderConfig, NavElement, NavItem, NavMenu, NavMenuItem};
use components::{footer, header, hero, icon, link, menu, menu_item, shell, IconType, LinkTarget};
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
                div class="mb-4 p-4 bg-blue-50 dark:bg-blue-950 rounded-md" {
                    p class="text-sm text-blue-900 dark:text-blue-100" {
                        "💡 All buttons include DataStar onclick handlers. Open browser console to see click events."
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Button Variants" }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Primary - Main call-to-action" }
                            (button::button("Primary Button")
                                .with_id("primary-button")
                                .onclick("console.log('Primary button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary - Alternative action" }
                            (button::button("Secondary Button")
                                .with_id("secondary-button")
                                .variant(ButtonVariant::Secondary)
                                .onclick("console.log('Secondary button clicked!')")
                                .build())
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Icon Buttons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Icon-only buttons for compact actions like closing dialogs or opening menus"
                    }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Default Icon Buttons" }
                            p class="text-xs text-gray-500 dark:text-gray-500 mb-2" { "Default icon buttons use subtle gray colors (text-gray-900 dark:text-gray-300)" }
                            div class="flex items-center gap-4" {
                                (button::icon_button(icon::icon(IconType::Hamburger))
                                    .with_id("hamburger-button")
                                    .onclick("console.log('Hamburger icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::Close))
                                    .with_id("close-button")
                                    .onclick("console.log('Close icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::ChevronDown))
                                    .with_id("chevron-button")
                                    .onclick("console.log('ChevronDown icon clicked!')")
                                    .build())
                            }
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Icon Buttons with Custom Colors" }
                            p class="text-xs text-gray-500 dark:text-gray-500 mb-2" { "Use .color() to override with custom Tailwind color classes" }
                            div class="flex items-center gap-4" {
                                (button::icon_button(icon::icon(IconType::Star))
                                    .with_id("star-button")
                                    .color("text-yellow-500 hover:bg-yellow-50 dark:hover:bg-yellow-950")
                                    .onclick("console.log('Star icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::Code))
                                    .with_id("code-button")
                                    .color("text-blue-500 hover:bg-blue-50 dark:hover:bg-blue-950")
                                    .onclick("console.log('Code icon clicked!')")
                                    .build())
                                (button::icon_button(icon::icon(IconType::Flag))
                                    .with_id("flag-button")
                                    .color("text-red-500 hover:bg-red-50 dark:hover:bg-red-950")
                                    .onclick("console.log('Flag icon clicked!')")
                                    .build())
                            }
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Disabled States" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "Disabled buttons also implement onclick but do not trigger onclick events"
                    }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Primary Disabled" }
                            (button::button("Disabled Primary")
                                .with_id("disabled-primary")
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary Disabled" }
                            (button::button("Disabled Secondary")
                                .with_id("disabled-secondary")
                                .variant(ButtonVariant::Secondary)
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Icon Button Disabled" }
                            (button::icon_button(icon::icon(IconType::Close))
                                .with_id("disabled-icon-button")
                                .disabled()
                                .onclick("console.log('This should not fire!')")
                                .build())
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Buttons with Icons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Add leading or trailing icons to standard buttons for enhanced visual communication"
                    }
                    div class="flex flex-col gap-4" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Button with Leading Icon" }
                            (button::button("Download")
                                .with_id("download-button")
                                .with_leading_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Download button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Button with Trailing Icon" }
                            (button::button("Next")
                                .with_id("next-button")
                                .with_trailing_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Next button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Button with Both Icons" }
                            (button::button("Code")
                                .with_id("code-action-button")
                                .with_leading_icon(icon::icon(IconType::Code))
                                .with_trailing_icon(icon::icon(IconType::ChevronDown))
                                .onclick("console.log('Code button clicked!')")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-2" { "Secondary with Icon" }
                            (button::button("Star")
                                .with_id("star-action-button")
                                .variant(ButtonVariant::Secondary)
                                .with_leading_icon(icon::icon(IconType::Star))
                                .onclick("console.log('Star button clicked!')")
                                .build())
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

/// Icon component renderer
pub struct IconRenderer;

impl ComponentRenderer for IconRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "icon".to_string(),
                variant: variant.to_string(),
            });
        }

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Available Icons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "All icons from Heroicons with consistent styling"
                    }
                    div class="grid grid-cols-2 md:grid-cols-3 gap-4" {
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Star))
                            span class="text-sm font-medium" { "Star" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Code))
                            span class="text-sm font-medium" { "Code" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Flag))
                            span class="text-sm font-medium" { "Flag" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Hamburger))
                            span class="text-sm font-medium" { "Hamburger" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::Close))
                            span class="text-sm font-medium" { "Close" }
                        }
                        div class="flex items-center gap-3 p-4 border border-gray-200 rounded-md dark:border-gray-700" {
                            (icon::icon(IconType::ChevronDown))
                            span class="text-sm font-medium" { "ChevronDown" }
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Custom Sizes" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Icons can be sized using Tailwind size classes"
                    }
                    div class="flex items-center gap-6" {
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-4"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-4" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-5"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-5 (default)" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-6"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-6" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-8"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-8" }
                        }
                        div class="flex flex-col items-center gap-2" {
                            (icon::icon_with_class(IconType::Star, "size-12"))
                            span class="text-xs text-gray-600 dark:text-gray-400" { "size-12" }
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Custom Colors" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Icons inherit currentColor and can be styled with text color classes"
                    }
                    div class="flex items-center gap-6" {
                        (icon::icon_with_class(IconType::Star, "size-8 text-yellow-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-blue-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-green-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-red-500"))
                        (icon::icon_with_class(IconType::Star, "size-8 text-purple-500"))
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu Item Icons" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-4" {
                        "Use icon_for_menu_item() for proper menu item styling"
                    }
                    div class="bg-white rounded-md shadow-lg w-56 dark:bg-gray-800 dark:-outline-offset-1 dark:outline-white/10" {
                        div class="py-1" {
                            (menu_item::link_menu_item_with_icon("Favorites", "/favorites", icon::icon_for_menu_item(IconType::Star)))
                            (menu_item::link_menu_item_with_icon("View Source", "/source", icon::icon_for_menu_item(IconType::Code)))
                            (menu_item::link_menu_item_with_icon("Report", "/report", icon::icon_for_menu_item(IconType::Flag)))
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

        // Create sample icons for demonstration
        let star_icon = icon::icon_for_menu_item(IconType::Star);
        let code_icon = icon::icon_for_menu_item(IconType::Code);
        let flag_icon = icon::icon_for_menu_item(IconType::Flag);

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

        // Create sample icons for demonstration
        let star_icon = icon::icon_for_menu_item(IconType::Star);
        let code_icon = icon::icon_for_menu_item(IconType::Code);
        let flag_icon = icon::icon_for_menu_item(IconType::Flag);

        let markup = html! {
            div class="flex flex-col gap-6 p-8" {
                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu with Text Trigger" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "A complete menu showcasing links, buttons, icons, and dividers. The menu automatically positions itself optimally based on available screen space."
                    }
                    (menu::menu()
                        .with_id("demo-menu")
                        .with_text_trigger("Open Menu")
                        .with_item(menu_item::link_menu_item("Profile", "/profile"))
                        .with_item(menu_item::link_menu_item("Settings", "/settings"))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::link_menu_item_with_icon("Add to favorites", "/favorites", star_icon.clone()))
                        .with_item(menu_item::link_menu_item_with_icon("View source", "/source", code_icon.clone()))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item_with_icon("Share", star_icon.clone()))
                        .with_item(menu_item::button_menu_item_with_icon("Download", code_icon.clone()))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item_with_icon("Delete", flag_icon.clone()))
                        .build())
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Menu with Icon Trigger" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "Using an icon button trigger for a compact UI. Icon buttons are keyboard accessible and semantically correct."
                    }
                    (menu::menu()
                        .with_id("icon-menu")
                        .with_icon_trigger(icon::icon(IconType::Hamburger))
                        .with_item(menu_item::link_menu_item("Dashboard", "/dashboard"))
                        .with_item(menu_item::link_menu_item("Profile", "/profile"))
                        .with_item(menu_item::link_menu_item("Settings", "/settings"))
                        .with_item(menu_item::menu_item_divider())
                        .with_item(menu_item::button_menu_item("Sign Out"))
                        .build())
                }

                section {
                    h3 class="text-lg font-semibold mb-3" { "Conditional Menu Building" }
                    p class="text-sm text-gray-600 dark:text-gray-400 mb-2" {
                        "The builder pattern allows flexible, conditional menu construction"
                    }
                    @let is_admin = true;
                    @let builder = menu::menu()
                        .with_id("conditional-menu")
                        .with_text_trigger("Dynamic Menu");
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
            "icon" => Some(Box::new(IconRenderer)),
            "link" => Some(Box::new(LinkRenderer)),
            "menu" => Some(Box::new(MenuRenderer)),
            "menu-item" => Some(Box::new(MenuItemRenderer)),
            "shell" => Some(Box::new(ShellRenderer)),
            _ => None,
        }
    }
}
