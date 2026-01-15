use std::sync::Arc;

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Html;

use crate::app_state::AppState;
use crate::domain::project::Project;
use crate::domain::shortcode::Shortcode;
use crate::error::ServerError;
use crate::views::project::{ProjectIndexTemplate, ProjectShowTemplate};

/// GET / — returns the homepage
pub(crate) async fn projects_index_handler(State(state): State<Arc<AppState>>) -> Result<Html<String>, ServerError> {
    let projects: Vec<Project> = state
        .project_repository
        .list()
        .into_iter()
        .take(12)
        .filter_map(|p| {
            let sc = state.project_repository.find_shortcode(p);
            sc.and_then(|sc| state.project_repository.read(sc))
        })
        .collect::<Vec<Project>>();

    let view = (ProjectIndexTemplate { projects }).render().unwrap();
    Ok(Html(view))
}

fn return_ok_or_404<B>(body: Option<B>, otherwise: B) -> (StatusCode, B) {
    match body {
        None => (StatusCode::NOT_FOUND, otherwise),
        Some(body) => (StatusCode::OK, body),
    }
}

// GET /v2/projects/:id — returns a single project as HTML
pub(crate) async fn project_show_handler(
    Path(shortcode): Path<Shortcode>,
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Html<String>), ServerError> {
    let project: Option<Project> = state.project_repository.read(shortcode);

    let view: Option<String> = project.map(|project| ProjectShowTemplate { project }.render().unwrap());
    Ok(return_ok_or_404(view.map(Html), Html("missing".to_string())))
}

// GET /v2/projects/:id.json — returns a single project as JSON
pub(crate) async fn project_show_json_handler(
    Path(shortcode): Path<Shortcode>,
    State(state): State<Arc<AppState>>,
) -> Result<(StatusCode, Html<String>), ServerError> {
    let project_json: Option<String> = state
        .project_repository
        .read(shortcode)
        .map(|p| serde_json::to_string(&p).unwrap());

    Ok(return_ok_or_404(project_json.map(Html), Html("missing".to_string())))
}
