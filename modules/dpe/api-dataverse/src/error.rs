//! Errors returned by the Dataverse-compatible endpoints.
//!
//! Unlike OAI-PMH — where protocol errors are carried in the body of a 200 —
//! this is a plain JSON API and the crawler distinguishes outcomes on the HTTP
//! status, so each variant maps to a status code.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum DataverseError {
    /// The required `persistentId` query parameter was absent or empty.
    #[error("persistentId query parameter is required")]
    MissingPersistentId,

    /// No dataset or record matches the given identifier.
    #[error("dataset not found: {0}")]
    DatasetNotFound(String),

    /// No file has the given numeric id.
    #[error("file not found: {0}")]
    FileNotFound(u64),

    /// The file exists but is behind an access restriction. Returned instead of
    /// bytes so a client that ignored the `restricted` flag gets a clear answer.
    #[error("file is restricted: {0}")]
    FileRestricted(u64),
}

impl DataverseError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::MissingPersistentId => StatusCode::BAD_REQUEST,
            Self::DatasetNotFound(_) | Self::FileNotFound(_) => StatusCode::NOT_FOUND,
            Self::FileRestricted(_) => StatusCode::FORBIDDEN,
        }
    }
}

impl IntoResponse for DataverseError {
    /// Mirrors Dataverse's own error envelope (`{"status":"ERROR","message":…}`),
    /// so a client written against the real API can read our failures too.
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "status": "ERROR",
            "message": self.to_string(),
        }));
        (self.status(), body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_match_the_contract() {
        assert_eq!(DataverseError::MissingPersistentId.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            DataverseError::DatasetNotFound("oai:x:1".to_string()).status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(DataverseError::FileNotFound(7).status(), StatusCode::NOT_FOUND);
        assert_eq!(DataverseError::FileRestricted(7).status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn messages_name_the_offending_value() {
        // The identifier/id is echoed so a harvester operator can tell which record
        // failed from the response alone.
        assert!(DataverseError::DatasetNotFound("oai:x:1".to_string())
            .to_string()
            .contains("oai:x:1"));
        assert!(DataverseError::FileNotFound(42).to_string().contains("42"));
    }
}
