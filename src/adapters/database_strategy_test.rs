use super::mock_strategy::MockStrategyHandler;
use crate::adapters::state_manager::StateManager;
use crate::config::{MockConfig, MockStrategyType, DatabaseConfig};
use serde_json::json;
use std::sync::Arc;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

#[tokio::test]
async fn test_generate_database_sqlite() {
    sqlx::any::install_default_drivers();
    // Setup in-memory SQLite DB
    let pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to connect to SQLite");

    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&pool)
        .await
        .expect("Failed to create table");

    sqlx::query("INSERT INTO users (name) VALUES ('Alice')")
        .execute(&pool)
        .await
        .expect("Failed to insert data");

    // We need to use the same DB for the handler. 
    // However, the handler creates its own pool based on the URL.
    // For "sqlite::memory:", each connection is a new DB unless shared cache is used.
    // URL: "sqlite::memory:?cache=shared"
    
    // Use a file-based DB to ensure sharing works across pools
    let db_path = "test_db.sqlite";
    // Clean up previous run
    let _ = std::fs::remove_file(db_path);
    
    let db_url = format!("sqlite://{}?mode=rwc", db_path);
    
    let pool = SqlitePoolOptions::new()
        .connect(&db_url)
        .await
        .expect("Failed to connect to SQLite");
        
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")
        .execute(&pool)
        .await
        .expect("Failed to create table");

    sqlx::query("INSERT INTO users (name) VALUES ('Alice')")
        .execute(&pool)
        .await
        .expect("Failed to insert data");
        
    // Close pool to ensure data is flushed (WAL mode might delay, but file should be there)
    pool.close().await; 
    
    // Re-open or just let the handler open it.
    // Handler will open it.


    let handler = MockStrategyHandler::new(Arc::new(StateManager::new()));
    let config = MockConfig {
        strategy: MockStrategyType::Database,
        template: None,
        faker_type: None,
        stateful: None,
        file: None,
        pattern: None,
        script: None,
        llm: None,
        database: Some(DatabaseConfig {
            url: db_url.clone(),
            query: "SELECT name FROM users WHERE id = ?".to_string(),
            params: vec!["user_id".to_string()],
        }),
    };
    
    let args = json!({ "user_id": 1 });

    let result = handler.generate(&config, Some(&args)).await;
    if let Err(e) = &result {
        println!("Error: {}", e);
    }
    assert!(result.is_ok());
    let value = result.unwrap();
    
    // Cleanup
    let _ = std::fs::remove_file(db_path);

    
    // Expecting array of objects
    assert!(value.is_array());
    let arr = value.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "Alice");
}
