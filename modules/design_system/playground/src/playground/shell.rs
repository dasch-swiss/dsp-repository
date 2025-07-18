use axum::extract::Query;
use axum::http::StatusCode;
use maud::{html, Markup};

use crate::playground::components::{get_all_components, ComponentInfo};
use crate::playground::parameters::PlaygroundParams;
use crate::playground::templates::{
    render_component_controls, render_component_sidebar, render_component_tabs, render_page_shell,
};

struct ShellData<'a> {
    components: &'a [ComponentInfo],
    current_component: &'a str,
    current_component_info: &'a ComponentInfo,
    current_variant: &'a str,
    current_theme: &'a str,
    current_view: &'a str,
    current_params: &'a str,
}

fn render_shell_content(data: &ShellData) -> Markup {
    let iframe_src = format!("/iframe?{}", data.current_params);
    html! {
        div class="playground-layout" {
            (render_component_sidebar(data.components, data.current_component, data.current_params))
            main class="playground-main" {
                (render_component_controls(data.current_component_info, data.current_variant, data.current_theme, &iframe_src))
                (render_component_tabs(&iframe_src, data.current_component_info, data.current_view))
            }
        }
        script src="/playground-assets/js/playground.js" {}
        script src="/playground-assets/js/reload.js" {}
    }
}

pub async fn shell(Query(playground_params): Query<PlaygroundParams>) -> Result<Markup, (StatusCode, Markup)> {
    let all_components = get_all_components();

    // Get current component info, with fallback for missing documentation
    let current_component_info = playground_params
        .get_component_info()
        .or_else(|| all_components.iter().find(|c| c.route_name == "button").cloned())
        .or_else(|| all_components.first().cloned())
        .unwrap_or_else(|| {
            eprintln!(
                "Warning: No component documentation found for '{}'",
                playground_params.component
            );
            // Create a fallback component info
            ComponentInfo {
                name: playground_params.component.clone(),
                route_name: playground_params.component.clone(),
                variants: vec![],
            }
        });

    let current_variant = playground_params.variant.as_ref().cloned().unwrap_or_else(|| {
        current_component_info
            .get_default_variant()
            .map(|v| v.value.clone())
            .unwrap_or_else(|| "default".to_string())
    });

    let current_theme = playground_params.theme.to_string();
    let current_view = playground_params.view.to_string();
    let current_params = playground_params.to_query_string();

    let shell_data = ShellData {
        components: &all_components,
        current_component: &playground_params.component,
        current_component_info: &current_component_info,
        current_variant: &current_variant,
        current_theme: &current_theme,
        current_view: &current_view,
        current_params: &current_params,
    };

    Ok(render_page_shell(
        &current_theme,
        "DSP Design System Playground",
        "/playground-assets/css/playground.css",
        render_shell_content(&shell_data),
    ))
}
