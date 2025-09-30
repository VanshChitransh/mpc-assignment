use thiserror::Error;

#[derive(Error, Debug)]
pub enum MpcError {
    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Cryptographic error: {0}")]
    CryptographicError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Key generation error: {0}")]
    KeyGenerationError(String),

    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid participant ID: {0}")]
    InvalidParticipantId(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Insufficient participants: {0}")]
    InsufficientParticipants(String),

    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),
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
pub type MpcResult<T> = Result<T, MpcError>;

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
            MpcError::InsufficientParticipants(_) => true, // Might recover when more nodes join
            MpcError::AggregationFailed(_) => false, // Usually not recoverable
            MpcError::KeyGenerationError(_) => false,
            MpcError::InvalidThreshold(_) => false,
            MpcError::SessionNotFound(_) => false,
            MpcError::InvalidMessageFormat(_) => false,
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
            MpcError::InsufficientParticipants(_) => "coordination",
            MpcError::ProtocolError(_) => "protocol",
            MpcError::TimeoutError(_) => "timeout",
            MpcError::ConfigurationError(_) => "configuration",
            MpcError::AuthenticationError(_) => "authentication",
            MpcError::RateLimitExceeded(_) => "rate_limiting",
            MpcError::AggregationFailed(_) => "aggregation",
            MpcError::KeyGenerationError(_) => "key_generation",
            MpcError::InvalidThreshold(_) => "validation",
            MpcError::SessionNotFound(_) => "session_management",
            MpcError::InvalidMessageFormat(_) => "message_format",
        }
    }
}
