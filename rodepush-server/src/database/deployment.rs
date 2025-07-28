//! Deployment management and data models

use chrono::{DateTime, Utc};
use rodepush_core::{BundleId, Result, RodePushError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

use crate::database::{application::ApplicationId, connection::DatabasePool, error::DatabaseError};
use sqlx::Row;

/// Deployment identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeploymentId(Uuid);

impl DeploymentId {
    /// Generate a new deployment ID
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

impl Default for DeploymentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DeploymentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Deployment status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment is pending
    Pending,
    /// Deployment is active
    Active,
    /// Deployment is paused
    Paused,
    /// Deployment is rolled back
    RolledBack,
    /// Deployment failed
    Failed,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentStatus::Pending => write!(f, "pending"),
            DeploymentStatus::Active => write!(f, "active"),
            DeploymentStatus::Paused => write!(f, "paused"),
            DeploymentStatus::RolledBack => write!(f, "rolled_back"),
            DeploymentStatus::Failed => write!(f, "failed"),
        }
    }
}

impl std::str::FromStr for DeploymentStatus {
    type Err = RodePushError;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(DeploymentStatus::Pending),
            "active" => Ok(DeploymentStatus::Active),
            "paused" => Ok(DeploymentStatus::Paused),
            "rolled_back" => Ok(DeploymentStatus::RolledBack),
            "failed" => Ok(DeploymentStatus::Failed),
            _ => Err(RodePushError::Validation {
                message: format!("Invalid deployment status: {}", s),
            }),
        }
    }
}

/// Deployment model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Deployment ID
    pub id: DeploymentId,
    /// Associated application ID
    pub application_id: ApplicationId,
    /// Associated bundle ID
    pub bundle_id: BundleId,
    /// Deployment environment (e.g., "production", "staging")
    pub environment: String,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Rollout percentage (0-100)
    pub rollout_percentage: u32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Deployment timestamp
    pub deployed_at: Option<DateTime<Utc>>,
    /// Rollback timestamp
    pub rolled_back_at: Option<DateTime<Utc>>,
    /// Deployment description
    pub description: Option<String>,
    /// Deployment metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Deployment {
    /// Create a new deployment
    pub fn new(application_id: ApplicationId, bundle_id: BundleId, environment: String) -> Self {
        let now = Utc::now();
        Self {
            id: DeploymentId::new(),
            application_id,
            bundle_id,
            environment,
            status: DeploymentStatus::Pending,
            rollout_percentage: 100,
            created_at: now,
            deployed_at: None,
            rolled_back_at: None,
            description: None,
            metadata: HashMap::new(),
        }
    }

    /// Set deployment description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set rollout percentage
    pub fn with_rollout_percentage(mut self, percentage: u32) -> Self {
        self.rollout_percentage = percentage.min(100);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Mark deployment as active
    pub fn activate(&mut self) {
        self.status = DeploymentStatus::Active;
        self.deployed_at = Some(Utc::now());
    }

    /// Mark deployment as rolled back
    pub fn rollback(&mut self) {
        self.status = DeploymentStatus::RolledBack;
        self.rolled_back_at = Some(Utc::now());
    }

    /// Mark deployment as failed
    pub fn fail(&mut self) {
        self.status = DeploymentStatus::Failed;
    }

    /// Pause deployment
    pub fn pause(&mut self) {
        self.status = DeploymentStatus::Paused;
    }

    /// Resume deployment (from paused state)
    pub fn resume(&mut self) {
        if self.status == DeploymentStatus::Paused {
            self.status = DeploymentStatus::Active;
        }
    }
}

/// Deployment service for database operations
pub struct DeploymentService;

impl DeploymentService {
    /// Create a new deployment in the database
    pub async fn create(pool: &DatabasePool, deployment: &Deployment) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::create_postgres(pg_pool, deployment).await,
            DatabasePool::MySql(mysql_pool) => Self::create_mysql(mysql_pool, deployment).await,
        }
    }

    /// Get deployment by ID from the database
    pub async fn get_by_id(pool: &DatabasePool, id: &DeploymentId) -> Result<Option<Deployment>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::get_by_id_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::get_by_id_mysql(mysql_pool, id).await,
        }
    }

    /// Update deployment in the database
    pub async fn update(pool: &DatabasePool, deployment: &Deployment) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::update_postgres(pg_pool, deployment).await,
            DatabasePool::MySql(mysql_pool) => Self::update_mysql(mysql_pool, deployment).await,
        }
    }

    /// Delete deployment from the database
    pub async fn delete(pool: &DatabasePool, id: &DeploymentId) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::delete_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::delete_mysql(mysql_pool, id).await,
        }
    }

    /// Get active deployments for an application in an environment
    pub async fn get_active_for_application(
        pool: &DatabasePool,
        application_id: &ApplicationId,
        environment: &str,
    ) -> Result<Vec<Deployment>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_active_for_application_postgres(pg_pool, application_id, environment)
                    .await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_active_for_application_mysql(mysql_pool, application_id, environment)
                    .await
            }
        }
    }

    /// List deployments for an application with pagination
    pub async fn list_for_application(
        pool: &DatabasePool,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::list_for_application_postgres(pg_pool, application_id, limit, offset).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::list_for_application_mysql(mysql_pool, application_id, limit, offset).await
            }
        }
    }

    /// Get deployments by status
    pub async fn get_by_status(
        pool: &DatabasePool,
        status: &DeploymentStatus,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_by_status_postgres(pg_pool, status, limit, offset).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_by_status_mysql(mysql_pool, status, limit, offset).await
            }
        }
    }

    // PostgreSQL implementations
    async fn create_postgres(pool: &sqlx::PgPool, deployment: &Deployment) -> Result<()> {
        let query = r#"
            INSERT INTO deployments (id, application_id, bundle_id, environment, status, rollout_percentage, created_at, deployed_at, rolled_back_at, description, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#;

        sqlx::query(query)
            .bind(deployment.id.as_uuid())
            .bind(deployment.application_id.as_uuid())
            .bind(deployment.bundle_id.as_uuid())
            .bind(&deployment.environment)
            .bind(deployment.status.to_string())
            .bind(deployment.rollout_percentage as i32)
            .bind(deployment.created_at)
            .bind(deployment.deployed_at)
            .bind(deployment.rolled_back_at)
            .bind(&deployment.description)
            .bind(serde_json::to_value(&deployment.metadata)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created deployment: {}", deployment.id);
        Ok(())
    }

    async fn get_by_id_postgres(
        pool: &sqlx::PgPool,
        id: &DeploymentId,
    ) -> Result<Option<Deployment>> {
        let query = "SELECT * FROM deployments WHERE id = $1";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let deployment = Deployment {
                    id: DeploymentId::from_uuid(row.get("id")),
                    application_id: ApplicationId::from_uuid(row.get("application_id")),
                    bundle_id: BundleId::from_uuid(row.get("bundle_id")),
                    environment: row.get("environment"),
                    status: DeploymentStatus::from_str(&row.get::<String, _>("status"))?,
                    rollout_percentage: row.get::<i32, _>("rollout_percentage") as u32,
                    created_at: row.get("created_at"),
                    deployed_at: row.get("deployed_at"),
                    rolled_back_at: row.get("rolled_back_at"),
                    description: row.get("description"),
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(deployment))
            }
            None => Ok(None),
        }
    }

    async fn update_postgres(pool: &sqlx::PgPool, deployment: &Deployment) -> Result<()> {
        let query = r#"
            UPDATE deployments 
            SET status = $2, rollout_percentage = $3, deployed_at = $4, rolled_back_at = $5, description = $6, metadata = $7
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(deployment.id.as_uuid())
            .bind(deployment.status.to_string())
            .bind(deployment.rollout_percentage as i32)
            .bind(deployment.deployed_at)
            .bind(deployment.rolled_back_at)
            .bind(&deployment.description)
            .bind(serde_json::to_value(&deployment.metadata)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated deployment: {}", deployment.id);
        Ok(())
    }

    async fn delete_postgres(pool: &sqlx::PgPool, id: &DeploymentId) -> Result<()> {
        let query = "DELETE FROM deployments WHERE id = $1";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted deployment: {}", id);
        Ok(())
    }

    async fn get_active_for_application_postgres(
        pool: &sqlx::PgPool,
        application_id: &ApplicationId,
        environment: &str,
    ) -> Result<Vec<Deployment>> {
        let query = r#"
            SELECT * FROM deployments 
            WHERE application_id = $1 AND environment = $2 AND status = 'active'
            ORDER BY created_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(application_id.as_uuid())
            .bind(environment)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_deployments_postgres(rows)
    }

    async fn list_for_application_postgres(
        pool: &sqlx::PgPool,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        let query = r#"
            SELECT * FROM deployments 
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

        Self::rows_to_deployments_postgres(rows)
    }

    async fn get_by_status_postgres(
        pool: &sqlx::PgPool,
        status: &DeploymentStatus,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        let query = r#"
            SELECT * FROM deployments 
            WHERE status = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
        "#;

        let rows = sqlx::query(query)
            .bind(status.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_deployments_postgres(rows)
    }

    fn rows_to_deployments_postgres(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<Deployment>> {
        let mut deployments = Vec::new();
        for row in rows {
            let deployment = Deployment {
                id: DeploymentId::from_uuid(row.get("id")),
                application_id: ApplicationId::from_uuid(row.get("application_id")),
                bundle_id: BundleId::from_uuid(row.get("bundle_id")),
                environment: row.get("environment"),
                status: DeploymentStatus::from_str(&row.get::<String, _>("status"))?,
                rollout_percentage: row.get::<i32, _>("rollout_percentage") as u32,
                created_at: row.get("created_at"),
                deployed_at: row.get("deployed_at"),
                rolled_back_at: row.get("rolled_back_at"),
                description: row.get("description"),
                metadata: serde_json::from_value(row.get("metadata"))?,
            };
            deployments.push(deployment);
        }
        Ok(deployments)
    }

    // MySQL implementations
    async fn create_mysql(pool: &sqlx::MySqlPool, deployment: &Deployment) -> Result<()> {
        let query = r#"
            INSERT INTO deployments (id, application_id, bundle_id, environment, status, rollout_percentage, created_at, deployed_at, rolled_back_at, description, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(deployment.id.as_uuid())
            .bind(deployment.application_id.as_uuid())
            .bind(deployment.bundle_id.as_uuid())
            .bind(&deployment.environment)
            .bind(deployment.status.to_string())
            .bind(deployment.rollout_percentage as i32)
            .bind(deployment.created_at)
            .bind(deployment.deployed_at)
            .bind(deployment.rolled_back_at)
            .bind(&deployment.description)
            .bind(serde_json::to_value(&deployment.metadata)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created deployment: {}", deployment.id);
        Ok(())
    }

    async fn get_by_id_mysql(
        pool: &sqlx::MySqlPool,
        id: &DeploymentId,
    ) -> Result<Option<Deployment>> {
        let query = "SELECT * FROM deployments WHERE id = ?";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let deployment = Deployment {
                    id: DeploymentId::from_uuid(row.get("id")),
                    application_id: ApplicationId::from_uuid(row.get("application_id")),
                    bundle_id: BundleId::from_uuid(row.get("bundle_id")),
                    environment: row.get("environment"),
                    status: DeploymentStatus::from_str(&row.get::<String, _>("status"))?,
                    rollout_percentage: row.get::<i32, _>("rollout_percentage") as u32,
                    created_at: row.get("created_at"),
                    deployed_at: row.get("deployed_at"),
                    rolled_back_at: row.get("rolled_back_at"),
                    description: row.get("description"),
                    metadata: serde_json::from_value(row.get("metadata"))?,
                };
                Ok(Some(deployment))
            }
            None => Ok(None),
        }
    }

    async fn update_mysql(pool: &sqlx::MySqlPool, deployment: &Deployment) -> Result<()> {
        let query = r#"
            UPDATE deployments 
            SET status = ?, rollout_percentage = ?, deployed_at = ?, rolled_back_at = ?, description = ?, metadata = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(deployment.status.to_string())
            .bind(deployment.rollout_percentage as i32)
            .bind(deployment.deployed_at)
            .bind(deployment.rolled_back_at)
            .bind(&deployment.description)
            .bind(serde_json::to_value(&deployment.metadata)?)
            .bind(deployment.id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated deployment: {}", deployment.id);
        Ok(())
    }

    async fn delete_mysql(pool: &sqlx::MySqlPool, id: &DeploymentId) -> Result<()> {
        let query = "DELETE FROM deployments WHERE id = ?";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted deployment: {}", id);
        Ok(())
    }

    async fn get_active_for_application_mysql(
        pool: &sqlx::MySqlPool,
        application_id: &ApplicationId,
        environment: &str,
    ) -> Result<Vec<Deployment>> {
        let query = r#"
            SELECT * FROM deployments 
            WHERE application_id = ? AND environment = ? AND status = 'active'
            ORDER BY created_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(application_id.as_uuid())
            .bind(environment)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_deployments_mysql(rows)
    }

    async fn list_for_application_mysql(
        pool: &sqlx::MySqlPool,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        let query = r#"
            SELECT * FROM deployments 
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

        Self::rows_to_deployments_mysql(rows)
    }

    async fn get_by_status_mysql(
        pool: &sqlx::MySqlPool,
        status: &DeploymentStatus,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        let query = r#"
            SELECT * FROM deployments 
            WHERE status = ?
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let rows = sqlx::query(query)
            .bind(status.to_string())
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        Self::rows_to_deployments_mysql(rows)
    }

    fn rows_to_deployments_mysql(rows: Vec<sqlx::mysql::MySqlRow>) -> Result<Vec<Deployment>> {
        let mut deployments = Vec::new();
        for row in rows {
            let deployment = Deployment {
                id: DeploymentId::from_uuid(row.get("id")),
                application_id: ApplicationId::from_uuid(row.get("application_id")),
                bundle_id: BundleId::from_uuid(row.get("bundle_id")),
                environment: row.get("environment"),
                status: DeploymentStatus::from_str(&row.get::<String, _>("status"))?,
                rollout_percentage: row.get::<i32, _>("rollout_percentage") as u32,
                created_at: row.get("created_at"),
                deployed_at: row.get("deployed_at"),
                rolled_back_at: row.get("rolled_back_at"),
                description: row.get("description"),
                metadata: serde_json::from_value(row.get("metadata"))?,
            };
            deployments.push(deployment);
        }
        Ok(deployments)
    }
}
