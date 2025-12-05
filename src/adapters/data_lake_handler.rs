//! REST API handlers for Data Lake management
//!
//! Provides CRUD endpoints for data lakes and their records.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adapters::api_handler::{ApiState, sync_item_to_s3_if_active, delete_item_from_s3_if_active};
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::config::{DataLakeConfig, DataLakeSchemaRef, DataRecord, FakerSchemaConfig};
use crate::persistence::models::ArchetypeType;
use crate::persistence::repository::ArchetypeRepository;
use crate::persistence::DataRecordRepository;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }

    pub fn ok() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
        }
    }
}

// ============================================================================
// DTOs
// ============================================================================

/// Data Lake DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLakeDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub schemas: Vec<DataLakeSchemaRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Storage mode: database, file, or both
    #[serde(default)]
    pub storage_mode: crate::config::DataLakeStorageMode,
    /// File format: parquet or jsonl
    #[serde(default)]
    pub file_format: crate::config::DataLakeFileFormat,
    /// Enable SQL queries via DataFusion
    #[serde(default)]
    pub enable_sql_queries: bool,
}

/// Schema reference DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLakeSchemaRefDto {
    pub schema_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

impl From<&DataLakeConfig> for DataLakeDto {
    fn from(config: &DataLakeConfig) -> Self {
        Self {
            name: config.name.clone(),
            description: config.description.clone(),
            tags: config.tags.clone(),
            schemas: config.schemas.iter().map(DataLakeSchemaRefDto::from).collect(),
            metadata: config.metadata.clone(),
            storage_mode: config.storage_mode.clone(),
            file_format: config.file_format.clone(),
            enable_sql_queries: config.enable_sql_queries,
        }
    }
}

impl From<&DataLakeSchemaRef> for DataLakeSchemaRefDto {
    fn from(s: &DataLakeSchemaRef) -> Self {
        Self {
            schema_name: s.schema_name.clone(),
            schema_version: s.schema_version.clone(),
            alias: s.alias.clone(),
        }
    }
}

impl From<DataLakeDto> for DataLakeConfig {
    fn from(dto: DataLakeDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            schemas: dto.schemas.into_iter().map(DataLakeSchemaRef::from).collect(),
            metadata: dto.metadata,
            storage_mode: dto.storage_mode,
            file_format: dto.file_format,
            enable_sql_queries: dto.enable_sql_queries,
        }
    }
}

impl From<DataLakeSchemaRefDto> for DataLakeSchemaRef {
    fn from(dto: DataLakeSchemaRefDto) -> Self {
        Self {
            schema_name: dto.schema_name,
            schema_version: dto.schema_version,
            alias: dto.alias,
        }
    }
}

/// Data Record DTO for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecordDto {
    pub id: String,
    pub data_lake: String,
    pub schema_name: String,
    pub data: Value,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Response for paginated record listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRecordsResponse {
    pub records: Vec<DataRecordDto>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

impl From<&DataRecord> for DataRecordDto {
    fn from(record: &DataRecord) -> Self {
        Self {
            id: record.id.clone(),
            data_lake: record.data_lake.clone(),
            schema_name: record.schema_name.clone(),
            data: record.data.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
            created_by: record.created_by.clone(),
            metadata: record.metadata.clone(),
        }
    }
}

impl From<DataRecord> for DataRecordDto {
    fn from(record: DataRecord) -> Self {
        Self {
            id: record.id,
            data_lake: record.data_lake,
            schema_name: record.schema_name,
            data: record.data,
            created_at: record.created_at,
            updated_at: record.updated_at,
            created_by: record.created_by,
            metadata: record.metadata,
        }
    }
}

/// Request to create a new record
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRecordRequest {
    pub schema_name: String,
    pub data: Value,
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// Request to update a record
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRecordRequest {
    pub data: Value,
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// Request to generate records
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateRecordsRequest {
    pub schema_name: String,
    pub count: usize,
    pub strategy: String,
    #[serde(default)]
    pub strategy_config: Option<Value>,
}

/// Response for generate records
#[derive(Debug, Clone, Serialize)]
pub struct GenerateRecordsResponse {
    pub generated: usize,
    pub records: Vec<DataRecordDto>,
}

/// Response for count records
#[derive(Debug, Clone, Serialize)]
pub struct CountRecordsResponse {
    pub count: usize,
}

/// Request for bulk delete records
#[derive(Debug, Clone, Deserialize)]
pub struct BulkDeleteRecordsRequest {
    pub ids: Vec<String>,
}

/// Response for bulk delete records
#[derive(Debug, Clone, Serialize)]
pub struct BulkDeleteRecordsResponse {
    pub deleted: usize,
    pub failed: usize,
    pub errors: Vec<String>,
}

/// Query params for listing records
#[derive(Debug, Clone, Deserialize)]
pub struct ListRecordsQuery {
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    100
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get data lake configuration from database or in-memory settings
/// Checks database first if available, then falls back to in-memory settings
async fn get_data_lake_config(state: &ApiState, name: &str) -> Option<DataLakeDto> {
    // Try database first if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::DataLake.as_str(), name).await {
            Ok(Some(data_lake)) => {
                match serde_json::from_value::<DataLakeDto>(data_lake.clone()) {
                    Ok(dto) => {
                        tracing::debug!(
                            "Retrieved data lake '{}' from database with storage_mode: {:?}",
                            name, dto.storage_mode
                        );
                        return Some(dto);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to deserialize data lake '{}' from database: {}. Raw JSON: {}",
                            name, e, data_lake
                        );
                        // Fall through to in-memory settings
                    }
                }
            }
            Ok(None) => {
                tracing::debug!("Data lake '{}' not found in database, checking in-memory", name);
            }
            Err(e) => {
                tracing::warn!("Error fetching data lake '{}' from database: {}", name, e);
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(config) = settings.data_lakes.iter().find(|d| d.name == name) {
        let dto = DataLakeDto::from(config);
        tracing::debug!(
            "Retrieved data lake '{}' from in-memory with storage_mode: {:?}",
            name, dto.storage_mode
        );
        Some(dto)
    } else {
        tracing::debug!("Data lake '{}' not found in database or in-memory settings", name);
        None
    }
}

// ============================================================================
// Data Lake CRUD Endpoints
// ============================================================================

/// GET /api/data-lakes - List all data lakes
pub async fn list_data_lakes(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::DataLake.as_str()).await {
            Ok(data_lakes) => {
                let dtos: Vec<DataLakeDto> = data_lakes
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<DataLakeDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let data_lakes: Vec<DataLakeDto> = settings.data_lakes.iter().map(DataLakeDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(data_lakes)))
}

/// GET /api/data-lakes/:name - Get a single data lake
pub async fn get_data_lake(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::DataLake.as_str(), &name).await {
            Ok(Some(data_lake)) => {
                match serde_json::from_value::<DataLakeDto>(data_lake) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<DataLakeDto>::error(format!("Failed to parse data lake: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataLakeDto>::error("Data lake not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<DataLakeDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(data_lake) = settings.data_lakes.iter().find(|d| d.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(DataLakeDto::from(data_lake))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<DataLakeDto>::error("Data lake not found")))
    }
}

/// POST /api/data-lakes - Create a new data lake
pub async fn create_data_lake(
    State(state): State<ApiState>,
    Json(dto): Json<DataLakeDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<DataLakeDto>::error(format!("Invalid data lake data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::DataLake.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "data_lakes", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync data lake to S3: {}", e);
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<DataLakeDto>::error("Data lake with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<DataLakeDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.data_lakes.iter().any(|d| d.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<DataLakeDto>::error("Data lake with this name already exists")),
        );
    }

    let data_lake = DataLakeConfig::from(dto.clone());
    settings.data_lakes.push(data_lake);
    drop(settings);

    // Auto-sync to S3 if configured
    if let Err(e) = sync_item_to_s3_if_active(&state, "data_lakes", &dto.name, &dto).await {
        tracing::warn!("Failed to sync data lake to S3: {}", e);
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/data-lakes/:name - Update a data lake
pub async fn update_data_lake(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<DataLakeDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<DataLakeDto>::error(format!("Invalid data lake data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::DataLake.as_str(), &name, &definition, None).await {
            Ok(_) => {
                // Auto-sync to S3 if configured
                // If name changed, delete old key first
                if dto.name != name {
                    if let Err(e) = delete_item_from_s3_if_active(&state, "data_lakes", &name).await {
                        tracing::warn!("Failed to delete old data lake from S3: {}", e);
                    }
                }
                if let Err(e) = sync_item_to_s3_if_active(&state, "data_lakes", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync data lake to S3: {}", e);
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataLakeDto>::error("Data lake not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<DataLakeDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check if name is being changed and would conflict
    if dto.name != name {
        let new_name_exists = settings.data_lakes.iter().any(|d| d.name == dto.name);
        if new_name_exists {
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::<DataLakeDto>::error("Data lake with this name already exists")),
            );
        }
    }

    if let Some(data_lake) = settings.data_lakes.iter_mut().find(|d| d.name == name) {
        data_lake.name = dto.name.clone();
        data_lake.description = dto.description.clone();
        data_lake.tags = dto.tags.clone();
        data_lake.schemas = dto.schemas.iter().map(|s| DataLakeSchemaRef::from(s.clone())).collect();
        data_lake.metadata = dto.metadata.clone();
        data_lake.storage_mode = dto.storage_mode.clone();
        data_lake.file_format = dto.file_format.clone();
        data_lake.enable_sql_queries = dto.enable_sql_queries;
        drop(settings);

        // Auto-sync to S3 if configured
        // If name changed, delete old key first
        if dto.name != name {
            if let Err(e) = delete_item_from_s3_if_active(&state, "data_lakes", &name).await {
                tracing::warn!("Failed to delete old data lake from S3: {}", e);
            }
        }
        if let Err(e) = sync_item_to_s3_if_active(&state, "data_lakes", &dto.name, &dto).await {
            tracing::warn!("Failed to sync data lake to S3: {}", e);
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<DataLakeDto>::error("Data lake not found")))
    }
}

/// DELETE /api/data-lakes/:name - Delete a data lake
pub async fn delete_data_lake(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        // First delete all records for this data lake
        if let Err(e) = store.records().delete_by_lake(&name).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Failed to delete records: {}", e))),
            );
        }

        match store.archetypes().delete(ArchetypeType::DataLake.as_str(), &name).await {
            Ok(true) => {
                // Auto-delete from S3 if configured
                if let Err(e) = delete_item_from_s3_if_active(&state, "data_lakes", &name).await {
                    tracing::warn!("Failed to delete data lake from S3: {}", e);
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Data lake not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.data_lakes.len();
    settings.data_lakes.retain(|d| d.name != name);

    if settings.data_lakes.len() < initial_len {
        drop(settings);
        // Auto-delete from S3 if configured
        if let Err(e) = delete_item_from_s3_if_active(&state, "data_lakes", &name).await {
            tracing::warn!("Failed to delete data lake from S3: {}", e);
        }
        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Data lake not found")))
    }
}

// ============================================================================
// Data Record CRUD Endpoints
// ============================================================================

/// GET /api/data-lakes/:name/records - List records for a data lake
pub async fn list_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Query(query): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<ListRecordsResponse>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;

    // For file-only storage mode, read from file storage (excluding soft-deleted records)
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            // Get all schemas or specific schema
            let schemas_to_query: Vec<String> = if let Some(schema) = &query.schema {
                vec![schema.clone()]
            } else {
                data_lake_config.schemas.iter().map(|s| s.schema_name.clone()).collect()
            };

            let mut all_records = Vec::new();
            for schema_name in schemas_to_query {
                // Use read_active_records to filter out deleted/superseded records
                match file_storage.read_active_records(&name, &schema_name).await {
                    Ok(records) => all_records.extend(records),
                    Err(e) => {
                        // Log error but continue - schema might have no files yet
                        tracing::warn!("Failed to read records for schema {}: {}", schema_name, e);
                    }
                }
            }

            // Calculate total before pagination
            let total = all_records.len();
            let offset = query.offset;
            let limit = query.limit;

            // Apply pagination
            let paginated: Vec<DataRecordDto> = all_records
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(DataRecordDto::from)
                .collect();

            let response = ListRecordsResponse {
                records: paginated,
                total,
                limit,
                offset,
            };
            return (StatusCode::OK, Json(ApiResponse::success(response)));
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<ListRecordsResponse>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // For database or both modes, use database
    if let Some(store) = &state.data_store {
        // Get total count first
        let total = match store.records().count(&name, query.schema.as_deref()).await {
            Ok(count) => count,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ListRecordsResponse>::error(format!("Failed to count records: {}", e))),
                );
            }
        };

        match store.records().list(&name, query.schema.as_deref(), query.limit, query.offset).await {
            Ok(records) => {
                let dtos: Vec<DataRecordDto> = records.into_iter().map(DataRecordDto::from).collect();
                let response = ListRecordsResponse {
                    records: dtos,
                    total,
                    limit: query.limit,
                    offset: query.offset,
                };
                return (StatusCode::OK, Json(ApiResponse::success(response)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ListRecordsResponse>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<ListRecordsResponse>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// GET /api/data-lakes/:name/records/:id - Get a single record
pub async fn get_record(
    State(state): State<ApiState>,
    Path((name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;

    // For file-only storage mode, search in file storage (using find_record to exclude deleted)
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            // Search all schemas for the record using find_record (respects tombstones)
            for schema in &data_lake_config.schemas {
                match file_storage.find_record(&name, &schema.schema_name, &id).await {
                    Ok(Some(record)) => {
                        return (StatusCode::OK, Json(ApiResponse::success(DataRecordDto::from(record))));
                    }
                    Ok(None) => continue, // Record not in this schema, try next
                    Err(_) => continue, // Schema might have no files yet
                }
            }
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Record not found")));
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<DataRecordDto>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // For database or both modes, use database
    if let Some(store) = &state.data_store {
        match store.records().get(&id).await {
            Ok(Some(record)) => {
                if record.data_lake != name {
                    return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Record not found in this data lake")));
                }
                return (StatusCode::OK, Json(ApiResponse::success(DataRecordDto::from(record))));
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Record not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<DataRecordDto>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<DataRecordDto>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// POST /api/data-lakes/:name/records - Create a new record
pub async fn create_record(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<CreateRecordRequest>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Data lake not found")));
        }
    };

    // Create the record
    let record = DataRecord::new(name.clone(), req.schema_name.clone(), req.data)
        .with_source("manual");
    let record = DataRecord {
        metadata: req.metadata,
        ..record
    };

    let storage_mode = &data_lake_config.storage_mode;
    let file_format = &data_lake_config.file_format;

    // Handle file storage if needed
    if storage_mode.uses_files() {
        if let Some(file_storage) = &state.file_storage {
            match file_storage.write_records(&name, &req.schema_name, &[record.clone()], file_format).await {
                Ok(_path) => {
                    // File write successful
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<DataRecordDto>::error(format!("Failed to write to file storage: {}", e))),
                    );
                }
            }
        } else if *storage_mode == DataLakeStorageMode::File {
            // File-only mode but file storage not configured
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<DataRecordDto>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // Handle database storage if needed
    if storage_mode.uses_database() {
        if let Some(store) = &state.data_store {
            match store.records().create(&record).await {
                Ok(created) => {
                    return (StatusCode::CREATED, Json(ApiResponse::success(DataRecordDto::from(created))));
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<DataRecordDto>::error(e.to_string())),
                    );
                }
            }
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<DataRecordDto>::error(
                    "Database not configured. This data lake requires database persistence.",
                )),
            );
        }
    }

    // For file-only mode, return success with the record
    (StatusCode::CREATED, Json(ApiResponse::success(DataRecordDto::from(record))))
}

/// PUT /api/data-lakes/:name/records/:id - Update a record
pub async fn update_record(
    State(state): State<ApiState>,
    Path((name, id)): Path<(String, String)>,
    Json(req): Json<UpdateRecordRequest>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;
    let file_format = &data_lake_config.file_format;

    // For file-only storage mode, use append-only update with tombstone
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            // Find which schema the record belongs to
            for schema in &data_lake_config.schemas {
                match file_storage.find_record(&name, &schema.schema_name, &id).await {
                    Ok(Some(existing_record)) => {
                        // Found the record, create updated version
                        let mut updated_record = existing_record.clone();
                        updated_record.data = req.data.clone();
                        if req.metadata.is_some() {
                            updated_record.metadata = req.metadata.clone();
                        }

                        match file_storage.update_record(
                            &name,
                            &schema.schema_name,
                            &id,
                            updated_record,
                            file_format,
                        ).await {
                            Ok(new_record) => {
                                return (StatusCode::OK, Json(ApiResponse::success(DataRecordDto::from(new_record))));
                            }
                            Err(e) => {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(ApiResponse::<DataRecordDto>::error(format!("Failed to update record: {}", e))),
                                );
                            }
                        }
                    }
                    Ok(None) => continue, // Record not in this schema, try next
                    Err(_) => continue,
                }
            }
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Record not found")));
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<DataRecordDto>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // For database or both modes, use database
    if let Some(store) = &state.data_store {
        // Get existing record
        match store.records().get(&id).await {
            Ok(Some(mut record)) => {
                if record.data_lake != name {
                    return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Record not found in this data lake")));
                }

                record.data = req.data;
                if req.metadata.is_some() {
                    record.metadata = req.metadata;
                }

                match store.records().update(&record).await {
                    Ok(updated) => {
                        return (StatusCode::OK, Json(ApiResponse::success(DataRecordDto::from(updated))));
                    }
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<DataRecordDto>::error(e.to_string())),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Record not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<DataRecordDto>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<DataRecordDto>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// DELETE /api/data-lakes/:name/records/:id - Delete a record
pub async fn delete_record(
    State(state): State<ApiState>,
    Path((name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;

    // For file-only storage mode, use soft delete via tombstones
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            // Find which schema the record belongs to
            for schema in &data_lake_config.schemas {
                match file_storage.find_record(&name, &schema.schema_name, &id).await {
                    Ok(Some(_record)) => {
                        // Found the record, soft delete it
                        match file_storage.soft_delete_record(&name, &schema.schema_name, &id).await {
                            Ok(()) => {
                                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
                            }
                            Err(e) => {
                                return (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(ApiResponse::<()>::error(format!("Failed to delete record: {}", e))),
                                );
                            }
                        }
                    }
                    Ok(None) => continue, // Record not in this schema, try next
                    Err(_) => continue,
                }
            }
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Record not found")));
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<()>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // For database or both modes, use database
    if let Some(store) = &state.data_store {
        // Verify record belongs to this data lake
        match store.records().get(&id).await {
            Ok(Some(record)) => {
                if record.data_lake != name {
                    return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Record not found in this data lake")));
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Record not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }

        match store.records().delete(&id).await {
            Ok(true) => {
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Record not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<()>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// POST /api/data-lakes/:name/records/bulk-delete - Delete multiple records by IDs
pub async fn bulk_delete_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<BulkDeleteRecordsRequest>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    if req.ids.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<BulkDeleteRecordsResponse>::error("No IDs provided")),
        );
    }

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<BulkDeleteRecordsResponse>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;
    let mut deleted = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();

    // For file-only storage mode, use soft delete via tombstones
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            for id in &req.ids {
                let mut found = false;
                for schema in &data_lake_config.schemas {
                    match file_storage.find_record(&name, &schema.schema_name, id).await {
                        Ok(Some(_record)) => {
                            found = true;
                            match file_storage.soft_delete_record(&name, &schema.schema_name, id).await {
                                Ok(()) => {
                                    deleted += 1;
                                }
                                Err(e) => {
                                    failed += 1;
                                    errors.push(format!("Failed to delete {}: {}", id, e));
                                }
                            }
                            break;
                        }
                        Ok(None) => continue,
                        Err(_) => continue,
                    }
                }
                if !found {
                    failed += 1;
                    errors.push(format!("Record not found: {}", id));
                }
            }

            return (
                StatusCode::OK,
                Json(ApiResponse::success(BulkDeleteRecordsResponse { deleted, failed, errors })),
            );
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<BulkDeleteRecordsResponse>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // For database or both modes, use database
    if let Some(store) = &state.data_store {
        for id in &req.ids {
            // Verify record belongs to this data lake
            match store.records().get(id).await {
                Ok(Some(record)) => {
                    if record.data_lake != name {
                        failed += 1;
                        errors.push(format!("Record {} not found in this data lake", id));
                        continue;
                    }
                    match store.records().delete(id).await {
                        Ok(true) => deleted += 1,
                        Ok(false) => {
                            failed += 1;
                            errors.push(format!("Record not found: {}", id));
                        }
                        Err(e) => {
                            failed += 1;
                            errors.push(format!("Failed to delete {}: {}", id, e));
                        }
                    }
                }
                Ok(None) => {
                    failed += 1;
                    errors.push(format!("Record not found: {}", id));
                }
                Err(e) => {
                    failed += 1;
                    errors.push(format!("Error finding {}: {}", id, e));
                }
            }
        }

        return (
            StatusCode::OK,
            Json(ApiResponse::success(BulkDeleteRecordsResponse { deleted, failed, errors })),
        );
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<BulkDeleteRecordsResponse>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// GET /api/data-lakes/:name/records/count - Get record count
pub async fn count_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Query(query): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<CountRecordsResponse>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;

    // For file-only storage mode, count from file storage
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            let schemas_to_query: Vec<String> = if let Some(schema) = &query.schema {
                vec![schema.clone()]
            } else {
                data_lake_config.schemas.iter().map(|s| s.schema_name.clone()).collect()
            };

            let mut total_count = 0usize;
            for schema_name in schemas_to_query {
                // Use count_active_records to exclude deleted/superseded records
                match file_storage.count_active_records(&name, &schema_name).await {
                    Ok(count) => total_count += count,
                    Err(_) => continue, // Schema might have no files yet
                }
            }

            return (StatusCode::OK, Json(ApiResponse::success(CountRecordsResponse { count: total_count })));
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<CountRecordsResponse>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // For database or both modes, use database
    if let Some(store) = &state.data_store {
        match store.records().count(&name, query.schema.as_deref()).await {
            Ok(count) => {
                return (StatusCode::OK, Json(ApiResponse::success(CountRecordsResponse { count })));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<CountRecordsResponse>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<CountRecordsResponse>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// POST /api/data-lakes/:name/records/generate - Generate records using mock strategy
pub async fn generate_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<GenerateRecordsRequest>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<GenerateRecordsResponse>::error("Data lake not found")));
        }
    };

    // Verify the schema is part of this data lake
    if !data_lake_config.schemas.iter().any(|s| s.schema_name == req.schema_name || s.alias.as_ref() == Some(&req.schema_name)) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<GenerateRecordsResponse>::error(format!("Schema '{}' not found in data lake", req.schema_name))),
        );
    }

    let storage_mode = &data_lake_config.storage_mode;
    let file_format = &data_lake_config.file_format;

    // Generate the records based on strategy
    let source = format!("mock:{}", req.strategy);
    let mut records = Vec::new();

    // Create mock strategy handler for faker generation
    let mock_handler = MockStrategyHandler::new(state.state_manager.clone());

    for _ in 0..req.count {
        let data = match req.strategy.as_str() {
            "static" => {
                // Static strategy: use the config directly as the data
                req.strategy_config.clone().unwrap_or(serde_json::json!({}))
            }
            "random" => {
                // Random/faker strategy: parse config as FakerSchemaConfig and generate
                if let Some(config_value) = &req.strategy_config {
                    match serde_json::from_value::<FakerSchemaConfig>(config_value.clone()) {
                        Ok(faker_config) => {
                            mock_handler.generate_from_faker_config(&faker_config)
                                .unwrap_or_else(|e| {
                                    tracing::warn!("Faker generation failed: {}, using empty object", e);
                                    serde_json::json!({})
                                })
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse faker config: {}, using empty object", e);
                            serde_json::json!({})
                        }
                    }
                } else {
                    serde_json::json!({})
                }
            }
            _ => {
                // Other strategies: use config as-is or empty
                req.strategy_config.clone().unwrap_or(serde_json::json!({}))
            }
        };

        let record = DataRecord::new(name.clone(), req.schema_name.clone(), data)
            .with_source(&source);
        records.push(record);
    }

    // Handle file storage if needed
    if storage_mode.uses_files() {
        if let Some(file_storage) = &state.file_storage {
            match file_storage.write_records(&name, &req.schema_name, &records, file_format).await {
                Ok(_path) => {
                    // File write successful
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<GenerateRecordsResponse>::error(format!("Failed to write to file storage: {}", e))),
                    );
                }
            }
        } else if *storage_mode == DataLakeStorageMode::File {
            // File-only mode but file storage not configured
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<GenerateRecordsResponse>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    // Handle database storage if needed
    if storage_mode.uses_database() {
        if let Some(store) = &state.data_store {
            let mut created_records = Vec::new();
            for record in &records {
                match store.records().create(record).await {
                    Ok(created) => {
                        created_records.push(DataRecordDto::from(created));
                    }
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<GenerateRecordsResponse>::error(e.to_string())),
                        );
                    }
                }
            }
            let response = GenerateRecordsResponse {
                generated: created_records.len(),
                records: created_records,
            };
            return (StatusCode::CREATED, Json(ApiResponse::success(response)));
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<GenerateRecordsResponse>::error(
                    "Database not configured. This data lake requires database persistence.",
                )),
            );
        }
    }

    // For file-only mode, return success with the generated records
    let record_dtos: Vec<DataRecordDto> = records.into_iter().map(DataRecordDto::from).collect();
    let response = GenerateRecordsResponse {
        generated: record_dtos.len(),
        records: record_dtos,
    };
    (StatusCode::CREATED, Json(ApiResponse::success(response)))
}

/// DELETE /api/data-lakes/:name/records - Delete all records (with optional schema filter)
pub async fn delete_all_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Query(query): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    use crate::config::DataLakeStorageMode;

    // Get data lake configuration (checks both DB and in-memory)
    let data_lake_config = match get_data_lake_config(&state, &name).await {
        Some(config) => config,
        None => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<usize>::error("Data lake not found")));
        }
    };

    let storage_mode = &data_lake_config.storage_mode;
    let mut total_deleted = 0usize;

    // Handle database storage if needed
    if storage_mode.uses_database() {
        if let Some(store) = &state.data_store {
            let count = if let Some(schema) = &query.schema {
                match store.records().delete_by_schema(&name, schema).await {
                    Ok(c) => c,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<usize>::error(e.to_string())),
                        );
                    }
                }
            } else {
                match store.records().delete_by_lake(&name).await {
                    Ok(c) => c,
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<usize>::error(e.to_string())),
                        );
                    }
                }
            };
            total_deleted = count;
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<usize>::error(
                    "Database not configured. This data lake requires database persistence.",
                )),
            );
        }
    }

    // Handle file storage if needed (for file-only mode, delete by removing/clearing files)
    if *storage_mode == DataLakeStorageMode::File {
        if let Some(file_storage) = &state.file_storage {
            // For file-only mode, we need to delete the actual files
            let schemas_to_delete: Vec<String> = if let Some(schema) = &query.schema {
                vec![schema.clone()]
            } else {
                data_lake_config.schemas.iter().map(|s| s.schema_name.clone()).collect()
            };

            for schema in schemas_to_delete {
                // Count records before deletion for reporting
                if let Ok(records) = file_storage.read_all_records(&name, &schema).await {
                    total_deleted += records.len();
                }
                // Delete the files for this schema
                if let Err(e) = file_storage.delete_schema_files(&name, &schema).await {
                    tracing::warn!("Failed to delete files for {}/{}: {}", name, schema, e);
                }
            }
        } else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<usize>::error(
                    "File storage not configured. This data lake requires file storage.",
                )),
            );
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(total_deleted)))
}

// ============================================================================
// File Storage & SQL Query Endpoints
// ============================================================================

/// Request to execute a SQL query
#[derive(Debug, Clone, Deserialize)]
pub struct SqlQueryRequest {
    /// SQL query to execute (use $table as placeholder for table name)
    pub sql: String,
    /// Schema to query (required)
    pub schema_name: String,
}

/// Response from a SQL query
#[derive(Debug, Clone, Serialize)]
pub struct SqlQueryResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
    pub total_rows: usize,
}

/// Request to sync records to file storage
#[derive(Debug, Clone, Deserialize)]
pub struct SyncRequest {
    /// Optional: sync only this schema (if omitted, sync all schemas)
    #[serde(default)]
    pub schema_name: Option<String>,
    /// Format to use for sync (overrides data lake default)
    #[serde(default)]
    pub format: Option<crate::config::DataLakeFileFormat>,
}

/// Response from sync operation
#[derive(Debug, Clone, Serialize)]
pub struct SyncResponse {
    pub files_written: usize,
    pub records_synced: usize,
    pub paths: Vec<String>,
}

/// File info response
#[derive(Debug, Clone, Serialize)]
pub struct FileInfoDto {
    pub path: String,
    pub size_bytes: usize,
    pub last_modified: String,
    pub format: String,
}

/// POST /api/data-lakes/:name/query - Execute SQL query via DataFusion
pub async fn execute_query(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<SqlQueryRequest>,
) -> impl IntoResponse {
    // Get the DataFusion handler
    let datafusion = match &state.datafusion {
        Some(df) => df,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<SqlQueryResponse>::error(
                    "SQL queries not enabled. Configure file_storage in settings.",
                )),
            );
        }
    };

    // Get data lake configuration
    let data_lake = if let Some(store) = &state.data_store {
        match store.archetypes().get(crate::persistence::models::ArchetypeType::DataLake.as_str(), &name).await {
            Ok(Some(v)) => match serde_json::from_value::<DataLakeConfig>(v) {
                Ok(dl) => dl,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<SqlQueryResponse>::error(format!("Failed to parse data lake: {}", e))),
                    );
                }
            },
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<SqlQueryResponse>::error("Data lake not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<SqlQueryResponse>::error(e.to_string())),
                );
            }
        }
    } else {
        // Fallback to in-memory settings
        let settings = state.settings.read().await;
        match settings.data_lakes.iter().find(|d| d.name == name) {
            Some(dl) => dl.clone(),
            None => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<SqlQueryResponse>::error("Data lake not found")));
            }
        }
    };

    // Check if SQL queries are enabled
    if !data_lake.enable_sql_queries {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<SqlQueryResponse>::error(
                "SQL queries not enabled for this data lake",
            )),
        );
    }

    // Register ALL data lakes for cross-lake JOINs
    // Get all data lakes that have SQL queries enabled
    let all_data_lakes: Vec<DataLakeConfig> = if let Some(store) = &state.data_store {
        match store.archetypes().list(crate::persistence::models::ArchetypeType::DataLake.as_str()).await {
            Ok(lakes) => lakes
                .into_iter()
                .filter_map(|v| serde_json::from_value::<DataLakeConfig>(v).ok())
                .filter(|dl| dl.enable_sql_queries)
                .collect(),
            Err(_) => vec![data_lake.clone()],
        }
    } else {
        let settings = state.settings.read().await;
        settings.data_lakes.iter()
            .filter(|dl| dl.enable_sql_queries)
            .cloned()
            .collect()
    };

    // Register all tables from all data lakes
    let data_lake_refs: Vec<&DataLakeConfig> = all_data_lakes.iter().collect();
    if let Err(e) = datafusion.register_multiple_data_lakes(&data_lake_refs).await {
        tracing::warn!("Failed to register some data lakes: {}", e);
    }

    // Execute the query (still register the primary table for $table replacement)
    match datafusion.query_data_lake(&data_lake, &req.schema_name, &req.sql).await {
        Ok(result) => {
            let response = SqlQueryResponse {
                columns: result.columns,
                rows: result.rows,
                total_rows: result.total_rows,
            };
            (StatusCode::OK, Json(ApiResponse::success(response)))
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<SqlQueryResponse>::error(e.to_string())),
        ),
    }
}

/// GET /api/data-lakes/:name/files - List data files for a data lake
pub async fn list_files(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Query(query): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    // Get the file storage handler
    let file_storage = match &state.file_storage {
        Some(fs) => fs,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<Vec<FileInfoDto>>::error(
                    "File storage not enabled. Configure file_storage in settings.",
                )),
            );
        }
    };

    // Get schemas to list from query or all schemas in data lake
    let schemas_to_list: Vec<String> = if let Some(schema) = &query.schema {
        vec![schema.clone()]
    } else {
        // Get data lake to find all schemas
        let data_lake = if let Some(store) = &state.data_store {
            match store.archetypes().get(crate::persistence::models::ArchetypeType::DataLake.as_str(), &name).await {
                Ok(Some(v)) => serde_json::from_value::<DataLakeConfig>(v).ok(),
                _ => None,
            }
        } else {
            let settings = state.settings.read().await;
            settings.data_lakes.iter().find(|d| d.name == name).cloned()
        };

        match data_lake {
            Some(dl) => dl.schemas.iter().map(|s| s.schema_name.clone()).collect(),
            None => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<Vec<FileInfoDto>>::error("Data lake not found")));
            }
        }
    };

    let mut all_files = Vec::new();
    for schema in schemas_to_list {
        match file_storage.list_files(&name, &schema).await {
            Ok(files) => {
                for f in files {
                    all_files.push(FileInfoDto {
                        path: f.path,
                        size_bytes: f.size_bytes,
                        last_modified: f.last_modified,
                        format: format!("{:?}", f.format).to_lowercase(),
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Failed to list files for {}/{}: {}", name, schema, e);
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(all_files)))
}

/// POST /api/data-lakes/:name/sync - Sync database records to file storage
pub async fn sync_to_files(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<SyncRequest>,
) -> impl IntoResponse {
    // Get the file storage handler
    let file_storage = match &state.file_storage {
        Some(fs) => fs,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<SyncResponse>::error(
                    "File storage not enabled. Configure file_storage in settings.",
                )),
            );
        }
    };

    // Get data store
    let store = match &state.data_store {
        Some(s) => s,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiResponse::<SyncResponse>::error(
                    "Database not configured. Sync requires database persistence.",
                )),
            );
        }
    };

    // Get data lake configuration
    let data_lake = match store.archetypes().get(crate::persistence::models::ArchetypeType::DataLake.as_str(), &name).await {
        Ok(Some(v)) => match serde_json::from_value::<DataLakeConfig>(v) {
            Ok(dl) => dl,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<SyncResponse>::error(format!("Failed to parse data lake: {}", e))),
                );
            }
        },
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(ApiResponse::<SyncResponse>::error("Data lake not found")));
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<SyncResponse>::error(e.to_string())),
            );
        }
    };

    // Determine format to use
    let format = req.format.unwrap_or_else(|| data_lake.file_format.clone());

    // Get schemas to sync
    let schemas_to_sync: Vec<String> = if let Some(schema) = req.schema_name {
        vec![schema]
    } else {
        data_lake.schemas.iter().map(|s| s.schema_name.clone()).collect()
    };

    let mut total_records = 0;
    let mut total_files = 0;
    let mut paths = Vec::new();

    for schema in schemas_to_sync {
        // Get all records for this schema
        match store.records().list(&name, Some(&schema), usize::MAX, 0).await {
            Ok(records) => {
                if records.is_empty() {
                    continue;
                }

                // Write records to file
                match file_storage.write_records(&name, &schema, &records, &format).await {
                    Ok(path) => {
                        if !path.is_empty() {
                            total_files += 1;
                            total_records += records.len();
                            paths.push(path);
                        }
                    }
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<SyncResponse>::error(format!(
                                "Failed to write records for schema {}: {}",
                                schema, e
                            ))),
                        );
                    }
                }
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<SyncResponse>::error(format!(
                        "Failed to read records for schema {}: {}",
                        schema, e
                    ))),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(SyncResponse {
            files_written: total_files,
            records_synced: total_records,
            paths,
        })),
    )
}

// ============================================================================
// Schema Info Endpoint (for DataLakeCrud UI)
// ============================================================================

/// Response for schema info endpoint
#[derive(Debug, Clone, Serialize)]
pub struct SchemaInfoResponse {
    pub data_lake: String,
    pub schema_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_definition: Option<Value>,
    pub record_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_record: Option<DataRecordDto>,
}

/// GET /api/data-lakes/:name/schema-info/:schema_name - Get schema information for DataLakeCrud UI
pub async fn get_schema_info(
    State(state): State<ApiState>,
    Path((name, schema_name)): Path<(String, String)>,
) -> impl IntoResponse {
    // Get data lake config
    let data_lake_config: Option<DataLakeConfig> = if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::DataLake.as_str(), &name).await {
            Ok(Some(v)) => serde_json::from_value(v).ok(),
            _ => None,
        }
    } else {
        let settings = state.settings.read().await;
        settings.data_lakes.iter().find(|d| d.name == name).cloned()
    };

    let data_lake_config = match data_lake_config {
        Some(config) => config,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<SchemaInfoResponse>::error("Data lake not found")),
            );
        }
    };

    // Verify schema exists in data lake
    let schema_ref = data_lake_config.schemas.iter().find(|s|
        s.schema_name == schema_name ||
        s.alias.as_ref() == Some(&schema_name)
    );

    if schema_ref.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<SchemaInfoResponse>::error("Schema not found in data lake")),
        );
    }

    // Get actual schema name (handle alias)
    let actual_schema = schema_ref
        .map(|s| s.schema_name.clone())
        .unwrap_or(schema_name.clone());

    // Try to fetch schema definition from schemas API
    let schema_definition: Option<Value> = if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Schema.as_str(), &actual_schema).await {
            Ok(Some(schema_value)) => {
                // The schema document has a "schema" field containing the JSON Schema
                schema_value.get("schema").cloned()
            }
            _ => None
        }
    } else {
        // Fallback to in-memory settings
        let settings = state.settings.read().await;
        settings.schemas.iter()
            .find(|s| s.name == actual_schema)
            .map(|s| s.schema.clone())
    };

    // Get record count and sample
    let (record_count, sample_record) = if let Some(file_storage) = &state.file_storage {
        let records = file_storage.read_active_records(&name, &actual_schema).await.unwrap_or_default();
        let count = records.len();
        let sample = records.into_iter().next().map(DataRecordDto::from);
        (count, sample)
    } else if let Some(store) = &state.data_store {
        let count = store.records().count(&name, Some(&actual_schema)).await.unwrap_or(0);
        let sample = store.records().list(&name, Some(&actual_schema), 1, 0).await
            .ok()
            .and_then(|records| records.into_iter().next())
            .map(DataRecordDto::from);
        (count, sample)
    } else {
        (0, None)
    };

    let response = SchemaInfoResponse {
        data_lake: name,
        schema_name: actual_schema,
        schema_definition,
        record_count,
        sample_record,
    };

    (StatusCode::OK, Json(ApiResponse::success(response)))
}
