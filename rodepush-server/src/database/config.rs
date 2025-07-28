//! Database configuration types and utilities

use rodepush_core::{Result, RodePushError};
use serde::{Deserialize, Serialize};

/// Supported database types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
    /// PostgreSQL database (primary)
    Postgres,
    /// MySQL database (planned)
    MySql,
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseType::Postgres => write!(f, "postgres"),
            DatabaseType::MySql => write!(f, "mysql"),
        }
    }
}

impl std::str::FromStr for DatabaseType {
    type Err = RodePushError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "postgres" | "postgresql" => Ok(DatabaseType::Postgres),
            "mysql" | "mariadb" => Ok(DatabaseType::MySql),
            _ => Err(RodePushError::Validation {
                message: format!("Unsupported database type: {}", s),
            }),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type
    pub database_type: DatabaseType,
    /// Database connection URL
    pub url: String,
    /// Maximum number of connections
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub timeout_seconds: u64,
    /// Whether to enable SSL
    pub ssl: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_type: DatabaseType::Postgres,
            url: "postgresql://localhost/rodepush".to_string(),
            max_connections: 10,
            timeout_seconds: 30,
            ssl: false,
        }
    }
}
