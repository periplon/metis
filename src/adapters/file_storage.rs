//! File Storage Handler for Data Lake Records
//!
//! Provides abstraction over local filesystem and S3 storage backends
//! for storing data lake records in Parquet and JSONL formats.

use std::path::PathBuf;
use std::sync::Arc;

use arrow::array::{Array, ArrayRef, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use bytes::Bytes;
use object_store::aws::{AmazonS3Builder, AwsCredential};
use object_store::local::LocalFileSystem;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, PutPayload, StaticCredentialProvider};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use serde_json::Value;

use crate::config::data_lake::DataRecord;
use crate::config::file_storage::{DataLakeFileFormat, FileStorageConfig, S3DataConfig};

/// Result type for file storage operations
pub type FileStorageResult<T> = Result<T, FileStorageError>;

/// Errors that can occur during file storage operations
#[derive(Debug, thiserror::Error)]
pub enum FileStorageError {
    #[error("File storage is not enabled")]
    NotEnabled,

    #[error("Storage backend not configured")]
    NotConfigured,

    #[error("Object store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),

    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Handler for file storage operations
pub struct FileStorageHandler {
    /// The underlying object store (local or S3)
    store: Arc<dyn ObjectStore>,
    /// Configuration for this handler
    config: FileStorageConfig,
    /// Whether using S3 (affects path handling)
    is_s3: bool,
    /// Base prefix for all paths
    base_prefix: String,
}

impl FileStorageHandler {
    /// Create a new FileStorageHandler from configuration
    ///
    /// If `secrets` is provided, AWS credentials will be looked up from:
    /// 1. file_storage.s3 config (access_key_id, secret_access_key)
    /// 2. Secrets store (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 3. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    pub async fn new(
        config: FileStorageConfig,
        secrets: Option<&crate::adapters::secrets::SharedSecretsStore>,
    ) -> FileStorageResult<Self> {
        if !config.enabled {
            return Err(FileStorageError::NotEnabled);
        }

        let (store, is_s3, base_prefix): (Arc<dyn ObjectStore>, bool, String) =
            if let Some(s3_config) = &config.s3 {
                let store = Self::create_s3_store(s3_config, secrets).await?;
                let prefix = s3_config.effective_prefix();
                (Arc::new(store), true, prefix)
            } else if let Some(local_path) = &config.local_path {
                let path = PathBuf::from(local_path);
                // Ensure directory exists
                std::fs::create_dir_all(&path)?;
                let store = LocalFileSystem::new_with_prefix(&path)?;
                (Arc::new(store), false, String::new())
            } else {
                return Err(FileStorageError::NotConfigured);
            };

        Ok(Self {
            store,
            config,
            is_s3,
            base_prefix,
        })
    }

    /// Create an S3 object store from configuration
    /// Credentials are resolved in order: config -> secrets store -> environment variables
    async fn create_s3_store(
        config: &S3DataConfig,
        secrets: Option<&crate::adapters::secrets::SharedSecretsStore>,
    ) -> FileStorageResult<object_store::aws::AmazonS3> {
        use crate::adapters::secrets::keys;

        // Get credentials from: 1) config, 2) secrets store, 3) environment variables
        let access_key = if let Some(ak) = &config.access_key_id {
            Some(ak.clone())
        } else if let Some(secrets) = secrets {
            secrets.get_or_env(keys::AWS_ACCESS_KEY_ID).await
        } else {
            std::env::var("AWS_ACCESS_KEY_ID").ok()
        };

        let secret_key = if let Some(sk) = &config.secret_access_key {
            Some(sk.clone())
        } else if let Some(secrets) = secrets {
            secrets.get_or_env(keys::AWS_SECRET_ACCESS_KEY).await
        } else {
            std::env::var("AWS_SECRET_ACCESS_KEY").ok()
        };

        // We MUST have credentials to avoid IMDS timeout errors
        // If no credentials are found, fail fast with a clear error
        let (access_key, secret_key) = match (access_key, secret_key) {
            (Some(ak), Some(sk)) => (ak, sk),
            (None, None) => {
                return Err(FileStorageError::InvalidConfig(
                    "S3 credentials not found. Set them in one of: \
                     1) file_storage.s3 config (access_key_id, secret_access_key), \
                     2) secrets section (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY), or \
                     3) environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)".to_string()
                ));
            }
            (Some(_), None) => {
                return Err(FileStorageError::InvalidConfig(
                    "S3 secret_access_key not found. Set it in file_storage.s3 config, \
                     secrets section, or AWS_SECRET_ACCESS_KEY environment variable.".to_string()
                ));
            }
            (None, Some(_)) => {
                return Err(FileStorageError::InvalidConfig(
                    "S3 access_key_id not found. Set it in file_storage.s3 config, \
                     secrets section, or AWS_ACCESS_KEY_ID environment variable.".to_string()
                ));
            }
        };

        tracing::info!(
            "Configuring S3 file storage for bucket '{}' with endpoint {:?}",
            config.bucket,
            config.endpoint
        );

        // Create static credentials to completely bypass the default credential chain
        // (which includes EC2 IMDS that causes timeout errors outside AWS)
        let credential = AwsCredential {
            key_id: access_key,
            secret_key,
            token: None,
        };
        let credential_provider = Arc::new(StaticCredentialProvider::new(credential));

        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(&config.bucket)
            .with_credentials(credential_provider);

        // Region is required for signing - default to us-east-1 if not specified
        let region = config.region.clone().unwrap_or_else(|| "us-east-1".to_string());
        builder = builder.with_region(&region);

        if let Some(endpoint) = &config.endpoint {
            builder = builder.with_endpoint(endpoint);
        }

        if config.force_path_style {
            builder = builder.with_virtual_hosted_style_request(false);
        }

        if config.allow_http {
            builder = builder.with_allow_http(true);
        }

        builder.build().map_err(FileStorageError::ObjectStore)
    }

    /// Get the path for a data lake's schema directory
    fn data_path(&self, data_lake: &str, schema_name: &str) -> ObjectPath {
        let path_str = format!(
            "{}data-lakes/{}/{}/",
            self.base_prefix, data_lake, schema_name
        );
        ObjectPath::from(path_str)
    }

    /// Generate a filename for a new data file
    fn generate_filename(&self, format: &DataLakeFileFormat) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let uuid = uuid::Uuid::new_v4().to_string()[..8].to_string();
        format!("{}_{}.{}", timestamp, uuid, format.extension())
    }

    /// Write records to a file in the specified format
    pub async fn write_records(
        &self,
        data_lake: &str,
        schema_name: &str,
        records: &[DataRecord],
        format: &DataLakeFileFormat,
    ) -> FileStorageResult<String> {
        if records.is_empty() {
            return Ok(String::new());
        }

        let base_path = self.data_path(data_lake, schema_name);
        let filename = self.generate_filename(format);
        let full_path = base_path.child(filename.as_str());

        let bytes = match format {
            DataLakeFileFormat::Parquet => self.records_to_parquet(records)?,
            DataLakeFileFormat::Jsonl => self.records_to_jsonl(records)?,
        };

        self.store
            .put(&full_path, PutPayload::from_bytes(bytes))
            .await?;

        Ok(full_path.to_string())
    }

    /// Convert records to Parquet bytes
    fn records_to_parquet(&self, records: &[DataRecord]) -> FileStorageResult<Bytes> {
        // Create Arrow schema for data records
        let schema = Self::create_arrow_schema();

        // Create arrays for each column
        let id_array: ArrayRef =
            Arc::new(StringArray::from_iter_values(records.iter().map(|r| &r.id)));
        let data_lake_array: ArrayRef = Arc::new(StringArray::from_iter_values(
            records.iter().map(|r| &r.data_lake),
        ));
        let schema_name_array: ArrayRef = Arc::new(StringArray::from_iter_values(
            records.iter().map(|r| &r.schema_name),
        ));
        let data_array: ArrayRef = Arc::new(StringArray::from_iter_values(
            records.iter().map(|r| r.data.to_string()),
        ));
        let created_at_array: ArrayRef = Arc::new(StringArray::from_iter_values(
            records.iter().map(|r| &r.created_at),
        ));
        let updated_at_array: ArrayRef = Arc::new(StringArray::from_iter_values(
            records.iter().map(|r| &r.updated_at),
        ));
        let created_by_array: ArrayRef = Arc::new(StringArray::from(
            records
                .iter()
                .map(|r| r.created_by.as_deref())
                .collect::<Vec<_>>(),
        ));
        let metadata_array: ArrayRef = Arc::new(StringArray::from(
            records
                .iter()
                .map(|r| r.metadata.as_ref().map(|m| m.to_string()))
                .collect::<Vec<_>>(),
        ));

        let batch = RecordBatch::try_new(
            Arc::new(schema),
            vec![
                id_array,
                data_lake_array,
                schema_name_array,
                data_array,
                created_at_array,
                updated_at_array,
                created_by_array,
                metadata_array,
            ],
        )?;

        // Write to Parquet
        let mut buffer = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), None)?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(Bytes::from(buffer))
    }

    /// Convert records to JSONL bytes
    fn records_to_jsonl(&self, records: &[DataRecord]) -> FileStorageResult<Bytes> {
        let mut buffer = Vec::new();
        for record in records {
            let json = serde_json::to_string(record)?;
            buffer.extend_from_slice(json.as_bytes());
            buffer.push(b'\n');
        }
        Ok(Bytes::from(buffer))
    }

    /// Create the Arrow schema for data records
    fn create_arrow_schema() -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("data_lake", DataType::Utf8, false),
            Field::new("schema_name", DataType::Utf8, false),
            Field::new("data", DataType::Utf8, false), // JSON stored as string
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("created_by", DataType::Utf8, true),
            Field::new("metadata", DataType::Utf8, true),
        ])
    }

    /// Read records from a Parquet file
    /// Note: path should be the URL-encoded path from list operations
    pub async fn read_parquet_records(
        &self,
        path: &str,
    ) -> FileStorageResult<Vec<DataRecord>> {
        // Decode the path first to avoid double-encoding
        // ObjectPath::from() will encode it, so we need the raw path
        let decoded_path = urlencoding::decode(path)
            .map_err(|e| FileStorageError::InvalidConfig(format!("Invalid path encoding: {}", e)))?;
        tracing::debug!(
            "read_parquet_records: input={}, decoded={}",
            path, decoded_path
        );
        let object_path = ObjectPath::from(decoded_path.as_ref());
        tracing::debug!("read_parquet_records: object_path={}", object_path);
        let data = self.store.get(&object_path).await?.bytes().await?;
        tracing::debug!("read_parquet_records: got {} bytes", data.len());

        let reader = ParquetRecordBatchReaderBuilder::try_new(data)?.build()?;

        let mut records = Vec::new();
        for batch_result in reader {
            let batch = batch_result?;
            records.extend(Self::batch_to_records(&batch)?);
        }

        Ok(records)
    }

    /// Convert a RecordBatch to DataRecords
    fn batch_to_records(batch: &RecordBatch) -> FileStorageResult<Vec<DataRecord>> {
        let id_col = batch
            .column_by_name("id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| FileStorageError::InvalidConfig("Missing id column".into()))?;

        let data_lake_col = batch
            .column_by_name("data_lake")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| FileStorageError::InvalidConfig("Missing data_lake column".into()))?;

        let schema_name_col = batch
            .column_by_name("schema_name")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| FileStorageError::InvalidConfig("Missing schema_name column".into()))?;

        let data_col = batch
            .column_by_name("data")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| FileStorageError::InvalidConfig("Missing data column".into()))?;

        let created_at_col = batch
            .column_by_name("created_at")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| FileStorageError::InvalidConfig("Missing created_at column".into()))?;

        let updated_at_col = batch
            .column_by_name("updated_at")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| FileStorageError::InvalidConfig("Missing updated_at column".into()))?;

        let created_by_col = batch
            .column_by_name("created_by")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let metadata_col = batch
            .column_by_name("metadata")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let mut records = Vec::with_capacity(batch.num_rows());
        for i in 0..batch.num_rows() {
            let data_str = data_col.value(i);
            let data: Value = serde_json::from_str(data_str)?;

            let created_by = created_by_col.and_then(|c| {
                if c.is_null(i) {
                    None
                } else {
                    Some(c.value(i).to_string())
                }
            });

            let metadata = metadata_col.and_then(|c| {
                if c.is_null(i) {
                    None
                } else {
                    serde_json::from_str(c.value(i)).ok()
                }
            });

            records.push(DataRecord {
                id: id_col.value(i).to_string(),
                data_lake: data_lake_col.value(i).to_string(),
                schema_name: schema_name_col.value(i).to_string(),
                data,
                created_at: created_at_col.value(i).to_string(),
                updated_at: updated_at_col.value(i).to_string(),
                created_by,
                metadata,
            });
        }

        Ok(records)
    }

    /// Read records from a JSONL file
    /// Note: path should be the URL-encoded path from list operations
    pub async fn read_jsonl_records(&self, path: &str) -> FileStorageResult<Vec<DataRecord>> {
        // Decode the path first to avoid double-encoding
        let decoded_path = urlencoding::decode(path)
            .map_err(|e| FileStorageError::InvalidConfig(format!("Invalid path encoding: {}", e)))?;
        let object_path = ObjectPath::from(decoded_path.as_ref());
        let data = self.store.get(&object_path).await?.bytes().await?;

        let content = String::from_utf8_lossy(&data);
        let mut records = Vec::new();

        for line in content.lines() {
            if !line.trim().is_empty() {
                let record: DataRecord = serde_json::from_str(line)?;
                records.push(record);
            }
        }

        Ok(records)
    }

    /// List all data files for a data lake schema
    pub async fn list_files(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<Vec<FileInfo>> {
        let base_path = self.data_path(data_lake, schema_name);
        tracing::debug!(
            "list_files: data_lake={}, schema={}, base_path={}",
            data_lake, schema_name, base_path
        );

        let mut files = Vec::new();
        let mut stream = self.store.list(Some(&base_path));

        use futures::StreamExt;
        while let Some(result) = stream.next().await {
            let meta = result?;
            let path_str = meta.location.to_string();

            // Skip tombstone files - they are soft-delete markers, not data
            if path_str.contains("_tombstones") {
                continue;
            }

            let format = if path_str.ends_with(".parquet") {
                Some(DataLakeFileFormat::Parquet)
            } else if path_str.ends_with(".jsonl") {
                Some(DataLakeFileFormat::Jsonl)
            } else {
                None
            };

            if let Some(format) = format {
                files.push(FileInfo {
                    path: path_str,
                    size_bytes: meta.size,
                    last_modified: meta.last_modified.to_rfc3339(),
                    format,
                });
            }
        }

        Ok(files)
    }

    /// Read all records from all files for a data lake schema
    pub async fn read_all_records(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<Vec<DataRecord>> {
        let files = self.list_files(data_lake, schema_name).await?;
        tracing::debug!(
            "read_all_records: data_lake={}, schema={}, found {} files",
            data_lake, schema_name, files.len()
        );
        let mut all_records = Vec::new();

        for file in &files {
            tracing::debug!("Reading file: {} (format: {:?})", file.path, file.format);
            let records = match file.format {
                DataLakeFileFormat::Parquet => self.read_parquet_records(&file.path).await?,
                DataLakeFileFormat::Jsonl => self.read_jsonl_records(&file.path).await?,
            };
            tracing::debug!("Read {} records from {}", records.len(), file.path);
            all_records.extend(records);
        }

        tracing::debug!("read_all_records: returning {} total records", all_records.len());
        Ok(all_records)
    }

    /// Delete a specific file
    /// Note: path should be the URL-encoded path from list operations
    pub async fn delete_file(&self, path: &str) -> FileStorageResult<()> {
        // Decode the path first to avoid double-encoding
        let decoded_path = urlencoding::decode(path)
            .map_err(|e| FileStorageError::InvalidConfig(format!("Invalid path encoding: {}", e)))?;
        let object_path = ObjectPath::from(decoded_path.as_ref());
        self.store.delete(&object_path).await?;
        Ok(())
    }

    /// Delete all files for a data lake schema
    pub async fn delete_schema_files(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<usize> {
        let files = self.list_files(data_lake, schema_name).await?;
        let mut deleted_count = 0;

        for file in files {
            if let Err(e) = self.delete_file(&file.path).await {
                tracing::warn!("Failed to delete file {}: {}", file.path, e);
            } else {
                deleted_count += 1;
            }
        }

        // Also delete tombstone files if they exist
        let tombstone_path = self.tombstone_path(data_lake, schema_name);
        let mut stream = self.store.list(Some(&tombstone_path));

        use futures::StreamExt;
        while let Some(result) = stream.next().await {
            if let Ok(meta) = result {
                if let Err(e) = self.store.delete(&meta.location).await {
                    tracing::warn!("Failed to delete tombstone file {:?}: {}", meta.location, e);
                }
            }
        }

        Ok(deleted_count)
    }

    /// Get the underlying object store for DataFusion registration
    pub fn object_store(&self) -> Arc<dyn ObjectStore> {
        Arc::clone(&self.store)
    }

    /// Check if using S3 backend
    pub fn is_s3(&self) -> bool {
        self.is_s3
    }

    /// Get the base prefix for paths
    pub fn base_prefix(&self) -> &str {
        &self.base_prefix
    }

    /// Get the default file format
    pub fn default_format(&self) -> &DataLakeFileFormat {
        &self.config.default_format
    }

    /// Get the batch size configuration
    pub fn batch_size(&self) -> usize {
        self.config.batch_size
    }

    /// Get the S3 bucket name if using S3
    pub fn s3_bucket(&self) -> Option<&str> {
        self.config.s3.as_ref().map(|s| s.bucket.as_str())
    }

    // ========================================================================
    // Tombstone Operations (Soft Delete / Update Tracking)
    // ========================================================================

    /// Get the path for tombstone files
    fn tombstone_path(&self, data_lake: &str, schema_name: &str) -> ObjectPath {
        let path_str = format!(
            "{}data-lakes/{}/_tombstones/{}/",
            self.base_prefix, data_lake, schema_name
        );
        ObjectPath::from(path_str)
    }

    /// Write a tombstone record
    pub async fn write_tombstone(&self, tombstone: &RecordTombstone) -> FileStorageResult<String> {
        let base_path = self.tombstone_path(&tombstone.data_lake, &tombstone.schema_name);
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let uuid = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let filename = format!("{}_{}.jsonl", timestamp, uuid);
        let full_path = base_path.child(filename.as_str());

        let json = serde_json::to_string(tombstone)?;
        let bytes = Bytes::from(format!("{}\n", json));

        self.store
            .put(&full_path, PutPayload::from_bytes(bytes))
            .await?;

        Ok(full_path.to_string())
    }

    /// Read all tombstones for a data lake schema
    pub async fn read_tombstones(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<Vec<RecordTombstone>> {
        let base_path = self.tombstone_path(data_lake, schema_name);
        let mut tombstones = Vec::new();

        use futures::StreamExt;
        let mut stream = self.store.list(Some(&base_path));

        while let Some(result) = stream.next().await {
            match result {
                Ok(meta) => {
                    let path_str = meta.location.to_string();
                    if path_str.ends_with(".jsonl") {
                        match self.store.get(&meta.location).await {
                            Ok(get_result) => {
                                match get_result.bytes().await {
                                    Ok(data) => {
                                        let content = String::from_utf8_lossy(&data);
                                        for line in content.lines() {
                                            if !line.trim().is_empty() {
                                                if let Ok(tombstone) = serde_json::from_str::<RecordTombstone>(line) {
                                                    tombstones.push(tombstone);
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => continue,
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(tombstones)
    }

    /// Get set of deleted/superseded record IDs for a schema
    pub async fn get_deleted_record_ids(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<std::collections::HashSet<String>> {
        let tombstones = self.read_tombstones(data_lake, schema_name).await?;
        let deleted_ids: std::collections::HashSet<String> = tombstones
            .into_iter()
            .map(|t| t.record_id)
            .collect();
        Ok(deleted_ids)
    }

    /// Read all active (non-deleted) records from all files for a data lake schema
    pub async fn read_active_records(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<Vec<DataRecord>> {
        let all_records = self.read_all_records(data_lake, schema_name).await?;
        let deleted_ids = self.get_deleted_record_ids(data_lake, schema_name).await?;

        let active_records: Vec<DataRecord> = all_records
            .into_iter()
            .filter(|r| !deleted_ids.contains(&r.id))
            .collect();

        Ok(active_records)
    }

    /// Soft delete a record by writing a tombstone
    pub async fn soft_delete_record(
        &self,
        data_lake: &str,
        schema_name: &str,
        record_id: &str,
    ) -> FileStorageResult<()> {
        // Verify record exists
        let records = self.read_active_records(data_lake, schema_name).await?;
        if !records.iter().any(|r| r.id == record_id) {
            return Err(FileStorageError::InvalidConfig(format!(
                "Record {} not found in schema {}",
                record_id, schema_name
            )));
        }

        let tombstone = RecordTombstone::delete(
            record_id.to_string(),
            data_lake.to_string(),
            schema_name.to_string(),
        );
        self.write_tombstone(&tombstone).await?;
        Ok(())
    }

    /// Update a record by writing the new version and a tombstone for the old one
    pub async fn update_record(
        &self,
        data_lake: &str,
        schema_name: &str,
        record_id: &str,
        mut updated_record: DataRecord,
        format: &DataLakeFileFormat,
    ) -> FileStorageResult<DataRecord> {
        // Verify the original record exists
        let records = self.read_active_records(data_lake, schema_name).await?;
        let original = records.iter().find(|r| r.id == record_id);

        if original.is_none() {
            return Err(FileStorageError::InvalidConfig(format!(
                "Record {} not found in schema {}",
                record_id, schema_name
            )));
        }

        // Generate a new ID for the updated record
        let new_id = uuid::Uuid::new_v4().to_string();
        updated_record.id = new_id.clone();
        updated_record.updated_at = chrono::Utc::now().to_rfc3339();

        // Write the updated record
        self.write_records(data_lake, schema_name, &[updated_record.clone()], format).await?;

        // Write tombstone for the old record
        let tombstone = RecordTombstone::update(
            record_id.to_string(),
            data_lake.to_string(),
            schema_name.to_string(),
            new_id,
        );
        self.write_tombstone(&tombstone).await?;

        Ok(updated_record)
    }

    /// Find a specific active record by ID
    pub async fn find_record(
        &self,
        data_lake: &str,
        schema_name: &str,
        record_id: &str,
    ) -> FileStorageResult<Option<DataRecord>> {
        let records = self.read_active_records(data_lake, schema_name).await?;
        Ok(records.into_iter().find(|r| r.id == record_id))
    }

    /// Count active (non-deleted) records
    pub async fn count_active_records(
        &self,
        data_lake: &str,
        schema_name: &str,
    ) -> FileStorageResult<usize> {
        let records = self.read_active_records(data_lake, schema_name).await?;
        Ok(records.len())
    }
}

/// Information about a data file
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size_bytes: usize,
    pub last_modified: String,
    pub format: DataLakeFileFormat,
}

// ============================================================================
// Soft Delete / Update Tracking (Tombstones)
// ============================================================================

/// Operation type for tombstone records
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TombstoneOperation {
    /// Record was soft-deleted
    Delete,
    /// Record was updated (superseded by new version)
    Update,
}

/// A tombstone record marking a record as deleted or superseded
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecordTombstone {
    /// The ID of the affected record
    pub record_id: String,
    /// The data lake name
    pub data_lake: String,
    /// The schema name
    pub schema_name: String,
    /// The operation (delete or update)
    pub operation: TombstoneOperation,
    /// Timestamp of the operation
    pub timestamp: String,
    /// For updates: the new record ID that supersedes this one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
}

impl RecordTombstone {
    /// Create a new delete tombstone
    pub fn delete(record_id: String, data_lake: String, schema_name: String) -> Self {
        Self {
            record_id,
            data_lake,
            schema_name,
            operation: TombstoneOperation::Delete,
            timestamp: chrono::Utc::now().to_rfc3339(),
            superseded_by: None,
        }
    }

    /// Create a new update tombstone
    pub fn update(record_id: String, data_lake: String, schema_name: String, new_record_id: String) -> Self {
        Self {
            record_id,
            data_lake,
            schema_name,
            operation: TombstoneOperation::Update,
            timestamp: chrono::Utc::now().to_rfc3339(),
            superseded_by: Some(new_record_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arrow_schema_creation() {
        let schema = FileStorageHandler::create_arrow_schema();
        assert_eq!(schema.fields().len(), 8);
        assert!(schema.field_with_name("id").is_ok());
        assert!(schema.field_with_name("data").is_ok());
        assert!(schema.field_with_name("data_lake").is_ok());
    }

    #[test]
    fn test_generate_filename() {
        // Test that we can create the file storage config types
        let format = DataLakeFileFormat::Parquet;
        assert_eq!(format.extension(), "parquet");

        let format = DataLakeFileFormat::Jsonl;
        assert_eq!(format.extension(), "jsonl");
    }

    #[test]
    fn test_tombstone_delete_creation() {
        let tombstone = RecordTombstone::delete(
            "record-123".to_string(),
            "test-lake".to_string(),
            "test-schema".to_string(),
        );

        assert_eq!(tombstone.record_id, "record-123");
        assert_eq!(tombstone.data_lake, "test-lake");
        assert_eq!(tombstone.schema_name, "test-schema");
        assert_eq!(tombstone.operation, TombstoneOperation::Delete);
        assert!(tombstone.superseded_by.is_none());
    }

    #[test]
    fn test_tombstone_update_creation() {
        let tombstone = RecordTombstone::update(
            "record-123".to_string(),
            "test-lake".to_string(),
            "test-schema".to_string(),
            "record-456".to_string(),
        );

        assert_eq!(tombstone.record_id, "record-123");
        assert_eq!(tombstone.data_lake, "test-lake");
        assert_eq!(tombstone.schema_name, "test-schema");
        assert_eq!(tombstone.operation, TombstoneOperation::Update);
        assert_eq!(tombstone.superseded_by, Some("record-456".to_string()));
    }

    #[test]
    fn test_tombstone_serialization() {
        let tombstone = RecordTombstone::delete(
            "record-123".to_string(),
            "test-lake".to_string(),
            "test-schema".to_string(),
        );

        let json = serde_json::to_string(&tombstone).unwrap();
        assert!(json.contains("\"record_id\":\"record-123\""));
        assert!(json.contains("\"operation\":\"delete\""));

        let deserialized: RecordTombstone = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.record_id, tombstone.record_id);
        assert_eq!(deserialized.operation, tombstone.operation);
    }
}
