use components::{link, LinkTarget};
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
