// backend/src/error.rs
use thiserror::Error;
use actix_web::{error::ResponseError, HttpResponse, http::StatusCode};
use serde::{Serialize, Deserialize};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("MPC error: {0}")]
    Mpc(#[from] crate::services::mpc::MpcError),
    
    #[error("Jupiter error: {0}")]
    Jupiter(#[from] crate::services::jupiter::JupiterError),
    
    #[error("Solana error: {0}")]
    Solana(#[from] crate::blockchain::solana::SolanaError),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Internal server error: {0}")]
    Internal(String),
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let response = ErrorResponse {
            success: false,
            error: self.to_string(),
        };
        
        match self {
            AppError::Auth(_) => HttpResponse::Unauthorized().json(response),
            AppError::Validation(_) => HttpResponse::BadRequest().json(response),
            AppError::Database(_) | AppError::Internal(_) => {
                HttpResponse::InternalServerError().json(response)
            }
            AppError::Mpc(_) | AppError::Jupiter(_) | AppError::Solana(_) => {
                HttpResponse::BadGateway().json(response)
            }
        }
    }
    
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Auth(_) => StatusCode::UNAUTHORIZED,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Database(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Mpc(_) | AppError::Jupiter(_) | AppError::Solana(_) => StatusCode::BAD_GATEWAY,
        }
    }
}