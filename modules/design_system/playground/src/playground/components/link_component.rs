use components::{link, ComponentBuilder, LinkTarget};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

/// Link component renderer for Component Store (isolated component variants)
pub struct LinkComponentRenderer;

impl ComponentRenderer for LinkComponentRenderer {
    fn render_variant(&self, variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        match variant {
            "internal" => Ok(html! {
                (link::link("Go to homepage", "/")
                    .with_id("homepage-link")
                    .with_test_id("homepage")
                    .build())
            }),
            "blank" => Ok(html! {
                (link::link("Visit GitHub", "https://github.com")
                    .target(LinkTarget::Blank)
                    .with_id("github-link")
                    .with_test_id("github")
                    .build())
            }),
            "parent" => Ok(html! {
                (link::link("Button component", "/?component=button")
                    .target(LinkTarget::Parent)
                    .with_id("parent-link")
                    .with_test_id("parent")
                    .build())
            }),
            "top" => Ok(html! {
                (link::link("Top window", "/?component=button")
                    .target(LinkTarget::Top)
                    .with_id("top-link")
                    .with_test_id("top")
                    .build())
            }),
            _ => Err(PlaygroundError::InvalidVariant { component: "link".to_string(), variant: variant.to_string() }),
        }
    }
}
