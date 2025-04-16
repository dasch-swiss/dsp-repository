use std::sync::Arc;
use async_stream::stream;
use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Response};
use datastar::prelude::MergeFragments;
use datastar::Sse;
use types::calculator::DcfForm;
use crate::app_state::AppState;
use crate::error::ServerError;

/// GET /calculator — returns the calculator page
pub(crate) async fn calculator_index_page_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Html<String>, ServerError> {
    let view = api::routes::calculator::get_index_page();
    Ok(Html(view))
}

/// GET /calculator/style.css - returns the calculator style.css
pub(crate) async fn calculator_style_css_handler(
    State(_state): State<Arc<AppState>>,
) -> Result<Response, ServerError> {
    Ok(
        (
            [("Content-Type", "text/css")], 
            api::routes::calculator::get_style_css(),
        ).into_response()
    )
}

pub(crate) async fn calculate_action_handler(
    State(state): State<Arc<AppState>>,
    Query(form): Query<DcfForm>
) -> impl IntoResponse {
    
    let dcf_result = state.calculator_service.compute_dcf_result(
        form.fcf,
        form.growth,
        form.discount,
        form.terminal,
        form.years,
    );

    let res = api::routes::calculator::get_result_table_page_fragment(&dcf_result);
    Sse(stream! {
        yield MergeFragments::new(format!("<div id='intrinsic_value' class='value-display'>Intrinsic Value: ${}</div>", dcf_result.total_intrinsic_value)).into();
        yield MergeFragments::new(res.clone()).into()
    })
}