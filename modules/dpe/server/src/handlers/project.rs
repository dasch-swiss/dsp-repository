use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::Html;
use types::metadata::metadata_service::MetadataService;
use types::metadata::model::Shortcode;

use crate::app_state::AppState;
use crate::error::ServerError;

/// GET /projects — returns a list of all projects as HTML
pub(crate) async fn projects_list_handler(State(state): State<Arc<AppState>>) -> Result<Html<String>, ServerError> {
    let projects = state.metadata_service.find_all().await?;
    let view = api::routes::project::get_projects_list_page(projects);
    Ok(Html(view))
}

/// GET /projects/:id — returns a single project as HTML
pub(crate) async fn project_details_handler(
    Path(shortcode): Path<Shortcode>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, ServerError> {
    let project_metadata = state.metadata_service.find_by_id(&shortcode).await?;
    let view = match project_metadata {
        None => api::routes::project::get_not_found_page(),
        Some(pm) => api::routes::project::get_project_details_page(pm.research_project),
    };
    Ok(Html(view))
}
