use thiserror::Error;

#[derive(Error, Debug)]
pub enum MpcError {
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Cryptographic error: {0}")]
    CryptographicError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Signing error: {0}")]
    SigningError(String),
    
    #[error("Invalid participant ID: {0}")]
    InvalidParticipantId(String),
    
    #[error("Insufficient participants: required {required}, available {available}")]
    InsufficientParticipants { required: usize, available: usize },
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
}

// Implement conversion from sled::Error
impl From<sled::Error> for MpcError {
    fn from(err: sled::Error) -> Self {
        MpcError::StorageError(err.to_string())
    }
}

// Implement conversion from reqwest::Error
impl From<reqwest::Error> for MpcError {
    fn from(err: reqwest::Error) -> Self {
        MpcError::NetworkError(err.to_string())
    }
}

// Custom result type for convenience

impl MpcError {
    /// Check if the error is recoverable (worth retrying)
    pub fn is_recoverable(&self) -> bool {
        match self {
            MpcError::NetworkError(_) => true,
            MpcError::TimeoutError(_) => true,
            MpcError::RateLimitExceeded(_) => true,
            MpcError::StorageError(_) => false, // Usually not recoverable
            MpcError::CryptographicError(_) => false,
            MpcError::KeyNotFound(_) => false,
            MpcError::InvalidParticipantId(_) => false,
            MpcError::ProtocolError(_) => false,
            MpcError::ConfigurationError(_) => false,
            MpcError::AuthenticationError(_) => false,
            MpcError::SerializationError(_) => false,
            MpcError::SigningError(_) => true, // Might be recoverable if temporary
            MpcError::InsufficientParticipants { .. } => true, // Might recover when more nodes join
        }
    }

    /// Get error category for monitoring/alerting
    pub fn category(&self) -> &'static str {
        match self {
            MpcError::StorageError(_) => "storage",
            MpcError::SerializationError(_) => "serialization",
            MpcError::CryptographicError(_) => "cryptographic",
            MpcError::NetworkError(_) => "network",
            MpcError::KeyNotFound(_) => "key_management",
            MpcError::SigningError(_) => "signing",
            MpcError::InvalidParticipantId(_) => "validation",
            MpcError::InsufficientParticipants { .. } => "coordination",
            MpcError::ProtocolError(_) => "protocol",
            MpcError::TimeoutError(_) => "timeout",
            MpcError::ConfigurationError(_) => "configuration",
            MpcError::AuthenticationError(_) => "authentication",
            MpcError::RateLimitExceeded(_) => "rate_limiting",
        }
    }
}