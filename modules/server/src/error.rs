use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use types::error::AppError;

/// Local wrapper so we can implement Axum traits
pub struct ServerError(pub AppError);

/// Allows to return AppError in ? which will be automatically transformed into ServerError
impl From<AppError> for ServerError {
    fn from(err: AppError) -> Self {
        ServerError(err)
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, body) = match self.0 {
            AppError::Message(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Msg(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.to_string()),
        };

        (status, body).into_response()
    }
}