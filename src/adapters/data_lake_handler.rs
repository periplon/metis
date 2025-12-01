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

use crate::adapters::api_handler::ApiState;
use crate::config::{DataLakeConfig, DataLakeSchemaRef, DataRecord};
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
        data_lake.schemas = dto.schemas.iter().map(|s| DataLakeSchemaRef::from(s.clone())).collect();
        data_lake.metadata = dto.metadata.clone();
        drop(settings);

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
    if let Some(store) = &state.data_store {
        match store.records().list(&name, query.schema.as_deref(), query.limit, query.offset).await {
            Ok(records) => {
                let dtos: Vec<DataRecordDto> = records.into_iter().map(DataRecordDto::from).collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<DataRecordDto>>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<Vec<DataRecordDto>>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// GET /api/data-lakes/:name/records/:id - Get a single record
pub async fn get_record(
    State(state): State<ApiState>,
    Path((name, id)): Path<(String, String)>,
) -> impl IntoResponse {
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
    if let Some(store) = &state.data_store {
        // Verify data lake exists
        match store.archetypes().get(ArchetypeType::DataLake.as_str(), &name).await {
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<DataRecordDto>::error("Data lake not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<DataRecordDto>::error(e.to_string())),
                );
            }
            Ok(Some(_)) => {}
        }

        let record = DataRecord::new(name, req.schema_name, req.data)
            .with_source("manual");
        let record = DataRecord {
            metadata: req.metadata,
            ..record
        };

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
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<DataRecordDto>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// PUT /api/data-lakes/:name/records/:id - Update a record
pub async fn update_record(
    State(state): State<ApiState>,
    Path((name, id)): Path<(String, String)>,
    Json(req): Json<UpdateRecordRequest>,
) -> impl IntoResponse {
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

/// GET /api/data-lakes/:name/records/count - Get record count
pub async fn count_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Query(query): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.records().count(&name, query.schema.as_deref()).await {
            Ok(count) => {
                return (StatusCode::OK, Json(ApiResponse::success(count)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<usize>::error(e.to_string())),
                );
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<usize>::error(
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
    if let Some(store) = &state.data_store {
        // Verify data lake exists and has the requested schema
        let data_lake: DataLakeConfig = match store.archetypes().get(ArchetypeType::DataLake.as_str(), &name).await {
            Ok(Some(v)) => match serde_json::from_value(v) {
                Ok(dl) => dl,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<Vec<DataRecordDto>>::error(format!("Failed to parse data lake: {}", e))),
                    );
                }
            },
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<Vec<DataRecordDto>>::error("Data lake not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<DataRecordDto>>::error(e.to_string())),
                );
            }
        };

        // Verify the schema is part of this data lake
        if !data_lake.schemas.iter().any(|s| s.schema_name == req.schema_name || s.alias.as_ref() == Some(&req.schema_name)) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<Vec<DataRecordDto>>::error(format!("Schema '{}' not found in data lake", req.schema_name))),
            );
        }

        // For now, generate simple placeholder data
        // TODO: Integrate with MockStrategyHandler for real generation
        let mut records = Vec::new();
        let source = format!("mock:{}", req.strategy);

        for _ in 0..req.count {
            let data = req.strategy_config.clone().unwrap_or(serde_json::json!({}));
            let record = DataRecord::new(name.clone(), req.schema_name.clone(), data)
                .with_source(&source);

            match store.records().create(&record).await {
                Ok(created) => {
                    records.push(DataRecordDto::from(created));
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<Vec<DataRecordDto>>::error(e.to_string())),
                    );
                }
            }
        }

        return (StatusCode::CREATED, Json(ApiResponse::success(records)));
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<Vec<DataRecordDto>>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}

/// DELETE /api/data-lakes/:name/records - Delete all records (with optional schema filter)
pub async fn delete_all_records(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Query(query): Query<ListRecordsQuery>,
) -> impl IntoResponse {
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

        return (StatusCode::OK, Json(ApiResponse::success(count)));
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiResponse::<usize>::error(
            "Database not configured. Data records require database persistence.",
        )),
    )
}
