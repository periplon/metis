//! Database models for the persistence layer

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Archetype stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeRow {
    /// Unique identifier (UUID)
    pub id: String,
    /// Type of archetype: resource, resource_template, tool, prompt, workflow, agent, orchestration, schema
    pub archetype_type: String,
    /// Unique name within the archetype type
    pub name: String,
    /// JSON serialized configuration
    pub definition: String,
    /// Version for optimistic locking
    pub version: i64,
    /// Creation timestamp (ISO8601)
    pub created_at: String,
    /// Last update timestamp (ISO8601)
    pub updated_at: String,
    /// Soft delete timestamp (ISO8601), None if not deleted
    pub deleted_at: Option<String>,
}

/// Commit representing a point in version history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRow {
    /// Unique identifier (UUID)
    pub id: String,
    /// SHA-256 hash of commit content
    pub commit_hash: String,
    /// Hash of parent commit (None for root commit)
    pub parent_hash: Option<String>,
    /// Commit message
    pub message: String,
    /// Author of the commit (optional)
    pub author: Option<String>,
    /// Timestamp when committed (ISO8601)
    pub committed_at: String,
    /// Whether this commit includes a full snapshot
    pub is_snapshot: bool,
}

/// Changeset representing a single change within a commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesetRow {
    /// Unique identifier (UUID)
    pub id: String,
    /// Reference to the parent commit
    pub commit_id: String,
    /// Operation type: create, update, delete
    pub operation: String,
    /// Type of archetype affected
    pub archetype_type: String,
    /// Name of the archetype affected
    pub archetype_name: String,
    /// Previous definition (None for create operations)
    pub old_definition: Option<String>,
    /// New definition (None for delete operations)
    pub new_definition: Option<String>,
}

/// Snapshot of an archetype at a specific commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRow {
    /// Unique identifier (UUID)
    pub id: String,
    /// Reference to the parent commit
    pub commit_id: String,
    /// Type of archetype
    pub archetype_type: String,
    /// Name of the archetype
    pub archetype_name: String,
    /// Full definition at this point in time
    pub definition: String,
}

/// Tag pointing to a specific commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRow {
    /// Unique identifier (UUID)
    pub id: String,
    /// Tag name (e.g., "v1.0", "production")
    pub name: String,
    /// Reference to the commit
    pub commit_id: String,
    /// Optional message describing the tag
    pub message: Option<String>,
    /// Creation timestamp (ISO8601)
    pub created_at: String,
}

/// API response types for version history

/// Commit information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: String,
    pub commit_hash: String,
    pub parent_hash: Option<String>,
    pub message: String,
    pub author: Option<String>,
    pub committed_at: String,
    pub is_snapshot: bool,
    pub changes_count: usize,
    pub tag: Option<String>,
}

impl From<CommitRow> for Commit {
    fn from(row: CommitRow) -> Self {
        Self {
            id: row.id,
            commit_hash: row.commit_hash,
            parent_hash: row.parent_hash,
            message: row.message,
            author: row.author,
            committed_at: row.committed_at,
            is_snapshot: row.is_snapshot,
            changes_count: 0,
            tag: None,
        }
    }
}

/// Changeset information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changeset {
    pub operation: String,
    pub archetype_type: String,
    pub archetype_name: String,
    pub old_definition: Option<Value>,
    pub new_definition: Option<Value>,
}

impl TryFrom<ChangesetRow> for Changeset {
    type Error = serde_json::Error;

    fn try_from(row: ChangesetRow) -> Result<Self, Self::Error> {
        Ok(Self {
            operation: row.operation,
            archetype_type: row.archetype_type,
            archetype_name: row.archetype_name,
            old_definition: row
                .old_definition
                .map(|s| serde_json::from_str(&s))
                .transpose()?,
            new_definition: row
                .new_definition
                .map(|s| serde_json::from_str(&s))
                .transpose()?,
        })
    }
}

/// Tag information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub commit_hash: String,
    pub message: Option<String>,
    pub created_at: String,
}

/// Operation type for changesets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Create,
    Update,
    Delete,
}

impl Operation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Operation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "create" => Ok(Self::Create),
            "update" => Ok(Self::Update),
            "delete" => Ok(Self::Delete),
            _ => Err(format!("Invalid operation: {}", s)),
        }
    }
}

/// Archetype type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeType {
    Resource,
    ResourceTemplate,
    Tool,
    Prompt,
    Workflow,
    Agent,
    Orchestration,
    Schema,
    DataLake,
}

impl ArchetypeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Resource => "resource",
            Self::ResourceTemplate => "resource_template",
            Self::Tool => "tool",
            Self::Prompt => "prompt",
            Self::Workflow => "workflow",
            Self::Agent => "agent",
            Self::Orchestration => "orchestration",
            Self::Schema => "schema",
            Self::DataLake => "data_lake",
        }
    }

    pub fn all() -> &'static [ArchetypeType] {
        &[
            Self::Resource,
            Self::ResourceTemplate,
            Self::Tool,
            Self::Prompt,
            Self::Workflow,
            Self::Agent,
            Self::Orchestration,
            Self::Schema,
            Self::DataLake,
        ]
    }
}

impl std::fmt::Display for ArchetypeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ArchetypeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "resource" => Ok(Self::Resource),
            "resource_template" => Ok(Self::ResourceTemplate),
            "tool" => Ok(Self::Tool),
            "prompt" => Ok(Self::Prompt),
            "workflow" => Ok(Self::Workflow),
            "agent" => Ok(Self::Agent),
            "orchestration" => Ok(Self::Orchestration),
            "schema" => Ok(Self::Schema),
            "data_lake" => Ok(Self::DataLake),
            _ => Err(format!("Invalid archetype type: {}", s)),
        }
    }
}
