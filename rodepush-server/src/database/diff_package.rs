//! Differential package management and data models

use chrono::{DateTime, Utc};
use rodepush_core::{BundleId, Platform, Result, RodePushError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::database::{connection::DatabasePool, error::DatabaseError};
use sqlx::Row;

/// Differential package identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiffPackageId(Uuid);

impl DiffPackageId {
    /// Generate a new differential package ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create from string representation
    pub fn from_string(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s).map_err(|e| RodePushError::Validation {
            message: format!("Invalid UUID: {}", e),
        })?;
        Ok(Self(uuid))
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get string representation
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for DiffPackageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DiffPackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Differential package model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffPackage {
    /// Differential package ID
    pub id: DiffPackageId,
    /// Source bundle ID
    pub source_bundle_id: BundleId,
    /// Target bundle ID
    pub target_bundle_id: BundleId,
    /// Storage key for the differential package data
    pub storage_key: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Compression ratio
    pub compression_ratio: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Checksum of the differential package
    pub checksum: String,
    /// Platform compatibility
    pub platform: Platform,
    /// Differential package metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DiffPackage {
    /// Create a new differential package
    pub fn new(
        source_bundle_id: BundleId,
        target_bundle_id: BundleId,
        storage_key: String,
        size_bytes: u64,
        compression_ratio: f64,
        checksum: String,
        platform: Platform,
    ) -> Self {
        Self {
            id: DiffPackageId::new(),
            source_bundle_id,
            target_bundle_id,
            storage_key,
            size_bytes,
            compression_ratio,
            created_at: Utc::now(),
            checksum,
            platform,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Check if this diff package can be applied to transform from source to target
    pub fn is_applicable(&self, from_bundle: &BundleId, to_bundle: &BundleId) -> bool {
        self.source_bundle_id == *from_bundle && self.target_bundle_id == *to_bundle
    }

    /// Calculate the space savings compared to full bundle download
    pub fn space_savings_bytes(&self, full_bundle_size: u64) -> u64 {
        if full_bundle_size > self.size_bytes {
            full_bundle_size - self.size_bytes
        } else {
            0
        }
    }

    /// Get the efficiency ratio (smaller is better, 0.1 = 90% space savings)
    pub fn efficiency_ratio(&self, full_bundle_size: u64) -> f64 {
        if full_bundle_size > 0 {
            self.size_bytes as f64 / full_bundle_size as f64
        } else {
            1.0
        }
    }
}

/// Differential package service for database operations
pub struct DiffPackageService;

impl DiffPackageService {
    /// Create a new differential package in the database
    pub async fn create(pool: &DatabasePool, diff_package: &DiffPackage) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::create_postgres(pg_pool, diff_package).await,
            DatabasePool::MySql(mysql_pool) => Self::create_mysql(mysql_pool, diff_package).await,
        }
    }

    /// Get differential package by ID from the database
    pub async fn get_by_id(pool: &DatabasePool, id: &DiffPackageId) -> Result<Option<DiffPackage>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::get_by_id_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::get_by_id_mysql(mysql_pool, id).await,
        }
    }

    /// Get differential package by source and target bundle IDs
    pub async fn get_by_bundles(
        pool: &DatabasePool,
        source_bundle_id: &BundleId,
        target_bundle_id: &BundleId,
    ) -> Result<Option<DiffPackage>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_by_bundles_postgres(pg_pool, source_bundle_id, target_bundle_id).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_by_bundles_mysql(mysql_pool, source_bundle_id, target_bundle_id).await
            }
        }
    }

    /// Update differential package in the database
    pub async fn update(pool: &DatabasePool, diff_package: &DiffPackage) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::update_postgres(pg_pool, diff_package).await,
            DatabasePool::MySql(mysql_pool) => Self::update_mysql(mysql_pool, diff_package).await,
        }
    }

    /// Delete differential package from the database
    pub async fn delete(pool: &DatabasePool, id: &DiffPackageId) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::delete_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::delete_mysql(mysql_pool, id).await,
        }
    }

    /// List differential packages for a specific source bundle
    pub async fn list_for_source_bundle(
        pool: &DatabasePool,
        source_bundle_id: &BundleId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::list_for_source_bundle_postgres(pg_pool, source_bundle_id, limit, offset)
                    .await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::list_for_source_bundle_mysql(mysql_pool, source_bundle_id, limit, offset)
                    .await
            }
        }
    }

    /// List differential packages for a specific target bundle
    pub async fn list_for_target_bundle(
        pool: &DatabasePool,
        target_bundle_id: &BundleId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::list_for_target_bundle_postgres(pg_pool, target_bundle_id, limit, offset)
                    .await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::list_for_target_bundle_mysql(mysql_pool, target_bundle_id, limit, offset)
                    .await
            }
        }
    }

    /// Get differential packages by platform with pagination
    pub async fn get_by_platform(
        pool: &DatabasePool,
        platform: &Platform,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_by_platform_postgres(pg_pool, platform, limit, offset).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_by_platform_mysql(mysql_pool, platform, limit, offset).await
            }
        }
    }

    /// Clean up old differential packages (older than specified days)
    pub async fn cleanup_old_packages(pool: &DatabasePool, older_than_days: i32) -> Result<u64> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::cleanup_old_packages_postgres(pg_pool, older_than_days).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::cleanup_old_packages_mysql(mysql_pool, older_than_days).await
            }
        }
    }

    // PostgreSQL implementations
    async fn create_postgres(pool: &sqlx::PgPool, diff_package: &DiffPackage) -> Result<()> {
        let query = r#"
            INSERT INTO diff_packages (id, source_bundle_id, target_bundle_id, storage_key, size_bytes, compression_ratio, created_at, checksum, platform, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#;

        sqlx::query(query)
            .bind(diff_package.id.as_uuid())
            .bind(diff_package.source_bundle_id.as_uuid())
            .bind(diff_package.target_bundle_id.as_uuid())
            .bind(&diff_package.storage_key)
            .bind(diff_package.size_bytes as i64)
            .bind(diff_package.compression_ratio)
            .bind(diff_package.created_at)
            .bind(&diff_package.checksum)
            .bind(diff_package.platform.to_string())
            .bind(serde_json::to_value(&diff_package.metadata)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created differential package: {}", diff_package.id);
        Ok(())
    }

    async fn get_by_id_postgres(
        pool: &sqlx::PgPool,
        id: &DiffPackageId,
    ) -> Result<Option<DiffPackage>> {
        let query = "SELECT * FROM diff_packages WHERE id = $1";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let diff_package = DiffPackage {
                    id: DiffPackageId::from_uuid(row.get("id")),
                    source_bundle_id: BundleId::from_uuid(row.get("source_bundle_id")),
                    target_bundle_id: BundleId::from_uuid(row.get("target_bundle_id")),
                    storage_key: row.get("storage_key"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    compression_ratio: row.get("compression_ratio"),
                    created_at: row.get("created_at"),
                    checksum: row.get("checksum"),
                    platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(diff_package))
            }
            None => Ok(None),
        }
    }

    async fn get_by_bundles_postgres(
        pool: &sqlx::PgPool,
        source_bundle_id: &BundleId,
        target_bundle_id: &BundleId,
    ) -> Result<Option<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
            WHERE source_bundle_id = $1 AND target_bundle_id = $2
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let row = sqlx::query(query)
            .bind(source_bundle_id.as_uuid())
            .bind(target_bundle_id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let diff_package = DiffPackage {
                    id: DiffPackageId::from_uuid(row.get("id")),
                    source_bundle_id: BundleId::from_uuid(row.get("source_bundle_id")),
                    target_bundle_id: BundleId::from_uuid(row.get("target_bundle_id")),
                    storage_key: row.get("storage_key"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    compression_ratio: row.get("compression_ratio"),
                    created_at: row.get("created_at"),
                    checksum: row.get("checksum"),
                    platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(diff_package))
            }
            None => Ok(None),
        }
    }

    async fn update_postgres(pool: &sqlx::PgPool, diff_package: &DiffPackage) -> Result<()> {
        let query = r#"
            UPDATE diff_packages 
            SET storage_key = $2, size_bytes = $3, compression_ratio = $4, checksum = $5, metadata = $6
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(diff_package.id.as_uuid())
            .bind(&diff_package.storage_key)
            .bind(diff_package.size_bytes as i64)
            .bind(diff_package.compression_ratio)
            .bind(&diff_package.checksum)
            .bind(serde_json::to_value(&diff_package.metadata)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated differential package: {}", diff_package.id);
        Ok(())
    }

    async fn delete_postgres(pool: &sqlx::PgPool, id: &DiffPackageId) -> Result<()> {
        let query = "DELETE FROM diff_packages WHERE id = $1";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted differential package: {}", id);
        Ok(())
    }

    async fn list_for_source_bundle_postgres(
        pool: &sqlx::PgPool,
        source_bundle_id: &BundleId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
            WHERE source_bundle_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let rows = sqlx::query(query)
            .bind(source_bundle_id.as_uuid())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_diff_packages_postgres(rows)
    }

    async fn list_for_target_bundle_postgres(
        pool: &sqlx::PgPool,
        target_bundle_id: &BundleId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
            WHERE target_bundle_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let rows = sqlx::query(query)
            .bind(target_bundle_id.as_uuid())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_diff_packages_postgres(rows)
    }

    async fn get_by_platform_postgres(
        pool: &sqlx::PgPool,
        platform: &Platform,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
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

        Self::rows_to_diff_packages_postgres(rows)
    }

    async fn cleanup_old_packages_postgres(
        pool: &sqlx::PgPool,
        older_than_days: i32,
    ) -> Result<u64> {
        let query = r#"
            DELETE FROM diff_packages 
            WHERE created_at < NOW() - INTERVAL '$1 days'
        "#;

        let result = sqlx::query(query)
            .bind(older_than_days)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let deleted_count = result.rows_affected();
        tracing::info!("Cleaned up {} old differential packages", deleted_count);
        Ok(deleted_count)
    }

    fn rows_to_diff_packages_postgres(
        rows: Vec<sqlx::postgres::PgRow>,
    ) -> Result<Vec<DiffPackage>> {
        let mut diff_packages = Vec::new();
        for row in rows {
            let diff_package = DiffPackage {
                id: DiffPackageId::from_uuid(row.get("id")),
                source_bundle_id: BundleId::from_uuid(row.get("source_bundle_id")),
                target_bundle_id: BundleId::from_uuid(row.get("target_bundle_id")),
                storage_key: row.get("storage_key"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                compression_ratio: row.get("compression_ratio"),
                created_at: row.get("created_at"),
                checksum: row.get("checksum"),
                platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                metadata: serde_json::from_value(row.get("metadata"))?,
            };
            diff_packages.push(diff_package);
        }
        Ok(diff_packages)
    }

    // MySQL implementations
    async fn create_mysql(pool: &sqlx::MySqlPool, diff_package: &DiffPackage) -> Result<()> {
        let query = r#"
            INSERT INTO diff_packages (id, source_bundle_id, target_bundle_id, storage_key, size_bytes, compression_ratio, created_at, checksum, platform, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(diff_package.id.as_uuid())
            .bind(diff_package.source_bundle_id.as_uuid())
            .bind(diff_package.target_bundle_id.as_uuid())
            .bind(&diff_package.storage_key)
            .bind(diff_package.size_bytes as i64)
            .bind(diff_package.compression_ratio)
            .bind(diff_package.created_at)
            .bind(&diff_package.checksum)
            .bind(diff_package.platform.to_string())
            .bind(serde_json::to_value(&diff_package.metadata)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created differential package: {}", diff_package.id);
        Ok(())
    }

    async fn get_by_id_mysql(
        pool: &sqlx::MySqlPool,
        id: &DiffPackageId,
    ) -> Result<Option<DiffPackage>> {
        let query = "SELECT * FROM diff_packages WHERE id = ?";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let diff_package = DiffPackage {
                    id: DiffPackageId::from_uuid(row.get("id")),
                    source_bundle_id: BundleId::from_uuid(row.get("source_bundle_id")),
                    target_bundle_id: BundleId::from_uuid(row.get("target_bundle_id")),
                    storage_key: row.get("storage_key"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    compression_ratio: row.get("compression_ratio"),
                    created_at: row.get("created_at"),
                    checksum: row.get("checksum"),
                    platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(diff_package))
            }
            None => Ok(None),
        }
    }

    async fn get_by_bundles_mysql(
        pool: &sqlx::MySqlPool,
        source_bundle_id: &BundleId,
        target_bundle_id: &BundleId,
    ) -> Result<Option<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
            WHERE source_bundle_id = ? AND target_bundle_id = ?
            ORDER BY created_at DESC
            LIMIT 1
        "#;

        let row = sqlx::query(query)
            .bind(source_bundle_id.as_uuid())
            .bind(target_bundle_id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let diff_package = DiffPackage {
                    id: DiffPackageId::from_uuid(row.get("id")),
                    source_bundle_id: BundleId::from_uuid(row.get("source_bundle_id")),
                    target_bundle_id: BundleId::from_uuid(row.get("target_bundle_id")),
                    storage_key: row.get("storage_key"),
                    size_bytes: row.get::<i64, _>("size_bytes") as u64,
                    compression_ratio: row.get("compression_ratio"),
                    created_at: row.get("created_at"),
                    checksum: row.get("checksum"),
                    platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(diff_package))
            }
            None => Ok(None),
        }
    }

    async fn update_mysql(pool: &sqlx::MySqlPool, diff_package: &DiffPackage) -> Result<()> {
        let query = r#"
            UPDATE diff_packages 
            SET storage_key = ?, size_bytes = ?, compression_ratio = ?, checksum = ?, metadata = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(&diff_package.storage_key)
            .bind(diff_package.size_bytes as i64)
            .bind(diff_package.compression_ratio)
            .bind(&diff_package.checksum)
            .bind(serde_json::to_value(&diff_package.metadata)?)
            .bind(diff_package.id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated differential package: {}", diff_package.id);
        Ok(())
    }

    async fn delete_mysql(pool: &sqlx::MySqlPool, id: &DiffPackageId) -> Result<()> {
        let query = "DELETE FROM diff_packages WHERE id = ?";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted differential package: {}", id);
        Ok(())
    }

    async fn list_for_source_bundle_mysql(
        pool: &sqlx::MySqlPool,
        source_bundle_id: &BundleId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
            WHERE source_bundle_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let rows = sqlx::query(query)
            .bind(source_bundle_id.as_uuid())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_diff_packages_mysql(rows)
    }

    async fn list_for_target_bundle_mysql(
        pool: &sqlx::MySqlPool,
        target_bundle_id: &BundleId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
            WHERE target_bundle_id = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let rows = sqlx::query(query)
            .bind(target_bundle_id.as_uuid())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_diff_packages_mysql(rows)
    }

    async fn get_by_platform_mysql(
        pool: &sqlx::MySqlPool,
        platform: &Platform,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DiffPackage>> {
        let query = r#"
            SELECT * FROM diff_packages 
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

        Self::rows_to_diff_packages_mysql(rows)
    }

    async fn cleanup_old_packages_mysql(
        pool: &sqlx::MySqlPool,
        older_than_days: i32,
    ) -> Result<u64> {
        let query = r#"
            DELETE FROM diff_packages 
            WHERE created_at < DATE_SUB(NOW(), INTERVAL ? DAY)
        "#;

        let result = sqlx::query(query)
            .bind(older_than_days)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let deleted_count = result.rows_affected();
        tracing::info!("Cleaned up {} old differential packages", deleted_count);
        Ok(deleted_count)
    }

    fn rows_to_diff_packages_mysql(rows: Vec<sqlx::mysql::MySqlRow>) -> Result<Vec<DiffPackage>> {
        let mut diff_packages = Vec::new();
        for row in rows {
            let diff_package = DiffPackage {
                id: DiffPackageId::from_uuid(row.get("id")),
                source_bundle_id: BundleId::from_uuid(row.get("source_bundle_id")),
                target_bundle_id: BundleId::from_uuid(row.get("target_bundle_id")),
                storage_key: row.get("storage_key"),
                size_bytes: row.get::<i64, _>("size_bytes") as u64,
                compression_ratio: row.get("compression_ratio"),
                created_at: row.get("created_at"),
                checksum: row.get("checksum"),
                platform: Platform::from_str(&row.get::<String, _>("platform"))?,
                metadata: serde_json::from_value(row.get("metadata"))?,
            };
            diff_packages.push(diff_package);
        }
        Ok(diff_packages)
    }
}
