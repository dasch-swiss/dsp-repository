use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;
use crate::handlers;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projects", get(handlers::project::projects_list_handler))
        .route("/projects/{id}", get(handlers::project::project_details_handler))
        .route("/", get(handlers::home::home_page_handler))
        .route("/hello-world", get(handlers::hello_world::hello_world_handler))
        .route("/calculator", get(handlers::calculator::calculator_index_page_handler))
        .route(
            "/calculator/styles.css",
            get(handlers::calculator::calculator_style_css_handler),
        )
        .route("/calculator/calculate", get(handlers::calculator::calculate_action_handler))
}
