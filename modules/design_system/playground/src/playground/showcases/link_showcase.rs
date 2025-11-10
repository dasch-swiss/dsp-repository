use components::{link, ComponentBuilder, LinkTarget};

use crate::playground::error::{PlaygroundError, PlaygroundResult};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::{example_with_description, ComponentRenderer, ComponentSection};

/// Link component renderer for Examples and Variants tab
pub struct LinkRenderer;

impl ComponentRenderer for LinkRenderer {
    fn render_variant_with_code(
        &self,
        variant: &str,
        _params: &PlaygroundParams,
    ) -> PlaygroundResult<Option<Vec<ComponentSection>>> {
        if variant != "default" {
            return Err(PlaygroundError::InvalidVariant {
                component: "link".to_string(),
                variant: variant.to_string(),
            });
        }

        Ok(Some(vec![
            ComponentSection {
                title: "Basic Links",
                description: Some("Standard links for navigation within the application"),
                examples: vec![
                    example_with_description(
                        "internal-link",
                        "Internal Link",
                        "Opens in same window",
                        r#"link::link("Go to homepage", "/")
    .with_id("homepage-link")
    .with_test_id("homepage")
    .build()"#,
                        link::link("Go to homepage", "/")
                            .with_id("homepage-link")
                            .with_test_id("homepage")
                            .build(),
                    ),
                    example_with_description(
                        "blank-link",
                        "External Link",
                        "Opens in new tab with security (rel=\"noopener noreferrer\")",
                        r#"link::link("Visit GitHub", "https://github.com")
    .target(LinkTarget::Blank)
    .with_id("github-link")
    .with_test_id("github")
    .build()"#,
                        link::link("Visit GitHub", "https://github.com")
                            .target(LinkTarget::Blank)
                            .with_id("github-link")
                            .with_test_id("github")
                            .build(),
                    ),
                ],
            },
            ComponentSection {
                title: "Link Targets",
                description: Some("Control where links open using different target options"),
                examples: vec![
                    example_with_description(
                        "parent-link",
                        "Parent Frame Link",
                        "Opens in parent frame",
                        r#"link::link("Button component", "/?component=button")
    .target(LinkTarget::Parent)
    .with_id("parent-link")
    .with_test_id("parent")
    .build()"#,
                        link::link("Button component", "/?component=button")
                            .target(LinkTarget::Parent)
                            .with_id("parent-link")
                            .with_test_id("parent")
                            .build(),
                    ),
                    example_with_description(
                        "top-link",
                        "Top Frame Link",
                        "Opens in top-most frame",
                        r#"link::link("Top window", "/?component=button")
    .target(LinkTarget::Top)
    .with_id("top-link")
    .with_test_id("top")
    .build()"#,
                        link::link("Top window", "/?component=button")
                            .target(LinkTarget::Top)
                            .with_id("top-link")
                            .with_test_id("top")
                            .build(),
                    ),
                ],
            },
        ]))
    }
}
