//! Reusable JSON Schema definitions.
//!
//! This module provides support for defining reusable JSON schemas that can be
//! referenced across tools, agents, workflows, and other archetypes using
//! JSON `$ref` style references.
//!
//! ## Example Schema Definition
//!
//! ```yaml
//! name: UserInput
//! description: Standard user information schema
//! schema:
//!   type: object
//!   properties:
//!     name:
//!       type: string
//!     email:
//!       type: string
//!       format: email
//!   required:
//!     - name
//!     - email
//! ```
//!
//! ## Using Schema References
//!
//! In tool/agent definitions, reference a schema using:
//! ```yaml
//! input_schema:
//!   $ref: UserInput
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for a reusable JSON schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaConfig {
    /// Unique name for this schema (used in $ref references)
    pub name: String,
    /// Human-readable description of the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// The actual JSON Schema definition
    pub schema: Value,
}

/// Resolve schema references in a JSON value.
///
/// This function looks for `{"$ref": "SchemaName"}` patterns and replaces them
/// with the actual schema definition from the schemas list.
///
/// # Arguments
///
/// * `value` - The JSON value that may contain $ref references
/// * `schemas` - List of available schema definitions
///
/// # Returns
///
/// The resolved JSON value with all $ref replaced, or an error if a referenced
/// schema is not found.
pub fn resolve_schema_refs(value: &Value, schemas: &[SchemaConfig]) -> Result<Value, String> {
    match value {
        Value::Object(map) => {
            // Check if this object is a $ref
            if map.len() == 1 {
                if let Some(Value::String(ref_name)) = map.get("$ref") {
                    // Find the schema by name
                    let schema = schemas
                        .iter()
                        .find(|s| &s.name == ref_name)
                        .ok_or_else(|| format!("Schema reference '{}' not found", ref_name))?;

                    // Return a clone of the schema definition
                    return Ok(schema.schema.clone());
                }
            }

            // Recursively resolve refs in nested objects
            let mut resolved = serde_json::Map::new();
            for (key, val) in map {
                resolved.insert(key.clone(), resolve_schema_refs(val, schemas)?);
            }
            Ok(Value::Object(resolved))
        }
        Value::Array(arr) => {
            // Recursively resolve refs in array items
            let resolved: Result<Vec<Value>, String> = arr
                .iter()
                .map(|v| resolve_schema_refs(v, schemas))
                .collect();
            Ok(Value::Array(resolved?))
        }
        // Other value types are returned as-is
        _ => Ok(value.clone()),
    }
}

/// Check if a value contains a schema reference
pub fn is_schema_ref(value: &Value) -> bool {
    if let Value::Object(map) = value {
        map.len() == 1 && map.contains_key("$ref")
    } else {
        false
    }
}

/// Extract the schema name from a $ref value
pub fn get_ref_name(value: &Value) -> Option<String> {
    if let Value::Object(map) = value {
        if map.len() == 1 {
            if let Some(Value::String(name)) = map.get("$ref") {
                return Some(name.clone());
            }
        }
    }
    None
}

/// Create a schema reference value
pub fn make_schema_ref(name: &str) -> Value {
    serde_json::json!({ "$ref": name })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_schemas() -> Vec<SchemaConfig> {
        vec![
            SchemaConfig {
                name: "UserInput".to_string(),
                description: Some("User information".to_string()),
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "email": { "type": "string", "format": "email" }
                    },
                    "required": ["name", "email"]
                }),
            },
            SchemaConfig {
                name: "Address".to_string(),
                description: None,
                schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "city": { "type": "string" }
                    }
                }),
            },
        ]
    }

    #[test]
    fn test_resolve_simple_ref() {
        let schemas = sample_schemas();
        let value = serde_json::json!({ "$ref": "UserInput" });

        let resolved = resolve_schema_refs(&value, &schemas).unwrap();

        assert_eq!(resolved["type"], "object");
        assert!(resolved["properties"]["name"].is_object());
    }

    #[test]
    fn test_resolve_nested_ref() {
        let schemas = sample_schemas();
        let value = serde_json::json!({
            "type": "object",
            "properties": {
                "user": { "$ref": "UserInput" },
                "address": { "$ref": "Address" }
            }
        });

        let resolved = resolve_schema_refs(&value, &schemas).unwrap();

        assert_eq!(resolved["properties"]["user"]["type"], "object");
        assert!(resolved["properties"]["user"]["properties"]["email"].is_object());
        assert!(resolved["properties"]["address"]["properties"]["street"].is_object());
    }

    #[test]
    fn test_resolve_missing_ref() {
        let schemas = sample_schemas();
        let value = serde_json::json!({ "$ref": "NonExistent" });

        let result = resolve_schema_refs(&value, &schemas);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("NonExistent"));
    }

    #[test]
    fn test_is_schema_ref() {
        assert!(is_schema_ref(&serde_json::json!({ "$ref": "Test" })));
        assert!(!is_schema_ref(&serde_json::json!({ "type": "string" })));
        assert!(!is_schema_ref(&serde_json::json!({ "$ref": "Test", "extra": true })));
        assert!(!is_schema_ref(&serde_json::json!("string")));
    }

    #[test]
    fn test_get_ref_name() {
        assert_eq!(get_ref_name(&serde_json::json!({ "$ref": "Test" })), Some("Test".to_string()));
        assert_eq!(get_ref_name(&serde_json::json!({ "type": "string" })), None);
    }

    #[test]
    fn test_make_schema_ref() {
        let ref_val = make_schema_ref("MySchema");
        assert_eq!(ref_val, serde_json::json!({ "$ref": "MySchema" }));
    }
}
