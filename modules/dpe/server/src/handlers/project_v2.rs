use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Html;
use types::metadata::model::Shortcode;

use crate::app_state::AppState;
use crate::error::ServerError;

fn return_ok_or_404<B>(body: Option<B>, otherwise: B) -> (StatusCode, B) {
    match body {
        None => (StatusCode::NOT_FOUND, otherwise),
        Some(body) => (StatusCode::OK, body),
    }
}

// GET /v2/projects/:id.json — returns a single project as HTML
pub(crate) async fn project_json_handler(
    Path(shortcode): Path<Shortcode>,
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Html<String>), ServerError> {
    let project_json: Option<String> = state
        .project_repository
        .read(shortcode)
        .map(|p| serde_json::to_string(&p).unwrap());

    Ok(return_ok_or_404(project_json.map(Html), Html("missing".to_string())))
}
