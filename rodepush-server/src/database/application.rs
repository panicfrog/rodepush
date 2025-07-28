//! Application management and data models

use chrono::{DateTime, Utc};
use rodepush_core::{Result, RodePushError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::database::{connection::DatabasePool, error::DatabaseError};
use sqlx::Row;

/// Application identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApplicationId(Uuid);

impl ApplicationId {
    /// Generate a new application ID
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

impl Default for ApplicationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ApplicationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Application model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    /// Application ID
    pub id: ApplicationId,
    /// Application name
    pub name: String,
    /// API key for authentication
    pub api_key: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Application description
    pub description: Option<String>,
    /// Application owner
    pub owner: Option<String>,
    /// Application settings
    pub settings: HashMap<String, serde_json::Value>,
}

impl Application {
    /// Create a new application
    pub fn new(name: String, api_key: String) -> Self {
        let now = Utc::now();
        Self {
            id: ApplicationId::new(),
            name,
            api_key,
            created_at: now,
            updated_at: now,
            description: None,
            owner: None,
            settings: HashMap::new(),
        }
    }

    /// Set application description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set application owner
    pub fn with_owner(mut self, owner: String) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Add a setting
    pub fn with_setting(mut self, key: String, value: serde_json::Value) -> Self {
        self.settings.insert(key, value);
        self
    }
}

/// Application service for database operations
pub struct ApplicationService;

impl ApplicationService {
    /// Create a new application in the database
    pub async fn create(pool: &DatabasePool, application: &Application) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::create_postgres(pg_pool, application).await,
            DatabasePool::MySql(mysql_pool) => Self::create_mysql(mysql_pool, application).await,
        }
    }

    /// Get application by ID from the database
    pub async fn get_by_id(pool: &DatabasePool, id: &ApplicationId) -> Result<Option<Application>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::get_by_id_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::get_by_id_mysql(mysql_pool, id).await,
        }
    }

    /// Get application by API key from the database
    pub async fn get_by_api_key(pool: &DatabasePool, api_key: &str) -> Result<Option<Application>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => {
                Self::get_by_api_key_postgres(pg_pool, api_key).await
            }
            DatabasePool::MySql(mysql_pool) => {
                Self::get_by_api_key_mysql(mysql_pool, api_key).await
            }
        }
    }

    /// Update application in the database
    pub async fn update(pool: &DatabasePool, application: &Application) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::update_postgres(pg_pool, application).await,
            DatabasePool::MySql(mysql_pool) => Self::update_mysql(mysql_pool, application).await,
        }
    }

    /// Delete application from the database
    pub async fn delete(pool: &DatabasePool, id: &ApplicationId) -> Result<()> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::delete_postgres(pg_pool, id).await,
            DatabasePool::MySql(mysql_pool) => Self::delete_mysql(mysql_pool, id).await,
        }
    }

    /// List all applications with pagination
    pub async fn list(pool: &DatabasePool, limit: i64, offset: i64) -> Result<Vec<Application>> {
        match pool {
            DatabasePool::Postgres(pg_pool) => Self::list_postgres(pg_pool, limit, offset).await,
            DatabasePool::MySql(mysql_pool) => Self::list_mysql(mysql_pool, limit, offset).await,
        }
    }

    // PostgreSQL implementations
    async fn create_postgres(pool: &sqlx::PgPool, application: &Application) -> Result<()> {
        let query = r#"
            INSERT INTO applications (id, name, api_key, created_at, updated_at, description, owner, settings)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#;

        sqlx::query(query)
            .bind(application.id.as_uuid())
            .bind(&application.name)
            .bind(&application.api_key)
            .bind(application.created_at)
            .bind(application.updated_at)
            .bind(&application.description)
            .bind(&application.owner)
            .bind(serde_json::to_value(&application.settings)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created application: {}", application.id);
        Ok(())
    }

    async fn get_by_id_postgres(
        pool: &sqlx::PgPool,
        id: &ApplicationId,
    ) -> Result<Option<Application>> {
        let query = "SELECT * FROM applications WHERE id = $1";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let application = Application {
                    id: ApplicationId::from_uuid(row.get("id")),
                    name: row.get("name"),
                    api_key: row.get("api_key"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    description: row.get("description"),
                    owner: row.get("owner"),
                    settings: serde_json::from_value(row.get("settings"))?,
                };
                Ok(Some(application))
            }
            None => Ok(None),
        }
    }

    async fn get_by_api_key_postgres(
        pool: &sqlx::PgPool,
        api_key: &str,
    ) -> Result<Option<Application>> {
        let query = "SELECT * FROM applications WHERE api_key = $1";

        let row = sqlx::query(query)
            .bind(api_key)
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let application = Application {
                    id: ApplicationId::from_uuid(row.get("id")),
                    name: row.get("name"),
                    api_key: row.get("api_key"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    description: row.get("description"),
                    owner: row.get("owner"),
                    settings: serde_json::from_value(row.get("settings"))?,
                };
                Ok(Some(application))
            }
            None => Ok(None),
        }
    }

    async fn update_postgres(pool: &sqlx::PgPool, application: &Application) -> Result<()> {
        let query = r#"
            UPDATE applications 
            SET name = $2, api_key = $3, updated_at = $4, description = $5, owner = $6, settings = $7
            WHERE id = $1
        "#;

        sqlx::query(query)
            .bind(application.id.as_uuid())
            .bind(&application.name)
            .bind(&application.api_key)
            .bind(application.updated_at)
            .bind(&application.description)
            .bind(&application.owner)
            .bind(serde_json::to_value(&application.settings)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated application: {}", application.id);
        Ok(())
    }

    async fn delete_postgres(pool: &sqlx::PgPool, id: &ApplicationId) -> Result<()> {
        let query = "DELETE FROM applications WHERE id = $1";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted application: {}", id);
        Ok(())
    }

    async fn list_postgres(
        pool: &sqlx::PgPool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Application>> {
        let query = r#"
            SELECT * FROM applications 
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
        "#;

        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let mut applications = Vec::new();
        for row in rows {
            let application = Application {
                id: ApplicationId::from_uuid(row.get("id")),
                name: row.get("name"),
                api_key: row.get("api_key"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                description: row.get("description"),
                owner: row.get("owner"),
                settings: serde_json::from_value(row.get("settings"))?,
            };
            applications.push(application);
        }

        Ok(applications)
    }

    // MySQL implementations
    async fn create_mysql(pool: &sqlx::MySqlPool, application: &Application) -> Result<()> {
        let query = r#"
            INSERT INTO applications (id, name, api_key, created_at, updated_at, description, owner, settings)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        sqlx::query(query)
            .bind(application.id.as_uuid())
            .bind(&application.name)
            .bind(&application.api_key)
            .bind(application.created_at)
            .bind(application.updated_at)
            .bind(&application.description)
            .bind(&application.owner)
            .bind(serde_json::to_value(&application.settings)?)
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Created application: {}", application.id);
        Ok(())
    }

    async fn get_by_id_mysql(
        pool: &sqlx::MySqlPool,
        id: &ApplicationId,
    ) -> Result<Option<Application>> {
        let query = "SELECT * FROM applications WHERE id = ?";

        let row = sqlx::query(query)
            .bind(id.as_uuid())
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let application = Application {
                    id: ApplicationId::from_uuid(row.get("id")),
                    name: row.get("name"),
                    api_key: row.get("api_key"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    description: row.get("description"),
                    owner: row.get("owner"),
                    settings: serde_json::from_value(row.get("settings"))?,
                };
                Ok(Some(application))
            }
            None => Ok(None),
        }
    }

    async fn get_by_api_key_mysql(
        pool: &sqlx::MySqlPool,
        api_key: &str,
    ) -> Result<Option<Application>> {
        let query = "SELECT * FROM applications WHERE api_key = ?";

        let row = sqlx::query(query)
            .bind(api_key)
            .fetch_optional(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        match row {
            Some(row) => {
                let application = Application {
                    id: ApplicationId::from_uuid(row.get("id")),
                    name: row.get("name"),
                    api_key: row.get("api_key"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    description: row.get("description"),
                    owner: row.get("owner"),
                    settings: serde_json::from_value(row.get("settings"))?,
                };
                Ok(Some(application))
            }
            None => Ok(None),
        }
    }

    async fn update_mysql(pool: &sqlx::MySqlPool, application: &Application) -> Result<()> {
        let query = r#"
            UPDATE applications 
            SET name = ?, api_key = ?, updated_at = ?, description = ?, owner = ?, settings = ?
            WHERE id = ?
        "#;

        sqlx::query(query)
            .bind(&application.name)
            .bind(&application.api_key)
            .bind(application.updated_at)
            .bind(&application.description)
            .bind(&application.owner)
            .bind(serde_json::to_value(&application.settings)?)
            .bind(application.id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Updated application: {}", application.id);
        Ok(())
    }

    async fn delete_mysql(pool: &sqlx::MySqlPool, id: &ApplicationId) -> Result<()> {
        let query = "DELETE FROM applications WHERE id = ?";

        sqlx::query(query)
            .bind(id.as_uuid())
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        tracing::info!("Deleted application: {}", id);
        Ok(())
    }

    async fn list_mysql(
        pool: &sqlx::MySqlPool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Application>> {
        let query = r#"
            SELECT * FROM applications 
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
        "#;

        let rows = sqlx::query(query)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(|e| DatabaseError::Query {
                message: e.to_string(),
            })?;

        let mut applications = Vec::new();
        for row in rows {
            let application = Application {
                id: ApplicationId::from_uuid(row.get("id")),
                name: row.get("name"),
                api_key: row.get("api_key"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                description: row.get("description"),
                owner: row.get("owner"),
                settings: serde_json::from_value(row.get("settings"))?,
            };
            applications.push(application);
        }

        Ok(applications)
    }
}
