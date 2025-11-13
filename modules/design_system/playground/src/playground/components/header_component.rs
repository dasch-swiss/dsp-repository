use components::header;
use maud::Markup;

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Header component renderer for Component Store
pub struct HeaderComponentRenderer;

impl ComponentRenderer for HeaderComponentRenderer {
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
}
