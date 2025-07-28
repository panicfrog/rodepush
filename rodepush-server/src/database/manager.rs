//! Database manager and coordination layer

use crate::database::{
    application::{Application, ApplicationId, ApplicationService},
    config::DatabaseConfig,
    connection::{DatabaseConnection, DatabasePool},
    deployment::{Deployment, DeploymentId, DeploymentService},
    diff_package::{DiffPackage, DiffPackageId, DiffPackageService},
};
use rodepush_core::{BundleId, Result};

/// Database manager for high-level coordination
///
/// This is a lightweight manager that coordinates database operations
/// by delegating to specific service modules. It doesn't contain business logic
/// or SQL queries - those are handled by the respective service modules.
pub struct DatabaseManager {
    connection: DatabaseConnection,
}

impl DatabaseManager {
    /// Create a new database manager
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        let connection = DatabaseConnection::new(config).await?;
        Ok(Self { connection })
    }

    /// Get the underlying connection pool
    pub fn pool(&self) -> &DatabasePool {
        self.connection.pool()
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        self.connection.run_migrations().await
    }

    /// Check database health
    pub async fn health_check(&self) -> Result<bool> {
        self.connection.health_check().await
    }

    /// Get the underlying connection (for testing purposes only)
    pub fn connection_for_testing(&self) -> &DatabaseConnection {
        &self.connection
    }

    // Application operations - delegate to ApplicationService

    /// Create a new application
    pub async fn create_application(&self, application: &Application) -> Result<()> {
        ApplicationService::create(self.pool(), application).await
    }

    /// Get application by ID
    pub async fn get_application(&self, id: &ApplicationId) -> Result<Option<Application>> {
        ApplicationService::get_by_id(self.pool(), id).await
    }

    /// Get application by API key
    pub async fn get_application_by_api_key(&self, api_key: &str) -> Result<Option<Application>> {
        ApplicationService::get_by_api_key(self.pool(), api_key).await
    }

    /// Update application
    pub async fn update_application(&self, application: &Application) -> Result<()> {
        ApplicationService::update(self.pool(), application).await
    }

    /// Delete application
    pub async fn delete_application(&self, id: &ApplicationId) -> Result<()> {
        ApplicationService::delete(self.pool(), id).await
    }

    /// List applications with pagination
    pub async fn list_applications(&self, limit: i64, offset: i64) -> Result<Vec<Application>> {
        ApplicationService::list(self.pool(), limit, offset).await
    }

    // Deployment operations - delegate to DeploymentService

    /// Create a new deployment
    pub async fn create_deployment(&self, deployment: &Deployment) -> Result<()> {
        DeploymentService::create(self.pool(), deployment).await
    }

    /// Get deployment by ID
    pub async fn get_deployment(&self, id: &DeploymentId) -> Result<Option<Deployment>> {
        DeploymentService::get_by_id(self.pool(), id).await
    }

    /// Update deployment
    pub async fn update_deployment(&self, deployment: &Deployment) -> Result<()> {
        DeploymentService::update(self.pool(), deployment).await
    }

    /// Delete deployment
    pub async fn delete_deployment(&self, id: &DeploymentId) -> Result<()> {
        DeploymentService::delete(self.pool(), id).await
    }

    /// Get active deployments for an application
    pub async fn get_active_deployments(
        &self,
        application_id: &ApplicationId,
        environment: &str,
    ) -> Result<Vec<Deployment>> {
        DeploymentService::get_active_for_application(self.pool(), application_id, environment)
            .await
    }

    /// List deployments for an application
    pub async fn list_deployments_for_application(
        &self,
        application_id: &ApplicationId,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Deployment>> {
        DeploymentService::list_for_application(self.pool(), application_id, limit, offset).await
    }

    // Differential package operations - delegate to DiffPackageService

    /// Create a new differential package
    pub async fn create_diff_package(&self, diff_package: &DiffPackage) -> Result<()> {
        DiffPackageService::create(self.pool(), diff_package).await
    }

    /// Get differential package by ID
    pub async fn get_diff_package(&self, id: &DiffPackageId) -> Result<Option<DiffPackage>> {
        DiffPackageService::get_by_id(self.pool(), id).await
    }

    /// Get differential package by source and target bundle IDs
    pub async fn get_diff_package_by_bundles(
        &self,
        source_bundle_id: &BundleId,
        target_bundle_id: &BundleId,
    ) -> Result<Option<DiffPackage>> {
        DiffPackageService::get_by_bundles(self.pool(), source_bundle_id, target_bundle_id).await
    }

    /// Update differential package
    pub async fn update_diff_package(&self, diff_package: &DiffPackage) -> Result<()> {
        DiffPackageService::update(self.pool(), diff_package).await
    }

    /// Delete differential package
    pub async fn delete_diff_package(&self, id: &DiffPackageId) -> Result<()> {
        DiffPackageService::delete(self.pool(), id).await
    }

    /// Clean up old differential packages
    pub async fn cleanup_old_diff_packages(&self, older_than_days: i32) -> Result<u64> {
        DiffPackageService::cleanup_old_packages(self.pool(), older_than_days).await
    }

    // Cross-service coordination methods

    /// Deploy a bundle - this involves multiple services working together
    /// This is an example of where the manager provides coordination logic
    pub async fn deploy_bundle(
        &self,
        application_id: &ApplicationId,
        bundle_id: &BundleId,
        environment: String,
        description: Option<String>,
    ) -> Result<DeploymentId> {
        // 1. Verify application exists
        let _app = self.get_application(application_id).await?.ok_or_else(|| {
            rodepush_core::RodePushError::Validation {
                message: format!("Application {} not found", application_id),
            }
        })?;

        // 2. Create deployment
        let mut deployment =
            Deployment::new(application_id.clone(), bundle_id.clone(), environment);
        if let Some(desc) = description {
            deployment = deployment.with_description(desc);
        }

        let deployment_id = deployment.id.clone();
        self.create_deployment(&deployment).await?;

        // 3. Could add more coordination logic here (e.g., notifications, metrics)

        Ok(deployment_id)
    }

    /// Rollback a deployment - another coordination example
    pub async fn rollback_deployment(&self, deployment_id: &DeploymentId) -> Result<()> {
        // 1. Get current deployment
        let mut deployment = self.get_deployment(deployment_id).await?.ok_or_else(|| {
            rodepush_core::RodePushError::Validation {
                message: format!("Deployment {} not found", deployment_id),
            }
        })?;

        // 2. Mark as rolled back
        deployment.rollback();

        // 3. Update in database
        self.update_deployment(&deployment).await?;

        // 4. Could add more coordination logic here (e.g., cleanup, notifications)

        Ok(())
    }
}
