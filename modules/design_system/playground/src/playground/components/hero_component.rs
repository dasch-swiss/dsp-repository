use components::hero;
use maud::Markup;

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Hero component renderer for Component Store
pub struct HeroComponentRenderer;

impl ComponentRenderer for HeroComponentRenderer {
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
}
