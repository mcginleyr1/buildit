//! API error handling.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// API error type.
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}

impl From<buildit_core::Error> for ApiError {
    fn from(err: buildit_core::Error) -> Self {
        match err {
            buildit_core::Error::NotFound(msg) => ApiError::NotFound(msg),
            buildit_core::Error::InvalidInput(msg) => ApiError::BadRequest(msg),
            buildit_core::Error::Unauthorized(msg) => ApiError::Unauthorized(msg),
            buildit_core::Error::Forbidden(msg) => ApiError::Forbidden(msg),
            buildit_core::Error::Conflict(msg) => ApiError::Conflict(msg),
            _ => ApiError::Internal(err.to_string()),
        }
    }
}

impl From<buildit_db::DbError> for ApiError {
    fn from(err: buildit_db::DbError) -> Self {
        match err {
            buildit_db::DbError::NotFound(msg) => ApiError::NotFound(msg),
            buildit_db::DbError::Duplicate(msg) => ApiError::Conflict(msg),
            _ => ApiError::Internal(err.to_string()),
        }
    }
}
