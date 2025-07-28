//! Bundle management and data models

use chrono::{DateTime, Utc};
use rodepush_core::{BundleId, Platform, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use crate::database::{application::ApplicationId, connection::DatabasePool, error::DatabaseError};
use sqlx::Row;

/// Bundle identifier (reuses core BundleId)
pub type DatabaseBundleId = BundleId;

/// Bundle model for database operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    /// Bundle ID
    pub id: DatabaseBundleId,
    /// Associated application ID
    pub application_id: ApplicationId,
    /// Bundle version
    pub version: String,
    /// Target platform
    pub platform: Platform,
    /// Bundle metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Storage key for the bundle data
    pub storage_key: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Bundle checksum
    pub checksum: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Bundle {
    /// Create a new bundle
    pub fn new(
        application_id: ApplicationId,
        version: String,
        platform: Platform,
        storage_key: String,
        size_bytes: u64,
        checksum: String,
    ) -> Self {
        Self {
            id: BundleId::new(),
            application_id,
            version,
            platform,
            metadata: HashMap::new(),
            storage_key,
            size_bytes,
            checksum,
            created_at: Utc::now(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set bundle ID (for testing purposes)
    pub fn with_id(mut self, id: BundleId) -> Self {
        self.id = id;
        self
    }
}

/// Bundle service for database operations
pub struct BundleService;

impl BundleService {
    /// Create a new bundle in the database
    pub async fn create(pool: &DatabasePool, bundle: &Bundle) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::create_postgres(pg_pool, bundle).await,
            DatabasePool::MySql(mysql_pool) => Self::create_mysql(mysql_pool, bundle).await,
        }
    }

    /// Get bundle by ID from the database
    pub async fn get_by_id(pool: &DatabasePool, id: &DatabaseBundleId) -> Result<Option<Bundle>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::get_by_id_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::get_by_id_mysql(mysql_pool, id).await,
        }
    }

    /// Get bundles by application ID
    pub async fn get_by_application(
        pool: &DatabasePool,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Bundle>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_by_application_postgres(pg_pool, application_id, limit, offset).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_by_application_mysql(mysql_pool, application_id, limit, offset).await
            }
        }
    }

    /// Update bundle in the database
    pub async fn update(pool: &DatabasePool, bundle: &Bundle) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::update_postgres(pg_pool, bundle).await,
            DatabasePool::MySql(mysql_pool) => Self::update_mysql(mysql_pool, bundle).await,
        }
    }

    /// Delete bundle from the database
    pub async fn delete(pool: &DatabasePool, id: &DatabaseBundleId) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::delete_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::delete_mysql(mysql_pool, id).await,
        }
    }

    /// Get bundles by platform
    pub async fn get_by_platform(
        pool: &DatabasePool,
        platform: &Platform,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Bundle>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_by_platform_postgres(pg_pool, platform, limit, offset).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_by_platform_mysql(mysql_pool, platform, limit, offset).await
            }
        }
    }

    // PostgreSQL implementations
    async fn create_postgres(pool: &sqlx::PgPool, bundle: &Bundle) -> Result<()> {
        let query = r#"
            INSERT INTO bundles (id, application_id, version, platform, metadata, storage_key, size_bytes, checksum, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#;

        sqlx::query(query)
            .bind(bundle.id.as_uuid())
            .bind(bundle.application_id.as_uuid())
            .bind(&bundle.version)
            .bind(bundle.platform.to_string())
            .bind(serde_json::to_value(&bundle.metadata)?)
            .bind(&bundle.storage_key)
            .bind(bundle.size_bytes as i64)
            .bind(&bundle.checksum)
            .bind(bundle.created_at)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created bundle: {}", bundle.id);
        Ok(())
    }

    async fn get_by_id_postgres(
        pool: &sqlx::PgPool,
        id: &DatabaseBundleId,
    ) -> Result<Option<Bundle>> {
        let query = "SELECT * FROM bundles WHERE id = $1";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let bundle = Bundle {
                    id: DatabaseBundleId::from_uuid(row.get("id")),
                    application_id: ApplicationId::from_uuid(row.get("application_id")),
                    version: row.get("version"),
                    platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                    metadata: serde_json::from_value(row.get("metadata"))?,
                    storage_key: row.get("storage_key"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    checksum: row.get("checksum"),
                    created_at: row.get("created_at"),
                };
                Ok(Some(bundle))
            }
            None => Ok(None),
        }
    }

    async fn get_by_application_postgres(
        pool: &sqlx::PgPool,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Bundle>> {
        let query = r#"
            SELECT * FROM bundles 
            WHERE application_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let rows = sqlx::query(query)
            .bind(application_id.as_uuid())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let mut bundles = Vec::new();
        for row in rows {
            let bundle = Bundle {
                id: DatabaseBundleId::from_uuid(row.get("id")),
                application_id: ApplicationId::from_uuid(row.get("application_id")),
                version: row.get("version"),
                platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                metadata: serde_json::from_value(row.get("metadata"))?,
                storage_key: row.get("storage_key"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                checksum: row.get("checksum"),
                created_at: row.get("created_at"),
            };
            bundles.push(bundle);
        }

        Ok(bundles)
    }

    async fn update_postgres(pool: &sqlx::PgPool, bundle: &Bundle) -> Result<()> {
        let query = r#"
            UPDATE bundles 
            SET application_id = $2, version = $3, platform = $4, metadata = $5, 
                storage_key = $6, size_bytes = $7, checksum = $8
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(bundle.id.as_uuid())
            .bind(bundle.application_id.as_uuid())
            .bind(&bundle.version)
            .bind(bundle.platform.to_string())
            .bind(serde_json::to_value(&bundle.metadata)?)
            .bind(&bundle.storage_key)
            .bind(bundle.size_bytes as i64)
            .bind(&bundle.checksum)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated bundle: {}", bundle.id);
        Ok(())
    }

    async fn delete_postgres(pool: &sqlx::PgPool, id: &DatabaseBundleId) -> Result<()> {
        let query = "DELETE FROM bundles WHERE id = $1";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted bundle: {}", id);
        Ok(())
    }

    async fn get_by_platform_postgres(
        pool: &sqlx::PgPool,
        platform: &Platform,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Bundle>> {
        let query = r#"
            SELECT * FROM bundles 
            WHERE platform = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let rows = sqlx::query(query)
            .bind(platform.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let mut bundles = Vec::new();
        for row in rows {
            let bundle = Bundle {
                id: DatabaseBundleId::from_uuid(row.get("id")),
                application_id: ApplicationId::from_uuid(row.get("application_id")),
                version: row.get("version"),
                platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                metadata: serde_json::from_value(row.get("metadata"))?,
                storage_key: row.get("storage_key"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                checksum: row.get("checksum"),
                created_at: row.get("created_at"),
            };
            bundles.push(bundle);
        }

        Ok(bundles)
    }

    // MySQL implementations
    async fn create_mysql(pool: &sqlx::MySqlPool, bundle: &Bundle) -> Result<()> {
        let query = r#"
            INSERT INTO bundles (id, application_id, version, platform, metadata, storage_key, size_bytes, checksum, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(bundle.id.as_uuid())
            .bind(bundle.application_id.as_uuid())
            .bind(&bundle.version)
            .bind(bundle.platform.to_string())
            .bind(serde_json::to_value(&bundle.metadata)?)
            .bind(&bundle.storage_key)
            .bind(bundle.size_bytes as i64)
            .bind(&bundle.checksum)
            .bind(bundle.created_at)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created bundle: {}", bundle.id);
        Ok(())
    }

    async fn get_by_id_mysql(
        pool: &sqlx::MySqlPool,
        id: &DatabaseBundleId,
    ) -> Result<Option<Bundle>> {
        let query = "SELECT * FROM bundles WHERE id = ?";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let bundle = Bundle {
                    id: DatabaseBundleId::from_uuid(row.get("id")),
                    application_id: ApplicationId::from_uuid(row.get("application_id")),
                    version: row.get("version"),
                    platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                    metadata: serde_json::from_value(row.get("metadata"))?,
                    storage_key: row.get("storage_key"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    checksum: row.get("checksum"),
                    created_at: row.get("created_at"),
                };
                Ok(Some(bundle))
            }
            None => Ok(None),
        }
    }

    async fn get_by_application_mysql(
        pool: &sqlx::MySqlPool,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Bundle>> {
        let query = r#"
            SELECT * FROM bundles 
            WHERE application_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let rows = sqlx::query(query)
            .bind(application_id.as_uuid())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let mut bundles = Vec::new();
        for row in rows {
            let bundle = Bundle {
                id: DatabaseBundleId::from_uuid(row.get("id")),
                application_id: ApplicationId::from_uuid(row.get("application_id")),
                version: row.get("version"),
                platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                metadata: serde_json::from_value(row.get("metadata"))?,
                storage_key: row.get("storage_key"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                checksum: row.get("checksum"),
                created_at: row.get("created_at"),
            };
            bundles.push(bundle);
        }

        Ok(bundles)
    }

    async fn update_mysql(pool: &sqlx::MySqlPool, bundle: &Bundle) -> Result<()> {
        let query = r#"
            UPDATE bundles 
            SET application_id = ?, version = ?, platform = ?, metadata = ?, 
                storage_key = ?, size_bytes = ?, checksum = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(bundle.application_id.as_uuid())
            .bind(&bundle.version)
            .bind(bundle.platform.to_string())
            .bind(serde_json::to_value(&bundle.metadata)?)
            .bind(&bundle.storage_key)
            .bind(bundle.size_bytes as i64)
            .bind(&bundle.checksum)
            .bind(bundle.id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated bundle: {}", bundle.id);
        Ok(())
    }

    async fn delete_mysql(pool: &sqlx::MySqlPool, id: &DatabaseBundleId) -> Result<()> {
        let query = "DELETE FROM bundles WHERE id = ?";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted bundle: {}", id);
        Ok(())
    }

    async fn get_by_platform_mysql(
        pool: &sqlx::MySqlPool,
        platform: &Platform,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Bundle>> {
        let query = r#"
            SELECT * FROM bundles 
            WHERE platform = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let rows = sqlx::query(query)
            .bind(platform.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let mut bundles = Vec::new();
        for row in rows {
            let bundle = Bundle {
                id: DatabaseBundleId::from_uuid(row.get("id")),
                application_id: ApplicationId::from_uuid(row.get("application_id")),
                version: row.get("version"),
                platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                metadata: serde_json::from_value(row.get("metadata"))?,
                storage_key: row.get("storage_key"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                checksum: row.get("checksum"),
                created_at: row.get("created_at"),
            };
            bundles.push(bundle);
        }

        Ok(bundles)
    }
}
