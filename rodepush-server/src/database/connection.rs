//! Database connection pool management

use crate::database::{config::DatabaseConfig, config::DatabaseType};
use rodepush_core::{Result, RodePushError};
use sqlx::{MySqlPool, PgPool, mysql::MySqlPoolOptions, postgres::PgPoolOptions};
use tracing::{debug, error, info};

/// Database connection pool wrapper
#[derive(Debug)]
pub enum DatabasePool {
    /// PostgreSQL connection pool
    Postgres(PgPool),
    /// MySQL connection pool
    MySql(MySqlPool),
}

impl DatabasePool {
    /// Execute a health check query on the connection pool
    pub async fn health_check(&self) -> Result<bool> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("SELECT 1").fetch_one(pool).await;
                match result {
                    Ok(_) => {
                        debug!("PostgreSQL health check passed");
                        Ok(true)
                    }
                    Err(e) => {
                        error!("PostgreSQL health check failed: {}", e);
                        Ok(false)
                    }
                }
            }
            DatabasePool::MySql(pool) => {
                let result = sqlx::query("SELECT 1").fetch_one(pool).await;
                match result {
                    Ok(_) => {
                        debug!("MySQL health check passed");
                        Ok(true)
                    }
                    Err(e) => {
                        error!("MySQL health check failed: {}", e);
                        Ok(false)
                    }
                }
            }
        }
    }

    /// Get the underlying PostgreSQL pool (for testing purposes only)
    /// Returns None if this is not a PostgreSQL pool
    pub fn as_postgres(&self) -> Option<&PgPool> {
        match self {
            DatabasePool::Postgres(pool) => Some(pool),
            DatabasePool::MySql(_) => None,
        }
    }

    /// Get the underlying MySQL pool (for testing purposes only)
    /// Returns None if this is not a MySQL pool
    pub fn as_mysql(&self) -> Option<&MySqlPool> {
        match self {
            DatabasePool::Postgres(_) => None,
            DatabasePool::MySql(pool) => Some(pool),
        }
    }
}

/// Database connection wrapper
pub struct DatabaseConnection {
    pool: DatabasePool,
    config: DatabaseConfig,
}

impl DatabaseConnection {
    /// Create a new database connection
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        match config.database_type {
            DatabaseType::Postgres => {
                let pool = PgPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(std::time::Duration::from_secs(config.timeout_seconds))
                    .connect(&config.url)
                    .await
                    .map_err(|e| RodePushError::Internal {
                        message: format!("Failed to connect to PostgreSQL: {}", e),
                    })?;

                info!("PostgreSQL connection established");
                Ok(Self {
                    pool: DatabasePool::Postgres(pool),
                    config: config.clone(),
                })
            }
            DatabaseType::MySql => {
                let pool = MySqlPoolOptions::new()
                    .max_connections(config.max_connections)
                    .acquire_timeout(std::time::Duration::from_secs(config.timeout_seconds))
                    .connect(&config.url)
                    .await
                    .map_err(|e| RodePushError::Internal {
                        message: format!("Failed to connect to MySQL: {}", e),
                    })?;

                info!("MySQL connection established");
                Ok(Self {
                    pool: DatabasePool::MySql(pool),
                    config: config.clone(),
                })
            }
        }
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    /// Get the underlying connection pool (for testing purposes)
    pub fn pool_for_testing(&self) -> &DatabasePool {
        &self.pool
    }

    /// Get database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!(
            "Running database migrations for {}",
            self.config.database_type
        );

        match &self.pool {
            DatabasePool::Postgres(pool) => {
                sqlx::migrate!("./migrations")
                    .run(pool)
                    .await
                    .map_err(|e| RodePushError::Internal {
                        message: format!("PostgreSQL migration failed: {}", e),
                    })?;
            }
            DatabasePool::MySql(pool) => {
                sqlx::migrate!("./migrations")
                    .run(pool)
                    .await
                    .map_err(|e| RodePushError::Internal {
                        message: format!("MySQL migration failed: {}", e),
                    })?;
            }
        }

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Check database health
    pub async fn health_check(&self) -> Result<bool> {
        self.pool.health_check().await
    }
}
