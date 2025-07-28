//! DiffPackageService integration tests
//!
//! These tests verify all DiffPackageService methods and business logic
//! by calling the service methods rather than writing raw SQL.

use rodepush_core::{BundleId, Platform};
use rodepush_server::database::{
    Application, ApplicationId, ApplicationService, Bundle, BundleService, DatabaseConfig,
    DatabaseConnection, DatabaseType, DiffPackage, DiffPackageId, DiffPackageService,
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
        sqlx::query("TRUNCATE TABLE diff_packages CASCADE")
            .execute(pool)
            .await?;
        sqlx::query("TRUNCATE TABLE bundles CASCADE")
            .execute(pool)
            .await?;
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

/// Create a test application for diff package tests
async fn create_test_application(
    connection: &DatabaseConnection,
) -> Result<Application, Box<dyn std::error::Error>> {
    let app = Application::new(
        "Test App for DiffPackages".to_string(),
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

/// Setup test environment and create dummy bundles for testing
async fn setup_test_env_with_bundles()
-> Result<(DatabaseConnection, Application, BundleId, BundleId), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;
    let source_bundle_id = BundleId::new();
    let target_bundle_id = BundleId::new();
    create_dummy_bundles(
        &connection,
        &app.id,
        &[source_bundle_id.clone(), target_bundle_id.clone()],
    )
    .await?;
    Ok((connection, app, source_bundle_id, target_bundle_id))
}

#[tokio::test]
#[serial]
async fn test_diff_package_create() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, _app, source_bundle_id, target_bundle_id) =
        setup_test_env_with_bundles().await?;

    // Create a new differential package
    let diff_package = DiffPackage::new(
        source_bundle_id.clone(),
        target_bundle_id.clone(),
        "storage/diff_12345.bin".to_string(),
        1024 * 512, // 512KB
        0.3,        // 70% compression
        "sha256:abcdef123456".to_string(),
        Platform::Ios,
    )
    .with_metadata(
        "algorithm".to_string(),
        serde_json::Value::String("bsdiff".to_string()),
    )
    .with_metadata(
        "created_by".to_string(),
        serde_json::Value::String("test_suite".to_string()),
    );

    // Test create operation
    DiffPackageService::create(connection.pool(), &diff_package).await?;

    // Verify creation by retrieving
    let retrieved = DiffPackageService::get_by_id(connection.pool(), &diff_package.id).await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, diff_package.id);
    assert_eq!(retrieved.source_bundle_id, source_bundle_id);
    assert_eq!(retrieved.target_bundle_id, target_bundle_id);
    assert_eq!(retrieved.storage_key, "storage/diff_12345.bin");
    assert_eq!(retrieved.size_bytes, 1024 * 512);
    assert_eq!(retrieved.compression_ratio, 0.3);
    assert_eq!(retrieved.checksum, "sha256:abcdef123456");
    assert_eq!(retrieved.platform, Platform::Ios);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_get_by_bundles() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    // Create test differential packages
    let source_bundle_id = BundleId::new();
    let target_bundle_id1 = BundleId::new();
    let target_bundle_id2 = BundleId::new();

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(
        &connection,
        &app.id,
        &[
            source_bundle_id.clone(),
            target_bundle_id1.clone(),
            target_bundle_id2.clone(),
        ],
    )
    .await?;

    let diff_package1 = DiffPackage::new(
        source_bundle_id.clone(),
        target_bundle_id1.clone(),
        "storage/diff_1.bin".to_string(),
        1024,
        0.5,
        "checksum1".to_string(),
        Platform::Android,
    );

    let diff_package2 = DiffPackage::new(
        source_bundle_id.clone(),
        target_bundle_id2.clone(),
        "storage/diff_2.bin".to_string(),
        2048,
        0.4,
        "checksum2".to_string(),
        Platform::Android,
    );

    DiffPackageService::create(connection.pool(), &diff_package1).await?;
    DiffPackageService::create(connection.pool(), &diff_package2).await?;

    // Test get by specific bundle combination
    let retrieved = DiffPackageService::get_by_bundles(
        connection.pool(),
        &source_bundle_id,
        &target_bundle_id1,
    )
    .await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, diff_package1.id);
    assert_eq!(retrieved.storage_key, "storage/diff_1.bin");

    // Test with non-existent bundle combination
    let not_found =
        DiffPackageService::get_by_bundles(connection.pool(), &BundleId::new(), &target_bundle_id1)
            .await?;
    assert!(not_found.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_update() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    let source_bundle_id = BundleId::new();
    let target_bundle_id = BundleId::new();

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(
        &connection,
        &app.id,
        &[source_bundle_id.clone(), target_bundle_id.clone()],
    )
    .await?;

    // Create original diff package
    let mut diff_package = DiffPackage::new(
        source_bundle_id,
        target_bundle_id,
        "storage/original.bin".to_string(),
        1000,
        0.6,
        "original_checksum".to_string(),
        Platform::Ios,
    );

    DiffPackageService::create(connection.pool(), &diff_package).await?;

    // Update diff package
    diff_package.storage_key = "storage/updated.bin".to_string();
    diff_package.size_bytes = 1500;
    diff_package.compression_ratio = 0.4;
    diff_package.checksum = "updated_checksum".to_string();
    diff_package
        .metadata
        .insert("updated".to_string(), serde_json::Value::Bool(true));

    DiffPackageService::update(connection.pool(), &diff_package).await?;

    // Verify update
    let retrieved = DiffPackageService::get_by_id(connection.pool(), &diff_package.id).await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.storage_key, "storage/updated.bin");
    assert_eq!(retrieved.size_bytes, 1500);
    assert_eq!(retrieved.compression_ratio, 0.4);
    assert_eq!(retrieved.checksum, "updated_checksum");
    assert_eq!(
        retrieved.metadata.get("updated"),
        Some(&serde_json::Value::Bool(true))
    );

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_delete() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    let source_bundle_id = BundleId::new();
    let target_bundle_id = BundleId::new();

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(
        &connection,
        &app.id,
        &[source_bundle_id.clone(), target_bundle_id.clone()],
    )
    .await?;

    // Create diff package to delete
    let diff_package = DiffPackage::new(
        source_bundle_id,
        target_bundle_id,
        "storage/to_delete.bin".to_string(),
        500,
        0.8,
        "delete_checksum".to_string(),
        Platform::Android,
    );

    DiffPackageService::create(connection.pool(), &diff_package).await?;

    // Verify it exists
    let before_delete = DiffPackageService::get_by_id(connection.pool(), &diff_package.id).await?;
    assert!(before_delete.is_some());

    // Delete diff package
    DiffPackageService::delete(connection.pool(), &diff_package.id).await?;

    // Verify deletion
    let after_delete = DiffPackageService::get_by_id(connection.pool(), &diff_package.id).await?;
    assert!(after_delete.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_list_for_source_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    let source_bundle_id = BundleId::new();
    let other_source_bundle_id = BundleId::new();

    // Create all bundle IDs that will be used
    let mut bundle_ids = vec![source_bundle_id.clone(), other_source_bundle_id.clone()];
    for _ in 1..=5 {
        bundle_ids.push(BundleId::new());
    }
    bundle_ids.push(BundleId::new()); // for other_diff_package

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(&connection, &app.id, &bundle_ids).await?;

    // Create multiple diff packages with same source bundle
    for i in 1..=5 {
        let diff_package = DiffPackage::new(
            source_bundle_id.clone(),
            bundle_ids[i + 1].clone(), // Use pre-created bundle IDs
            format!("storage/diff_{}.bin", i),
            1000 * i as u64,
            0.5,
            format!("checksum_{}", i),
            Platform::Ios,
        );
        DiffPackageService::create(connection.pool(), &diff_package).await?;
        sleep(Duration::from_millis(10)).await; // Ensure different created_at times
    }

    // Create one with different source bundle
    let other_diff_package = DiffPackage::new(
        other_source_bundle_id,
        bundle_ids[7].clone(), // Use pre-created bundle ID
        "storage/other.bin".to_string(),
        999,
        0.5,
        "other_checksum".to_string(),
        Platform::Android,
    );
    DiffPackageService::create(connection.pool(), &other_diff_package).await?;

    // Test list for specific source bundle
    let source_diffs =
        DiffPackageService::list_for_source_bundle(connection.pool(), &source_bundle_id, 10, 0)
            .await?;
    assert_eq!(source_diffs.len(), 5);

    // Test pagination
    let page1 =
        DiffPackageService::list_for_source_bundle(connection.pool(), &source_bundle_id, 3, 0)
            .await?;
    assert_eq!(page1.len(), 3);

    let page2 =
        DiffPackageService::list_for_source_bundle(connection.pool(), &source_bundle_id, 3, 3)
            .await?;
    assert_eq!(page2.len(), 2);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_list_for_target_bundle() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    let target_bundle_id = BundleId::new();

    // Create all bundle IDs that will be used
    let mut bundle_ids = vec![target_bundle_id.clone()];
    for _ in 1..=3 {
        bundle_ids.push(BundleId::new());
    }

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(&connection, &app.id, &bundle_ids).await?;

    // Create multiple diff packages with same target bundle
    for i in 1..=3 {
        let diff_package = DiffPackage::new(
            bundle_ids[i].clone(), // Use pre-created bundle IDs
            target_bundle_id.clone(),
            format!("storage/to_target_{}.bin", i),
            1000 * i as u64,
            0.5,
            format!("target_checksum_{}", i),
            Platform::Android,
        );
        DiffPackageService::create(connection.pool(), &diff_package).await?;
    }

    // Test list for specific target bundle
    let target_diffs =
        DiffPackageService::list_for_target_bundle(connection.pool(), &target_bundle_id, 10, 0)
            .await?;
    assert_eq!(target_diffs.len(), 3);

    // Verify all have the same target bundle
    for diff in target_diffs {
        assert_eq!(diff.target_bundle_id, target_bundle_id);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_get_by_platform() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    // Create all bundle IDs that will be used
    let mut bundle_ids = Vec::new();
    for _ in 0..12 {
        // 6 pairs (source, target) for 6 diff packages
        bundle_ids.push(BundleId::new());
    }

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(&connection, &app.id, &bundle_ids).await?;

    // Create diff packages for different platforms
    for i in 1..=3 {
        let ios_diff = DiffPackage::new(
            bundle_ids[(i - 1) * 4].clone(),     // source for iOS
            bundle_ids[(i - 1) * 4 + 1].clone(), // target for iOS
            format!("storage/ios_{}.bin", i),
            1000,
            0.5,
            format!("ios_checksum_{}", i),
            Platform::Ios,
        );
        DiffPackageService::create(connection.pool(), &ios_diff).await?;

        let android_diff = DiffPackage::new(
            bundle_ids[(i - 1) * 4 + 2].clone(), // source for Android
            bundle_ids[(i - 1) * 4 + 3].clone(), // target for Android
            format!("storage/android_{}.bin", i),
            1000,
            0.5,
            format!("android_checksum_{}", i),
            Platform::Android,
        );
        DiffPackageService::create(connection.pool(), &android_diff).await?;
    }

    // Test get by iOS platform
    let ios_diffs =
        DiffPackageService::get_by_platform(connection.pool(), &Platform::Ios, 10, 0).await?;
    assert_eq!(ios_diffs.len(), 3);
    for diff in ios_diffs {
        assert_eq!(diff.platform, Platform::Ios);
    }

    // Test get by Android platform
    let android_diffs =
        DiffPackageService::get_by_platform(connection.pool(), &Platform::Android, 10, 0).await?;
    assert_eq!(android_diffs.len(), 3);
    for diff in android_diffs {
        assert_eq!(diff.platform, Platform::Android);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_cleanup_old_packages() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    // Create bundle IDs for 5 diff packages
    let mut bundle_ids = Vec::new();
    for _ in 0..10 {
        // 5 pairs (source, target) for 5 diff packages
        bundle_ids.push(BundleId::new());
    }

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(&connection, &app.id, &bundle_ids).await?;

    // Create some diff packages (they will have current timestamp)
    for i in 1..=5 {
        let diff_package = DiffPackage::new(
            bundle_ids[(i - 1) * 2].clone(),     // source
            bundle_ids[(i - 1) * 2 + 1].clone(), // target
            format!("storage/cleanup_{}.bin", i),
            1000,
            0.5,
            format!("cleanup_checksum_{}", i),
            Platform::Ios,
        );
        DiffPackageService::create(connection.pool(), &diff_package).await?;
    }

    // Cleanup packages older than 30 days (should not delete any since they're new)
    let deleted_count = DiffPackageService::cleanup_old_packages(connection.pool(), 30).await?;
    assert_eq!(deleted_count, 0);

    // Verify all packages still exist
    let remaining_packages =
        DiffPackageService::get_by_platform(connection.pool(), &Platform::Ios, 10, 0).await?;
    assert_eq!(remaining_packages.len(), 5);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_get_nonexistent() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let connection = setup_test_db().await?;

    // Test get non-existent diff package by ID
    let non_existent_id = DiffPackageId::new();
    let result = DiffPackageService::get_by_id(connection.pool(), &non_existent_id).await?;
    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_business_logic() -> Result<(), Box<dyn std::error::Error>> {
    wait_for_database().await?;
    let _connection = setup_test_db().await?;

    let source_bundle_id = BundleId::new();
    let target_bundle_id = BundleId::new();

    // Test business logic methods
    let diff_package = DiffPackage::new(
        source_bundle_id.clone(),
        target_bundle_id.clone(),
        "storage/test.bin".to_string(),
        1024, // 1KB diff for 5KB bundle
        0.2,  // 80% compression
        "test_checksum".to_string(),
        Platform::Ios,
    );

    // Test is_applicable
    assert!(diff_package.is_applicable(&source_bundle_id, &target_bundle_id));
    assert!(!diff_package.is_applicable(&BundleId::new(), &target_bundle_id));

    // Test space savings calculation
    let full_bundle_size = 5120; // 5KB
    let savings = diff_package.space_savings_bytes(full_bundle_size);
    assert_eq!(savings, 5120 - 1024); // 4KB savings

    // Test efficiency ratio
    let efficiency = diff_package.efficiency_ratio(full_bundle_size);
    assert_eq!(efficiency, 1024.0 / 5120.0); // 0.2 (20% of original size)

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_diff_package_complex_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let (connection, app) = setup_test_env().await?;

    let source_bundle_id = BundleId::new();
    let target_bundle_id = BundleId::new();

    // Create dummy bundles to satisfy foreign key constraints
    create_dummy_bundles(
        &connection,
        &app.id,
        &[source_bundle_id.clone(), target_bundle_id.clone()],
    )
    .await?;

    // Create diff package with complex metadata
    let diff_package = DiffPackage::new(
        source_bundle_id,
        target_bundle_id,
        "storage/complex.bin".to_string(),
        2048,
        0.35,
        "complex_checksum".to_string(),
        Platform::Android,
    )
    .with_metadata(
        "algorithm".to_string(),
        serde_json::Value::String("xdelta3".to_string()),
    )
    .with_metadata(
        "processing_time_ms".to_string(),
        serde_json::Value::Number(1250.into()),
    )
    .with_metadata(
        "statistics".to_string(),
        serde_json::json!({
            "chunks_added": 15,
            "chunks_removed": 8,
            "chunks_modified": 23,
            "optimization_passes": 3
        }),
    );

    DiffPackageService::create(connection.pool(), &diff_package).await?;

    // Retrieve and verify complex metadata
    let retrieved = DiffPackageService::get_by_id(connection.pool(), &diff_package.id).await?;
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(
        retrieved.metadata.get("algorithm"),
        Some(&serde_json::Value::String("xdelta3".to_string()))
    );
    assert_eq!(
        retrieved.metadata.get("processing_time_ms"),
        Some(&serde_json::Value::Number(1250.into()))
    );

    let stats = retrieved.metadata.get("statistics").unwrap();
    assert_eq!(stats["chunks_added"], serde_json::Value::Number(15.into()));
    assert_eq!(
        stats["optimization_passes"],
        serde_json::Value::Number(3.into())
    );

    Ok(())
}
