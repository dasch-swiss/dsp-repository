use components::button::{self, ButtonVariant};
use components::tag::{self, TagVariant};
use components::{banner, link, tile};
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
        let button_variant = match variant {
            "secondary" => ButtonVariant::Secondary,
            "outline" => ButtonVariant::Outline,
            "primary" => ButtonVariant::Primary,
            other => {
                return Err(PlaygroundError::InvalidVariant {
                    component: "button".to_string(),
                    variant: other.to_string(),
                })
            }
        };

        let markup = button::button_with_variant("Sample Button", button_variant, false);
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "primary"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["primary", "secondary", "outline"]
    }
}

/// Banner component renderer
pub struct BannerRenderer;

impl ComponentRenderer for BannerRenderer {
    fn render_variant(&self, variant: &str, params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let markup = match variant {
            "with_prefix" => banner::with_prefix("Sample Prefix", "Sample Banner"),
            "with_suffix" => banner::with_suffix("Sample Banner", "Sample Suffix"),
            "full" => banner::full("Sample Prefix", "Sample Banner", "Sample Suffix"),
            "accent_only" => banner::accent_only("Sample Banner"),
            other => {
                return Err(PlaygroundError::InvalidVariant {
                    component: params.component.clone(),
                    variant: other.to_string(),
                })
            }
        };
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "accent_only"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["accent_only", "with_prefix", "with_suffix", "full"]
    }
}

/// Link component renderer
pub struct LinkRenderer;

impl ComponentRenderer for LinkRenderer {
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let markup = link::link("Sample Link", "#");
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
    fn render_variant(&self, _variant: &str, _params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        // Shell component appears to be more complex, using placeholder for now
        let markup = html! {
            div class="dsp-shell" {
                header class="dsp-shell__header" { "Sample Shell Header" }
                main class="dsp-shell__main" { "Sample Shell Content" }
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

/// Tag component renderer
pub struct TagRenderer;

impl ComponentRenderer for TagRenderer {
    fn render_variant(&self, variant: &str, params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let tag_variant = match variant {
            "blue" => TagVariant::Blue,
            "green" => TagVariant::Green,
            "gray" => TagVariant::Gray,
            other => {
                return Err(PlaygroundError::InvalidVariant {
                    component: params.component.clone(),
                    variant: other.to_string(),
                })
            }
        };

        let markup = tag::tag_with_variant("Sample Tag", tag_variant);
        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "gray"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["gray", "blue", "green"]
    }
}

/// Tile component renderer
pub struct TileRenderer;

impl ComponentRenderer for TileRenderer {
    fn render_variant(&self, variant: &str, params: &PlaygroundParams) -> PlaygroundResult<Markup> {
        let content = html! {
            h3 { "Sample Tile" }
            p {
                @if variant == "clickable" {
                    "Clickable content"
                } @else {
                    "Base content"
                }
            }
        };

        let markup = match variant {
            "clickable" => tile::clickable("#", content),
            "base" => tile::base(content),
            other => {
                return Err(PlaygroundError::InvalidVariant {
                    component: params.component.clone(),
                    variant: other.to_string(),
                })
            }
        };

        Ok(markup)
    }

    fn default_variant(&self) -> &'static str {
        "base"
    }

    fn supported_variants(&self) -> Vec<&'static str> {
        vec!["base", "clickable"]
    }
}

/// Registry for all component renderers
pub struct ComponentRendererRegistry;

impl ComponentRendererRegistry {
    pub fn get_renderer(component: &str) -> Option<Box<dyn ComponentRenderer>> {
        match component {
            "button" => Some(Box::new(ButtonRenderer)),
            "banner" => Some(Box::new(BannerRenderer)),
            "link" => Some(Box::new(LinkRenderer)),
            "shell" => Some(Box::new(ShellRenderer)),
            "tag" => Some(Box::new(TagRenderer)),
            "tile" => Some(Box::new(TileRenderer)),
            _ => None,
        }
    }
}
