//! DeploymentService integration tests
//!
//! These tests verify all DeploymentService methods and business logic
//! by calling the service methods rather than writing raw SQL.

use rodepush_core::{BundleId, Platform};
use rodepush_server::database::{
    Application, ApplicationId, ApplicationService, Bundle, BundleService, DatabaseConfig,
    DatabaseConnection, DatabaseType, Deployment, DeploymentId, DeploymentService,
    DeploymentStatus,
};
use serial_test::serial;
use std::time::Duration;
use tokio::time::sleep;

/// Test database configuration for PostgreSQL
fn test_postgres_config() -> DatabaseConfig {
    DatabaseConfig {
        database_type: DatabaseType::Postgres,
        url: "postgresql://rodepush:rodepush123@localhost:5432/rodepush_test".to_string(),
        max_connections: 5,
        timeout_seconds: 10,
        ssl: false,
    }
}

/// Setup test database connection and clean tables
async fn setup_test_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let config = test_postgres_config();
    let connection = DatabaseConnection::new(&config).await?;

    // Clean tables in correct order (due to foreign key constraints)
    if let Some(pool) = connection.pool().as_postgres() {
        sqlx::query("TRUNCATE TABLE deployments CASCADE")
            .execute(pool)
            .await?;
        sqlx::query("TRUNCATE TABLE applications CASCADE")
            .execute(pool)
            .await?;
    }

    Ok(connection)
}

/// Create a test application for deployment tests
async fn create_test_application(
    connection: &DatabaseConnection,
) -> Result<Application, Box<dyn std::error::Error>> {
    let app = Application::new(
        "Test App for Deployments".to_string(),
        format!("test-key-{}", uuid::Uuid::new_v4()),
    );

    ApplicationService::create(connection.pool(), &app).await?;
    Ok(app)
}

/// Create dummy bundles for testing
async fn create_dummy_bundles(
    connection: &DatabaseConnection,
    app_id: &ApplicationId,
    bundle_ids: &[BundleId],
) -> Result<(), Box<dyn std::error::Error>> {
    for bundle_id in bundle_ids {
        let bundle = Bundle::new(
            app_id.clone(),
            "1.0.0".to_string(),
            Platform::Ios,
            "storage/dummy_bundle.bin".to_string(),
            1024,
            "dummy_checksum".to_string(),
        )
        .with_id(bundle_id.clone());

        BundleService::create(connection.pool(), &bundle).await?;
    }
    Ok(())
}

/// Setup test environment with clean database and test application
async fn setup_test_env() -> Result<(DatabaseConnection, Application), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;
    let app = create_test_application(&connection).await?;
    Ok((connection, app))
}

/// Setup test environment and create dummy bundle for testing
async fn setup_test_env_with_bundle()
-> Result<(DatabaseConnection, Application, BundleId), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;
    let bundle_id = BundleId::new();
    create_dummy_bundles(&connection, &app.id, &[bundle_id.clone()]).await?;
    Ok((connection, app, bundle_id))
}

/// Wait for database to be ready
async fn wait_for_database() -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..30 {
        let config = test_postgres_config();
        match DatabaseConnection::new(&config).await {
            Ok(conn) => {
                if conn.health_check().await.unwrap_or(false) {
                    println!("Database is ready after {} seconds", i);
                    return Ok(());
                }
            }
            Err(_) => {}
        }
        sleep(Duration::from_secs(1)).await;
    }
    Err("Database not ready after 30 seconds".into())
}

#[tokio::test]
#[serial]
async fn test_deployment_create() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create a new deployment
    let deployment = Deployment::new(app.id.clone(), bundle_id.clone(), "production".to_string())
        .with_description("Test deployment".to_string())
        .with_rollout_percentage(50)
        .with_metadata(
            "version".to_string(),
            serde_json::Value::String("1.0.0".to_string()),
        );

    // Test create operation
    DeploymentService::create(connection.pool(), &deployment).await?;

    // Verify creation by retrieving
    let retrieved = DeploymentService::get_by_id(connection.pool(), &deployment.id).await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, deployment.id);
    assert_eq!(retrieved.application_id, app.id);
    assert_eq!(retrieved.bundle_id, bundle_id);
    assert_eq!(retrieved.environment, "production");
    assert_eq!(retrieved.status, DeploymentStatus::Pending);
    assert_eq!(retrieved.rollout_percentage, 50);
    assert_eq!(retrieved.description, Some("Test deployment".to_string()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_update() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create original deployment
    let mut deployment = Deployment::new(app.id.clone(), bundle_id, "staging".to_string());

    DeploymentService::create(connection.pool(), &deployment).await?;

    // Update deployment
    deployment.activate(); // Change status to Active
    deployment.rollout_percentage = 100;
    deployment.description = Some("Updated deployment".to_string());
    deployment
        .metadata
        .insert("updated".to_string(), serde_json::Value::Bool(true));

    DeploymentService::update(connection.pool(), &deployment).await?;

    // Verify update
    let retrieved = DeploymentService::get_by_id(connection.pool(), &deployment.id).await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.status, DeploymentStatus::Active);
    assert_eq!(retrieved.rollout_percentage, 100);
    assert_eq!(
        retrieved.description,
        Some("Updated deployment".to_string())
    );
    assert!(retrieved.deployed_at.is_some());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_status_transitions() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create deployment
    let mut deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "development".to_string());

    DeploymentService::create(connection.pool(), &deployment).await?;

    // Test status transitions
    assert_eq!(deployment.status, DeploymentStatus::Pending);

    // Activate deployment
    deployment.activate();
    DeploymentService::update(connection.pool(), &deployment).await?;
    assert_eq!(deployment.status, DeploymentStatus::Active);
    assert!(deployment.deployed_at.is_some());

    // Pause deployment
    deployment.pause();
    DeploymentService::update(connection.pool(), &deployment).await?;
    assert_eq!(deployment.status, DeploymentStatus::Paused);

    // Resume deployment
    deployment.resume();
    DeploymentService::update(connection.pool(), &deployment).await?;
    assert_eq!(deployment.status, DeploymentStatus::Active);

    // Rollback deployment
    deployment.rollback();
    DeploymentService::update(connection.pool(), &deployment).await?;
    assert_eq!(deployment.status, DeploymentStatus::RolledBack);
    assert!(deployment.rolled_back_at.is_some());

    // Fail deployment
    deployment.fail();
    DeploymentService::update(connection.pool(), &deployment).await?;
    assert_eq!(deployment.status, DeploymentStatus::Failed);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_delete() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create deployment to delete
    let deployment = Deployment::new(app.id.clone(), bundle_id.clone(), "test-delete".to_string());

    DeploymentService::create(connection.pool(), &deployment).await?;

    // Verify it exists
    let before_delete = DeploymentService::get_by_id(connection.pool(), &deployment.id).await?;
    assert!(before_delete.is_some());

    // Delete deployment
    DeploymentService::delete(connection.pool(), &deployment.id).await?;

    // Verify deletion
    let after_delete = DeploymentService::get_by_id(connection.pool(), &deployment.id).await?;
    assert!(after_delete.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_get_active_for_application() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create multiple deployments with different statuses
    let mut active_deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "production".to_string());
    active_deployment.activate();
    DeploymentService::create(connection.pool(), &active_deployment).await?;

    let pending_deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "production".to_string());
    DeploymentService::create(connection.pool(), &pending_deployment).await?;

    let mut failed_deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "production".to_string());
    failed_deployment.fail();
    DeploymentService::create(connection.pool(), &failed_deployment).await?;

    // Get active deployments
    let active_deployments =
        DeploymentService::get_active_for_application(connection.pool(), &app.id, "production")
            .await?;

    // Should only return the active deployment
    assert_eq!(active_deployments.len(), 1);
    assert_eq!(active_deployments[0].id, active_deployment.id);
    assert_eq!(active_deployments[0].status, DeploymentStatus::Active);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_list_for_application() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create multiple deployments for the application
    for i in 1..=5 {
        let deployment = Deployment::new(app.id.clone(), bundle_id.clone(), format!("env-{}", i));
        DeploymentService::create(connection.pool(), &deployment).await?;
        sleep(Duration::from_millis(10)).await; // Ensure different created_at times
    }

    // Test pagination - first page
    let page1 = DeploymentService::list_for_application(connection.pool(), &app.id, 3, 0).await?;
    assert_eq!(page1.len(), 3);

    // Test pagination - second page
    let page2 = DeploymentService::list_for_application(connection.pool(), &app.id, 3, 3).await?;
    assert_eq!(page2.len(), 2);

    // Test limit larger than total
    let all_deployments =
        DeploymentService::list_for_application(connection.pool(), &app.id, 10, 0).await?;
    assert_eq!(all_deployments.len(), 5);

    // Verify ordering (should be by created_at DESC)
    let first_deployment = &all_deployments[0];
    let last_deployment = &all_deployments[4];
    assert!(first_deployment.created_at >= last_deployment.created_at);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_get_by_status() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create deployments with different statuses
    let mut active_deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "prod".to_string());
    active_deployment.activate();
    DeploymentService::create(connection.pool(), &active_deployment).await?;

    let pending_deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "staging".to_string());
    DeploymentService::create(connection.pool(), &pending_deployment).await?;

    let mut failed_deployment =
        Deployment::new(app.id.clone(), bundle_id.clone(), "test".to_string());
    failed_deployment.fail();
    DeploymentService::create(connection.pool(), &failed_deployment).await?;

    // Get pending deployments
    let pending_deployments =
        DeploymentService::get_by_status(connection.pool(), &DeploymentStatus::Pending, 10, 0)
            .await?;
    assert_eq!(pending_deployments.len(), 1);
    assert_eq!(pending_deployments[0].status, DeploymentStatus::Pending);

    // Get active deployments
    let active_deployments =
        DeploymentService::get_by_status(connection.pool(), &DeploymentStatus::Active, 10, 0)
            .await?;
    assert_eq!(active_deployments.len(), 1);
    assert_eq!(active_deployments[0].status, DeploymentStatus::Active);

    // Get failed deployments
    let failed_deployments =
        DeploymentService::get_by_status(connection.pool(), &DeploymentStatus::Failed, 10, 0)
            .await?;
    assert_eq!(failed_deployments.len(), 1);
    assert_eq!(failed_deployments[0].status, DeploymentStatus::Failed);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_get_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;

    // Test get non-existent deployment by ID
    let non_existent_id = DeploymentId::new();
    let result = DeploymentService::get_by_id(connection.pool(), &non_existent_id).await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_deployment_complex_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app, bundle_id) = setup_test_env_with_bundle().await?;

    // Create deployment with complex metadata
    let deployment = Deployment::new(app.id.clone(), bundle_id.clone(), "production".to_string())
        .with_metadata(
            "version".to_string(),
            serde_json::Value::String("2.1.0".to_string()),
        )
        .with_metadata(
            "build_number".to_string(),
            serde_json::Value::Number(142.into()),
        )
        .with_metadata(
            "features".to_string(),
            serde_json::json!({
                "dark_mode": true,
                "new_ui": false,
                "experimental": ["feature_a", "feature_b"]
            }),
        );

    DeploymentService::create(connection.pool(), &deployment).await?;

    // Retrieve and verify complex metadata
    let retrieved = DeploymentService::get_by_id(connection.pool(), &deployment.id).await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(
        retrieved.metadata.get("version"),
        Some(&serde_json::Value::String("2.1.0".to_string()))
    );
    assert_eq!(
        retrieved.metadata.get("build_number"),
        Some(&serde_json::Value::Number(142.into()))
    );

    let features = retrieved.metadata.get("features").unwrap();
    assert_eq!(features["dark_mode"], serde_json::Value::Bool(true));
    assert_eq!(features["new_ui"], serde_json::Value::Bool(false));

    Ok(())
}
