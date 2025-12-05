//! DataFusion Query Handler for Data Lake SQL Queries
//!
//! Provides SQL query execution capabilities over data lake records
//! stored in Parquet or JSONL files using Apache DataFusion.
//!
//! Tables are registered with schema-based naming: `datalake_name.schema_name`
//! This allows JOINs between tables from different data lakes.

use std::collections::HashMap;
use std::sync::Arc;

use datafusion::arrow::array::{Array, RecordBatch};
use datafusion::arrow::datatypes::Schema;
use datafusion::catalog_common::MemorySchemaProvider;
use datafusion::common::DataFusionError;
use datafusion::execution::context::SessionContext;
use serde_json::Value;
use tokio::sync::RwLock;

use crate::config::{DataLakeConfig, DataRecord};

use super::file_storage::FileStorageHandler;

/// Result type for DataFusion operations
pub type DataFusionResult<T> = Result<T, DataFusionHandlerError>;

/// Errors that can occur during DataFusion operations
#[derive(Debug, thiserror::Error)]
pub enum DataFusionHandlerError {
    #[error("DataFusion error: {0}")]
    DataFusion(#[from] DataFusionError),

    #[error("Arrow error: {0}")]
    Arrow(#[from] datafusion::arrow::error::ArrowError),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("File storage error: {0}")]
    FileStorage(#[from] super::file_storage::FileStorageError),

    #[error("Data lake not found: {0}")]
    DataLakeNotFound(String),

    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    #[error("SQL queries not enabled for data lake: {0}")]
    SqlQueriesDisabled(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

/// Handler for DataFusion SQL query operations
pub struct DataFusionHandler {
    /// DataFusion session context
    ctx: SessionContext,
    /// Registered tables (data_lake/schema -> table_name)
    registered_tables: RwLock<HashMap<String, String>>,
    /// File storage handler for accessing data files
    file_storage: Option<Arc<FileStorageHandler>>,
}

impl DataFusionHandler {
    /// Create a new DataFusionHandler
    pub fn new() -> Self {
        let ctx = SessionContext::new();
        // Register JSON functions for querying JSON data
        Self::register_json_functions(&ctx);

        Self {
            ctx,
            registered_tables: RwLock::new(HashMap::new()),
            file_storage: None,
        }
    }

    /// Create a new DataFusionHandler with file storage
    pub fn with_file_storage(file_storage: Arc<FileStorageHandler>) -> Self {
        let ctx = SessionContext::new();

        // Register JSON functions for querying JSON data
        Self::register_json_functions(&ctx);

        // Register the object store with the session context
        let store = file_storage.object_store();
        let url = if file_storage.is_s3() {
            if let Some(bucket) = file_storage.s3_bucket() {
                format!("s3://{}", bucket)
            } else {
                "s3://default".to_string()
            }
        } else {
            "file://".to_string()
        };

        // Only register if we can parse the URL
        if let Ok(parsed_url) = reqwest::Url::parse(&url) {
            ctx.runtime_env()
                .register_object_store(&parsed_url, store);
        }

        Self {
            ctx,
            registered_tables: RwLock::new(HashMap::new()),
            file_storage: Some(file_storage),
        }
    }

    /// Register JSON UDFs for querying JSON data in columns
    fn register_json_functions(ctx: &SessionContext) {
        // Register all JSON functions from datafusion-functions-json
        use datafusion_functions_json::udfs::*;

        let udfs = vec![
            json_get_udf(),
            json_get_bool_udf(),
            json_get_float_udf(),
            json_get_int_udf(),
            json_get_json_udf(),
            json_as_text_udf(),
            json_get_str_udf(),
            json_contains_udf(),
            json_length_udf(),
            json_object_keys_udf(),
        ];

        for udf in udfs {
            ctx.register_udf((*udf).clone());
        }
    }

    /// Sanitize a name for use as SQL identifier
    fn sanitize_identifier(name: &str) -> String {
        name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_")
    }

    /// Ensure a schema exists for a data lake, creating it if needed
    fn ensure_schema(&self, data_lake_name: &str) -> DataFusionResult<String> {
        let sanitized_name = Self::sanitize_identifier(data_lake_name);

        // Check if schema already exists
        let catalog = self.ctx.catalog("datafusion")
            .ok_or_else(|| DataFusionHandlerError::InvalidQuery("Default catalog not found".into()))?;

        if catalog.schema(&sanitized_name).is_none() {
            // Create a new schema for this data lake
            let schema_provider = Arc::new(MemorySchemaProvider::new());
            catalog.register_schema(&sanitized_name, schema_provider)
                .map_err(|e| DataFusionHandlerError::DataFusion(e))?;
            tracing::debug!("Created schema '{}' for data lake '{}'", sanitized_name, data_lake_name);
        }

        Ok(sanitized_name)
    }

    /// Register a data lake schema as a queryable table
    /// Tables are registered as `datalake_name.schema_name` for easy JOINs
    pub async fn register_data_lake_table(
        &self,
        data_lake: &DataLakeConfig,
        schema_name: &str,
    ) -> DataFusionResult<String> {
        if !data_lake.enable_sql_queries {
            return Err(DataFusionHandlerError::SqlQueriesDisabled(
                data_lake.name.clone(),
            ));
        }

        let table_key = format!("{}/{}", data_lake.name, schema_name);

        let file_storage = self.file_storage.as_ref().ok_or_else(|| {
            DataFusionHandlerError::InvalidQuery("File storage not configured".into())
        })?;

        // Ensure the data lake has a schema in the catalog
        let catalog_schema = self.ensure_schema(&data_lake.name)?;

        // Generate table name (sanitize special chars for SQL identifier)
        let sanitized_table = Self::sanitize_identifier(schema_name);

        // Full qualified name: datalake.table
        let full_table_name = format!("{}.{}", catalog_schema, sanitized_table);

        // Deregister existing table if present (to refresh with latest data)
        {
            let tables = self.registered_tables.read().await;
            if tables.contains_key(&table_key) {
                drop(tables);
                // Ignore errors if table doesn't exist in context
                let _ = self.ctx.deregister_table(&full_table_name);
                let mut tables_write = self.registered_tables.write().await;
                tables_write.remove(&table_key);
            }
        }

        // Read active records (filters out soft-deleted records via tombstones)
        let records = file_storage
            .read_active_records(&data_lake.name, schema_name)
            .await?;

        // Create Arrow schema for our data structure
        let arrow_schema = Arc::new(Self::create_datafusion_arrow_schema());

        if records.is_empty() {
            // Create an empty table with the standard schema
            let empty_batch = RecordBatch::new_empty(arrow_schema);
            let df = self.ctx.read_batch(empty_batch)?;
            self.ctx
                .register_table(&full_table_name, df.into_view())?;

            let mut tables = self.registered_tables.write().await;
            tables.insert(table_key, full_table_name.clone());
            return Ok(full_table_name);
        }

        // Convert records to Arrow arrays
        let batch = Self::records_to_record_batch(&records, arrow_schema)?;

        // Create DataFrame from the batch and register as table
        let df = self.ctx.read_batch(batch)?;
        self.ctx
            .register_table(&full_table_name, df.into_view())?;

        tracing::debug!(
            "Registered table {} with {} active records",
            full_table_name, records.len()
        );

        let mut tables = self.registered_tables.write().await;
        tables.insert(table_key, full_table_name.clone());

        Ok(full_table_name)
    }

    /// Execute a SQL query against registered tables
    pub async fn execute_sql(&self, sql: &str) -> DataFusionResult<QueryResult> {
        // Basic SQL injection prevention
        let sql_lower = sql.to_lowercase();
        if sql_lower.contains("drop ")
            || sql_lower.contains("delete ")
            || sql_lower.contains("truncate ")
            || sql_lower.contains("alter ")
            || sql_lower.contains("insert ")
            || sql_lower.contains("update ")
        {
            return Err(DataFusionHandlerError::InvalidQuery(
                "Only SELECT queries are allowed".into(),
            ));
        }

        let df = self.ctx.sql(sql).await?;
        let batches = df.collect().await?;

        self.batches_to_result(batches)
    }

    /// Execute a query against a specific data lake
    pub async fn query_data_lake(
        &self,
        data_lake: &DataLakeConfig,
        schema_name: &str,
        sql: &str,
    ) -> DataFusionResult<QueryResult> {
        // Ensure table is registered
        let table_name = self
            .register_data_lake_table(data_lake, schema_name)
            .await?;

        // Replace placeholder table name with actual table name
        let actual_sql = sql.replace("$table", &table_name);

        self.execute_sql(&actual_sql).await
    }

    /// Register all schemas from multiple data lakes for cross-data-lake JOINs
    /// Returns a map of original names to registered table names
    pub async fn register_multiple_data_lakes(
        &self,
        data_lakes: &[&DataLakeConfig],
    ) -> DataFusionResult<HashMap<String, String>> {
        let mut registered = HashMap::new();

        for data_lake in data_lakes {
            if !data_lake.enable_sql_queries {
                continue;
            }

            for schema_ref in &data_lake.schemas {
                let key = format!("{}.{}", data_lake.name, schema_ref.schema_name);
                match self.register_data_lake_table(data_lake, &schema_ref.schema_name).await {
                    Ok(table_name) => {
                        registered.insert(key, table_name);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to register table {}.{}: {}",
                            data_lake.name, schema_ref.schema_name, e
                        );
                    }
                }
            }
        }

        Ok(registered)
    }

    /// List all available schemas (data lakes) in the catalog
    pub fn list_schemas(&self) -> Vec<String> {
        if let Some(catalog) = self.ctx.catalog("datafusion") {
            catalog.schema_names()
                .into_iter()
                .filter(|s| s != "public" && s != "information_schema")
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Convert Arrow RecordBatches to JSON-friendly QueryResult
    fn batches_to_result(&self, batches: Vec<RecordBatch>) -> DataFusionResult<QueryResult> {
        let mut rows = Vec::new();
        let mut columns = Vec::new();
        let mut total_rows = 0;

        for batch in &batches {
            if columns.is_empty() {
                // Extract column names from schema
                columns = batch
                    .schema()
                    .fields()
                    .iter()
                    .map(|f| f.name().clone())
                    .collect();
            }

            total_rows += batch.num_rows();

            // Convert each row to a JSON object
            for row_idx in 0..batch.num_rows() {
                let mut row_map = serde_json::Map::new();

                for (col_idx, column) in batch.columns().iter().enumerate() {
                    let col_name = &columns[col_idx];
                    let value = self.array_value_to_json(column.as_ref(), row_idx);
                    row_map.insert(col_name.clone(), value);
                }

                rows.push(Value::Object(row_map));
            }
        }

        Ok(QueryResult {
            columns,
            rows,
            total_rows,
        })
    }

    /// Convert an Arrow array value at a specific index to JSON
    fn array_value_to_json(&self, array: &dyn Array, idx: usize) -> Value {
        use datafusion::arrow::array::*;
        use datafusion::arrow::datatypes::*;

        if array.is_null(idx) {
            return Value::Null;
        }

        match array.data_type() {
            DataType::Boolean => {
                let arr = array.as_any().downcast_ref::<BooleanArray>().unwrap();
                Value::Bool(arr.value(idx))
            }
            DataType::Int8 => {
                let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::Int16 => {
                let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::Int32 => {
                let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::Int64 => {
                let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::UInt8 => {
                let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::UInt16 => {
                let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::UInt32 => {
                let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::UInt64 => {
                let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
                Value::Number(arr.value(idx).into())
            }
            DataType::Float32 => {
                let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
                serde_json::Number::from_f64(arr.value(idx) as f64)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            DataType::Float64 => {
                let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
                serde_json::Number::from_f64(arr.value(idx))
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            }
            DataType::Utf8 => {
                let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
                Value::String(arr.value(idx).to_string())
            }
            DataType::LargeUtf8 => {
                let arr = array.as_any().downcast_ref::<LargeStringArray>().unwrap();
                Value::String(arr.value(idx).to_string())
            }
            DataType::Utf8View => {
                let arr = array.as_any().downcast_ref::<StringViewArray>().unwrap();
                Value::String(arr.value(idx).to_string())
            }
            DataType::Binary => {
                let arr = array.as_any().downcast_ref::<BinaryArray>().unwrap();
                Value::String(format!("<binary {} bytes>", arr.value(idx).len()))
            }
            DataType::LargeBinary => {
                let arr = array.as_any().downcast_ref::<LargeBinaryArray>().unwrap();
                Value::String(format!("<binary {} bytes>", arr.value(idx).len()))
            }
            DataType::BinaryView => {
                let arr = array.as_any().downcast_ref::<BinaryViewArray>().unwrap();
                Value::String(format!("<binary {} bytes>", arr.value(idx).len()))
            }
            _ => {
                // For unsupported types, format as type name
                Value::String(format!("<{:?}>", array.data_type()))
            }
        }
    }

    /// Get the Arrow schema for a registered table
    pub async fn get_table_schema(&self, table_name: &str) -> DataFusionResult<Arc<Schema>> {
        let df = self.ctx.table(table_name).await?;
        Ok(df.schema().inner().clone())
    }

    /// List registered tables
    pub async fn list_tables(&self) -> Vec<String> {
        let tables = self.registered_tables.read().await;
        tables.values().cloned().collect()
    }

    /// Check if a table is registered
    pub async fn is_table_registered(&self, data_lake: &str, schema_name: &str) -> bool {
        let key = format!("{}/{}", data_lake, schema_name);
        let tables = self.registered_tables.read().await;
        tables.contains_key(&key)
    }

    /// Unregister a table
    pub async fn unregister_table(&self, data_lake: &str, schema_name: &str) -> DataFusionResult<()> {
        let key = format!("{}/{}", data_lake, schema_name);
        let table_name = {
            let mut tables = self.registered_tables.write().await;
            tables.remove(&key)
        };

        if let Some(name) = table_name {
            self.ctx.deregister_table(&name)?;
        }

        Ok(())
    }

    /// Get session context for advanced operations
    pub fn session_context(&self) -> &SessionContext {
        &self.ctx
    }

    /// Create the Arrow schema for data records (using DataFusion's bundled arrow)
    fn create_datafusion_arrow_schema() -> Schema {
        use datafusion::arrow::datatypes::{DataType, Field};
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("data_lake", DataType::Utf8, false),
            Field::new("schema_name", DataType::Utf8, false),
            Field::new("data", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("created_by", DataType::Utf8, true),
            Field::new("metadata", DataType::Utf8, true),
        ])
    }

    /// Convert DataRecords to an Arrow RecordBatch
    fn records_to_record_batch(
        records: &[DataRecord],
        schema: Arc<Schema>,
    ) -> DataFusionResult<RecordBatch> {
        use datafusion::arrow::array::StringArray;

        let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
        let data_lakes: Vec<&str> = records.iter().map(|r| r.data_lake.as_str()).collect();
        let schema_names: Vec<&str> = records.iter().map(|r| r.schema_name.as_str()).collect();
        let data_strs: Vec<String> = records
            .iter()
            .map(|r| serde_json::to_string(&r.data).unwrap_or_default())
            .collect();
        let data_refs: Vec<&str> = data_strs.iter().map(|s| s.as_str()).collect();
        let created_ats: Vec<&str> = records.iter().map(|r| r.created_at.as_str()).collect();
        let updated_ats: Vec<&str> = records.iter().map(|r| r.updated_at.as_str()).collect();
        let created_bys: Vec<Option<&str>> = records
            .iter()
            .map(|r| r.created_by.as_deref())
            .collect();
        let metadata_strs: Vec<Option<String>> = records
            .iter()
            .map(|r| r.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap_or_default()))
            .collect();
        let metadata_refs: Vec<Option<&str>> = metadata_strs
            .iter()
            .map(|s| s.as_deref())
            .collect();

        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(StringArray::from(data_lakes)),
                Arc::new(StringArray::from(schema_names)),
                Arc::new(StringArray::from(data_refs)),
                Arc::new(StringArray::from(created_ats)),
                Arc::new(StringArray::from(updated_ats)),
                Arc::new(StringArray::from(created_bys)),
                Arc::new(StringArray::from(metadata_refs)),
            ],
        )?;

        Ok(batch)
    }
}

impl Default for DataFusionHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a SQL query execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryResult {
    /// Column names
    pub columns: Vec<String>,
    /// Result rows as JSON objects
    pub rows: Vec<Value>,
    /// Total number of rows returned
    pub total_rows: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datafusion_handler_creation() {
        let handler = DataFusionHandler::new();
        assert!(handler.list_tables().await.is_empty());
    }

    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let handler = DataFusionHandler::new();

        // These should all fail
        let dangerous_queries = vec![
            "DROP TABLE users",
            "DELETE FROM records",
            "TRUNCATE TABLE data",
            "ALTER TABLE records ADD column",
            "INSERT INTO records VALUES (1)",
            "UPDATE records SET data = 'x'",
        ];

        for query in dangerous_queries {
            let result = handler.execute_sql(query).await;
            assert!(result.is_err(), "Query should have been rejected: {}", query);
        }
    }
}
