//! Data Lake configuration types.
//!
//! A Data Lake is a collection of schema references (a data model) plus data records
//! that conform to those schemas. Data Lakes enable structured data management with
//! automatic form generation from JSON Schemas and integration with mock strategies
//! for data generation.
//!
//! ## Example Data Lake Definition
//!
//! ```yaml
//! name: customer_data
//! description: Customer information data lake
//! schemas:
//!   - schema_name: UserInput
//!     alias: users
//!   - schema_name: Address
//!     schema_version: "1.0"
//!     alias: addresses
//! ```
//!
//! ## Using Data Lakes
//!
//! Data Lakes reference existing schemas defined in `/api/schemas`. Each schema
//! reference can optionally pin to a specific version and provide an alias for
//! use within the data lake context.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::MockStrategyType;

/// Configuration for a Data Lake (data model + records)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLakeConfig {
    /// Unique name for this data lake
    pub name: String,
    /// Human-readable description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Schema references that define the data model
    #[serde(default)]
    pub schemas: Vec<DataLakeSchemaRef>,
    /// Optional metadata for this data lake
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Reference to a schema within a data lake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLakeSchemaRef {
    /// Name of the schema (references /api/schemas)
    pub schema_name: String,
    /// Pin to a specific schema version (None = use latest)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    /// Friendly alias for this schema within the data lake
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// A data record stored in a data lake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    /// Unique identifier (UUID)
    pub id: String,
    /// Data lake this record belongs to
    pub data_lake: String,
    /// Schema name this record conforms to
    pub schema_name: String,
    /// The actual data (JSON conforming to schema)
    pub data: Value,
    /// Creation timestamp (ISO8601)
    pub created_at: String,
    /// Last update timestamp (ISO8601)
    pub updated_at: String,
    /// Source of the record: "manual", "mock:template", "mock:llm", etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Optional metadata for this record
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Request to generate records using a mock strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRecordsRequest {
    /// Schema to generate records for
    pub schema_name: String,
    /// Number of records to generate
    pub count: usize,
    /// Mock strategy to use for generation
    pub strategy: MockStrategyType,
    /// Strategy-specific configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_config: Option<Value>,
}

/// Result of record validation against a schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the data is valid
    pub valid: bool,
    /// List of validation errors (empty if valid)
    pub errors: Vec<ValidationError>,
}

/// A single validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// JSON path to the invalid field (e.g., "users[0].email")
    pub path: String,
    /// Human-readable error message
    pub message: String,
}

impl DataLakeConfig {
    /// Create a new data lake with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            tags: Vec::new(),
            schemas: Vec::new(),
            metadata: None,
        }
    }

    /// Add a schema reference to this data lake
    pub fn with_schema(mut self, schema_name: impl Into<String>) -> Self {
        self.schemas.push(DataLakeSchemaRef {
            schema_name: schema_name.into(),
            schema_version: None,
            alias: None,
        });
        self
    }

    /// Add a schema reference with an alias
    pub fn with_schema_alias(
        mut self,
        schema_name: impl Into<String>,
        alias: impl Into<String>,
    ) -> Self {
        self.schemas.push(DataLakeSchemaRef {
            schema_name: schema_name.into(),
            schema_version: None,
            alias: Some(alias.into()),
        });
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get a schema reference by name or alias
    pub fn get_schema(&self, name_or_alias: &str) -> Option<&DataLakeSchemaRef> {
        self.schemas.iter().find(|s| {
            s.schema_name == name_or_alias
                || s.alias.as_ref().map(|a| a == name_or_alias).unwrap_or(false)
        })
    }
}

impl DataRecord {
    /// Create a new data record
    pub fn new(
        data_lake: impl Into<String>,
        schema_name: impl Into<String>,
        data: Value,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            data_lake: data_lake.into(),
            schema_name: schema_name.into(),
            data,
            created_at: now.clone(),
            updated_at: now,
            created_by: None,
            metadata: None,
        }
    }

    /// Create a record with a specific source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.created_by = Some(source.into());
        self
    }
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result with errors
    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, path: impl Into<String>, message: impl Into<String>) {
        self.valid = false;
        self.errors.push(ValidationError {
            path: path.into(),
            message: message.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_lake_builder() {
        let lake = DataLakeConfig::new("test_lake")
            .with_description("Test data lake")
            .with_schema("UserInput")
            .with_schema_alias("Address", "user_addresses");

        assert_eq!(lake.name, "test_lake");
        assert_eq!(lake.description, Some("Test data lake".to_string()));
        assert_eq!(lake.schemas.len(), 2);
        assert_eq!(lake.schemas[0].schema_name, "UserInput");
        assert_eq!(lake.schemas[1].alias, Some("user_addresses".to_string()));
    }

    #[test]
    fn test_get_schema_by_name() {
        let lake = DataLakeConfig::new("test")
            .with_schema("UserInput")
            .with_schema_alias("Address", "addr");

        assert!(lake.get_schema("UserInput").is_some());
        assert!(lake.get_schema("Address").is_some());
        assert!(lake.get_schema("addr").is_some());
        assert!(lake.get_schema("NonExistent").is_none());
    }

    #[test]
    fn test_data_record_new() {
        let record = DataRecord::new("test_lake", "UserInput", serde_json::json!({"name": "John"}))
            .with_source("manual");

        assert!(!record.id.is_empty());
        assert_eq!(record.data_lake, "test_lake");
        assert_eq!(record.schema_name, "UserInput");
        assert_eq!(record.created_by, Some("manual".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::valid();
        assert!(result.valid);
        assert!(result.errors.is_empty());

        result.add_error("email", "Invalid email format");
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].path, "email");
    }
}
