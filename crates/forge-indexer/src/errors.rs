//! Comprehensive error handling for forge-indexer

use thiserror::Error;

/// Main error type for forge-indexer operations
#[derive(Error, Debug)]
pub enum ForgeIndexerError {
    #[error("Embedding operation failed: {message}")]
    EmbeddingError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Vector database operation failed: {operation}")]
    VectorDbError {
        operation: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("File processing error: {path}")]
    FileProcessingError {
        path: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Chunking operation failed: {reason}")]
    ChunkingError {
        reason: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Configuration error: {field}")]
    ConfigurationError {
        field: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Authentication failed: {reason}")]
    AuthenticationError { reason: String },

    #[error("Authorization failed: {resource}")]
    AuthorizationError { resource: String },

    #[error("Rate limit exceeded: {limit} requests per {window}")]
    RateLimitError { limit: u32, window: String },

    #[error("Validation failed: {field}")]
    ValidationError { field: String, message: String },

    #[error("External service error: {service}")]
    ExternalServiceError {
        service: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Internal server error: {message}")]
    InternalError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl ForgeIndexerError {
    /// Create a new embedding error
    pub fn embedding_error(message: impl Into<String>) -> Self {
        Self::EmbeddingError { message: message.into(), source: None }
    }

    /// Create a new embedding error with source
    pub fn embedding_error_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::EmbeddingError { message: message.into(), source: Some(Box::new(source)) }
    }

    /// Create a new vector database error
    pub fn vector_db_error(operation: impl Into<String>) -> Self {
        Self::VectorDbError { operation: operation.into(), source: None }
    }

    /// Create a new vector database error with source
    pub fn vector_db_error_with_source(
        operation: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::VectorDbError { operation: operation.into(), source: Some(Box::new(source)) }
    }

    /// Create a new file processing error
    pub fn file_processing_error(path: impl Into<String>) -> Self {
        Self::FileProcessingError { path: path.into(), source: None }
    }

    /// Create a new file processing error with source
    pub fn file_processing_error_with_source(
        path: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::FileProcessingError { path: path.into(), source: Some(Box::new(source)) }
    }

    /// Create a new chunking error
    pub fn chunking_error(reason: impl Into<String>) -> Self {
        Self::ChunkingError { reason: reason.into(), source: None }
    }

    /// Create a new chunking error with source
    pub fn chunking_error_with_source(
        reason: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ChunkingError { reason: reason.into(), source: Some(Box::new(source)) }
    }

    /// Create a new configuration error
    pub fn configuration_error(field: impl Into<String>) -> Self {
        Self::ConfigurationError { field: field.into(), source: None }
    }

    /// Create a new configuration error with source
    pub fn configuration_error_with_source(
        field: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ConfigurationError { field: field.into(), source: Some(Box::new(source)) }
    }

    /// Create a new authentication error
    pub fn authentication_error(reason: impl Into<String>) -> Self {
        Self::AuthenticationError { reason: reason.into() }
    }

    /// Create a new authorization error
    pub fn authorization_error(resource: impl Into<String>) -> Self {
        Self::AuthorizationError { resource: resource.into() }
    }

    /// Create a new rate limit error
    pub fn rate_limit_error(limit: u32, window: impl Into<String>) -> Self {
        Self::RateLimitError { limit, window: window.into() }
    }

    /// Create a new validation error
    pub fn validation_error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError { field: field.into(), message: message.into() }
    }

    /// Create a new external service error
    pub fn external_service_error(service: impl Into<String>) -> Self {
        Self::ExternalServiceError { service: service.into(), source: None }
    }

    /// Create a new external service error with source
    pub fn external_service_error_with_source(
        service: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ExternalServiceError { service: service.into(), source: Some(Box::new(source)) }
    }

    /// Create a new internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError { message: message.into(), source: None }
    }

    /// Create a new internal error with source
    pub fn internal_error_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::InternalError { message: message.into(), source: Some(Box::new(source)) }
    }

    /// Get the error code for HTTP responses
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::EmbeddingError { .. } => "EMBEDDING_ERROR",
            Self::VectorDbError { .. } => "VECTOR_DB_ERROR",
            Self::FileProcessingError { .. } => "FILE_PROCESSING_ERROR",
            Self::ChunkingError { .. } => "CHUNKING_ERROR",
            Self::ConfigurationError { .. } => "CONFIGURATION_ERROR",
            Self::AuthenticationError { .. } => "AUTHENTICATION_ERROR",
            Self::AuthorizationError { .. } => "AUTHORIZATION_ERROR",
            Self::RateLimitError { .. } => "RATE_LIMIT_ERROR",
            Self::ValidationError { .. } => "VALIDATION_ERROR",
            Self::ExternalServiceError { .. } => "EXTERNAL_SERVICE_ERROR",
            Self::InternalError { .. } => "INTERNAL_ERROR",
        }
    }

    /// Get the HTTP status code for this error
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::AuthenticationError { .. } => 401,
            Self::AuthorizationError { .. } => 403,
            Self::ValidationError { .. } => 400,
            Self::RateLimitError { .. } => 429,
            Self::ConfigurationError { .. } => 400,
            Self::EmbeddingError { .. } => 500,
            Self::VectorDbError { .. } => 500,
            Self::FileProcessingError { .. } => 500,
            Self::ChunkingError { .. } => 500,
            Self::ExternalServiceError { .. } => 502,
            Self::InternalError { .. } => 500,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimitError { .. } => true,
            Self::ExternalServiceError { .. } => true,
            Self::VectorDbError { .. } => true,
            Self::InternalError { .. } => false, // Usually not retryable
            Self::AuthenticationError { .. } => false,
            Self::AuthorizationError { .. } => false,
            Self::ValidationError { .. } => false,
            Self::ConfigurationError { .. } => false,
            Self::EmbeddingError { .. } => true, // Might be transient
            Self::FileProcessingError { .. } => false,
            Self::ChunkingError { .. } => false,
        }
    }
}

/// Result type alias for forge-indexer operations
pub type Result<T> = std::result::Result<T, ForgeIndexerError>;

/// Convert anyhow::Error to ForgeIndexerError
impl From<anyhow::Error> for ForgeIndexerError {
    fn from(err: anyhow::Error) -> Self {
        Self::InternalError {
            message: format!("Internal error: {err}"),
            source: None, // anyhow::Error doesn't implement Send + Sync for its source
        }
    }
}

/// Convert qdrant_client::QdrantError to ForgeIndexerError
impl From<qdrant_client::QdrantError> for ForgeIndexerError {
    fn from(err: qdrant_client::QdrantError) -> Self {
        Self::vector_db_error_with_source("Qdrant operation failed", err)
    }
}
