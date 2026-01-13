use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::app_state::AppState;
use crate::domain::project::Project;
use crate::error::ServerError;
use crate::views::home::IndexTemplate;

/// GET / — returns the homepage
pub(crate) async fn home_page_handler(State(state): State<Arc<AppState>>) -> Result<Html<String>, ServerError> {
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

    let view = (IndexTemplate { projects }).render().unwrap();
    Ok(Html(view))
}
