use serde::{Deserialize, Serialize};
use actix_web::{HttpResponse, ResponseError};
use std::fmt;

/// Standardized API response wrapper
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

/// Standardized API error structure
#[derive(Serialize, Deserialize, Debug)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl<T> ApiResponse<T> {
    /// Create a successful response with data
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(code: &str, message: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }

    /// Convert to HttpResponse
    pub fn to_http_response(&self, status_code: actix_web::http::StatusCode) -> HttpResponse
    where
        T: Serialize,
    {
        HttpResponse::build(status_code).json(self)
    }
}

/// Common error codes
pub mod error_codes {
    pub const WALLET_ERROR: &str = "WALLET_ERROR";
    pub const AUTHENTICATION_ERROR: &str = "AUTHENTICATION_ERROR";
    pub const AUTHORIZATION_ERROR: &str = "AUTHORIZATION_ERROR";
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";
    pub const RATE_LIMIT_ERROR: &str = "RATE_LIMIT_ERROR";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
}

/// Helper trait for converting errors to ApiResponse
pub trait ToApiResponse<T> {
    fn to_api_response(self) -> ApiResponse<T>;
}

impl<T, E> ToApiResponse<T> for Result<T, E>
where
    E: fmt::Display,
{
    fn to_api_response(self) -> ApiResponse<T> {
        match self {
            Ok(data) => ApiResponse::success(data),
            Err(e) => ApiResponse::error(error_codes::INTERNAL_ERROR, &e.to_string()),
        }
    }
}
