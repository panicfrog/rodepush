use rodepush_server::*;
use std::error::Error;

#[tokio::test]
async fn test_cleanup_function() -> Result<(), Box<dyn Error>> {
    // 创建数据库配置
    let config = DatabaseConfig {
        database_type: DatabaseType::Postgres,
        url: "postgresql://rodepush:rodepush123@localhost:5432/rodepush_test".to_string(),
        max_connections: 5,
        timeout_seconds: 10,
        ssl: false,
    };

    // 创建数据库管理器
    let manager = DatabaseManager::new(&config).await?;
    manager.run_migrations().await?;

    // 检查当前数据数量
    let db_pool = manager.connection_for_testing().pool_for_testing();
    let pool = db_pool
        .as_postgres()
        .expect("Expected PostgreSQL pool for testing");

    let count_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM applications")
        .fetch_one(pool)
        .await?;

    println!("清理前应用程序数量: {}", count_before.0);

    // 执行清理函数
    let result = sqlx::query("TRUNCATE TABLE diff_packages, deployments, applications CASCADE")
        .execute(pool)
        .await;

    match result {
        Ok(_) => println!("清理函数执行成功"),
        Err(e) => println!("清理函数执行失败: {}", e),
    }

    // 检查清理后数据数量
    let count_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM applications")
        .fetch_one(pool)
        .await?;

    println!("清理后应用程序数量: {}", count_after.0);

    // 验证清理是否成功
    assert_eq!(count_after.0, 0, "清理后应该没有应用程序记录");

    Ok(())
}
