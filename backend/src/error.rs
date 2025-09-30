use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    DatabaseError(sqlx::Error),
    MPCError(String),
    AuthenticationError(String),
    ValidationError(String),
    NotFound(String),
    InternalError(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(e) => write!(f, "Database error: {}", e),
            AppError::MPCError(msg) => write!(f, "MPC error: {}", msg),
            AppError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        AppError::DatabaseError(error)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError::MPCError(format!("HTTP request failed: {}", error))
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::MPCError(_) => StatusCode::SERVICE_UNAVAILABLE,
            AppError::AuthenticationError(_) => StatusCode::UNAUTHORIZED,
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let error_type = match self {
            AppError::DatabaseError(_) => "database_error",
            AppError::MPCError(_) => "mpc_error",
            AppError::AuthenticationError(_) => "authentication_error",
            AppError::ValidationError(_) => "validation_error",
            AppError::NotFound(_) => "not_found",
            AppError::InternalError(_) => "internal_error",
        };

        HttpResponse::build(self.status_code()).json(ErrorResponse {
            error: error_type.to_string(),
            message: self.to_string(),
        })
    }
}

pub type Result<T> = std::result::Result<T, AppError>;