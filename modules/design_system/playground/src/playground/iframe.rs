use axum::extract::Query;
use axum::http::StatusCode;
use maud::Markup;

use crate::playground::error::PlaygroundError;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::renderer::ComponentRendererRegistry;
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
    let html_markup = render_page_shell(&theme, "Component Preview", "/assets/css/_styles.css", content);

    Ok(html_markup)
}

fn generate_component_markup(params: &PlaygroundParams) -> Result<Markup, PlaygroundError> {
    let renderer = ComponentRendererRegistry::get_renderer(&params.component)
        .ok_or_else(|| PlaygroundError::InvalidComponent(params.component.clone()))?;

    let variant = params.variant.as_deref().unwrap_or_else(|| renderer.default_variant());
    renderer.render_variant(variant, params)
}

fn render_error_page(error: &PlaygroundError) -> Markup {
    let content = render_error_content(&error.to_string());
    render_page_shell("light", "Error", "/assets/css/styles.css", content)
}
