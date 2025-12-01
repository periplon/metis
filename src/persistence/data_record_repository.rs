//! Repository for Data Lake records
//!
//! This repository handles CRUD operations for data records within data lakes.
//! Unlike archetypes, data records are NOT versioned (no commit history).

use crate::config::DataRecord;
use crate::persistence::error::PersistenceError;
use crate::persistence::pool::ConnectionPool;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::Row;

/// Repository trait for data record operations
#[async_trait]
pub trait DataRecordRepository: Send + Sync {
    /// Get a record by ID
    async fn get(&self, id: &str) -> Result<Option<DataRecord>, PersistenceError>;

    /// List records for a data lake, optionally filtered by schema
    async fn list(
        &self,
        data_lake: &str,
        schema_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DataRecord>, PersistenceError>;

    /// Create a new record
    async fn create(&self, record: &DataRecord) -> Result<DataRecord, PersistenceError>;

    /// Create multiple records in a batch
    async fn create_batch(&self, records: &[DataRecord]) -> Result<usize, PersistenceError>;

    /// Update an existing record
    async fn update(&self, record: &DataRecord) -> Result<DataRecord, PersistenceError>;

    /// Delete a record by ID
    async fn delete(&self, id: &str) -> Result<bool, PersistenceError>;

    /// Delete all records for a data lake
    async fn delete_by_lake(&self, data_lake: &str) -> Result<usize, PersistenceError>;

    /// Delete records by data lake and schema
    async fn delete_by_schema(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> Result<usize, PersistenceError>;

    /// Count records for a data lake, optionally filtered by schema
    async fn count(
        &self,
        data_lake: &str,
        schema_name: Option<&str>,
    ) -> Result<usize, PersistenceError>;

    /// Search records by a JSON path query (implementation-dependent)
    async fn search(
        &self,
        data_lake: &str,
        query: &Value,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DataRecord>, PersistenceError>;
}

/// SQLx-based implementation of DataRecordRepository
pub struct SqlxDataRecordRepository {
    pool: ConnectionPool,
}

impl SqlxDataRecordRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    /// Parse a row into a DataRecord
    fn parse_row(row: &sqlx::any::AnyRow) -> Result<DataRecord, PersistenceError> {
        let data_str: String = row.try_get("data")?;
        let data: Value = serde_json::from_str(&data_str)?;

        let metadata_str: Option<String> = row.try_get("metadata")?;
        let metadata: Option<Value> = metadata_str
            .as_ref()
            .map(|s| serde_json::from_str(s))
            .transpose()?;

        Ok(DataRecord {
            id: row.try_get("id")?,
            data_lake: row.try_get("data_lake")?,
            schema_name: row.try_get("schema_name")?,
            data,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            created_by: row.try_get("created_by")?,
            metadata,
        })
    }
}

#[async_trait]
impl DataRecordRepository for SqlxDataRecordRepository {
    async fn get(&self, id: &str) -> Result<Option<DataRecord>, PersistenceError> {
        let row = sqlx::query("SELECT * FROM data_records WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool.pool())
            .await?;

        match row {
            Some(row) => Ok(Some(Self::parse_row(&row)?)),
            None => Ok(None),
        }
    }

    async fn list(
        &self,
        data_lake: &str,
        schema_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DataRecord>, PersistenceError> {
        let rows = if let Some(schema) = schema_name {
            sqlx::query(
                "SELECT * FROM data_records WHERE data_lake = ? AND schema_name = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(data_lake)
            .bind(schema)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(self.pool.pool())
            .await?
        } else {
            sqlx::query(
                "SELECT * FROM data_records WHERE data_lake = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(data_lake)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(self.pool.pool())
            .await?
        };

        let mut records = Vec::new();
        for row in rows {
            records.push(Self::parse_row(&row)?);
        }

        Ok(records)
    }

    async fn create(&self, record: &DataRecord) -> Result<DataRecord, PersistenceError> {
        let data_str = serde_json::to_string(&record.data)?;
        let metadata_str = record
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()?;

        sqlx::query(
            "INSERT INTO data_records (id, data_lake, schema_name, data, created_at, updated_at, created_by, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(&record.data_lake)
        .bind(&record.schema_name)
        .bind(&data_str)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.created_by)
        .bind(&metadata_str)
        .execute(self.pool.pool())
        .await?;

        Ok(record.clone())
    }

    async fn create_batch(&self, records: &[DataRecord]) -> Result<usize, PersistenceError> {
        let mut count = 0;

        for record in records {
            self.create(record).await?;
            count += 1;
        }

        Ok(count)
    }

    async fn update(&self, record: &DataRecord) -> Result<DataRecord, PersistenceError> {
        let now = chrono::Utc::now().to_rfc3339();
        let data_str = serde_json::to_string(&record.data)?;
        let metadata_str = record
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m))
            .transpose()?;

        let result = sqlx::query(
            "UPDATE data_records SET data = ?, updated_at = ?, metadata = ? WHERE id = ?",
        )
        .bind(&data_str)
        .bind(&now)
        .bind(&metadata_str)
        .bind(&record.id)
        .execute(self.pool.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Err(PersistenceError::NotFound {
                entity_type: "data_record".to_string(),
                identifier: record.id.clone(),
            });
        }

        // Return updated record
        self.get(&record.id)
            .await?
            .ok_or_else(|| PersistenceError::NotFound {
                entity_type: "data_record".to_string(),
                identifier: record.id.clone(),
            })
    }

    async fn delete(&self, id: &str) -> Result<bool, PersistenceError> {
        let result = sqlx::query("DELETE FROM data_records WHERE id = ?")
            .bind(id)
            .execute(self.pool.pool())
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_lake(&self, data_lake: &str) -> Result<usize, PersistenceError> {
        let result = sqlx::query("DELETE FROM data_records WHERE data_lake = ?")
            .bind(data_lake)
            .execute(self.pool.pool())
            .await?;

        Ok(result.rows_affected() as usize)
    }

    async fn delete_by_schema(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> Result<usize, PersistenceError> {
        let result =
            sqlx::query("DELETE FROM data_records WHERE data_lake = ? AND schema_name = ?")
                .bind(data_lake)
                .bind(schema_name)
                .execute(self.pool.pool())
                .await?;

        Ok(result.rows_affected() as usize)
    }

    async fn count(
        &self,
        data_lake: &str,
        schema_name: Option<&str>,
    ) -> Result<usize, PersistenceError> {
        let row = if let Some(schema) = schema_name {
            sqlx::query(
                "SELECT COUNT(*) as count FROM data_records WHERE data_lake = ? AND schema_name = ?",
            )
            .bind(data_lake)
            .bind(schema)
            .fetch_one(self.pool.pool())
            .await?
        } else {
            sqlx::query("SELECT COUNT(*) as count FROM data_records WHERE data_lake = ?")
                .bind(data_lake)
                .fetch_one(self.pool.pool())
                .await?
        };

        let count: i64 = row.try_get("count")?;
        Ok(count as usize)
    }

    async fn search(
        &self,
        data_lake: &str,
        _query: &Value,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<DataRecord>, PersistenceError> {
        // For now, just return all records for the data lake
        // TODO: Implement JSON path search based on database backend
        // SQLite: json_extract, PostgreSQL: jsonb operators
        self.list(data_lake, None, limit, offset).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_record_new() {
        let record = DataRecord::new("test_lake", "UserInput", serde_json::json!({"name": "John"}));
        assert!(!record.id.is_empty());
        assert_eq!(record.data_lake, "test_lake");
        assert_eq!(record.schema_name, "UserInput");
    }
}
