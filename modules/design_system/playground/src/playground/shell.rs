use axum::extract::Query;
use axum::http::StatusCode;
use maud::{html, Markup};

use crate::playground::components::get_all_components;
use crate::playground::parameters::PlaygroundParams;
use crate::playground::templates::{
    render_component_controls, render_component_sidebar, render_component_tabs, render_page_shell,
};

fn render_shell_content(
    components: &[crate::playground::components::ComponentInfo],
    current_component: &str,
    current_component_info: &crate::playground::components::ComponentInfo,
    current_variant: &str,
    current_theme: &str,
    current_params: &str,
) -> Markup {
    html! {
        div class="playground-layout" {
            (render_component_sidebar(components, current_component))
            main class="playground-main" {
                (render_component_controls(current_component_info, current_variant, current_theme))
                (render_component_tabs(&format!("/iframe?{}", current_params), current_component_info))
            }
        }
        script src="/playground-assets/js/playground.js" {}
        script src="/playground-assets/js/reload.js" {}
    }
}

pub async fn shell(Query(playground_params): Query<PlaygroundParams>) -> Result<Markup, (StatusCode, Markup)> {
    let all_components = get_all_components();

    // Get current component info, default to button if not found
    let current_component_info = playground_params
        .get_component_info()
        .unwrap_or_else(|| get_all_components().into_iter().find(|c| c.route_name == "button").unwrap());

    let current_variant = playground_params.variant.as_ref().cloned().unwrap_or_else(|| {
        current_component_info
            .get_default_variant()
            .map(|v| v.value.clone())
            .unwrap_or_else(|| "default".to_string())
    });

    let current_theme = format!("{}", playground_params.theme);
    let current_params = playground_params.to_query_string();

    let content = render_shell_content(
        &all_components,
        &playground_params.component,
        &current_component_info,
        &current_variant,
        &current_theme,
        &current_params,
    );

    let html_markup = render_page_shell(
        &current_theme,
        "DSP Design System Playground",
        "/playground-assets/css/playground.css",
        content,
    );

    Ok(html_markup)
}
