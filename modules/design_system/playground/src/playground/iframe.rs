use axum::extract::Query;
use axum::http::StatusCode;
use maud::Markup;

use crate::playground::component_registry;
use crate::playground::error::PlaygroundError;
use crate::playground::parameters::{PlaygroundParams, ViewMode};
use crate::playground::renderer::{
    render_sections_with_code_view, ComponentIsolationRegistry, ComponentRendererRegistry,
};
use crate::playground::templates::{render_error_content, render_iframe_content, render_page_shell};

pub async fn iframe_component(Query(params): Query<PlaygroundParams>) -> Result<Markup, (StatusCode, Markup)> {
    let playground_params = match params.validate_self() {
        Ok(params) => params,
        Err(e) => return Ok(render_error_page(&e)),
    };

    // Generate component markup using the renderer registry
    let component_markup = match generate_component_markup(&playground_params) {
        Ok(markup) => markup,
        Err(e) => return Ok(render_error_page(&e)),
    };

    let theme = format!("{}", playground_params.theme);
    let content = render_iframe_content(component_markup);
    // TODO: Build stripped tailwind CSS
    let html_markup = render_page_shell(&theme, "Component Preview", "/assets/css/styles.css", content);

    Ok(html_markup)
}

fn generate_component_markup(params: &PlaygroundParams) -> Result<Markup, PlaygroundError> {
    // Get default variant from component registry (first variant)
    let default_variant = component_registry::get_component_spec_by_route_name(&params.component)
        .and_then(|spec| spec.variants.first())
        .map(|v| v.value)
        .unwrap_or("default");

    match params.view {
        ViewMode::ComponentStore => {
            // Component Store: Use ComponentIsolationRegistry for isolated component variants
            let renderer = ComponentIsolationRegistry::get_renderer(&params.component)
                .ok_or_else(|| PlaygroundError::InvalidComponent(params.component.clone()))?;

            let variant = params.variant.as_deref().unwrap_or(default_variant);
            renderer.render_variant(variant, params)
        }
        ViewMode::Examples => {
            // Examples and Variants: Use ComponentRendererRegistry with code-view support
            let renderer = ComponentRendererRegistry::get_renderer(&params.component)
                .ok_or_else(|| PlaygroundError::InvalidComponent(params.component.clone()))?;

            let variant = params.variant.as_deref().unwrap_or(default_variant);

            // Try to render with code-view support first
            if let Some(sections) = renderer.render_variant_with_code(variant, params)? {
                Ok(render_sections_with_code_view(sections))
            } else {
                // Fall back to regular rendering
                renderer.render_variant(variant, params)
            }
        }
        ViewMode::Documentation => {
            // Documentation is handled differently (not rendered in iframe)
            // This should not happen as documentation is rendered directly in the main page
            Err(PlaygroundError::InvalidComponent(
                "Documentation view should not be rendered in iframe".to_string(),
            ))
        }
    }
}

fn render_error_page(error: &PlaygroundError) -> Markup {
    let content = render_error_content(&error.to_string());
    render_page_shell("light", "Error", "/assets/css/styles.css", content)
}
