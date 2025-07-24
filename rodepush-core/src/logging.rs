use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use std::io;
use serde::{Deserialize, Serialize};

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    /// Output format (text, json)
    pub format: LogFormat,
    /// Whether to include file and line numbers
    pub include_location: bool,
    /// Whether to include thread information
    pub include_thread_id: bool,
    /// Whether to include span information
    pub include_spans: bool,
    /// Custom fields to include in all log entries
    pub custom_fields: std::collections::HashMap<String, String>,
}

/// Log output formats
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    /// Human-readable text format
    Text,
    /// JSON format for structured logging
    Json,
    /// Compact text format
    Compact,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Text,
            include_location: true,
            include_thread_id: false,
            include_spans: true,
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

/// Initialize logging with the given configuration
pub fn init_logging(config: &LogConfig) -> crate::Result<()> {
    let _level_filter = parse_log_level(&config.level)?;
    
    // Create environment filter with the specified level
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level))
        .add_directive(format!("rodepush={}", config.level).parse().unwrap());

    let registry = tracing_subscriber::registry().with(env_filter);

    // Configure the formatter based on the format type
    match config.format {
        LogFormat::Text => {
            let layer = fmt::layer()
                .with_target(true)
                .with_thread_ids(config.include_thread_id)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_span_events(if config.include_spans {
                    FmtSpan::NEW | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_writer(io::stderr);
                
            registry.with(layer).try_init()
                .map_err(|e| crate::RodePushError::config(format!("Failed to initialize logging: {}", e)))?;
        }
        LogFormat::Json => {
            let layer = fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(config.include_thread_id)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_span_events(if config.include_spans {
                    FmtSpan::NEW | FmtSpan::CLOSE
                } else {
                    FmtSpan::NONE
                })
                .with_writer(io::stderr);
                
            registry.with(layer).try_init()
                .map_err(|e| crate::RodePushError::config(format!("Failed to initialize logging: {}", e)))?;
        }
        LogFormat::Compact => {
            let layer = fmt::layer()
                .compact()
                .with_target(config.include_location)
                .with_thread_ids(config.include_thread_id)
                .with_file(config.include_location)
                .with_line_number(config.include_location)
                .with_writer(io::stderr);
                
            registry.with(layer).try_init()
                .map_err(|e| crate::RodePushError::config(format!("Failed to initialize logging: {}", e)))?;
        }
    }

    tracing::info!(
        level = %config.level,
        format = ?config.format,
        "Logging initialized"
    );

    Ok(())
}

/// Parse log level string to tracing Level
fn parse_log_level(level: &str) -> crate::Result<Level> {
    match level.to_lowercase().as_str() {
        "trace" => Ok(Level::TRACE),
        "debug" => Ok(Level::DEBUG),
        "info" => Ok(Level::INFO),
        "warn" => Ok(Level::WARN),
        "error" => Ok(Level::ERROR),
        _ => Err(crate::RodePushError::validation(format!(
            "Invalid log level: {}. Valid levels are: trace, debug, info, warn, error",
            level
        )))
    }
}

/// Create a tracing span for tracking operations
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        tracing::info_span!($name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!($name, $($field)*)
    };
}

/// Log an operation with timing
#[macro_export]
macro_rules! log_operation {
    ($level:ident, $operation:expr, $($field:tt)*) => {
        let _span = $crate::trace_span!($operation, $($field)*);
        tracing::$level!($operation);
    };
}

/// Correlation ID generator for request tracing
pub struct CorrelationId(String);

impl CorrelationId {
    /// Generate a new correlation ID
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Create from existing string
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Get the correlation ID as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for CorrelationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Structured logging context for operations
pub struct LogContext {
    correlation_id: CorrelationId,
    operation: String,
    component: String,
}

impl LogContext {
    /// Create a new log context
    pub fn new(operation: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            correlation_id: CorrelationId::new(),
            operation: operation.into(),
            component: component.into(),
        }
    }

    /// Create with existing correlation ID
    pub fn with_correlation_id(
        correlation_id: CorrelationId,
        operation: impl Into<String>,
        component: impl Into<String>,
    ) -> Self {
        Self {
            correlation_id,
            operation: operation.into(),
            component: component.into(),
        }
    }

    /// Get correlation ID
    pub fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// Get operation name
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Get component name
    pub fn component(&self) -> &str {
        &self.component
    }

    /// Create a tracing span for this context
    pub fn span(&self) -> tracing::Span {
        tracing::info_span!(
            "operation",
            correlation_id = %self.correlation_id,
            operation = %self.operation,
            component = %self.component
        )
    }

    /// Log an info message with this context
    pub fn info(&self, message: &str) {
        let _guard = self.span().entered();
        tracing::info!("{}", message);
    }

    /// Log a warning message with this context
    pub fn warn(&self, message: &str) {
        let _guard = self.span().entered();
        tracing::warn!("{}", message);
    }

    /// Log an error message with this context
    pub fn error(&self, message: &str) {
        let _guard = self.span().entered();
        tracing::error!("{}", message);
    }

    /// Log a debug message with this context
    pub fn debug(&self, message: &str) {
        let _guard = self.span().entered();
        tracing::debug!("{}", message);
    }
}

/// Configure logging for CLI applications
pub fn init_cli_logging() -> crate::Result<()> {
    let config = LogConfig {
        level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        format: LogFormat::Text,
        include_location: false,
        include_thread_id: false,
        include_spans: false,
        custom_fields: std::collections::HashMap::new(),
    };
    init_logging(&config)
}

/// Configure logging for server applications
pub fn init_server_logging() -> crate::Result<()> {
    let config = LogConfig {
        level: std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        format: if std::env::var("LOG_FORMAT").unwrap_or_default() == "json" {
            LogFormat::Json
        } else {
            LogFormat::Text
        },
        include_location: true,
        include_thread_id: true,
        include_spans: true,
        custom_fields: std::collections::HashMap::new(),
    };
    init_logging(&config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_parsing() {
        assert!(parse_log_level("debug").is_ok());
        assert!(parse_log_level("info").is_ok());
        assert!(parse_log_level("DEBUG").is_ok()); // case insensitive
        assert!(parse_log_level("invalid").is_err());
    }

    #[test]
    fn test_correlation_id() {
        let id1 = CorrelationId::new();
        let id2 = CorrelationId::new();
        assert_ne!(id1.as_str(), id2.as_str()); // Should be unique

        let custom_id = CorrelationId::from_string("test-123".to_string());
        assert_eq!(custom_id.as_str(), "test-123");
    }

    #[test]
    fn test_log_context() {
        let context = LogContext::new("test_operation", "test_component");
        assert_eq!(context.operation(), "test_operation");
        assert_eq!(context.component(), "test_component");
    }

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(matches!(config.format, LogFormat::Text));
        assert!(config.include_location);
    }

    #[test]
    fn test_log_config_serialization() {
        let config = LogConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LogConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.level, deserialized.level);
    }
}