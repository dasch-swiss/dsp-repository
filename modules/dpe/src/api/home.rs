use std::sync::Arc;

use crate::app_state::AppState;
use crate::error::ServerError;
use crate::views::home::IndexTemplate;
use askama::Template;
use axum::extract::State;
use axum::response::Html;

/// GET / — returns the homepage
pub(crate) async fn home_page_handler(State(_state): State<Arc<AppState>>) -> Result<Html<String>, ServerError> {
    let view = (IndexTemplate {}).render().unwrap();
    Ok(Html(view))
}
