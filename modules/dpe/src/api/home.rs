use std::sync::Arc;

use axum::extract::State;
use axum::response::Redirect;

use crate::app_state::AppState;

pub(crate) async fn home_redirect_to_project(State(_state): State<Arc<AppState>>) -> Redirect {
    Redirect::to("/projects")
}
