use std::sync::Arc;
use axum::extract::State;
use axum::response::Html;
use crate::app_state::AppState;
use crate::error::ServerError;

/// GET / — returns the homepage
pub(crate) async fn home_page_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Html<String>, ServerError> {
    let view = api::routes::home::get_home_page();
    Ok(Html(view))
}