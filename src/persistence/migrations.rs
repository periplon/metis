//! Database migrations for the persistence layer

use crate::persistence::error::PersistenceError;
use crate::persistence::pool::ConnectionPool;
use sqlx::Row;

/// Initial schema migration SQL
const MIGRATION_001_INITIAL: &str = r#"
-- Archetypes table (stores all 8 archetype types)
CREATE TABLE IF NOT EXISTS archetypes (
    id TEXT PRIMARY KEY,
    archetype_type TEXT NOT NULL,
    name TEXT NOT NULL,
    definition TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    UNIQUE(archetype_type, name)
);

-- Commits table (git-style version history)
CREATE TABLE IF NOT EXISTS commits (
    id TEXT PRIMARY KEY,
    commit_hash TEXT NOT NULL UNIQUE,
    parent_hash TEXT,
    message TEXT NOT NULL,
    author TEXT,
    committed_at TEXT NOT NULL,
    is_snapshot INTEGER NOT NULL DEFAULT 0
);

-- Changesets (diffs between commits)
CREATE TABLE IF NOT EXISTS changesets (
    id TEXT PRIMARY KEY,
    commit_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    archetype_type TEXT NOT NULL,
    archetype_name TEXT NOT NULL,
    old_definition TEXT,
    new_definition TEXT,
    FOREIGN KEY (commit_id) REFERENCES commits(id)
);

-- Snapshots (full state at specific commits)
CREATE TABLE IF NOT EXISTS snapshots (
    id TEXT PRIMARY KEY,
    commit_id TEXT NOT NULL,
    archetype_type TEXT NOT NULL,
    archetype_name TEXT NOT NULL,
    definition TEXT NOT NULL,
    FOREIGN KEY (commit_id) REFERENCES commits(id)
);

-- Tags (named commits)
CREATE TABLE IF NOT EXISTS tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    commit_id TEXT NOT NULL,
    message TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (commit_id) REFERENCES commits(id)
);

-- Migration tracking table
CREATE TABLE IF NOT EXISTS _metis_migrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    applied_at TEXT NOT NULL,
    checksum TEXT NOT NULL
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_archetypes_type ON archetypes(archetype_type);
CREATE INDEX IF NOT EXISTS idx_archetypes_type_name ON archetypes(archetype_type, name);
CREATE INDEX IF NOT EXISTS idx_archetypes_deleted ON archetypes(deleted_at);
CREATE INDEX IF NOT EXISTS idx_commits_hash ON commits(commit_hash);
CREATE INDEX IF NOT EXISTS idx_commits_time ON commits(committed_at);
CREATE INDEX IF NOT EXISTS idx_commits_parent ON commits(parent_hash);
CREATE INDEX IF NOT EXISTS idx_changesets_commit ON changesets(commit_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_commit ON snapshots(commit_id);
CREATE INDEX IF NOT EXISTS idx_tags_commit ON tags(commit_id);
"#;

/// Migration 002: Data Records table for Data Lakes
const MIGRATION_002_DATA_RECORDS: &str = r#"
-- Data Records table (stores data lake records, NOT versioned)
CREATE TABLE IF NOT EXISTS data_records (
    id TEXT PRIMARY KEY,
    data_lake TEXT NOT NULL,
    schema_name TEXT NOT NULL,
    data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    created_by TEXT,
    metadata TEXT
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_records_lake ON data_records(data_lake);
CREATE INDEX IF NOT EXISTS idx_records_schema ON data_records(data_lake, schema_name);
CREATE INDEX IF NOT EXISTS idx_records_created ON data_records(created_at);
"#;

/// Migration definition
struct Migration {
    name: &'static str,
    sql: &'static str,
    checksum: &'static str,
}

/// Get all migrations in order
fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            name: "001_initial_schema",
            sql: MIGRATION_001_INITIAL,
            checksum: "v1", // Simple version string, could be SHA256 of SQL
        },
        Migration {
            name: "002_data_records",
            sql: MIGRATION_002_DATA_RECORDS,
            checksum: "v1",
        },
    ]
}

/// Migration runner for the persistence layer
pub struct MigrationRunner {
    pool: ConnectionPool,
}

impl MigrationRunner {
    /// Create a new migration runner
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    /// Run all pending migrations
    pub async fn migrate_up(&self) -> Result<MigrationResult, PersistenceError> {
        let migrations = get_migrations();
        let mut applied = 0;
        let mut skipped = 0;

        // Ensure migrations table exists (bootstrap)
        self.ensure_migrations_table().await?;

        for migration in migrations {
            if self.is_migration_applied(migration.name).await? {
                tracing::debug!("Migration '{}' already applied, skipping", migration.name);
                skipped += 1;
                continue;
            }

            tracing::info!("Applying migration: {}", migration.name);

            // Execute migration SQL
            // For SQLite, we need to execute statements one by one
            for statement in migration.sql.split(';') {
                let statement = statement.trim();
                if statement.is_empty() || statement.starts_with("--") {
                    continue;
                }

                sqlx::query(statement)
                    .execute(self.pool.pool())
                    .await
                    .map_err(|e| {
                        PersistenceError::Migration(format!(
                            "Failed to execute migration '{}': {}",
                            migration.name, e
                        ))
                    })?;
            }

            // Record migration as applied
            self.record_migration(migration.name, migration.checksum)
                .await?;

            tracing::info!("Migration '{}' applied successfully", migration.name);
            applied += 1;
        }

        Ok(MigrationResult { applied, skipped })
    }

    /// Get migration status
    pub async fn status(&self) -> Result<Vec<MigrationStatus>, PersistenceError> {
        self.ensure_migrations_table().await?;

        let migrations = get_migrations();
        let mut statuses = Vec::new();

        for migration in migrations {
            let applied = self.is_migration_applied(migration.name).await?;
            let applied_at = if applied {
                self.get_migration_applied_at(migration.name).await?
            } else {
                None
            };

            statuses.push(MigrationStatus {
                name: migration.name.to_string(),
                applied,
                applied_at,
            });
        }

        Ok(statuses)
    }

    /// Ensure the migrations tracking table exists
    async fn ensure_migrations_table(&self) -> Result<(), PersistenceError> {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS _metis_migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL,
                checksum TEXT NOT NULL
            )
        "#;

        sqlx::query(sql)
            .execute(self.pool.pool())
            .await
            .map_err(|e| {
                PersistenceError::Migration(format!(
                    "Failed to create migrations table: {}",
                    e
                ))
            })?;

        Ok(())
    }

    /// Check if a migration has been applied
    async fn is_migration_applied(&self, name: &str) -> Result<bool, PersistenceError> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM _metis_migrations WHERE name = ?")
            .bind(name)
            .fetch_one(self.pool.pool())
            .await
            .map_err(|e| {
                PersistenceError::Migration(format!("Failed to check migration status: {}", e))
            })?;

        let count: i64 = result.try_get("count").unwrap_or(0);
        Ok(count > 0)
    }

    /// Get when a migration was applied
    async fn get_migration_applied_at(
        &self,
        name: &str,
    ) -> Result<Option<String>, PersistenceError> {
        let result =
            sqlx::query("SELECT applied_at FROM _metis_migrations WHERE name = ?")
                .bind(name)
                .fetch_optional(self.pool.pool())
                .await
                .map_err(|e| {
                    PersistenceError::Migration(format!(
                        "Failed to get migration applied_at: {}",
                        e
                    ))
                })?;

        Ok(result.map(|row| row.try_get("applied_at").unwrap_or_default()))
    }

    /// Record a migration as applied
    async fn record_migration(
        &self,
        name: &str,
        checksum: &str,
    ) -> Result<(), PersistenceError> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO _metis_migrations (name, applied_at, checksum) VALUES (?, ?, ?)",
        )
        .bind(name)
        .bind(&now)
        .bind(checksum)
        .execute(self.pool.pool())
        .await
        .map_err(|e| {
            PersistenceError::Migration(format!("Failed to record migration: {}", e))
        })?;

        Ok(())
    }
}

/// Result of running migrations
#[derive(Debug)]
pub struct MigrationResult {
    /// Number of migrations applied
    pub applied: usize,
    /// Number of migrations skipped (already applied)
    pub skipped: usize,
}

/// Status of a single migration
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    /// Migration name
    pub name: String,
    /// Whether the migration has been applied
    pub applied: bool,
    /// When the migration was applied (if applied)
    pub applied_at: Option<String>,
}
