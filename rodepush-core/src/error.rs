use thiserror::Error;

/// Main result type for RodePush operations
pub type Result<T> = std::result::Result<T, RodePushError>;

/// Main error type for RodePush operations
#[derive(Debug, Error)]
pub enum RodePushError {
    /// Bundle-related errors
    #[error("Bundle error: {0}")]
    Bundle(#[from] BundleError),

    /// Network-related errors
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// Storage-related errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Authentication-related errors
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// IO-related errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Internal errors (should not normally occur)
    #[error("Internal error: {message}")]
    Internal { message: String },
}

/// Bundle-specific errors
#[derive(Debug, Error)]
pub enum BundleError {
    /// Invalid bundle format
    #[error("Invalid bundle format: {reason}")]
    InvalidFormat { reason: String },

    /// Bundle checksum verification failed
    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// Compression/decompression failed
    #[error("Compression failed: {message}")]
    CompressionFailed { message: String },

    /// Decompression failed
    #[error("Decompression failed: {message}")]
    DecompressionFailed { message: String },

    /// Bundle size exceeds limits
    #[error("Bundle too large: {size} bytes (max: {max_size})")]
    TooLarge { size: u64, max_size: u64 },

    /// Bundle version is invalid
    #[error("Invalid version: {version}")]
    InvalidVersion { version: String },

    /// Platform not supported
    #[error("Unsupported platform: {platform}")]
    UnsupportedPlatform { platform: String },

    /// Chunk-related errors
    #[error("Chunk error: {message}")]
    ChunkError { message: String },

    /// Metadata parsing failed
    #[error("Failed to parse metadata: {reason}")]
    MetadataParseError { reason: String },

    /// Bundle validation failed with context
    #[error("Bundle validation failed: {reason} (bundle_id: {bundle_id})")]
    ValidationFailed { reason: String, bundle_id: String },

    /// Bundle size limit exceeded with context
    #[error("Bundle size limit exceeded: {actual_size} > {limit} bytes")]
    SizeLimitExceeded { actual_size: u64, limit: u64 },

    /// Bundle processing timeout
    #[error("Bundle processing timeout after {timeout_ms}ms")]
    ProcessingTimeout { timeout_ms: u64 },

    /// Bundle dependency resolution failed
    #[error("Dependency resolution failed: {dependency} - {reason}")]
    DependencyResolutionFailed { dependency: String, reason: String },

    /// Bundle signature verification failed
    #[error("Bundle signature verification failed: {reason}")]
    SignatureVerificationFailed { reason: String },
}

/// Network-related errors
#[derive(Debug, Error)]
pub enum NetworkError {
    /// HTTP request failed
    #[error("HTTP request failed: {status_code} - {message}")]
    HttpRequest { status_code: u16, message: String },

    /// Connection timeout
    #[error("Connection timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// DNS resolution failed
    #[error("DNS resolution failed for {host}")]
    DnsResolution { host: String },

    /// TLS/SSL error
    #[error("TLS error: {message}")]
    Tls { message: String },

    /// Connection refused
    #[error("Connection refused to {host}:{port}")]
    ConnectionRefused { host: String, port: u16 },

    /// Upload failed
    #[error("Upload failed: {reason}")]
    UploadFailed { reason: String },

    /// Download failed
    #[error("Download failed: {reason}")]
    DownloadFailed { reason: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: retry after {retry_after_seconds}s")]
    RateLimited { retry_after_seconds: u64 },
}

/// Storage-related errors
#[derive(Debug, Error)]
pub enum StorageError {
    /// File not found
    #[error("File not found: {path}")]
    NotFound { path: String },

    /// Permission denied
    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    /// Disk space exhausted
    #[error("No space left on device: {path}")]
    DiskSpaceExhausted { path: String },

    /// I/O error
    #[error("I/O error: {message}")]
    Io { message: String },

    /// Corrupted data
    #[error("Data corruption detected: {details}")]
    Corruption { details: String },

    /// Serialization error
    #[error("Serialization error: {message}")]
    Serialization { message: String },

    /// Invalid path
    #[error("Invalid path: {path}")]
    InvalidPath { path: String },

    /// Storage backend error
    #[error("Storage backend error: {backend} - {message}")]
    Backend { backend: String, message: String },

    /// Lock acquisition failed
    #[error("Failed to acquire lock: {resource}")]
    LockFailed { resource: String },

    /// Concurrent access error
    #[error("Concurrent access error: {message}")]
    ConcurrentAccess { message: String },
}

/// Authentication-related errors
#[derive(Debug, Error)]
pub enum AuthError {
    /// Invalid API key
    #[error("Invalid API key")]
    InvalidApiKey,

    /// Expired token
    #[error("Token expired at {expired_at}")]
    TokenExpired { expired_at: String },

    /// Missing authentication
    #[error("Authentication required")]
    MissingAuth,

    /// Insufficient permissions
    #[error("Insufficient permissions for operation: {operation}")]
    InsufficientPermissions { operation: String },

    /// Account suspended
    #[error("Account suspended: {reason}")]
    AccountSuspended { reason: String },

    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Token signature verification failed
    #[error("Token signature verification failed")]
    InvalidSignature,

    /// Application not found
    #[error("Application not found: {app_id}")]
    ApplicationNotFound { app_id: String },
}

/// Convenience methods for creating specific errors
impl RodePushError {
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

impl BundleError {
    /// Create an invalid format error
    pub fn invalid_format(reason: impl Into<String>) -> Self {
        Self::InvalidFormat {
            reason: reason.into(),
        }
    }

    /// Create a checksum mismatch error
    pub fn checksum_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::ChecksumMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a compression error
    pub fn compression_failed(message: impl Into<String>) -> Self {
        Self::CompressionFailed {
            message: message.into(),
        }
    }

    /// Create a chunk error
    pub fn chunk_error(message: impl Into<String>) -> Self {
        Self::ChunkError {
            message: message.into(),
        }
    }

    /// Create a validation failed error with bundle context
    pub fn validation_failed(reason: impl Into<String>, bundle_id: impl Into<String>) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
            bundle_id: bundle_id.into(),
        }
    }

    /// Create a size limit exceeded error
    pub fn size_limit_exceeded(actual_size: u64, limit: u64) -> Self {
        Self::SizeLimitExceeded { actual_size, limit }
    }

    /// Create a processing timeout error
    pub fn processing_timeout(timeout_ms: u64) -> Self {
        Self::ProcessingTimeout { timeout_ms }
    }

    /// Create a dependency resolution failed error
    pub fn dependency_resolution_failed(
        dependency: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::DependencyResolutionFailed {
            dependency: dependency.into(),
            reason: reason.into(),
        }
    }

    /// Create a signature verification failed error
    pub fn signature_verification_failed(reason: impl Into<String>) -> Self {
        Self::SignatureVerificationFailed {
            reason: reason.into(),
        }
    }

    /// Create a build failed error
    pub fn build_failed(message: impl Into<String>) -> Self {
        Self::InvalidFormat {
            reason: format!("Build failed: {}", message.into()),
        }
    }
}

impl NetworkError {
    /// Create an HTTP request error
    pub fn http_request(status_code: u16, message: impl Into<String>) -> Self {
        Self::HttpRequest {
            status_code,
            message: message.into(),
        }
    }

    /// Create a rate limited error
    pub fn rate_limited(retry_after_seconds: u64) -> Self {
        Self::RateLimited {
            retry_after_seconds,
        }
    }
}

impl StorageError {
    /// Create a not found error
    pub fn not_found(path: impl Into<String>) -> Self {
        Self::NotFound { path: path.into() }
    }

    /// Create a corruption error
    pub fn corruption(details: impl Into<String>) -> Self {
        Self::Corruption {
            details: details.into(),
        }
    }
}

impl AuthError {
    /// Create an insufficient permissions error
    pub fn insufficient_permissions(operation: impl Into<String>) -> Self {
        Self::InsufficientPermissions {
            operation: operation.into(),
        }
    }

    /// Create an application not found error
    pub fn application_not_found(app_id: impl Into<String>) -> Self {
        Self::ApplicationNotFound {
            app_id: app_id.into(),
        }
    }
}

/// Convert from standard I/O errors to StorageError
impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        use std::io::ErrorKind;

        match error.kind() {
            ErrorKind::NotFound => Self::NotFound {
                path: error.to_string(),
            },
            ErrorKind::PermissionDenied => Self::PermissionDenied {
                path: error.to_string(),
            },
            ErrorKind::InvalidInput | ErrorKind::InvalidData => Self::InvalidPath {
                path: error.to_string(),
            },
            _ => Self::Io {
                message: error.to_string(),
            },
        }
    }
}

/// Convert from serde_json errors to BundleError
impl From<serde_json::Error> for BundleError {
    fn from(error: serde_json::Error) -> Self {
        Self::MetadataParseError {
            reason: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for RodePushError {
    fn from(error: serde_json::Error) -> Self {
        RodePushError::Internal {
            message: format!("Serialization error: {}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let validation_error = RodePushError::validation("Invalid input");
        assert!(matches!(validation_error, RodePushError::Validation { .. }));

        let bundle_error = BundleError::invalid_format("Missing header");
        assert!(matches!(bundle_error, BundleError::InvalidFormat { .. }));

        let network_error = NetworkError::http_request(404, "Not found");
        assert!(matches!(network_error, NetworkError::HttpRequest { .. }));
    }

    #[test]
    fn test_error_conversion() {
        let bundle_err = BundleError::InvalidFormat {
            reason: "test".to_string(),
        };
        let main_err: RodePushError = bundle_err.into();
        assert!(matches!(main_err, RodePushError::Bundle(_)));
    }

    #[test]
    fn test_error_display() {
        let error = RodePushError::validation("Test validation error");
        let error_str = error.to_string();
        assert!(error_str.contains("Validation error"));
        assert!(error_str.contains("Test validation error"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file.txt");
        let storage_error: StorageError = io_error.into();
        assert!(matches!(storage_error, StorageError::NotFound { .. }));
    }
}
