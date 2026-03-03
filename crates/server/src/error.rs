use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use apihop_core::ApiError;
use apihop_core::storage::StorageError;

pub enum AppError {
    NotFound(String),
    Internal(String),
    BadGateway(String),
    BadRequest(String),
    Forbidden(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::BadGateway(msg) => (StatusCode::BAD_GATEWAY, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
        };
        (status, message).into_response()
    }
}

impl From<StorageError> for AppError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::NotFound(msg) => AppError::NotFound(msg),
            StorageError::Database(source) => AppError::Internal(source.to_string()),
        }
    }
}

impl From<ApiError> for AppError {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::Request(e) => AppError::BadGateway(e.to_string()),
            ApiError::Storage(e) => AppError::from(e),
            ApiError::Auth(msg) => AppError::BadRequest(msg),
        }
    }
}
