mod home;
mod project;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tower_http::services::ServeDir;

use crate::api::home::home_redirect_to_project;
use crate::app_state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    let serve_dir = ServeDir::new("assets");

    Router::new()
        .route("/projects", get(project::projects_index_handler))
        .route("/projects/{id}", get(project::project_show_handler))
        .route("/projects/{id}/json", get(project::project_show_json_handler))
        .route("/", get(home_redirect_to_project))
        .nest_service("/assets", serve_dir.clone())
}
