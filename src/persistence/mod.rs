//! Database persistence layer for Metis
//!
//! This module provides database-backed storage for archetype definitions with
//! git-style version history, supporting PostgreSQL, SQLite, and MySQL.
//!
//! # Architecture
//!
//! - `DataStore`: Main entry point for database operations
//! - `ArchetypeRepository`: CRUD operations for archetypes
//! - `CommitRepository`: Version history with commits, changesets, and tags
//! - `MigrationRunner`: Database schema migrations
//!
//! # Example
//!
//! ```rust,no_run
//! use metis::persistence::{DataStore, PersistenceConfig};
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = PersistenceConfig {
//!         url: "sqlite://metis.db".to_string(),
//!         max_connections: 5,
//!         auto_migrate: true,
//!         seed_on_startup: true,
//!         snapshot_interval: 10,
//!     };
//!
//!     let data_store = DataStore::new(&config).await?;
//!
//!     // Run migrations
//!     data_store.migrate().await?;
//!
//!     // Use repositories...
//!     Ok(())
//! }
//! ```

pub mod data_record_repository;
pub mod error;
pub mod migrations;
pub mod models;
pub mod pool;
pub mod repository;

pub use data_record_repository::{DataRecordRepository, SqlxDataRecordRepository};
pub use error::PersistenceError;
pub use migrations::{MigrationResult, MigrationRunner, MigrationStatus};
pub use models::{ArchetypeType, Changeset, Commit, Operation, Tag};
pub use pool::{ConnectionPool, DatabaseBackend};
pub use repository::{
    ArchetypeRepository, ChangesetInput, CommitRepository, SqlxArchetypeRepository,
    SqlxCommitRepository,
};

use crate::config::Settings;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for the persistence layer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistenceConfig {
    /// Database connection URL
    /// - SQLite: `sqlite://metis.db` or `sqlite::memory:`
    /// - PostgreSQL: `postgres://user:pass@host/db`
    /// - MySQL: `mysql://user:pass@host/db`
    pub url: String,

    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Run migrations automatically on startup
    #[serde(default = "default_auto_migrate")]
    pub auto_migrate: bool,

    /// Seed from config files if database is empty
    #[serde(default = "default_seed_on_startup")]
    pub seed_on_startup: bool,

    /// Create a full snapshot every N commits
    #[serde(default = "default_snapshot_interval")]
    pub snapshot_interval: u32,
}

fn default_max_connections() -> u32 {
    5
}

fn default_auto_migrate() -> bool {
    true
}

fn default_seed_on_startup() -> bool {
    true
}

fn default_snapshot_interval() -> u32 {
    10
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://metis.db".to_string(),
            max_connections: default_max_connections(),
            auto_migrate: default_auto_migrate(),
            seed_on_startup: default_seed_on_startup(),
            snapshot_interval: default_snapshot_interval(),
        }
    }
}

/// Main data store providing access to all persistence operations
pub struct DataStore {
    /// Connection pool
    pool: ConnectionPool,
    /// Configuration
    config: PersistenceConfig,
    /// Archetype repository
    archetypes: Arc<SqlxArchetypeRepository>,
    /// Commit repository
    commits: Arc<SqlxCommitRepository>,
    /// Data records repository
    records: Arc<SqlxDataRecordRepository>,
}

impl DataStore {
    /// Create a new DataStore with the given configuration
    pub async fn new(config: &PersistenceConfig) -> Result<Self, PersistenceError> {
        let pool = ConnectionPool::new(&config.url, config.max_connections, 30).await?;

        let archetypes = Arc::new(SqlxArchetypeRepository::new(pool.clone()));
        let commits = Arc::new(SqlxCommitRepository::new(pool.clone()));
        let records = Arc::new(SqlxDataRecordRepository::new(pool.clone()));

        Ok(Self {
            pool,
            config: config.clone(),
            archetypes,
            commits,
            records,
        })
    }

    /// Get the archetype repository
    pub fn archetypes(&self) -> &Arc<SqlxArchetypeRepository> {
        &self.archetypes
    }

    /// Get the commit repository
    pub fn commits(&self) -> &Arc<SqlxCommitRepository> {
        &self.commits
    }

    /// Get the data records repository
    pub fn records(&self) -> &Arc<SqlxDataRecordRepository> {
        &self.records
    }

    /// Get the connection pool
    pub fn pool(&self) -> &ConnectionPool {
        &self.pool
    }

    /// Get the database backend type
    pub fn backend(&self) -> DatabaseBackend {
        self.pool.backend()
    }

    /// Get the configuration
    pub fn config(&self) -> &PersistenceConfig {
        &self.config
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<MigrationResult, PersistenceError> {
        let runner = MigrationRunner::new(self.pool.clone());
        runner.migrate_up().await
    }

    /// Get migration status
    pub async fn migration_status(&self) -> Result<Vec<MigrationStatus>, PersistenceError> {
        let runner = MigrationRunner::new(self.pool.clone());
        runner.status().await
    }

    /// Check if the database is empty (no archetypes)
    pub async fn is_empty(&self) -> Result<bool, PersistenceError> {
        self.archetypes.is_empty().await
    }

    /// Seed the database from Settings (config files)
    pub async fn seed_from_settings(&self, settings: &Settings) -> Result<usize, PersistenceError> {
        let mut count = 0;

        // Seed each archetype type
        for resource in &settings.resources {
            let value = serde_json::to_value(resource)?;
            self.archetypes.create("resource", &resource.uri, &value).await?;
            count += 1;
        }

        for template in &settings.resource_templates {
            let value = serde_json::to_value(template)?;
            self.archetypes.create("resource_template", &template.uri_template, &value).await?;
            count += 1;
        }

        for tool in &settings.tools {
            let value = serde_json::to_value(tool)?;
            self.archetypes.create("tool", &tool.name, &value).await?;
            count += 1;
        }

        for prompt in &settings.prompts {
            let value = serde_json::to_value(prompt)?;
            self.archetypes.create("prompt", &prompt.name, &value).await?;
            count += 1;
        }

        for workflow in &settings.workflows {
            let value = serde_json::to_value(workflow)?;
            self.archetypes.create("workflow", &workflow.name, &value).await?;
            count += 1;
        }

        for agent in &settings.agents {
            let value = serde_json::to_value(agent)?;
            self.archetypes.create("agent", &agent.name, &value).await?;
            count += 1;
        }

        for orchestration in &settings.orchestrations {
            let value = serde_json::to_value(orchestration)?;
            self.archetypes.create("orchestration", &orchestration.name, &value).await?;
            count += 1;
        }

        for schema in &settings.schemas {
            let value = serde_json::to_value(schema)?;
            self.archetypes.create("schema", &schema.name, &value).await?;
            count += 1;
        }

        for data_lake in &settings.data_lakes {
            let value = serde_json::to_value(data_lake)?;
            self.archetypes.create("data_lake", &data_lake.name, &value).await?;
            count += 1;
        }

        // Create initial commit if we seeded anything
        if count > 0 {
            let changeset = ChangesetInput {
                operation: Operation::Create,
                archetype_type: "system".to_string(),
                archetype_name: "seed".to_string(),
                old_definition: None,
                new_definition: Some(serde_json::json!({
                    "count": count,
                    "source": "config_files"
                })),
            };

            self.commits
                .create_commit(
                    &format!("Initial seed from config files ({} archetypes)", count),
                    vec![changeset],
                    Some("system"),
                )
                .await?;
        }

        Ok(count)
    }

    /// Health check for the database connection
    pub async fn health_check(&self) -> Result<(), PersistenceError> {
        self.pool.health_check().await
    }

    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

impl Clone for DataStore {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            archetypes: self.archetypes.clone(),
            commits: self.commits.clone(),
            records: self.records.clone(),
        }
    }
}
