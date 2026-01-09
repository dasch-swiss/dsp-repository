mod home;
mod project;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projects/{id}/json", get(project::project_json_handler))
        .route("/", get(home::home_page_handler))
}
