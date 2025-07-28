//! ApplicationService integration tests
//!
//! These tests verify all ApplicationService methods and business logic
//! by calling the service methods rather than writing raw SQL.

use rodepush_server::database::{
    Application, ApplicationId, ApplicationService, DatabaseConfig, DatabaseConnection,
    DatabaseType,
};
use serial_test::serial;
use std::collections::HashMap;
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

/// Setup test database connection and clean applications table
async fn setup_test_db() -> Result<DatabaseConnection, Box<dyn std::error::Error>> {
    let config = test_postgres_config();
    let connection = DatabaseConnection::new(&config).await?;
    
    // Clean applications table
    if let Some(pool) = connection.pool().as_postgres() {
        sqlx::query("TRUNCATE TABLE applications CASCADE")
            .execute(pool)
            .await?;
    }
    
    Ok(connection)
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
async fn test_application_create() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;
    
    // Create a new application
    let app = Application::new(
        "Test App".to_string(),
        "test-api-key-123".to_string(),
    )
    .with_description("A test application".to_string())
    .with_owner("test@example.com".to_string())
    .with_setting("theme".to_string(), serde_json::Value::String("dark".to_string()));

    // Test create operation
    ApplicationService::create(connection.pool(), &app).await?;

    // Verify creation by retrieving
    let retrieved = ApplicationService::get_by_id(connection.pool(), &app.id).await?;
    assert!(retrieved.is_some());
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, app.id);
    assert_eq!(retrieved.name, "Test App");
    assert_eq!(retrieved.api_key, "test-api-key-123");
    assert_eq!(retrieved.description, Some("A test application".to_string()));
    assert_eq!(retrieved.owner, Some("test@example.com".to_string()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_application_get_by_api_key() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;
    
    // Create test application
    let app = Application::new(
        "API Key Test App".to_string(),
        "unique-api-key-456".to_string(),
    );
    
    ApplicationService::create(connection.pool(), &app).await?;

    // Test get by API key
    let retrieved = ApplicationService::get_by_api_key(connection.pool(), "unique-api-key-456").await?;
    assert!(retrieved.is_some());
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, app.id);
    assert_eq!(retrieved.name, "API Key Test App");

    // Test with non-existent API key
    let not_found = ApplicationService::get_by_api_key(connection.pool(), "non-existent-key").await?;
    assert!(not_found.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_application_update() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;
    
    // Create original application
    let mut app = Application::new(
        "Original Name".to_string(),
        "update-test-key".to_string(),
    );
    
    ApplicationService::create(connection.pool(), &app).await?;

    // Update application
    app.name = "Updated Name".to_string();
    app.description = Some("Updated description".to_string());
    app.settings.insert("new_setting".to_string(), serde_json::Value::Number(42.into()));
    app.updated_at = chrono::Utc::now();

    ApplicationService::update(connection.pool(), &app).await?;

    // Verify update
    let retrieved = ApplicationService::get_by_id(connection.pool(), &app.id).await?;
    assert!(retrieved.is_some());
    
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "Updated Name");
    assert_eq!(retrieved.description, Some("Updated description".to_string()));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_application_delete() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;
    
    // Create application to delete
    let app = Application::new(
        "Delete Test App".to_string(),
        "delete-test-key".to_string(),
    );
    
    ApplicationService::create(connection.pool(), &app).await?;

    // Verify it exists
    let before_delete = ApplicationService::get_by_id(connection.pool(), &app.id).await?;
    assert!(before_delete.is_some());

    // Delete application
    ApplicationService::delete(connection.pool(), &app.id).await?;

    // Verify deletion
    let after_delete = ApplicationService::get_by_id(connection.pool(), &app.id).await?;
    assert!(after_delete.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_application_list_pagination() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;
    
    // Create multiple applications
    for i in 1..=5 {
        let app = Application::new(
            format!("App {}", i),
            format!("api-key-{}", i),
        );
        ApplicationService::create(connection.pool(), &app).await?;
        sleep(Duration::from_millis(10)).await; // Ensure different created_at times
    }

    // Test pagination - first page
    let page1 = ApplicationService::list(connection.pool(), 3, 0).await?;
    assert_eq!(page1.len(), 3);

    // Test pagination - second page
    let page2 = ApplicationService::list(connection.pool(), 3, 3).await?;
    assert_eq!(page2.len(), 2);

    // Test limit larger than total
    let all_apps = ApplicationService::list(connection.pool(), 10, 0).await?;
    assert_eq!(all_apps.len(), 5);

    Ok(())
}
