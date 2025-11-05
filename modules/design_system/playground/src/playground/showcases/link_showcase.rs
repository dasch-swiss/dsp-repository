use components::{link, ComponentBuilder, LinkTarget};
use maud::{html, Markup};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRenderer;

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
                    h3 class="text-lg font-semibold mb-3" { "Basic Links" }
                    div class="flex flex-col gap-3" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "Default link (opens in same window - here iFrame)" }
                            (link::link("Go to homepage", "/")
                                .with_id("homepage-link")
                                .with_test_id("homepage")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "External link (opens in new tab with security)" }
                            (link::link("Visit GitHub", "https://github.com")
                                .target(LinkTarget::Blank)
                                .with_id("github-link")
                                .with_test_id("github")
                                .build())
                        }
                    }
                }

                section {
                    h3 class="text-lg font-semibold mb-3 mt-6" { "Link Targets" }
                    div class="flex flex-col gap-3" {
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "Parent - Opens in parent frame" }
                            (link::link("Button component", "/?component=button")
                                .target(LinkTarget::Parent)
                                .with_id("parent-link")
                                .with_test_id("parent")
                                .build())
                        }
                        div {
                            p class="text-sm text-gray-600 dark:text-gray-400 mb-1" { "Top - Opens in top-most frame" }
                            (link::link("Top window", "/?component=button")
                                .target(LinkTarget::Top)
                                .with_id("top-link")
                                .with_test_id("top")
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
