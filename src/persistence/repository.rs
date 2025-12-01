//! Repository traits and implementations for the persistence layer

use crate::persistence::error::PersistenceError;
use crate::persistence::models::{
    ArchetypeType, Changeset, Commit, Operation, Tag,
};
use crate::persistence::pool::ConnectionPool;
use async_trait::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::collections::HashMap;

/// Repository for archetype CRUD operations
#[async_trait]
pub trait ArchetypeRepository: Send + Sync {
    /// Get an archetype by type and name
    async fn get(
        &self,
        archetype_type: &str,
        name: &str,
    ) -> Result<Option<Value>, PersistenceError>;

    /// List all archetypes of a specific type
    async fn list(&self, archetype_type: &str) -> Result<Vec<Value>, PersistenceError>;

    /// Create a new archetype
    async fn create(
        &self,
        archetype_type: &str,
        name: &str,
        definition: &Value,
    ) -> Result<(), PersistenceError>;

    /// Update an existing archetype with optimistic locking
    async fn update(
        &self,
        archetype_type: &str,
        name: &str,
        definition: &Value,
        expected_version: Option<u64>,
    ) -> Result<u64, PersistenceError>;

    /// Soft delete an archetype
    async fn delete(&self, archetype_type: &str, name: &str) -> Result<bool, PersistenceError>;

    /// Get the current version of an archetype
    async fn get_version(
        &self,
        archetype_type: &str,
        name: &str,
    ) -> Result<Option<u64>, PersistenceError>;

    /// Check if the database has any archetypes (for seeding check)
    async fn is_empty(&self) -> Result<bool, PersistenceError>;

    /// Import multiple archetypes in a batch
    async fn import_batch(
        &self,
        archetype_type: &str,
        items: Vec<Value>,
    ) -> Result<usize, PersistenceError>;

    /// Get all archetypes (for export)
    async fn export_all(&self) -> Result<HashMap<String, Vec<Value>>, PersistenceError>;
}

/// Repository for commit/version history operations
#[async_trait]
pub trait CommitRepository: Send + Sync {
    /// Create a new commit with changesets
    async fn create_commit(
        &self,
        message: &str,
        changes: Vec<ChangesetInput>,
        author: Option<&str>,
    ) -> Result<Commit, PersistenceError>;

    /// Get a commit by hash
    async fn get_commit(&self, commit_hash: &str) -> Result<Option<Commit>, PersistenceError>;

    /// List commits with pagination
    async fn list_commits(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Commit>, PersistenceError>;

    /// Get changesets for a commit
    async fn get_changesets(&self, commit_id: &str) -> Result<Vec<Changeset>, PersistenceError>;

    /// Rollback to a specific commit
    async fn rollback_to(&self, commit_hash: &str) -> Result<Commit, PersistenceError>;

    /// Create a tag for a commit
    async fn create_tag(
        &self,
        name: &str,
        commit_hash: &str,
        message: Option<&str>,
    ) -> Result<Tag, PersistenceError>;

    /// Get a tag by name
    async fn get_tag(&self, name: &str) -> Result<Option<Tag>, PersistenceError>;

    /// List all tags
    async fn list_tags(&self) -> Result<Vec<Tag>, PersistenceError>;

    /// Delete a tag
    async fn delete_tag(&self, name: &str) -> Result<bool, PersistenceError>;

    /// Get the latest commit (HEAD)
    async fn get_head(&self) -> Result<Option<Commit>, PersistenceError>;

    /// Check if a snapshot should be created based on interval
    async fn should_create_snapshot(&self, interval: u32) -> Result<bool, PersistenceError>;

    /// Create a full snapshot at the current commit
    async fn create_snapshot(&self, commit_id: &str) -> Result<(), PersistenceError>;
}

/// Input for creating a changeset
#[derive(Debug, Clone)]
pub struct ChangesetInput {
    pub operation: Operation,
    pub archetype_type: String,
    pub archetype_name: String,
    pub old_definition: Option<Value>,
    pub new_definition: Option<Value>,
}

/// SQLx-based implementation of ArchetypeRepository
pub struct SqlxArchetypeRepository {
    pool: ConnectionPool,
}

impl SqlxArchetypeRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ArchetypeRepository for SqlxArchetypeRepository {
    async fn get(
        &self,
        archetype_type: &str,
        name: &str,
    ) -> Result<Option<Value>, PersistenceError> {
        let row = sqlx::query(
            "SELECT definition FROM archetypes WHERE archetype_type = ? AND name = ? AND deleted_at IS NULL",
        )
        .bind(archetype_type)
        .bind(name)
        .fetch_optional(self.pool.pool())
        .await?;

        match row {
            Some(row) => {
                let definition: String = row.try_get("definition")?;
                let value: Value = serde_json::from_str(&definition)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn list(&self, archetype_type: &str) -> Result<Vec<Value>, PersistenceError> {
        let rows = sqlx::query(
            "SELECT definition FROM archetypes WHERE archetype_type = ? AND deleted_at IS NULL ORDER BY name",
        )
        .bind(archetype_type)
        .fetch_all(self.pool.pool())
        .await?;

        let mut result = Vec::new();
        for row in rows {
            let definition: String = row.try_get("definition")?;
            let value: Value = serde_json::from_str(&definition)?;
            result.push(value);
        }

        Ok(result)
    }

    async fn create(
        &self,
        archetype_type: &str,
        name: &str,
        definition: &Value,
    ) -> Result<(), PersistenceError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let definition_str = serde_json::to_string(definition)?;

        // Check if already exists (including soft-deleted)
        let existing = sqlx::query(
            "SELECT id, deleted_at FROM archetypes WHERE archetype_type = ? AND name = ?",
        )
        .bind(archetype_type)
        .bind(name)
        .fetch_optional(self.pool.pool())
        .await?;

        if let Some(row) = existing {
            let deleted_at: Option<String> = row.try_get("deleted_at")?;
            if deleted_at.is_none() {
                return Err(PersistenceError::Duplicate {
                    entity_type: archetype_type.to_string(),
                    name: name.to_string(),
                });
            }

            // Reactivate soft-deleted record
            let existing_id: String = row.try_get("id")?;
            sqlx::query(
                "UPDATE archetypes SET definition = ?, version = version + 1, updated_at = ?, deleted_at = NULL WHERE id = ?",
            )
            .bind(&definition_str)
            .bind(&now)
            .bind(&existing_id)
            .execute(self.pool.pool())
            .await?;
        } else {
            sqlx::query(
                "INSERT INTO archetypes (id, archetype_type, name, definition, version, created_at, updated_at) VALUES (?, ?, ?, ?, 1, ?, ?)",
            )
            .bind(&id)
            .bind(archetype_type)
            .bind(name)
            .bind(&definition_str)
            .bind(&now)
            .bind(&now)
            .execute(self.pool.pool())
            .await?;
        }

        Ok(())
    }

    async fn update(
        &self,
        archetype_type: &str,
        name: &str,
        definition: &Value,
        expected_version: Option<u64>,
    ) -> Result<u64, PersistenceError> {
        let now = chrono::Utc::now().to_rfc3339();
        let definition_str = serde_json::to_string(definition)?;

        // If version checking is required
        if let Some(expected) = expected_version {
            let row = sqlx::query(
                "SELECT version FROM archetypes WHERE archetype_type = ? AND name = ? AND deleted_at IS NULL",
            )
            .bind(archetype_type)
            .bind(name)
            .fetch_optional(self.pool.pool())
            .await?;

            match row {
                Some(row) => {
                    let current: i64 = row.try_get("version")?;
                    if current as u64 != expected {
                        return Err(PersistenceError::VersionConflict {
                            expected,
                            actual: current as u64,
                        });
                    }
                }
                None => {
                    return Err(PersistenceError::NotFound {
                        entity_type: archetype_type.to_string(),
                        identifier: name.to_string(),
                    });
                }
            }
        }

        let result = sqlx::query(
            "UPDATE archetypes SET definition = ?, version = version + 1, updated_at = ? WHERE archetype_type = ? AND name = ? AND deleted_at IS NULL RETURNING version",
        )
        .bind(&definition_str)
        .bind(&now)
        .bind(archetype_type)
        .bind(name)
        .fetch_optional(self.pool.pool())
        .await?;

        match result {
            Some(row) => {
                let new_version: i64 = row.try_get("version")?;
                Ok(new_version as u64)
            }
            None => Err(PersistenceError::NotFound {
                entity_type: archetype_type.to_string(),
                identifier: name.to_string(),
            }),
        }
    }

    async fn delete(&self, archetype_type: &str, name: &str) -> Result<bool, PersistenceError> {
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            "UPDATE archetypes SET deleted_at = ?, updated_at = ? WHERE archetype_type = ? AND name = ? AND deleted_at IS NULL",
        )
        .bind(&now)
        .bind(&now)
        .bind(archetype_type)
        .bind(name)
        .execute(self.pool.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_version(
        &self,
        archetype_type: &str,
        name: &str,
    ) -> Result<Option<u64>, PersistenceError> {
        let row = sqlx::query(
            "SELECT version FROM archetypes WHERE archetype_type = ? AND name = ? AND deleted_at IS NULL",
        )
        .bind(archetype_type)
        .bind(name)
        .fetch_optional(self.pool.pool())
        .await?;

        match row {
            Some(row) => {
                let version: i64 = row.try_get("version")?;
                Ok(Some(version as u64))
            }
            None => Ok(None),
        }
    }

    async fn is_empty(&self) -> Result<bool, PersistenceError> {
        let row =
            sqlx::query("SELECT COUNT(*) as count FROM archetypes WHERE deleted_at IS NULL")
                .fetch_one(self.pool.pool())
                .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count == 0)
    }

    async fn import_batch(
        &self,
        archetype_type: &str,
        items: Vec<Value>,
    ) -> Result<usize, PersistenceError> {
        let mut count = 0;

        for item in items {
            // Extract name from the item based on archetype type
            let name = extract_name_from_value(archetype_type, &item)?;

            // Check if exists
            let existing = self.get(archetype_type, &name).await?;
            if existing.is_some() {
                // Update existing
                self.update(archetype_type, &name, &item, None).await?;
            } else {
                // Create new
                self.create(archetype_type, &name, &item).await?;
            }
            count += 1;
        }

        Ok(count)
    }

    async fn export_all(&self) -> Result<HashMap<String, Vec<Value>>, PersistenceError> {
        let mut result = HashMap::new();

        for archetype_type in ArchetypeType::all() {
            let items = self.list(archetype_type.as_str()).await?;
            result.insert(archetype_type.as_str().to_string(), items);
        }

        Ok(result)
    }
}

/// SQLx-based implementation of CommitRepository
pub struct SqlxCommitRepository {
    pool: ConnectionPool,
    archetype_repo: SqlxArchetypeRepository,
}

impl SqlxCommitRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        let archetype_repo = SqlxArchetypeRepository::new(pool.clone());
        Self {
            pool,
            archetype_repo,
        }
    }

    /// Generate a commit hash from parent hash and changes
    fn generate_commit_hash(
        parent_hash: Option<&str>,
        changes: &[ChangesetInput],
        timestamp: i64,
    ) -> String {
        let mut hasher = Sha256::new();

        // Include parent hash
        if let Some(parent) = parent_hash {
            hasher.update(parent.as_bytes());
        }

        // Include timestamp
        hasher.update(timestamp.to_le_bytes());

        // Include each change
        for change in changes {
            hasher.update(change.archetype_type.as_bytes());
            hasher.update(change.operation.as_str().as_bytes());
            hasher.update(change.archetype_name.as_bytes());
            if let Some(old) = &change.old_definition {
                hasher.update(old.to_string().as_bytes());
            }
            if let Some(new) = &change.new_definition {
                hasher.update(new.to_string().as_bytes());
            }
        }

        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl CommitRepository for SqlxCommitRepository {
    async fn create_commit(
        &self,
        message: &str,
        changes: Vec<ChangesetInput>,
        author: Option<&str>,
    ) -> Result<Commit, PersistenceError> {
        // Get current HEAD
        let head = self.get_head().await?;
        let parent_hash = head.as_ref().map(|h| h.commit_hash.as_str());

        // Generate commit hash
        let timestamp = chrono::Utc::now().timestamp();
        let commit_hash = Self::generate_commit_hash(parent_hash, &changes, timestamp);

        let commit_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Insert commit
        sqlx::query(
            "INSERT INTO commits (id, commit_hash, parent_hash, message, author, committed_at, is_snapshot) VALUES (?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&commit_id)
        .bind(&commit_hash)
        .bind(parent_hash)
        .bind(message)
        .bind(author)
        .bind(&now)
        .execute(self.pool.pool())
        .await?;

        // Insert changesets
        for change in &changes {
            let changeset_id = uuid::Uuid::new_v4().to_string();
            let old_def = change
                .old_definition
                .as_ref()
                .map(|v| v.to_string());
            let new_def = change
                .new_definition
                .as_ref()
                .map(|v| v.to_string());

            sqlx::query(
                "INSERT INTO changesets (id, commit_id, operation, archetype_type, archetype_name, old_definition, new_definition) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&changeset_id)
            .bind(&commit_id)
            .bind(change.operation.as_str())
            .bind(&change.archetype_type)
            .bind(&change.archetype_name)
            .bind(&old_def)
            .bind(&new_def)
            .execute(self.pool.pool())
            .await?;
        }

        Ok(Commit {
            id: commit_id,
            commit_hash,
            parent_hash: parent_hash.map(|s| s.to_string()),
            message: message.to_string(),
            author: author.map(|s| s.to_string()),
            committed_at: now,
            is_snapshot: false,
            changes_count: changes.len(),
            tag: None,
        })
    }

    async fn get_commit(&self, commit_hash: &str) -> Result<Option<Commit>, PersistenceError> {
        let row = sqlx::query(
            "SELECT c.*, (SELECT COUNT(*) FROM changesets WHERE commit_id = c.id) as changes_count, t.name as tag_name FROM commits c LEFT JOIN tags t ON t.commit_id = c.id WHERE c.commit_hash = ?",
        )
        .bind(commit_hash)
        .fetch_optional(self.pool.pool())
        .await?;

        match row {
            Some(row) => {
                let is_snapshot: i64 = row.try_get("is_snapshot")?;
                let changes_count: i64 = row.try_get("changes_count")?;

                Ok(Some(Commit {
                    id: row.try_get("id")?,
                    commit_hash: row.try_get("commit_hash")?,
                    parent_hash: row.try_get("parent_hash")?,
                    message: row.try_get("message")?,
                    author: row.try_get("author")?,
                    committed_at: row.try_get("committed_at")?,
                    is_snapshot: is_snapshot != 0,
                    changes_count: changes_count as usize,
                    tag: row.try_get("tag_name").ok(),
                }))
            }
            None => Ok(None),
        }
    }

    async fn list_commits(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Commit>, PersistenceError> {
        let rows = sqlx::query(
            "SELECT c.*, (SELECT COUNT(*) FROM changesets WHERE commit_id = c.id) as changes_count, t.name as tag_name FROM commits c LEFT JOIN tags t ON t.commit_id = c.id ORDER BY c.committed_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(self.pool.pool())
        .await?;

        let mut commits = Vec::new();
        for row in rows {
            let is_snapshot: i64 = row.try_get("is_snapshot")?;
            let changes_count: i64 = row.try_get("changes_count")?;

            commits.push(Commit {
                id: row.try_get("id")?,
                commit_hash: row.try_get("commit_hash")?,
                parent_hash: row.try_get("parent_hash")?,
                message: row.try_get("message")?,
                author: row.try_get("author")?,
                committed_at: row.try_get("committed_at")?,
                is_snapshot: is_snapshot != 0,
                changes_count: changes_count as usize,
                tag: row.try_get("tag_name").ok(),
            });
        }

        Ok(commits)
    }

    async fn get_changesets(&self, commit_id: &str) -> Result<Vec<Changeset>, PersistenceError> {
        let rows = sqlx::query("SELECT * FROM changesets WHERE commit_id = ?")
            .bind(commit_id)
            .fetch_all(self.pool.pool())
            .await?;

        let mut changesets = Vec::new();
        for row in rows {
            let old_def: Option<String> = row.try_get("old_definition")?;
            let new_def: Option<String> = row.try_get("new_definition")?;

            changesets.push(Changeset {
                operation: row.try_get("operation")?,
                archetype_type: row.try_get("archetype_type")?,
                archetype_name: row.try_get("archetype_name")?,
                old_definition: old_def
                    .as_ref()
                    .map(|s| serde_json::from_str(s))
                    .transpose()?,
                new_definition: new_def
                    .as_ref()
                    .map(|s| serde_json::from_str(s))
                    .transpose()?,
            });
        }

        Ok(changesets)
    }

    async fn rollback_to(&self, commit_hash: &str) -> Result<Commit, PersistenceError> {
        // Verify the target commit exists
        let _target_commit = self
            .get_commit(commit_hash)
            .await?
            .ok_or_else(|| PersistenceError::CommitNotFound {
                commit_hash: commit_hash.to_string(),
            })?;

        // Find the nearest snapshot at or before the target
        // For now, we'll replay from the beginning (simpler implementation)
        // TODO: Optimize with snapshot-based rollback

        // Get all commits from target to earliest (in reverse order)
        let all_commits = self.list_commits(10000, 0).await?;
        let target_idx = all_commits
            .iter()
            .position(|c| c.commit_hash == commit_hash)
            .ok_or_else(|| PersistenceError::CommitNotFound {
                commit_hash: commit_hash.to_string(),
            })?;

        // Get commits after target (the ones to undo)
        let commits_to_undo: Vec<_> = all_commits[..target_idx].to_vec();

        // Collect undo changesets
        let mut undo_changes = Vec::new();
        for commit in commits_to_undo.iter().rev() {
            let changesets = self.get_changesets(&commit.id).await?;
            for cs in changesets {
                // Reverse the operation
                let undo_change = match cs.operation.as_str() {
                    "create" => ChangesetInput {
                        operation: Operation::Delete,
                        archetype_type: cs.archetype_type.clone(),
                        archetype_name: cs.archetype_name.clone(),
                        old_definition: cs.new_definition.clone(),
                        new_definition: None,
                    },
                    "delete" => ChangesetInput {
                        operation: Operation::Create,
                        archetype_type: cs.archetype_type.clone(),
                        archetype_name: cs.archetype_name.clone(),
                        old_definition: None,
                        new_definition: cs.old_definition.clone(),
                    },
                    "update" => ChangesetInput {
                        operation: Operation::Update,
                        archetype_type: cs.archetype_type.clone(),
                        archetype_name: cs.archetype_name.clone(),
                        old_definition: cs.new_definition.clone(),
                        new_definition: cs.old_definition.clone(),
                    },
                    _ => continue,
                };

                // Apply the undo
                match &undo_change.operation {
                    Operation::Create => {
                        if let Some(def) = &undo_change.new_definition {
                            self.archetype_repo
                                .create(&undo_change.archetype_type, &undo_change.archetype_name, def)
                                .await?;
                        }
                    }
                    Operation::Update => {
                        if let Some(def) = &undo_change.new_definition {
                            self.archetype_repo
                                .update(&undo_change.archetype_type, &undo_change.archetype_name, def, None)
                                .await?;
                        }
                    }
                    Operation::Delete => {
                        self.archetype_repo
                            .delete(&undo_change.archetype_type, &undo_change.archetype_name)
                            .await?;
                    }
                }

                undo_changes.push(undo_change);
            }
        }

        // Create rollback commit
        let rollback_commit = self
            .create_commit(
                &format!("Rollback to {}", &commit_hash[..8]),
                undo_changes,
                None,
            )
            .await?;

        Ok(rollback_commit)
    }

    async fn create_tag(
        &self,
        name: &str,
        commit_hash: &str,
        message: Option<&str>,
    ) -> Result<Tag, PersistenceError> {
        // Check if tag already exists
        if self.get_tag(name).await?.is_some() {
            return Err(PersistenceError::TagExists {
                name: name.to_string(),
            });
        }

        // Get commit
        let commit = self
            .get_commit(commit_hash)
            .await?
            .ok_or_else(|| PersistenceError::CommitNotFound {
                commit_hash: commit_hash.to_string(),
            })?;

        let tag_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO tags (id, name, commit_id, message, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&tag_id)
        .bind(name)
        .bind(&commit.id)
        .bind(message)
        .bind(&now)
        .execute(self.pool.pool())
        .await?;

        Ok(Tag {
            name: name.to_string(),
            commit_hash: commit_hash.to_string(),
            message: message.map(|s| s.to_string()),
            created_at: now,
        })
    }

    async fn get_tag(&self, name: &str) -> Result<Option<Tag>, PersistenceError> {
        let row = sqlx::query(
            "SELECT t.*, c.commit_hash FROM tags t JOIN commits c ON t.commit_id = c.id WHERE t.name = ?",
        )
        .bind(name)
        .fetch_optional(self.pool.pool())
        .await?;

        match row {
            Some(row) => Ok(Some(Tag {
                name: row.try_get("name")?,
                commit_hash: row.try_get("commit_hash")?,
                message: row.try_get("message")?,
                created_at: row.try_get("created_at")?,
            })),
            None => Ok(None),
        }
    }

    async fn list_tags(&self) -> Result<Vec<Tag>, PersistenceError> {
        let rows = sqlx::query(
            "SELECT t.*, c.commit_hash FROM tags t JOIN commits c ON t.commit_id = c.id ORDER BY t.created_at DESC",
        )
        .fetch_all(self.pool.pool())
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(Tag {
                name: row.try_get("name")?,
                commit_hash: row.try_get("commit_hash")?,
                message: row.try_get("message")?,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(tags)
    }

    async fn delete_tag(&self, name: &str) -> Result<bool, PersistenceError> {
        let result = sqlx::query("DELETE FROM tags WHERE name = ?")
            .bind(name)
            .execute(self.pool.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_head(&self) -> Result<Option<Commit>, PersistenceError> {
        let row = sqlx::query(
            "SELECT c.*, (SELECT COUNT(*) FROM changesets WHERE commit_id = c.id) as changes_count, t.name as tag_name FROM commits c LEFT JOIN tags t ON t.commit_id = c.id ORDER BY c.committed_at DESC LIMIT 1",
        )
        .fetch_optional(self.pool.pool())
        .await?;

        match row {
            Some(row) => {
                let is_snapshot: i64 = row.try_get("is_snapshot")?;
                let changes_count: i64 = row.try_get("changes_count")?;

                Ok(Some(Commit {
                    id: row.try_get("id")?,
                    commit_hash: row.try_get("commit_hash")?,
                    parent_hash: row.try_get("parent_hash")?,
                    message: row.try_get("message")?,
                    author: row.try_get("author")?,
                    committed_at: row.try_get("committed_at")?,
                    is_snapshot: is_snapshot != 0,
                    changes_count: changes_count as usize,
                    tag: row.try_get("tag_name").ok(),
                }))
            }
            None => Ok(None),
        }
    }

    async fn should_create_snapshot(&self, interval: u32) -> Result<bool, PersistenceError> {
        // Count commits since last snapshot
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM commits WHERE is_snapshot = 0 AND committed_at > COALESCE((SELECT MAX(committed_at) FROM commits WHERE is_snapshot = 1), '')",
        )
        .fetch_one(self.pool.pool())
        .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count >= interval as i64)
    }

    async fn create_snapshot(&self, commit_id: &str) -> Result<(), PersistenceError> {
        // Get all current archetypes
        let all_archetypes = self.archetype_repo.export_all().await?;

        // Insert snapshots for each archetype
        for (archetype_type, items) in all_archetypes {
            for item in items {
                let name = extract_name_from_value(&archetype_type, &item)?;
                let snapshot_id = uuid::Uuid::new_v4().to_string();
                let definition = serde_json::to_string(&item)?;

                sqlx::query(
                    "INSERT INTO snapshots (id, commit_id, archetype_type, archetype_name, definition) VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&snapshot_id)
                .bind(commit_id)
                .bind(&archetype_type)
                .bind(&name)
                .bind(&definition)
                .execute(self.pool.pool())
                .await?;
            }
        }

        // Mark commit as snapshot
        sqlx::query("UPDATE commits SET is_snapshot = 1 WHERE id = ?")
            .bind(commit_id)
            .execute(self.pool.pool())
            .await?;

        Ok(())
    }
}

/// Extract the name field from an archetype value based on type
fn extract_name_from_value(archetype_type: &str, value: &Value) -> Result<String, PersistenceError> {
    let name = match archetype_type {
        "resource" => value.get("uri").and_then(|v| v.as_str()),
        "resource_template" => value.get("uri_template").and_then(|v| v.as_str()),
        _ => value.get("name").and_then(|v| v.as_str()),
    };

    name.map(|s| s.to_string())
        .ok_or_else(|| PersistenceError::Serialization(format!(
            "Missing name/uri field in {} definition",
            archetype_type
        )))
}
