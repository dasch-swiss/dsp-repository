use components::{hero, ComponentBuilder};
use maud::Markup;

use crate::playground::error::PlaygroundResult;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Hero component renderer for Component Store
pub struct HeroComponentRenderer;

impl ComponentRenderer for HeroComponentRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let markup = hero::hero("DaSCH Service Platform")
            .with_description(
                "Long-term archive for humanities research data with discovery and presentation tools for researchers.",
            )
            .with_announcement("New features available for data management.", "Read more", "#")
            .with_primary_button("Get started", "#")
            .with_secondary_button("Learn more", "#")
            .with_image(
                "https://images.unsplash.com/photo-1483389127117-b6a2102724ae?ixlib=rb-4.0.3&ixid=MnwxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8&auto=format&fit=crop&w=1587&q=80",
                "Research data visualization",
            )
            .build();

        Ok(markup)
    }
}
