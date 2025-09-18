use maud::{html, Markup};

mod footer;
mod header;
mod hero;

use footer::askama_footer;
use header::askama_header;
use hero::askama_hero;

// Constants
const TAILWIND_BROWSER_CDN: &str = "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4";
const TAILWIND_ELEMENTS_CDN: &str = "https://cdn.jsdelivr.net/npm/@tailwindplus/elements@1";

// Configuration structs that will be used by Askama templates
#[derive(Debug, Clone)]
pub struct HeaderConfig {
    pub company_name: &'static str,
    pub logo_light_url: &'static str,
    pub logo_dark_url: &'static str,
    pub login_href: &'static str,
}

#[derive(Debug, Clone)]
pub struct HeroConfig {
    pub headline: &'static str,
    pub description: &'static str,
    pub announcement_text: &'static str,
    pub announcement_link_text: &'static str,
    pub announcement_href: &'static str,
    pub primary_button_text: &'static str,
    pub primary_button_href: &'static str,
    pub secondary_button_text: &'static str,
    pub secondary_button_href: &'static str,
    pub image_url: &'static str,
    pub image_alt: &'static str,
}

#[derive(Debug, Clone)]
pub struct FooterConfig {
    pub company_name: &'static str,
    pub description: &'static str,
    pub copyright_text: &'static str,
    pub logo_light_url: &'static str,
    pub logo_dark_url: &'static str,
}

/// A navigation element can be either a single item or a menu with sub-items
#[derive(Debug, Clone)]
pub enum NavElement {
    Item { label: &'static str, href: &'static str },
    Menu { label: &'static str, items: Vec<NavMenuItem> },
}

/// An item within a navigation menu
#[derive(Debug, Clone)]
pub struct NavMenuItem {
    pub label: &'static str,
    pub href: &'static str,
}

pub fn tailwind_askama_experiment() -> Markup {
    let nav_elements = vec![
        NavElement::Item { label: "Projects", href: "#" },
        NavElement::Item { label: "Services", href: "#" },
        NavElement::Menu {
            label: "How to",
            items: vec![
                NavMenuItem { label: "Docs", href: "#" },
                NavMenuItem { label: "Knowledge center", href: "#" },
                NavMenuItem { label: "Documents", href: "#" },
            ],
        },
        NavElement::Item { label: "About us", href: "#" },
        NavElement::Item { label: "Platform", href: "#" },
        NavElement::Item { label: "FAQ", href: "#" },
    ];

    let header_config = HeaderConfig {
        company_name: "Your Company",
        logo_light_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600",
        logo_dark_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500",
        login_href: "#",
    };

    let hero_config = HeroConfig {
        headline: "Data to enrich your business",
        description: "Anim aute id magna aliqua ad ad non deserunt sunt. Qui irure qui lorem cupidatat commodo. Elit sunt amet fugiat veniam occaecat fugiat aliqua.",
        announcement_text: "Anim aute id magna aliqua ad ad non deserunt sunt.",
        announcement_link_text: "Read more",
        announcement_href: "#",
        primary_button_text: "Get started",
        primary_button_href: "#",
        secondary_button_text: "Learn more",
        secondary_button_href: "#",
        image_url: "https://images.unsplash.com/photo-1483389127117-b6a2102724ae?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1587&q=80",
        image_alt: "",
    };

    let footer_config = FooterConfig {
        company_name: "Your Company",
        description: "Making the world a better place through constructing elegant hierarchies.",
        copyright_text: "(c) 2024 Your Company, Inc. All rights reserved.",
        logo_light_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600",
        logo_dark_url: "https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500",
    };

    html! {
        script src=(TAILWIND_BROWSER_CDN) {}
        script src=(TAILWIND_ELEMENTS_CDN) type="module" {}
        div class="bg-white dark:bg-gray-900" {
            (askama_header(nav_elements, &header_config))
            main {
                (askama_hero(&hero_config))
                // Future sections will go here
            }
            (askama_footer(&footer_config))
        }
    }
}
