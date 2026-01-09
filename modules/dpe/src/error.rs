use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Local wrapper so we can implement Axum traits
pub struct ServerError(pub String);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}
