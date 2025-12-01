//! JSON Schema Editor Component
//!
//! A hierarchical, guided editor for JSON Schema definitions.
//! Supports full JSON Schema draft-07 constructs including:
//! - definitions/$defs blocks with $ref resolution
//! - additionalProperties
//! - oneOf/anyOf/allOf composition
//! - Pattern, format, and validation constraints
//! - Root-level metadata ($schema, $id, title, description)

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use serde_json::{json, Value, Map};
use crate::api;

/// Root-level schema metadata
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SchemaMetadata {
    /// JSON Schema version URI (e.g., "http://json-schema.org/draft-07/schema#")
    pub schema_uri: String,
    /// Schema identifier URI
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// Schema description
    pub description: String,
}

/// A definition entry in the definitions/$defs block
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SchemaDefinition {
    pub id: u32,
    /// Definition name (key in definitions object)
    pub name: String,
    /// Description of this definition
    pub description: String,
    /// The type of this definition
    pub def_type: String,
    /// Properties if this is an object type
    pub properties: Vec<SchemaProperty>,
    /// Enum values if this is an enum type
    pub enum_values: Vec<String>,
}

/// A single property in the schema
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SchemaProperty {
    pub id: u32,
    pub name: String,
    pub prop_type: String,
    pub description: String,
    pub required: bool,
    /// For arrays: the type of items
    pub items_type: String,
    /// For objects: nested properties (recursive)
    pub nested_properties: Vec<SchemaProperty>,
    /// For enums: allowed values (comma-separated in UI)
    pub enum_values: Vec<String>,
    /// For $ref: reference to internal schema name or external URL
    pub ref_value: String,
    /// For $ref: whether this is an external URL reference
    pub ref_is_external: bool,

    // === Advanced JSON Schema fields ===

    /// Regex pattern for string validation
    pub pattern: String,
    /// Format hint (e.g., "email", "uri", "date-time", "uuid")
    pub format: String,
    /// Default value (stored as JSON string)
    pub default_value: String,
    /// Const value constraint (stored as JSON string)
    pub const_value: String,
    /// Minimum value for numbers
    pub minimum: Option<f64>,
    /// Maximum value for numbers
    pub maximum: Option<f64>,
    /// Exclusive minimum for numbers
    pub exclusive_minimum: Option<f64>,
    /// Exclusive maximum for numbers
    pub exclusive_maximum: Option<f64>,
    /// Minimum string length
    pub min_length: Option<u64>,
    /// Maximum string length
    pub max_length: Option<u64>,
    /// Minimum array items
    pub min_items: Option<u64>,
    /// Maximum array items
    pub max_items: Option<u64>,
    /// Array items must be unique
    pub unique_items: bool,
    /// additionalProperties: true, false, or a schema (stored as $ref or type)
    pub additional_properties: AdditionalProperties,
    /// oneOf schemas (list of $ref values or inline types)
    pub one_of: Vec<TypeVariant>,
    /// anyOf schemas
    pub any_of: Vec<TypeVariant>,
    /// allOf schemas
    pub all_of: Vec<TypeVariant>,
    /// Example values (stored as JSON strings)
    pub examples: Vec<String>,
    /// Whether this property is deprecated
    pub deprecated: bool,
    /// Read-only property
    pub read_only: bool,
    /// Write-only property
    pub write_only: bool,
}

/// Represents additionalProperties which can be boolean or a schema reference
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AdditionalProperties {
    #[default]
    Unset,
    Boolean(bool),
    /// Reference to a definition or inline type
    Schema(String),
}

/// Represents a type variant for oneOf/anyOf/allOf
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TypeVariant {
    pub id: u32,
    /// Type: "string", "number", etc., or "$ref"
    pub variant_type: String,
    /// For $ref variants, the reference value
    pub ref_value: String,
    /// For inline object types, nested properties
    pub properties: Vec<SchemaProperty>,
    /// For enum variants
    pub enum_values: Vec<String>,
    /// Const value for this variant
    pub const_value: String,
}

static NEXT_PROP_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn next_prop_id() -> u32 {
    NEXT_PROP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

impl SchemaDefinition {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            id: next_prop_id(),
            name: String::new(),
            description: String::new(),
            def_type: "object".to_string(),
            properties: Vec::new(),
            enum_values: Vec::new(),
        }
    }

    /// Convert definition to JSON Schema Value
    pub fn to_schema_value(&self) -> Value {
        let mut def = Map::new();

        if !self.description.is_empty() {
            def.insert("description".to_string(), json!(self.description));
        }

        match self.def_type.as_str() {
            "enum" => {
                def.insert("type".to_string(), json!("string"));
                if !self.enum_values.is_empty() {
                    def.insert("enum".to_string(), json!(self.enum_values));
                }
            }
            "object" => {
                def.insert("type".to_string(), json!("object"));
                if !self.properties.is_empty() {
                    let schema = properties_to_schema(&self.properties);
                    if let Some(props) = schema.get("properties") {
                        def.insert("properties".to_string(), props.clone());
                    }
                    if let Some(req) = schema.get("required") {
                        def.insert("required".to_string(), req.clone());
                    }
                }
            }
            _ => {
                def.insert("type".to_string(), json!(self.def_type));
            }
        }

        Value::Object(def)
    }
}

impl TypeVariant {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            id: next_prop_id(),
            variant_type: "string".to_string(),
            ref_value: String::new(),
            properties: Vec::new(),
            enum_values: Vec::new(),
            const_value: String::new(),
        }
    }

    /// Convert to JSON Schema Value
    pub fn to_schema_value(&self) -> Value {
        let mut variant = Map::new();

        if self.variant_type == "$ref" && !self.ref_value.is_empty() {
            variant.insert("$ref".to_string(), json!(self.ref_value));
            return Value::Object(variant);
        }

        if !self.const_value.is_empty() {
            if let Ok(v) = serde_json::from_str::<Value>(&self.const_value) {
                variant.insert("const".to_string(), v);
            } else {
                variant.insert("const".to_string(), json!(self.const_value));
            }
            return Value::Object(variant);
        }

        if self.variant_type == "enum" && !self.enum_values.is_empty() {
            variant.insert("type".to_string(), json!("string"));
            variant.insert("enum".to_string(), json!(self.enum_values));
            return Value::Object(variant);
        }

        variant.insert("type".to_string(), json!(self.variant_type));

        if self.variant_type == "object" && !self.properties.is_empty() {
            let schema = properties_to_schema(&self.properties);
            if let Some(props) = schema.get("properties") {
                variant.insert("properties".to_string(), props.clone());
            }
            if let Some(req) = schema.get("required") {
                variant.insert("required".to_string(), req.clone());
            }
        }

        Value::Object(variant)
    }
}

impl SchemaProperty {
    pub fn new() -> Self {
        Self {
            id: next_prop_id(),
            name: String::new(),
            prop_type: "string".to_string(),
            description: String::new(),
            required: false,
            items_type: "string".to_string(),
            nested_properties: Vec::new(),
            enum_values: Vec::new(),
            ref_value: String::new(),
            ref_is_external: false,
            // Advanced fields
            pattern: String::new(),
            format: String::new(),
            default_value: String::new(),
            const_value: String::new(),
            minimum: None,
            maximum: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            min_length: None,
            max_length: None,
            min_items: None,
            max_items: None,
            unique_items: false,
            additional_properties: AdditionalProperties::Unset,
            one_of: Vec::new(),
            any_of: Vec::new(),
            all_of: Vec::new(),
            examples: Vec::new(),
            deprecated: false,
            read_only: false,
            write_only: false,
        }
    }

    /// Convert to JSON Schema Value
    pub fn to_schema_value(&self) -> Value {
        // Handle $ref type - returns just the reference object
        if self.prop_type == "$ref" {
            let mut prop = Map::new();
            if !self.ref_value.is_empty() {
                prop.insert("$ref".to_string(), json!(self.ref_value));
            }
            // Include description if present (valid alongside $ref in some implementations)
            if !self.description.is_empty() {
                prop.insert("description".to_string(), json!(self.description));
            }
            return Value::Object(prop);
        }

        // Handle oneOf/anyOf/allOf types
        if self.prop_type == "oneOf" && !self.one_of.is_empty() {
            let mut prop = Map::new();
            let variants: Vec<Value> = self.one_of.iter().map(|v| v.to_schema_value()).collect();
            prop.insert("oneOf".to_string(), json!(variants));
            if !self.description.is_empty() {
                prop.insert("description".to_string(), json!(self.description));
            }
            return Value::Object(prop);
        }

        if self.prop_type == "anyOf" && !self.any_of.is_empty() {
            let mut prop = Map::new();
            let variants: Vec<Value> = self.any_of.iter().map(|v| v.to_schema_value()).collect();
            prop.insert("anyOf".to_string(), json!(variants));
            if !self.description.is_empty() {
                prop.insert("description".to_string(), json!(self.description));
            }
            return Value::Object(prop);
        }

        let mut prop = Map::new();

        // Handle const type
        if !self.const_value.is_empty() {
            if let Ok(v) = serde_json::from_str::<Value>(&self.const_value) {
                prop.insert("const".to_string(), v);
            } else {
                prop.insert("const".to_string(), json!(self.const_value));
            }
            if !self.description.is_empty() {
                prop.insert("description".to_string(), json!(self.description));
            }
            return Value::Object(prop);
        }

        // Handle enum type specially - it's a string with enum constraint
        if self.prop_type == "enum" {
            prop.insert("type".to_string(), json!("string"));
            if !self.enum_values.is_empty() {
                prop.insert("enum".to_string(), json!(self.enum_values));
            }
        } else {
            prop.insert("type".to_string(), json!(self.prop_type));
        }

        if !self.description.is_empty() {
            prop.insert("description".to_string(), json!(self.description));
        }

        // String constraints
        if !self.pattern.is_empty() {
            prop.insert("pattern".to_string(), json!(self.pattern));
        }
        if !self.format.is_empty() {
            prop.insert("format".to_string(), json!(self.format));
        }
        if let Some(min_len) = self.min_length {
            prop.insert("minLength".to_string(), json!(min_len));
        }
        if let Some(max_len) = self.max_length {
            prop.insert("maxLength".to_string(), json!(max_len));
        }

        // Numeric constraints
        if let Some(min) = self.minimum {
            prop.insert("minimum".to_string(), json!(min));
        }
        if let Some(max) = self.maximum {
            prop.insert("maximum".to_string(), json!(max));
        }
        if let Some(emin) = self.exclusive_minimum {
            prop.insert("exclusiveMinimum".to_string(), json!(emin));
        }
        if let Some(emax) = self.exclusive_maximum {
            prop.insert("exclusiveMaximum".to_string(), json!(emax));
        }

        // Default value
        if !self.default_value.is_empty() {
            if let Ok(v) = serde_json::from_str::<Value>(&self.default_value) {
                prop.insert("default".to_string(), v);
            } else {
                prop.insert("default".to_string(), json!(self.default_value));
            }
        }

        // Examples
        if !self.examples.is_empty() {
            let examples: Vec<Value> = self.examples.iter()
                .filter_map(|e| serde_json::from_str::<Value>(e).ok().or_else(|| Some(json!(e))))
                .collect();
            prop.insert("examples".to_string(), json!(examples));
        }

        // Boolean flags
        if self.deprecated {
            prop.insert("deprecated".to_string(), json!(true));
        }
        if self.read_only {
            prop.insert("readOnly".to_string(), json!(true));
        }
        if self.write_only {
            prop.insert("writeOnly".to_string(), json!(true));
        }

        // allOf (can be combined with type)
        if !self.all_of.is_empty() {
            let variants: Vec<Value> = self.all_of.iter().map(|v| v.to_schema_value()).collect();
            prop.insert("allOf".to_string(), json!(variants));
        }

        match self.prop_type.as_str() {
            "array" => {
                let items = if self.items_type == "object" && !self.nested_properties.is_empty() {
                    let nested_schema = properties_to_schema(&self.nested_properties);
                    nested_schema
                } else if self.items_type == "enum" && !self.enum_values.is_empty() {
                    // Array of enum values
                    json!({"type": "string", "enum": self.enum_values})
                } else if self.items_type == "$ref" && !self.ref_value.is_empty() {
                    // Array of $ref items
                    json!({"$ref": self.ref_value})
                } else {
                    json!({"type": self.items_type})
                };
                prop.insert("items".to_string(), items);

                // Array constraints
                if let Some(min) = self.min_items {
                    prop.insert("minItems".to_string(), json!(min));
                }
                if let Some(max) = self.max_items {
                    prop.insert("maxItems".to_string(), json!(max));
                }
                if self.unique_items {
                    prop.insert("uniqueItems".to_string(), json!(true));
                }
            }
            "object" => {
                if !self.nested_properties.is_empty() {
                    let nested_schema = properties_to_schema(&self.nested_properties);
                    if let Some(nested_props) = nested_schema.get("properties") {
                        prop.insert("properties".to_string(), nested_props.clone());
                    }
                    if let Some(required) = nested_schema.get("required") {
                        prop.insert("required".to_string(), required.clone());
                    }
                }

                // additionalProperties
                match &self.additional_properties {
                    AdditionalProperties::Boolean(b) => {
                        prop.insert("additionalProperties".to_string(), json!(b));
                    }
                    AdditionalProperties::Schema(ref_val) => {
                        if ref_val.starts_with("#/") || ref_val.starts_with("http") {
                            prop.insert("additionalProperties".to_string(), json!({"$ref": ref_val}));
                        } else if !ref_val.is_empty() {
                            prop.insert("additionalProperties".to_string(), json!({"$ref": ref_val}));
                        }
                    }
                    AdditionalProperties::Unset => {}
                }
            }
            _ => {}
        }

        Value::Object(prop)
    }
}

/// Convert properties to JSON Schema
pub fn properties_to_schema(properties: &[SchemaProperty]) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), json!("object"));

    let mut props = Map::new();
    let mut required: Vec<String> = Vec::new();

    for property in properties {
        if !property.name.is_empty() {
            props.insert(property.name.clone(), property.to_schema_value());
            if property.required {
                required.push(property.name.clone());
            }
        }
    }

    if !props.is_empty() {
        schema.insert("properties".to_string(), Value::Object(props));
    }
    if !required.is_empty() {
        schema.insert("required".to_string(), json!(required));
    }

    Value::Object(schema)
}

/// Convert a full schema with metadata and definitions to JSON Schema
pub fn full_schema_to_value(
    metadata: &SchemaMetadata,
    definitions: &[SchemaDefinition],
    properties: &[SchemaProperty],
) -> Value {
    let mut schema = Map::new();

    // Root metadata
    if !metadata.schema_uri.is_empty() {
        schema.insert("$schema".to_string(), json!(metadata.schema_uri));
    }
    if !metadata.id.is_empty() {
        schema.insert("$id".to_string(), json!(metadata.id));
    }
    if !metadata.title.is_empty() {
        schema.insert("title".to_string(), json!(metadata.title));
    }
    if !metadata.description.is_empty() {
        schema.insert("description".to_string(), json!(metadata.description));
    }

    schema.insert("type".to_string(), json!("object"));

    // Properties
    let mut props = Map::new();
    let mut required: Vec<String> = Vec::new();

    for property in properties {
        if !property.name.is_empty() {
            props.insert(property.name.clone(), property.to_schema_value());
            if property.required {
                required.push(property.name.clone());
            }
        }
    }

    if !props.is_empty() {
        schema.insert("properties".to_string(), Value::Object(props));
    }
    if !required.is_empty() {
        schema.insert("required".to_string(), json!(required));
    }

    // Definitions
    if !definitions.is_empty() {
        let mut defs = Map::new();
        for def in definitions {
            if !def.name.is_empty() {
                defs.insert(def.name.clone(), def.to_schema_value());
            }
        }
        schema.insert("definitions".to_string(), Value::Object(defs));
    }

    Value::Object(schema)
}

/// Parse metadata from a JSON Schema
pub fn schema_to_metadata(schema: &Value) -> SchemaMetadata {
    SchemaMetadata {
        schema_uri: schema.get("$schema")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        id: schema.get("$id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        title: schema.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        description: schema.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }
}

/// Parse definitions from a JSON Schema
pub fn schema_to_definitions(schema: &Value) -> Vec<SchemaDefinition> {
    let mut definitions = Vec::new();

    // Try both "definitions" and "$defs" (draft-07 vs draft-2019-09+)
    let defs_obj = schema.get("definitions")
        .or_else(|| schema.get("$defs"))
        .and_then(|d| d.as_object());

    if let Some(defs) = defs_obj {
        for (name, def_schema) in defs {
            let description = def_schema.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let raw_type = def_schema.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("object")
                .to_string();

            // Check if it's an enum
            let enum_values: Vec<String> = def_schema.get("enum")
                .and_then(|e| e.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            let def_type = if !enum_values.is_empty() {
                "enum".to_string()
            } else {
                raw_type
            };

            let properties = if def_type == "object" {
                schema_to_properties(def_schema)
            } else {
                Vec::new()
            };

            definitions.push(SchemaDefinition {
                id: next_prop_id(),
                name: name.clone(),
                description,
                def_type,
                properties,
                enum_values,
            });
        }
    }

    definitions
}

/// Helper to parse a type variant from JSON
fn parse_type_variant(value: &Value) -> TypeVariant {
    let ref_value = value.get("$ref")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();

    let const_value = value.get("const")
        .map(|c| serde_json::to_string(c).unwrap_or_default())
        .unwrap_or_default();

    let raw_type = value.get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("string")
        .to_string();

    let enum_values: Vec<String> = value.get("enum")
        .and_then(|e| e.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let variant_type = if !ref_value.is_empty() {
        "$ref".to_string()
    } else if !const_value.is_empty() {
        "const".to_string()
    } else if !enum_values.is_empty() {
        "enum".to_string()
    } else {
        raw_type.clone()
    };

    let properties = if raw_type == "object" {
        schema_to_properties(value)
    } else {
        Vec::new()
    };

    TypeVariant {
        id: next_prop_id(),
        variant_type,
        ref_value,
        properties,
        enum_values,
        const_value,
    }
}

/// Parse JSON Schema to properties
pub fn schema_to_properties(schema: &Value) -> Vec<SchemaProperty> {
    let mut properties = Vec::new();

    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        let required_fields: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        for (name, prop) in props {
            // Check if this is a $ref
            let ref_value = prop.get("$ref")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            let raw_type = prop.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string")
                .to_string();

            // Check if this is an enum (has "enum" field)
            let enum_values: Vec<String> = prop.get("enum")
                .and_then(|e| e.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            // Check for oneOf/anyOf/allOf
            let one_of: Vec<TypeVariant> = prop.get("oneOf")
                .and_then(|o| o.as_array())
                .map(|arr| arr.iter().map(parse_type_variant).collect())
                .unwrap_or_default();

            let any_of: Vec<TypeVariant> = prop.get("anyOf")
                .and_then(|o| o.as_array())
                .map(|arr| arr.iter().map(parse_type_variant).collect())
                .unwrap_or_default();

            let all_of: Vec<TypeVariant> = prop.get("allOf")
                .and_then(|o| o.as_array())
                .map(|arr| arr.iter().map(parse_type_variant).collect())
                .unwrap_or_default();

            // Check for const
            let const_value = prop.get("const")
                .map(|c| serde_json::to_string(c).unwrap_or_default())
                .unwrap_or_default();

            // Determine property type: $ref > oneOf > anyOf > const > enum > raw type
            let prop_type = if !ref_value.is_empty() {
                "$ref".to_string()
            } else if !one_of.is_empty() {
                "oneOf".to_string()
            } else if !any_of.is_empty() {
                "anyOf".to_string()
            } else if !const_value.is_empty() {
                "const".to_string()
            } else if !enum_values.is_empty() {
                "enum".to_string()
            } else {
                raw_type.clone()
            };

            // Check if ref is external (starts with http:// or https://)
            let ref_is_external = ref_value.starts_with("http://") || ref_value.starts_with("https://");

            let description = prop.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            // Parse advanced string/number constraints
            let pattern = prop.get("pattern")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();

            let format = prop.get("format")
                .and_then(|f| f.as_str())
                .unwrap_or("")
                .to_string();

            let default_value = prop.get("default")
                .map(|d| serde_json::to_string(d).unwrap_or_default())
                .unwrap_or_default();

            let minimum = prop.get("minimum").and_then(|v| v.as_f64());
            let maximum = prop.get("maximum").and_then(|v| v.as_f64());
            let exclusive_minimum = prop.get("exclusiveMinimum").and_then(|v| v.as_f64());
            let exclusive_maximum = prop.get("exclusiveMaximum").and_then(|v| v.as_f64());
            let min_length = prop.get("minLength").and_then(|v| v.as_u64());
            let max_length = prop.get("maxLength").and_then(|v| v.as_u64());
            let min_items = prop.get("minItems").and_then(|v| v.as_u64());
            let max_items = prop.get("maxItems").and_then(|v| v.as_u64());
            let unique_items = prop.get("uniqueItems").and_then(|v| v.as_bool()).unwrap_or(false);

            // Parse examples
            let examples: Vec<String> = prop.get("examples")
                .and_then(|e| e.as_array())
                .map(|arr| arr.iter().map(|v| serde_json::to_string(v).unwrap_or_default()).collect())
                .unwrap_or_default();

            // Boolean flags
            let deprecated = prop.get("deprecated").and_then(|v| v.as_bool()).unwrap_or(false);
            let read_only = prop.get("readOnly").and_then(|v| v.as_bool()).unwrap_or(false);
            let write_only = prop.get("writeOnly").and_then(|v| v.as_bool()).unwrap_or(false);

            // Parse additionalProperties
            let additional_properties = if let Some(ap) = prop.get("additionalProperties") {
                if let Some(b) = ap.as_bool() {
                    AdditionalProperties::Boolean(b)
                } else if let Some(ref_val) = ap.get("$ref").and_then(|r| r.as_str()) {
                    AdditionalProperties::Schema(ref_val.to_string())
                } else if ap.is_object() {
                    // It's an inline schema - store as JSON for now
                    AdditionalProperties::Schema(serde_json::to_string(ap).unwrap_or_default())
                } else {
                    AdditionalProperties::Unset
                }
            } else {
                AdditionalProperties::Unset
            };

            let (items_type, nested_properties, items_enum_values, items_ref_value, items_ref_is_external) = if raw_type == "array" {
                if let Some(items) = prop.get("items") {
                    // Check for $ref in items
                    let items_ref = items.get("$ref")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    let items_t = items.get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("string")
                        .to_string();

                    // Check if array items have enum
                    let items_enum: Vec<String> = items.get("enum")
                        .and_then(|e| e.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                        .unwrap_or_default();

                    let actual_items_type = if !items_ref.is_empty() {
                        "$ref".to_string()
                    } else if !items_enum.is_empty() {
                        "enum".to_string()
                    } else {
                        items_t.clone()
                    };

                    let items_is_external = items_ref.starts_with("http://") || items_ref.starts_with("https://");

                    let nested = if items_t == "object" {
                        schema_to_properties(items)
                    } else {
                        Vec::new()
                    };
                    (actual_items_type, nested, items_enum, items_ref, items_is_external)
                } else {
                    ("string".to_string(), Vec::new(), Vec::new(), String::new(), false)
                }
            } else if raw_type == "object" {
                ("string".to_string(), schema_to_properties(prop), Vec::new(), String::new(), false)
            } else {
                ("string".to_string(), Vec::new(), Vec::new(), String::new(), false)
            };

            // Use items_enum_values if this is an array of enums, otherwise use the property's enum_values
            let final_enum_values = if raw_type == "array" && !items_enum_values.is_empty() {
                items_enum_values
            } else {
                enum_values
            };

            // Use items_ref if this is an array of refs, otherwise use the property's ref
            let final_ref_value = if raw_type == "array" && !items_ref_value.is_empty() {
                items_ref_value
            } else {
                ref_value
            };

            let final_ref_is_external = if raw_type == "array" && items_ref_is_external {
                items_ref_is_external
            } else {
                ref_is_external
            };

            properties.push(SchemaProperty {
                id: next_prop_id(),
                name: name.clone(),
                prop_type,
                description,
                required: required_fields.contains(name),
                items_type,
                nested_properties,
                enum_values: final_enum_values,
                ref_value: final_ref_value,
                ref_is_external: final_ref_is_external,
                // Advanced fields
                pattern,
                format,
                default_value,
                const_value,
                minimum,
                maximum,
                exclusive_minimum,
                exclusive_maximum,
                min_length,
                max_length,
                min_items,
                max_items,
                unique_items,
                additional_properties,
                one_of,
                any_of,
                all_of,
                examples,
                deprecated,
                read_only,
                write_only,
            });
        }
    }

    properties
}

/// Helper: get a mutable reference to nested properties at a given path
fn get_nested_properties_at_path_mut<'a>(properties: &'a mut [SchemaProperty], path: &[usize]) -> Option<&'a mut Vec<SchemaProperty>> {
    if path.is_empty() {
        return None; // Can't return root as mutable Vec
    }

    let mut current = properties;
    for (i, &idx) in path.iter().enumerate() {
        if i == path.len() - 1 {
            return current.get_mut(idx).map(|p| &mut p.nested_properties);
        }
        current = &mut current.get_mut(idx)?.nested_properties;
    }
    None
}

/// Helper: get a property at a given path
fn get_property_at_path<'a>(properties: &'a [SchemaProperty], path: &[usize]) -> Option<&'a SchemaProperty> {
    if path.is_empty() {
        return None;
    }

    let mut current = properties;
    for (i, &idx) in path.iter().enumerate() {
        if i == path.len() - 1 {
            return current.get(idx);
        }
        current = &current.get(idx)?.nested_properties;
    }
    None
}

/// Helper: mutate a property at a given path
fn mutate_property_at_path<F>(properties: &mut [SchemaProperty], path: &[usize], f: F)
where
    F: FnOnce(&mut SchemaProperty),
{
    if path.is_empty() {
        return;
    }

    let mut current = properties;
    for (i, &idx) in path.iter().enumerate() {
        if i == path.len() - 1 {
            if let Some(prop) = current.get_mut(idx) {
                f(prop);
            }
            return;
        }
        if let Some(prop) = current.get_mut(idx) {
            current = &mut prop.nested_properties;
        } else {
            return;
        }
    }
}

/// Main JSON Schema Editor component
#[component]
pub fn JsonSchemaEditor(
    properties: ReadSignal<Vec<SchemaProperty>>,
    set_properties: WriteSignal<Vec<SchemaProperty>>,
    #[prop(default = "Properties")] label: &'static str,
    #[prop(default = "green")] color: &'static str,
    /// Available schema names for $ref selection (optional)
    #[prop(optional)] available_schemas: Option<ReadSignal<Vec<String>>>,
) -> impl IntoView {
    let ring_color = match color {
        "orange" => "focus:ring-orange-500",
        "purple" => "focus:ring-purple-500",
        "blue" => "focus:ring-blue-500",
        _ => "focus:ring-green-500",
    };

    let btn_color = match color {
        "orange" => "text-orange-600 hover:bg-orange-50",
        "purple" => "text-purple-600 hover:bg-purple-50",
        "blue" => "text-blue-600 hover:bg-blue-50",
        _ => "text-green-600 hover:bg-green-50",
    };

    // Store available_schemas for passing to children
    let schemas_stored = StoredValue::new(available_schemas);

    let add_property = move |_| {
        set_properties.update(|props| {
            props.push(SchemaProperty::new());
        });
    };

    view! {
        <div class="border border-gray-200 rounded-lg p-4">
            <div class="flex justify-between items-center mb-3">
                <label class="block text-sm font-medium text-gray-700">{label}</label>
                <button
                    type="button"
                    class=format!("px-3 py-1 text-sm {} rounded flex items-center gap-1", btn_color)
                    on:click=add_property
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "Add Property"
                </button>
            </div>

            <div class="space-y-3">
                <Show
                    when=move || !properties.get().is_empty()
                    fallback=|| view! {
                        <div class="text-sm text-gray-500 italic p-3 bg-gray-50 rounded">
                            "No properties defined. Click 'Add Property' to add input parameters."
                        </div>
                    }
                >
                    <For
                        each=move || {
                            properties.get().into_iter().enumerate().collect::<Vec<_>>()
                        }
                        key=|(_, prop)| prop.id
                        children=move |(idx, _)| {
                            view! {
                                <RecursivePropertyEditor
                                    path=vec![idx]
                                    depth=0
                                    properties=properties
                                    set_properties=set_properties
                                    ring_color=ring_color
                                    btn_color=btn_color
                                    available_schemas=schemas_stored.get_value()
                                />
                            }
                        }
                    />
                </Show>
            </div>
        </div>
    }
}

/// Represents a schema reference option in the searchable dropdown
#[derive(Clone, Debug, PartialEq)]
pub struct SchemaRefOption {
    /// The full $ref value (e.g., "#/definitions/Address" or "schemas/User")
    pub value: String,
    /// Display label
    pub label: String,
    /// Category for grouping (e.g., "Definitions", "Schemas")
    pub category: String,
}

/// Searchable schema reference selector component
#[component]
fn SearchableSchemaSelector(
    /// Current selected value
    #[prop(into)]
    value: Signal<String>,
    /// Callback when value changes
    on_change: Callback<String>,
    /// Available local definitions (from current schema)
    local_definitions: Signal<Vec<String>>,
    /// Ring color class for focus styling
    ring_color: &'static str,
) -> impl IntoView {
    let (search_text, set_search_text) = signal(String::new());
    let (is_open, set_is_open) = signal(false);
    // Store external schemas with their definitions: (schema_name, vec of definition names)
    let (external_schema_defs, set_external_schema_defs) = signal(Vec::<(String, Vec<String>)>::new());

    // Load external schemas and their definitions on mount
    Effect::new(move |_| {
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(schemas) = api::list_schemas().await {
                let schema_defs: Vec<(String, Vec<String>)> = schemas.into_iter().map(|s| {
                    // Extract definitions from the schema's JSON
                    let mut defs = Vec::new();
                    if let Some(definitions) = s.schema.get("definitions").and_then(|d| d.as_object()) {
                        for key in definitions.keys() {
                            defs.push(key.clone());
                        }
                    }
                    // Also check $defs (newer JSON Schema versions)
                    if let Some(definitions) = s.schema.get("$defs").and_then(|d| d.as_object()) {
                        for key in definitions.keys() {
                            if !defs.contains(key) {
                                defs.push(key.clone());
                            }
                        }
                    }
                    (s.name, defs)
                }).collect();
                set_external_schema_defs.set(schema_defs);
            }
        });
    });

    // Combined options: local definitions + external schemas + their definitions
    let all_options = move || {
        let mut options = Vec::new();

        // Add local definitions (from current schema)
        for def in local_definitions.get() {
            options.push(SchemaRefOption {
                value: def.clone(), // Already formatted as #/definitions/Name
                label: def.replace("#/definitions/", ""),
                category: "Local Definitions".to_string(),
            });
        }

        // Add external schemas and their definitions
        for (schema_name, defs) in external_schema_defs.get() {
            // Always add the schema itself as an option
            options.push(SchemaRefOption {
                value: format!("schemas/{}", schema_name),
                label: schema_name.clone(),
                category: "External Schemas".to_string(),
            });

            // Also add its definitions if any
            for def_name in defs {
                options.push(SchemaRefOption {
                    value: format!("schemas/{}#/definitions/{}", schema_name, def_name),
                    label: format!("{} → {}", schema_name, def_name),
                    category: format!("Schema: {} (definitions)", schema_name),
                });
            }
        }

        options
    };

    // Filtered options based on search
    let filtered_options = move || {
        let search = search_text.get().to_lowercase();
        let opts = all_options();
        if search.is_empty() {
            opts
        } else {
            opts.into_iter()
                .filter(|o| o.label.to_lowercase().contains(&search) || o.value.to_lowercase().contains(&search))
                .collect()
        }
    };

    // Get display value for selected item
    let display_value = move || {
        let v = value.get();
        if v.is_empty() {
            "Select schema reference...".to_string()
        } else if v.starts_with("#/definitions/") {
            v.replace("#/definitions/", "def: ")
        } else if v.starts_with("schemas/") {
            v.replace("schemas/", "schema: ")
        } else {
            v
        }
    };

    view! {
        <div class="relative">
            // Input with dropdown trigger
            <div class="relative">
                <input
                    type="text"
                    class=format!("w-full px-2 py-1 pr-8 text-sm border border-gray-300 rounded {} bg-teal-50", ring_color)
                    placeholder="Search schemas..."
                    prop:value=move || {
                        if is_open.get() {
                            search_text.get()
                        } else {
                            display_value()
                        }
                    }
                    on:focus=move |_| {
                        set_is_open.set(true);
                        set_search_text.set(String::new());
                    }
                    on:input=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                        set_search_text.set(input.value());
                        set_is_open.set(true);
                    }
                    on:blur=move |_| {
                        // Delay closing to allow click on option
                        set_timeout(
                            move || set_is_open.set(false),
                            std::time::Duration::from_millis(200)
                        );
                    }
                />
                // Dropdown arrow
                <div class="absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">
                    <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                    </svg>
                </div>
            </div>

            // Dropdown options - use very high z-index to escape parent clipping
            <Show when=move || is_open.get()>
                <div
                    class="absolute left-0 right-0 mt-1 bg-white border border-gray-300 rounded-lg shadow-xl max-h-60 overflow-auto"
                    style="z-index: 9999;"
                >
                    // Clear option
                    <button
                        type="button"
                        class="w-full px-3 py-1.5 text-left text-sm text-gray-500 hover:bg-gray-100 border-b"
                        on:mousedown=move |ev| {
                            ev.prevent_default();
                            on_change.run(String::new());
                            set_is_open.set(false);
                        }
                    >
                        "Clear selection"
                    </button>

                    // Render filtered options grouped by category
                    {move || {
                        let opts = filtered_options();

                        if opts.is_empty() {
                            view! {
                                <div class="px-3 py-2 text-sm text-gray-500 italic">
                                    "No matching schemas found. Create definitions in the Definitions section or add Schemas."
                                </div>
                            }.into_any()
                        } else {
                            // Group options by category
                            let mut categories: Vec<(String, Vec<SchemaRefOption>)> = Vec::new();
                            for opt in opts {
                                if let Some((_, items)) = categories.iter_mut().find(|(cat, _)| *cat == opt.category) {
                                    items.push(opt);
                                } else {
                                    categories.push((opt.category.clone(), vec![opt]));
                                }
                            }

                            view! {
                                <div>
                                    <For
                                        each=move || categories.clone()
                                        key=|(cat, _)| cat.clone()
                                        children=move |(category, items)| {
                                            let is_local = category == "Local Definitions";
                                            let header_class = if is_local {
                                                "px-3 py-1 text-xs font-semibold text-purple-600 bg-purple-50 border-b sticky top-0"
                                            } else {
                                                "px-3 py-1 text-xs font-semibold text-blue-600 bg-blue-50 border-b sticky top-0"
                                            };
                                            let item_hover = if is_local { "hover:bg-purple-50" } else { "hover:bg-blue-50" };
                                            let item_selected = if is_local { "bg-purple-100" } else { "bg-blue-100" };
                                            let icon_color = if is_local { "text-purple-500" } else { "text-blue-500" };
                                            let icon = if is_local { "#" } else { "↗" };

                                            view! {
                                                <div>
                                                    <div class=header_class>
                                                        {category}
                                                    </div>
                                                    <For
                                                        each=move || items.clone()
                                                        key=|opt| opt.value.clone()
                                                        children=move |opt| {
                                                            let opt_value = opt.value.clone();
                                                            let opt_value_for_click = opt.value.clone();
                                                            let opt_label = opt.label.clone();
                                                            let is_selected = value.get() == opt_value;
                                                            let btn_class = format!(
                                                                "w-full px-3 py-1.5 text-left text-sm {} flex items-center gap-2 {}",
                                                                item_hover,
                                                                if is_selected { item_selected } else { "" }
                                                            );
                                                            view! {
                                                                <button
                                                                    type="button"
                                                                    class=btn_class
                                                                    on:mousedown=move |ev| {
                                                                        ev.prevent_default();
                                                                        on_change.run(opt_value_for_click.clone());
                                                                        set_is_open.set(false);
                                                                    }
                                                                >
                                                                    <span class=icon_color>{icon}</span>
                                                                    {opt_label}
                                                                </button>
                                                            }
                                                        }
                                                    />
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

/// Recursive property editor - can edit properties at any depth
#[component]
fn RecursivePropertyEditor(
    path: Vec<usize>,
    depth: usize,
    properties: ReadSignal<Vec<SchemaProperty>>,
    set_properties: WriteSignal<Vec<SchemaProperty>>,
    ring_color: &'static str,
    btn_color: &'static str,
    /// Available schema names for $ref selection (passed as Option)
    available_schemas: Option<ReadSignal<Vec<String>>>,
) -> AnyView {
    let (expanded, set_expanded) = signal(depth == 0); // Auto-expand top level

    // Store path and available_schemas for all derived signals and closures
    let path_stored = StoredValue::new(path.clone());
    let schemas_stored = StoredValue::new(available_schemas);

    // Derived signals for this property's fields
    let name = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.name.clone()).unwrap_or_default()
    };
    let prop_type = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.prop_type.clone()).unwrap_or_default()
    };
    let description = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.description.clone()).unwrap_or_default()
    };
    let required = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.required).unwrap_or(false)
    };
    let items_type = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.items_type.clone()).unwrap_or_default()
    };
    let enum_values = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.enum_values.join(", ")).unwrap_or_default()
    };
    let ref_value = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.ref_value.clone()).unwrap_or_default()
    };
    let ref_is_external = move || {
        let props = properties.get();
        get_property_at_path(&props, &path_stored.get_value()).map(|p| p.ref_is_external).unwrap_or(false)
    };
    let has_nested = move || {
        let props = properties.get();
        let path = path_stored.get_value();
        let pt = get_property_at_path(&props, &path).map(|p| p.prop_type.clone()).unwrap_or_default();
        let it = get_property_at_path(&props, &path).map(|p| p.items_type.clone()).unwrap_or_default();
        pt == "object" || (pt == "array" && it == "object")
    };
    // Check if we need to show $ref input (type is $ref, or array with items_type $ref)
    let needs_ref_input = move || {
        prop_type() == "$ref" || (prop_type() == "array" && items_type() == "$ref")
    };

    // Indentation based on depth
    let indent_class = match depth {
        0 => "bg-gray-50",
        1 => "bg-gray-100 ml-4",
        2 => "bg-gray-50 ml-8",
        _ => "bg-gray-100 ml-12",
    };

    view! {
        <div class=format!("border border-gray-200 rounded-lg p-3 {}", indent_class)>
            <div class="flex flex-wrap gap-2 items-start">
                // Property name
                <div class="flex-1 min-w-[120px]">
                    <input
                        type="text"
                        class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {}", ring_color)
                        placeholder="Property name"
                        prop:value=name
                        on:input=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            let value = input.value();
                            let path = path_stored.get_value();
                            set_properties.update(move |props| {
                                mutate_property_at_path(props, &path, |p| p.name = value.clone());
                            });
                        }
                    />
                </div>

                // Type selector
                <div class="w-28">
                    <select
                        class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {}", ring_color)
                        prop:value=prop_type
                        on:change=move |ev| {
                            let target = ev.target().unwrap();
                            let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                            let value = select.value();
                            let path = path_stored.get_value();
                            set_properties.update(move |props| {
                                mutate_property_at_path(props, &path, |p| {
                                    p.prop_type = value.clone();
                                    if p.prop_type != "object" && p.prop_type != "array" {
                                        p.nested_properties.clear();
                                    }
                                    if p.prop_type != "enum" {
                                        p.enum_values.clear();
                                    }
                                    if p.prop_type != "$ref" {
                                        p.ref_value.clear();
                                        p.ref_is_external = false;
                                    }
                                    if p.prop_type != "oneOf" {
                                        p.one_of.clear();
                                    }
                                    if p.prop_type != "anyOf" {
                                        p.any_of.clear();
                                    }
                                    if p.prop_type != "const" {
                                        p.const_value.clear();
                                    }
                                });
                            });
                        }
                    >
                        <option value="string">"string"</option>
                        <option value="number">"number"</option>
                        <option value="integer">"integer"</option>
                        <option value="boolean">"boolean"</option>
                        <option value="enum">"enum"</option>
                        <option value="array">"array"</option>
                        <option value="object">"object"</option>
                        <option value="$ref">"$ref"</option>
                        <option value="oneOf">"oneOf"</option>
                        <option value="anyOf">"anyOf"</option>
                        <option value="const">"const"</option>
                    </select>
                </div>

                // Items type for arrays
                <Show when=move || prop_type() == "array">
                    <div class="w-28">
                        <select
                            class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {}", ring_color)
                            prop:value=items_type
                            on:change=move |ev| {
                                let target = ev.target().unwrap();
                                let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                let value = select.value();
                                let path = path_stored.get_value();
                                set_properties.update(move |props| {
                                    mutate_property_at_path(props, &path, |p| {
                                        p.items_type = value.clone();
                                        if p.items_type != "object" {
                                            p.nested_properties.clear();
                                        }
                                        if p.items_type != "enum" {
                                            p.enum_values.clear();
                                        }
                                        if p.items_type != "$ref" {
                                            p.ref_value.clear();
                                            p.ref_is_external = false;
                                        }
                                    });
                                });
                            }
                        >
                            <option value="string">"[string]"</option>
                            <option value="number">"[number]"</option>
                            <option value="integer">"[integer]"</option>
                            <option value="boolean">"[boolean]"</option>
                            <option value="enum">"[enum]"</option>
                            <option value="object">"[object]"</option>
                            <option value="$ref">"[$ref]"</option>
                        </select>
                    </div>
                </Show>

                // Required checkbox
                <label class="flex items-center gap-1 text-sm text-gray-600">
                    <input
                        type="checkbox"
                        class="rounded text-green-500"
                        prop:checked=required
                        on:change=move |ev| {
                            let target = ev.target().unwrap();
                            let checkbox: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            let checked = checkbox.checked();
                            let path = path_stored.get_value();
                            set_properties.update(move |props| {
                                mutate_property_at_path(props, &path, |p| p.required = checked);
                            });
                        }
                    />
                    "Required"
                </label>

                // Expand/collapse for nested
                <Show when=has_nested>
                    <button
                        type="button"
                        class="px-2 py-1 text-sm text-gray-600 hover:bg-gray-200 rounded"
                        on:click=move |_| set_expanded.update(|e| *e = !*e)
                    >
                        {move || if expanded.get() { "▼" } else { "▶" }}
                    </button>
                </Show>

                // Delete button
                <button
                    type="button"
                    class="px-2 py-1 text-sm text-red-600 hover:bg-red-50 rounded"
                    on:click=move |_| {
                        let path = path_stored.get_value();
                        set_properties.update(move |props| {
                            if path.len() == 1 {
                                // Top-level property
                                props.remove(path[0]);
                            } else {
                                // Nested property - get parent and remove from its nested_properties
                                let parent_path = &path[..path.len() - 1];
                                let idx = path[path.len() - 1];
                                if let Some(nested) = get_nested_properties_at_path_mut(props, parent_path) {
                                    nested.remove(idx);
                                }
                            }
                        });
                    }
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                    </svg>
                </button>
            </div>

            // Description field
            <div class="mt-2">
                <input
                    type="text"
                    class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {}", ring_color)
                    placeholder="Description (optional)"
                    prop:value=description
                    on:input=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                        let value = input.value();
                        let path = path_stored.get_value();
                        set_properties.update(move |props| {
                            mutate_property_at_path(props, &path, |p| p.description = value.clone());
                        });
                    }
                />
            </div>

            // Enum values input (shown when type is enum or array of enum)
            <Show when=move || prop_type() == "enum" || (prop_type() == "array" && items_type() == "enum")>
                <div class="mt-2">
                    <input
                        type="text"
                        class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {} bg-yellow-50", ring_color)
                        placeholder="Enum values (comma-separated, e.g.: low, medium, high)"
                        value=enum_values
                        on:blur=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            let value = input.value();
                            let path = path_stored.get_value();
                            set_properties.update(move |props| {
                                mutate_property_at_path(props, &path, |p| {
                                    p.enum_values = value.split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                });
                            });
                        }
                    />
                    <p class="mt-1 text-xs text-gray-500">"Enter allowed values separated by commas (parsed on blur)"</p>
                </div>
            </Show>

            // Const value input (shown when type is const)
            <Show when=move || prop_type() == "const">
                <div class="mt-2">
                    <input
                        type="text"
                        class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {} bg-purple-50", ring_color)
                        placeholder="Const value (JSON, e.g.: \"object\" or 42 or true)"
                        prop:value=move || {
                            let props = properties.get();
                            get_property_at_path(&props, &path_stored.get_value())
                                .map(|p| p.const_value.clone())
                                .unwrap_or_default()
                        }
                        on:input=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            let value = input.value();
                            let path = path_stored.get_value();
                            set_properties.update(move |props| {
                                mutate_property_at_path(props, &path, |p| p.const_value = value.clone());
                            });
                        }
                    />
                    <p class="mt-1 text-xs text-gray-500">"Enter a JSON value (string in quotes, number, boolean, null)"</p>
                </div>
            </Show>

            // $ref input (shown when type is $ref or array of $ref)
            <Show when=needs_ref_input>
                <div class="mt-2 space-y-2">
                    // Toggle between internal and external reference
                    <div class="flex items-center gap-4">
                        <label class="flex items-center gap-1 text-sm text-gray-600">
                            <input
                                type="radio"
                                name=format!("ref-type-{}", path_stored.get_value().iter().map(|i| i.to_string()).collect::<Vec<_>>().join("-"))
                                checked=move || !ref_is_external()
                                on:change=move |_| {
                                    let path = path_stored.get_value();
                                    set_properties.update(move |props| {
                                        mutate_property_at_path(props, &path, |p| {
                                            p.ref_is_external = false;
                                            p.ref_value.clear();
                                        });
                                    });
                                }
                            />
                            "Internal Schema"
                        </label>
                        <label class="flex items-center gap-1 text-sm text-gray-600">
                            <input
                                type="radio"
                                name=format!("ref-type-{}", path_stored.get_value().iter().map(|i| i.to_string()).collect::<Vec<_>>().join("-"))
                                checked=ref_is_external
                                on:change=move |_| {
                                    let path = path_stored.get_value();
                                    set_properties.update(move |props| {
                                        mutate_property_at_path(props, &path, |p| {
                                            p.ref_is_external = true;
                                            p.ref_value.clear();
                                        });
                                    });
                                }
                            />
                            "External URL"
                        </label>
                    </div>

                    // Internal schema selector (searchable dropdown)
                    <Show when=move || !ref_is_external()>
                        {
                            let local_defs = Signal::derive(move || {
                                schemas_stored.get_value()
                                    .map(|s| s.get())
                                    .unwrap_or_default()
                            });
                            let ref_value_signal = Signal::derive(ref_value);
                            let on_ref_change = Callback::new(move |new_value: String| {
                                let path = path_stored.get_value();
                                set_properties.update(move |props| {
                                    mutate_property_at_path(props, &path, |p| p.ref_value = new_value.clone());
                                });
                            });
                            view! {
                                <SearchableSchemaSelector
                                    value=ref_value_signal
                                    on_change=on_ref_change
                                    local_definitions=local_defs
                                    ring_color=ring_color
                                />
                            }
                        }
                    </Show>

                    // External URL input
                    <Show when=ref_is_external>
                        <div>
                            <input
                                type="url"
                                class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded {} bg-blue-50", ring_color)
                                placeholder="https://example.com/schemas/my-schema.json"
                                prop:value=ref_value
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    let value = input.value();
                                    let path = path_stored.get_value();
                                    set_properties.update(move |props| {
                                        mutate_property_at_path(props, &path, |p| p.ref_value = value.clone());
                                    });
                                }
                            />
                            <p class="mt-1 text-xs text-gray-500">"Enter full URL to external JSON Schema"</p>
                        </div>
                    </Show>
                </div>
            </Show>

            // Advanced options (collapsible) - shown for string, number, integer, array, object
            <Show when=move || matches!(prop_type().as_str(), "string" | "number" | "integer" | "array" | "object")>
                <details class="mt-2">
                    <summary class="text-xs text-gray-500 cursor-pointer hover:text-gray-700 select-none">"▶ Advanced Options"</summary>
                    <div class="mt-2 p-3 bg-gray-50 rounded border border-gray-200 space-y-3">

                        // String constraints: pattern and format
                        <Show when=move || prop_type() == "string">
                            <div class="grid grid-cols-2 gap-2">
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Pattern (regex)"</label>
                                    <input
                                        type="text"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        placeholder="^\\d+\\.\\d+\\.\\d+$"
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .map(|p| p.pattern.clone())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.pattern = value.clone());
                                            });
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Format"</label>
                                    <select
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .map(|p| p.format.clone())
                                                .unwrap_or_default()
                                        }
                                        on:change=move |ev| {
                                            let target = ev.target().unwrap();
                                            let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                            let value = select.value();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.format = value.clone());
                                            });
                                        }
                                    >
                                        <option value="">"(none)"</option>
                                        <option value="email">"email"</option>
                                        <option value="uri">"uri"</option>
                                        <option value="uri-reference">"uri-reference"</option>
                                        <option value="date">"date"</option>
                                        <option value="date-time">"date-time"</option>
                                        <option value="time">"time"</option>
                                        <option value="uuid">"uuid"</option>
                                        <option value="hostname">"hostname"</option>
                                        <option value="ipv4">"ipv4"</option>
                                        <option value="ipv6">"ipv6"</option>
                                        <option value="regex">"regex"</option>
                                    </select>
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-2">
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Min Length"</label>
                                    <input
                                        type="number"
                                        min="0"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .and_then(|p| p.min_length)
                                                .map(|v| v.to_string())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value().parse::<u64>().ok();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.min_length = value);
                                            });
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Max Length"</label>
                                    <input
                                        type="number"
                                        min="0"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .and_then(|p| p.max_length)
                                                .map(|v| v.to_string())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value().parse::<u64>().ok();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.max_length = value);
                                            });
                                        }
                                    />
                                </div>
                            </div>
                        </Show>

                        // Numeric constraints: minimum and maximum
                        <Show when=move || matches!(prop_type().as_str(), "number" | "integer")>
                            <div class="grid grid-cols-2 gap-2">
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Minimum"</label>
                                    <input
                                        type="number"
                                        step="any"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .and_then(|p| p.minimum)
                                                .map(|v| v.to_string())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value().parse::<f64>().ok();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.minimum = value);
                                            });
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Maximum"</label>
                                    <input
                                        type="number"
                                        step="any"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .and_then(|p| p.maximum)
                                                .map(|v| v.to_string())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value().parse::<f64>().ok();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.maximum = value);
                                            });
                                        }
                                    />
                                </div>
                            </div>
                        </Show>

                        // Array constraints: minItems, maxItems, uniqueItems
                        <Show when=move || prop_type() == "array">
                            <div class="grid grid-cols-3 gap-2">
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Min Items"</label>
                                    <input
                                        type="number"
                                        min="0"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .and_then(|p| p.min_items)
                                                .map(|v| v.to_string())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value().parse::<u64>().ok();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.min_items = value);
                                            });
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-xs text-gray-600 mb-1">"Max Items"</label>
                                    <input
                                        type="number"
                                        min="0"
                                        class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                        prop:value=move || {
                                            let props = properties.get();
                                            get_property_at_path(&props, &path_stored.get_value())
                                                .and_then(|p| p.max_items)
                                                .map(|v| v.to_string())
                                                .unwrap_or_default()
                                        }
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                            let value = input.value().parse::<u64>().ok();
                                            let path = path_stored.get_value();
                                            set_properties.update(move |props| {
                                                mutate_property_at_path(props, &path, |p| p.max_items = value);
                                            });
                                        }
                                    />
                                </div>
                                <div class="flex items-end">
                                    <label class="flex items-center gap-1 text-xs text-gray-600 pb-1">
                                        <input
                                            type="checkbox"
                                            class="rounded text-green-500"
                                            prop:checked=move || {
                                                let props = properties.get();
                                                get_property_at_path(&props, &path_stored.get_value())
                                                    .map(|p| p.unique_items)
                                                    .unwrap_or(false)
                                            }
                                            on:change=move |ev| {
                                                let target = ev.target().unwrap();
                                                let checkbox: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                let checked = checkbox.checked();
                                                let path = path_stored.get_value();
                                                set_properties.update(move |props| {
                                                    mutate_property_at_path(props, &path, |p| p.unique_items = checked);
                                                });
                                            }
                                        />
                                        "Unique Items"
                                    </label>
                                </div>
                            </div>
                        </Show>

                        // Object constraints: additionalProperties
                        <Show when=move || prop_type() == "object">
                            <div>
                                <label class="block text-xs text-gray-600 mb-1">"Additional Properties"</label>
                                <select
                                    class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                    prop:value=move || {
                                        let props = properties.get();
                                        get_property_at_path(&props, &path_stored.get_value())
                                            .map(|p| match &p.additional_properties {
                                                AdditionalProperties::Unset => "".to_string(),
                                                AdditionalProperties::Boolean(true) => "true".to_string(),
                                                AdditionalProperties::Boolean(false) => "false".to_string(),
                                                AdditionalProperties::Schema(s) => format!("$ref:{}", s),
                                            })
                                            .unwrap_or_default()
                                    }
                                    on:change=move |ev| {
                                        let target = ev.target().unwrap();
                                        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                        let value = select.value();
                                        let path = path_stored.get_value();
                                        set_properties.update(move |props| {
                                            mutate_property_at_path(props, &path, |p| {
                                                p.additional_properties = match value.as_str() {
                                                    "true" => AdditionalProperties::Boolean(true),
                                                    "false" => AdditionalProperties::Boolean(false),
                                                    s if s.starts_with("$ref:") => {
                                                        AdditionalProperties::Schema(s.strip_prefix("$ref:").unwrap_or("").to_string())
                                                    }
                                                    _ => AdditionalProperties::Unset,
                                                };
                                            });
                                        });
                                    }
                                >
                                    <option value="">"(unset - default)"</option>
                                    <option value="true">"true (allow any)"</option>
                                    <option value="false">"false (strict)"</option>
                                </select>
                                <p class="mt-1 text-xs text-gray-400">"Controls whether extra properties are allowed"</p>
                            </div>
                        </Show>

                        // Default value (for all types)
                        <div>
                            <label class="block text-xs text-gray-600 mb-1">"Default Value (JSON)"</label>
                            <input
                                type="text"
                                class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                placeholder="e.g.: \"default\" or 0 or false"
                                prop:value=move || {
                                    let props = properties.get();
                                    get_property_at_path(&props, &path_stored.get_value())
                                        .map(|p| p.default_value.clone())
                                        .unwrap_or_default()
                                }
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    let value = input.value();
                                    let path = path_stored.get_value();
                                    set_properties.update(move |props| {
                                        mutate_property_at_path(props, &path, |p| p.default_value = value.clone());
                                    });
                                }
                            />
                        </div>

                        // Deprecated flag
                        <label class="flex items-center gap-2 text-xs text-gray-600">
                            <input
                                type="checkbox"
                                class="rounded text-orange-500"
                                prop:checked=move || {
                                    let props = properties.get();
                                    get_property_at_path(&props, &path_stored.get_value())
                                        .map(|p| p.deprecated)
                                        .unwrap_or(false)
                                }
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let checkbox: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    let checked = checkbox.checked();
                                    let path = path_stored.get_value();
                                    set_properties.update(move |props| {
                                        mutate_property_at_path(props, &path, |p| p.deprecated = checked);
                                    });
                                }
                            />
                            "Deprecated"
                        </label>
                    </div>
                </details>
            </Show>

            // Nested properties for object/array[object]
            <Show when=move || has_nested() && expanded.get()>
                <div class="mt-3 pt-3 border-t border-gray-200">
                    <div class="flex justify-between items-center mb-2">
                        <span class="text-xs font-medium text-gray-500 uppercase">
                            {move || if prop_type() == "array" { "Array Item Properties" } else { "Nested Properties" }}
                        </span>
                        <button
                            type="button"
                            class=format!("px-2 py-0.5 text-xs {} rounded", btn_color)
                            on:click=move |_| {
                                let path = path_stored.get_value();
                                set_properties.update(move |props| {
                                    mutate_property_at_path(props, &path, |p| {
                                        p.nested_properties.push(SchemaProperty::new());
                                    });
                                });
                            }
                        >
                            "+ Add"
                        </button>
                    </div>
                    <div class="space-y-2">
                        <Show
                            when=move || {
                                let props = properties.get();
                                get_property_at_path(&props, &path_stored.get_value())
                                    .map(|p| !p.nested_properties.is_empty())
                                    .unwrap_or(false)
                            }
                            fallback=|| view! {
                                <div class="text-xs text-gray-400 italic p-2">
                                    "No nested properties"
                                </div>
                            }
                        >
                            <For
                                each=move || {
                                    let props = properties.get();
                                    get_property_at_path(&props, &path_stored.get_value())
                                        .map(|p| p.nested_properties.iter().enumerate().map(|(i, np)| (i, np.id)).collect::<Vec<_>>())
                                        .unwrap_or_default()
                                }
                                key=|(_, id)| *id
                                children=move |(nested_idx, _)| {
                                    let mut child_path = path_stored.get_value();
                                    child_path.push(nested_idx);
                                    view! {
                                        <RecursivePropertyEditor
                                            path=child_path
                                            depth=depth + 1
                                            properties=properties
                                            set_properties=set_properties
                                            ring_color=ring_color
                                            btn_color=btn_color
                                            available_schemas=schemas_stored.get_value()
                                        />
                                    }
                                }
                            />
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    }.into_any()
}

/// Compact schema preview component
#[component]
pub fn SchemaPreview(
    properties: ReadSignal<Vec<SchemaProperty>>,
) -> impl IntoView {
    view! {
        <details class="mt-2">
            <summary class="text-xs text-gray-500 cursor-pointer hover:text-gray-700">"View JSON Schema"</summary>
            <pre class="mt-2 p-2 bg-gray-900 text-green-400 rounded text-xs overflow-x-auto">
                {move || {
                    let props = properties.get();
                    let schema = properties_to_schema(&props);
                    serde_json::to_string_pretty(&schema).unwrap_or_default()
                }}
            </pre>
        </details>
    }
}

/// Helper to find which definitions reference a given definition name
fn find_references_to_definition(definitions: &[SchemaDefinition], target_name: &str) -> Vec<String> {
    let mut referencing = Vec::new();
    let target_ref = format!("#/definitions/{}", target_name);

    for def in definitions {
        if def.name == target_name {
            continue;
        }
        // Check if any property references this definition
        if definition_references(&def.properties, &target_ref) {
            referencing.push(def.name.clone());
        }
    }
    referencing
}

/// Helper to check if properties contain a reference to a target
fn definition_references(properties: &[SchemaProperty], target_ref: &str) -> bool {
    for prop in properties {
        // Check direct $ref
        if prop.ref_value == target_ref {
            return true;
        }
        // Check additionalProperties
        if let AdditionalProperties::Schema(ref s) = prop.additional_properties {
            if s == target_ref {
                return true;
            }
        }
        // Check nested properties recursively
        if definition_references(&prop.nested_properties, target_ref) {
            return true;
        }
        // Check oneOf/anyOf/allOf
        for variant in &prop.one_of {
            if variant.ref_value == target_ref {
                return true;
            }
        }
        for variant in &prop.any_of {
            if variant.ref_value == target_ref {
                return true;
            }
        }
        for variant in &prop.all_of {
            if variant.ref_value == target_ref {
                return true;
            }
        }
    }
    false
}

/// Helper to update all $ref values in a property when a definition is renamed
fn update_refs_in_property(prop: &mut SchemaProperty, old_ref: &str, new_ref: &str) {
    // Update direct $ref
    if prop.ref_value == old_ref {
        prop.ref_value = new_ref.to_string();
    }

    // Update additionalProperties
    if let AdditionalProperties::Schema(ref s) = prop.additional_properties {
        if s == old_ref {
            prop.additional_properties = AdditionalProperties::Schema(new_ref.to_string());
        }
    }

    // Update nested properties recursively
    for nested in prop.nested_properties.iter_mut() {
        update_refs_in_property(nested, old_ref, new_ref);
    }

    // Update oneOf/anyOf/allOf variants
    for variant in prop.one_of.iter_mut() {
        if variant.ref_value == old_ref {
            variant.ref_value = new_ref.to_string();
        }
        for nested in variant.properties.iter_mut() {
            update_refs_in_property(nested, old_ref, new_ref);
        }
    }
    for variant in prop.any_of.iter_mut() {
        if variant.ref_value == old_ref {
            variant.ref_value = new_ref.to_string();
        }
        for nested in variant.properties.iter_mut() {
            update_refs_in_property(nested, old_ref, new_ref);
        }
    }
    for variant in prop.all_of.iter_mut() {
        if variant.ref_value == old_ref {
            variant.ref_value = new_ref.to_string();
        }
        for nested in variant.properties.iter_mut() {
            update_refs_in_property(nested, old_ref, new_ref);
        }
    }
}

/// Definitions Editor component - allows editing schema definitions
#[component]
pub fn DefinitionsEditor(
    definitions: ReadSignal<Vec<SchemaDefinition>>,
    set_definitions: WriteSignal<Vec<SchemaDefinition>>,
    /// Optional: main properties to update when a definition is renamed
    #[prop(optional)]
    set_properties: Option<WriteSignal<Vec<SchemaProperty>>>,
) -> impl IntoView {
    // Store set_properties to pass to child components
    let set_props_stored = StoredValue::new(set_properties);

    let add_definition = move |_| {
        set_definitions.update(|defs| {
            let mut new_def = SchemaDefinition::new();
            new_def.name = format!("NewDefinition{}", defs.len() + 1);
            defs.push(new_def);
        });
    };

    view! {
        <div class="border border-purple-200 rounded-lg p-4 bg-purple-50 overflow-visible">
            <div class="flex justify-between items-center mb-3">
                <div>
                    <label class="block text-sm font-medium text-purple-700">"Definitions"</label>
                    <p class="text-xs text-gray-500">"Reusable schema fragments referenced with #/definitions/Name"</p>
                </div>
                <button
                    type="button"
                    class="px-3 py-1 text-sm text-purple-600 hover:bg-purple-100 rounded flex items-center gap-1"
                    on:click=add_definition
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "Add Definition"
                </button>
            </div>

            <div class="space-y-3 overflow-visible">
                <Show
                    when=move || !definitions.get().is_empty()
                    fallback=|| view! {
                        <div class="text-sm text-gray-500 italic p-3 bg-white rounded border border-purple-100">
                            "No definitions. Click 'Add Definition' to create reusable schema fragments."
                        </div>
                    }
                >
                    <For
                        each=move || {
                            definitions.get().into_iter().enumerate().collect::<Vec<_>>()
                        }
                        key=|(_, def)| def.id
                        children=move |(idx, _)| {
                            // Get the optional WriteSignal directly
                            let set_props = set_props_stored.get_value();
                            // Pass it to the component using pattern matching
                            if let Some(sp) = set_props {
                                view! {
                                    <DefinitionEditor
                                        index=idx
                                        definitions=definitions
                                        set_definitions=set_definitions
                                        set_main_properties=sp
                                    />
                                }.into_any()
                            } else {
                                view! {
                                    <DefinitionEditor
                                        index=idx
                                        definitions=definitions
                                        set_definitions=set_definitions
                                    />
                                }.into_any()
                            }
                        }
                    />
                </Show>
            </div>
        </div>
    }
}

/// Single definition editor
#[component]
fn DefinitionEditor(
    index: usize,
    definitions: ReadSignal<Vec<SchemaDefinition>>,
    set_definitions: WriteSignal<Vec<SchemaDefinition>>,
    /// Optional: main properties to update when this definition is renamed
    #[prop(optional)]
    set_main_properties: Option<WriteSignal<Vec<SchemaProperty>>>,
) -> impl IntoView {
    let (expanded, set_expanded) = signal(false);
    let (show_properties, set_show_properties) = signal(false);

    // Get current definition values
    let name = move || definitions.get().get(index).map(|d| d.name.clone()).unwrap_or_default();
    let description = move || definitions.get().get(index).map(|d| d.description.clone()).unwrap_or_default();
    let def_type = move || definitions.get().get(index).map(|d| d.def_type.clone()).unwrap_or_default();
    let enum_values = move || definitions.get().get(index).map(|d| d.enum_values.join(", ")).unwrap_or_default();
    let properties_count = move || definitions.get().get(index).map(|d| d.properties.len()).unwrap_or(0);

    // Find references to this definition
    let references = move || {
        let defs = definitions.get();
        let current_name = defs.get(index).map(|d| d.name.clone()).unwrap_or_default();
        find_references_to_definition(&defs, &current_name)
    };

    // Find what this definition references
    let references_to = move || {
        let defs = definitions.get();
        if let Some(def) = defs.get(index) {
            let mut refs = Vec::new();
            for prop in &def.properties {
                if prop.prop_type == "$ref" && prop.ref_value.starts_with("#/definitions/") {
                    if let Some(ref_name) = prop.ref_value.strip_prefix("#/definitions/") {
                        if !refs.contains(&ref_name.to_string()) {
                            refs.push(ref_name.to_string());
                        }
                    }
                }
                // Check additionalProperties
                if let AdditionalProperties::Schema(ref s) = prop.additional_properties {
                    if s.starts_with("#/definitions/") {
                        if let Some(ref_name) = s.strip_prefix("#/definitions/") {
                            if !refs.contains(&ref_name.to_string()) {
                                refs.push(ref_name.to_string());
                            }
                        }
                    }
                }
            }
            refs
        } else {
            Vec::new()
        }
    };

    view! {
        <div class="bg-white rounded-lg border border-purple-200 overflow-visible">
            // Header row
            <div class="flex items-center gap-2 p-3 bg-purple-50 border-b border-purple-100">
                <button
                    type="button"
                    class="text-purple-600 hover:text-purple-800"
                    on:click=move |_| set_expanded.update(|e| *e = !*e)
                >
                    {move || if expanded.get() { "▼" } else { "▶" }}
                </button>

                // Name input - use on:change to avoid losing focus on each keystroke
                <input
                    type="text"
                    class="flex-1 px-2 py-1 text-sm border border-purple-200 rounded font-medium focus:ring-purple-500 focus:border-purple-500"
                    placeholder="DefinitionName"
                    value=name
                    on:change=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                        let new_name = input.value();

                        // Compute old/new refs before updating
                        let old_name = definitions.get().get(index).map(|d| d.name.clone()).unwrap_or_default();
                        let old_ref = format!("#/definitions/{}", old_name);
                        let new_ref = format!("#/definitions/{}", new_name);

                        // Update definitions
                        set_definitions.update(|defs| {
                            // Update the definition name
                            if let Some(def) = defs.get_mut(index) {
                                def.name = new_name;
                            }

                            // Update all references in all definitions' properties
                            for def in defs.iter_mut() {
                                for prop in def.properties.iter_mut() {
                                    update_refs_in_property(prop, &old_ref, &new_ref);
                                }
                            }
                        });

                        // Also update main properties if provided
                        if let Some(set_props) = set_main_properties {
                            set_props.update(|props| {
                                for prop in props.iter_mut() {
                                    update_refs_in_property(prop, &old_ref, &new_ref);
                                }
                            });
                        }
                    }
                />

                // Type selector
                <select
                    class="px-2 py-1 text-sm border border-purple-200 rounded focus:ring-purple-500"
                    prop:value=def_type
                    on:change=move |ev| {
                        let target = ev.target().unwrap();
                        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                        let value = select.value();
                        set_definitions.update(|defs| {
                            if let Some(def) = defs.get_mut(index) {
                                def.def_type = value.clone();
                                if value != "enum" {
                                    def.enum_values.clear();
                                }
                                if value != "object" {
                                    def.properties.clear();
                                }
                            }
                        });
                    }
                >
                    <option value="object">"object"</option>
                    <option value="string">"string"</option>
                    <option value="number">"number"</option>
                    <option value="integer">"integer"</option>
                    <option value="boolean">"boolean"</option>
                    <option value="array">"array"</option>
                    <option value="enum">"enum"</option>
                </select>

                // Reference badge
                <code class="text-xs text-purple-600 bg-purple-100 px-2 py-0.5 rounded">
                    {move || format!("#/definitions/{}", name())}
                </code>

                // Delete button
                <button
                    type="button"
                    class="px-2 py-1 text-sm text-red-600 hover:bg-red-50 rounded"
                    on:click=move |_| {
                        set_definitions.update(|defs| {
                            defs.remove(index);
                        });
                    }
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                    </svg>
                </button>
            </div>

            // Expanded content
            <Show when=move || expanded.get()>
                <div class="p-3 space-y-3 overflow-visible">
                    // Description
                    <div>
                        <label class="block text-xs text-gray-600 mb-1">"Description"</label>
                        <input
                            type="text"
                            class="w-full px-2 py-1 text-sm border border-gray-300 rounded focus:ring-purple-500"
                            placeholder="Definition description"
                            prop:value=description
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                let value = input.value();
                                set_definitions.update(|defs| {
                                    if let Some(def) = defs.get_mut(index) {
                                        def.description = value;
                                    }
                                });
                            }
                        />
                    </div>

                    // References section
                    <div class="flex flex-wrap gap-4 text-xs">
                        // Referenced by
                        <Show when=move || !references().is_empty()>
                            <div class="flex items-center gap-1">
                                <span class="text-gray-500">"Referenced by:"</span>
                                {move || references().iter().map(|r| {
                                    let r_clone = r.clone();
                                    view! {
                                        <span class="bg-blue-100 text-blue-700 px-1.5 py-0.5 rounded">{r_clone}</span>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </Show>

                        // References to
                        <Show when=move || !references_to().is_empty()>
                            <div class="flex items-center gap-1">
                                <span class="text-gray-500">"References:"</span>
                                {move || references_to().iter().map(|r| {
                                    let r_clone = r.clone();
                                    view! {
                                        <span class="bg-green-100 text-green-700 px-1.5 py-0.5 rounded">{r_clone}</span>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </Show>
                    </div>

                    // Enum values input
                    <Show when=move || def_type() == "enum">
                        <div>
                            <label class="block text-xs text-gray-600 mb-1">"Enum Values (comma-separated)"</label>
                            <input
                                type="text"
                                class="w-full px-2 py-1 text-sm border border-gray-300 rounded bg-yellow-50 focus:ring-purple-500"
                                placeholder="value1, value2, value3"
                                value=enum_values
                                on:blur=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    let value = input.value();
                                    set_definitions.update(|defs| {
                                        if let Some(def) = defs.get_mut(index) {
                                            def.enum_values = value.split(',')
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                        }
                                    });
                                }
                            />
                        </div>
                    </Show>

                    // Properties for object type
                    <Show when=move || def_type() == "object">
                        <div class="border-t border-gray-200 pt-3 overflow-visible">
                            <div class="flex justify-between items-center mb-2">
                                <button
                                    type="button"
                                    class="text-sm font-medium text-gray-700 flex items-center gap-1"
                                    on:click=move |_| set_show_properties.update(|s| *s = !*s)
                                >
                                    {move || if show_properties.get() { "▼" } else { "▶" }}
                                    {move || format!("Properties ({})", properties_count())}
                                </button>
                                <button
                                    type="button"
                                    class="px-2 py-0.5 text-xs text-purple-600 hover:bg-purple-50 rounded"
                                    on:click=move |_| {
                                        set_definitions.update(|defs| {
                                            if let Some(def) = defs.get_mut(index) {
                                                def.properties.push(SchemaProperty::new());
                                            }
                                        });
                                        set_show_properties.set(true);
                                    }
                                >
                                    "+ Add Property"
                                </button>
                            </div>

                            <Show when=move || show_properties.get()>
                                <div class="space-y-2 ml-2 overflow-visible">
                                    {move || {
                                        if properties_count() > 0 {
                                            // Create derived signal once for all properties in this definition
                                            let available_defs_signal = Signal::derive(move || {
                                                let defs = definitions.get();
                                                let current_name = defs.get(index).map(|d| d.name.clone()).unwrap_or_default();
                                                defs.iter()
                                                    .filter(|d| d.name != current_name)
                                                    .map(|d| d.name.clone())
                                                    .collect::<Vec<_>>()
                                            });
                                            view! {
                                                <For
                                                    each=move || {
                                                        definitions.get()
                                                            .get(index)
                                                            .map(|d| d.properties.iter().enumerate().map(|(i, p)| (i, p.id)).collect::<Vec<_>>())
                                                            .unwrap_or_default()
                                                    }
                                                    key=|(_, id)| *id
                                                    children=move |(prop_idx, _)| {
                                                        view! {
                                                            <DefinitionPropertyEditor
                                                                def_index=index
                                                                prop_index=prop_idx
                                                                definitions=definitions
                                                                set_definitions=set_definitions
                                                                available_definitions=available_defs_signal
                                                            />
                                                        }
                                                    }
                                                />
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div class="text-xs text-gray-400 italic p-2">
                                                    "No properties defined"
                                                </div>
                                            }.into_any()
                                        }
                                    }}
                                </div>
                            </Show>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

/// Property editor for definition properties
#[component]
fn DefinitionPropertyEditor(
    def_index: usize,
    prop_index: usize,
    definitions: ReadSignal<Vec<SchemaDefinition>>,
    set_definitions: WriteSignal<Vec<SchemaDefinition>>,
    available_definitions: Signal<Vec<String>>,
) -> impl IntoView {
    // Get property values
    let prop_name = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.name.clone())
            .unwrap_or_default()
    };
    let prop_type = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.prop_type.clone())
            .unwrap_or_default()
    };
    let prop_desc = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.description.clone())
            .unwrap_or_default()
    };
    let prop_required = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.required)
            .unwrap_or(false)
    };
    let prop_ref = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.ref_value.clone())
            .unwrap_or_default()
    };
    let prop_pattern = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.pattern.clone())
            .unwrap_or_default()
    };
    let prop_default = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| p.default_value.clone())
            .unwrap_or_default()
    };
    let additional_props = move || {
        definitions.get()
            .get(def_index)
            .and_then(|d| d.properties.get(prop_index))
            .map(|p| match &p.additional_properties {
                AdditionalProperties::Unset => "".to_string(),
                AdditionalProperties::Boolean(true) => "true".to_string(),
                AdditionalProperties::Boolean(false) => "false".to_string(),
                AdditionalProperties::Schema(s) => s.clone(),
            })
            .unwrap_or_default()
    };

    let (show_advanced, set_show_advanced) = signal(false);

    view! {
        <div class="bg-gray-50 rounded border border-gray-200 p-2 overflow-visible">
            <div class="flex flex-wrap gap-2 items-center overflow-visible">
                // Property name - use on:change to avoid losing focus on each keystroke
                <input
                    type="text"
                    class="flex-1 min-w-[100px] px-2 py-1 text-xs border border-gray-300 rounded focus:ring-purple-500"
                    placeholder="propertyName"
                    value=prop_name
                    on:change=move |ev| {
                        let target = ev.target().unwrap();
                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                        let value = input.value();
                        set_definitions.update(|defs| {
                            if let Some(def) = defs.get_mut(def_index) {
                                if let Some(prop) = def.properties.get_mut(prop_index) {
                                    prop.name = value;
                                }
                            }
                        });
                    }
                />

                // Type selector
                <select
                    class="px-2 py-1 text-xs border border-gray-300 rounded focus:ring-purple-500"
                    prop:value=prop_type
                    on:change=move |ev| {
                        let target = ev.target().unwrap();
                        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                        let value = select.value();
                        set_definitions.update(|defs| {
                            if let Some(def) = defs.get_mut(def_index) {
                                if let Some(prop) = def.properties.get_mut(prop_index) {
                                    prop.prop_type = value.clone();
                                    if value != "$ref" {
                                        prop.ref_value.clear();
                                    }
                                }
                            }
                        });
                    }
                >
                    <option value="string">"string"</option>
                    <option value="number">"number"</option>
                    <option value="integer">"integer"</option>
                    <option value="boolean">"boolean"</option>
                    <option value="array">"array"</option>
                    <option value="object">"object"</option>
                    <option value="$ref">"$ref"</option>
                </select>

                // $ref selector when type is $ref (searchable)
                <Show when=move || prop_type() == "$ref">
                    {
                        // Convert available_definitions to the format expected by SearchableSchemaSelector
                        let local_defs = Signal::derive(move || {
                            available_definitions.get().into_iter()
                                .map(|name| format!("#/definitions/{}", name))
                                .collect::<Vec<_>>()
                        });
                        let prop_ref_signal = Signal::derive(prop_ref);
                        let on_ref_change = Callback::new(move |new_value: String| {
                            set_definitions.update(|defs| {
                                if let Some(def) = defs.get_mut(def_index) {
                                    if let Some(prop) = def.properties.get_mut(prop_index) {
                                        prop.ref_value = new_value;
                                    }
                                }
                            });
                        });
                        view! {
                            <div class="flex-1 min-w-[120px]">
                                <SearchableSchemaSelector
                                    value=prop_ref_signal
                                    on_change=on_ref_change
                                    local_definitions=local_defs
                                    ring_color="focus:ring-purple-500"
                                />
                            </div>
                        }
                    }
                </Show>

                // Required checkbox
                <label class="flex items-center gap-1 text-xs text-gray-600">
                    <input
                        type="checkbox"
                        class="rounded text-purple-500"
                        prop:checked=prop_required
                        on:change=move |ev| {
                            let target = ev.target().unwrap();
                            let checkbox: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            let checked = checkbox.checked();
                            set_definitions.update(|defs| {
                                if let Some(def) = defs.get_mut(def_index) {
                                    if let Some(prop) = def.properties.get_mut(prop_index) {
                                        prop.required = checked;
                                    }
                                }
                            });
                        }
                    />
                    "Req"
                </label>

                // Advanced toggle
                <button
                    type="button"
                    class="text-xs text-gray-500 hover:text-gray-700"
                    on:click=move |_| set_show_advanced.update(|s| *s = !*s)
                >
                    {move || if show_advanced.get() { "▼ Less" } else { "▶ More" }}
                </button>

                // Delete button
                <button
                    type="button"
                    class="text-red-500 hover:text-red-700"
                    on:click=move |_| {
                        set_definitions.update(|defs| {
                            if let Some(def) = defs.get_mut(def_index) {
                                def.properties.remove(prop_index);
                            }
                        });
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>

            // Advanced options
            <Show when=move || show_advanced.get()>
                <div class="mt-2 pt-2 border-t border-gray-200 space-y-2">
                    // Description - use on:change to avoid losing focus
                    <input
                        type="text"
                        class="w-full px-2 py-1 text-xs border border-gray-300 rounded focus:ring-purple-500"
                        placeholder="Description"
                        value=prop_desc
                        on:change=move |ev| {
                            let target = ev.target().unwrap();
                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                            let value = input.value();
                            set_definitions.update(|defs| {
                                if let Some(def) = defs.get_mut(def_index) {
                                    if let Some(prop) = def.properties.get_mut(prop_index) {
                                        prop.description = value;
                                    }
                                }
                            });
                        }
                    />

                    <div class="grid grid-cols-2 gap-2">
                        // Pattern - use on:change to avoid losing focus
                        <Show when=move || prop_type() == "string">
                            <input
                                type="text"
                                class="px-2 py-1 text-xs border border-gray-300 rounded focus:ring-purple-500"
                                placeholder="Pattern (regex)"
                                value=prop_pattern
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                    let value = input.value();
                                    set_definitions.update(|defs| {
                                        if let Some(def) = defs.get_mut(def_index) {
                                            if let Some(prop) = def.properties.get_mut(prop_index) {
                                                prop.pattern = value;
                                            }
                                        }
                                    });
                                }
                            />
                        </Show>

                        // Default value - use on:change to avoid losing focus
                        <input
                            type="text"
                            class="px-2 py-1 text-xs border border-gray-300 rounded focus:ring-purple-500"
                            placeholder="Default (JSON)"
                            value=prop_default
                            on:change=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                let value = input.value();
                                set_definitions.update(|defs| {
                                    if let Some(def) = defs.get_mut(def_index) {
                                        if let Some(prop) = def.properties.get_mut(prop_index) {
                                            prop.default_value = value;
                                        }
                                    }
                                });
                            }
                        />
                    </div>

                    // Additional properties for object type
                    <Show when=move || prop_type() == "object">
                        <div>
                            <label class="block text-xs text-gray-500 mb-1">"additionalProperties"</label>
                            <select
                                class="w-full px-2 py-1 text-xs border border-gray-300 rounded focus:ring-purple-500"
                                prop:value=additional_props
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                    let value = select.value();
                                    set_definitions.update(|defs| {
                                        if let Some(def) = defs.get_mut(def_index) {
                                            if let Some(prop) = def.properties.get_mut(prop_index) {
                                                prop.additional_properties = match value.as_str() {
                                                    "true" => AdditionalProperties::Boolean(true),
                                                    "false" => AdditionalProperties::Boolean(false),
                                                    s if s.starts_with("#/definitions/") => {
                                                        AdditionalProperties::Schema(s.to_string())
                                                    }
                                                    _ => AdditionalProperties::Unset,
                                                };
                                            }
                                        }
                                    });
                                }
                            >
                                <option value="">"(unset)"</option>
                                <option value="true">"true"</option>
                                <option value="false">"false"</option>
                                {move || available_definitions.get().into_iter().map(|def_name| {
                                    let ref_value = format!("#/definitions/{}", def_name);
                                    let label = format!("$ref: {}", def_name);
                                    view! {
                                        <option value=ref_value>{label}</option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

/// Full Schema Editor - unified component with JSON mode, metadata, definitions, and properties
/// This is the recommended component for editing JSON Schemas in all archetypes
#[component]
pub fn FullSchemaEditor(
    /// Label for the schema section (e.g., "Input Schema", "Output Schema")
    #[prop(into)]
    label: String,
    /// Color theme: "green", "blue", "teal", "purple", etc.
    #[prop(into, default = "green".to_string())]
    color: String,
    /// The complete JSON schema value
    schema: ReadSignal<Value>,
    /// Setter for the complete JSON schema value
    set_schema: WriteSignal<Value>,
    /// Whether to show the definitions section (default: true)
    #[prop(default = true)]
    show_definitions: bool,
    /// Whether to show the metadata section (default: false for inline schemas)
    #[prop(default = false)]
    show_metadata_section: bool,
    /// Optional description text
    #[prop(optional, into)]
    description: Option<String>,
) -> impl IntoView {
    // Internal state
    let (json_mode, set_json_mode) = signal(false);
    let (json_text, set_json_text) = signal(String::new());
    let (properties, set_properties) = signal(Vec::<SchemaProperty>::new());
    let (metadata, set_metadata) = signal(SchemaMetadata::default());
    let (definitions, set_definitions) = signal(Vec::<SchemaDefinition>::new());
    let (show_meta_panel, set_show_meta_panel) = signal(false);

    // Initialize from schema
    Effect::new(move |prev: Option<bool>| {
        if prev.is_none() {
            let s = schema.get();
            set_properties.set(schema_to_properties(&s));
            set_metadata.set(schema_to_metadata(&s));
            set_definitions.set(schema_to_definitions(&s));
            set_json_text.set(serde_json::to_string_pretty(&s).unwrap_or_default());
            // Auto-expand metadata if it has content
            let meta = schema_to_metadata(&s);
            if !meta.schema_uri.is_empty() || !meta.id.is_empty() || !meta.title.is_empty() {
                set_show_meta_panel.set(true);
            }
        }
        true
    });

    // Create a derived signal for available definition names (formatted as $ref values)
    let (available_def_refs, set_available_def_refs) = signal(Vec::<String>::new());
    Effect::new(move |_| {
        let defs = definitions.get();
        let refs: Vec<String> = defs.iter()
            .map(|d| format!("#/definitions/{}", d.name))
            .collect();
        set_available_def_refs.set(refs);
    });

    // Sync changes back to schema
    let sync_to_schema = move || {
        if !json_mode.get() {
            let new_schema = if show_definitions {
                full_schema_to_value(&metadata.get(), &definitions.get(), &properties.get())
            } else {
                properties_to_schema(&properties.get())
            };
            set_schema.set(new_schema);
        }
    };

    // Watch for property changes
    Effect::new(move |_| {
        let _ = properties.get();
        let _ = definitions.get();
        let _ = metadata.get();
        if !json_mode.get() {
            sync_to_schema();
        }
    });

    // Convert color to static str to use in closures
    let color_static: &'static str = match color.as_str() {
        "green" => "green",
        "blue" => "blue",
        "teal" => "teal",
        "purple" => "purple",
        "orange" => "orange",
        _ => "green",
    };

    let ring_color = match color_static {
        "green" => "focus:ring-green-500",
        "blue" => "focus:ring-blue-500",
        "teal" => "focus:ring-teal-500",
        "purple" => "focus:ring-purple-500",
        "orange" => "focus:ring-orange-500",
        _ => "focus:ring-green-500",
    };

    let border_color = match color_static {
        "green" => "border-green-200",
        "blue" => "border-blue-200",
        "teal" => "border-teal-200",
        "purple" => "border-purple-200",
        "orange" => "border-orange-200",
        _ => "border-green-200",
    };

    let label_clone = label.clone();

    view! {
        <div class=format!("border {} rounded-lg p-4", border_color)>
            // Header with label and mode toggle
            <div class="flex justify-between items-center mb-3">
                <div>
                    <label class="block text-sm font-medium text-gray-700">{label_clone}</label>
                    {description.map(|d| view! {
                        <p class="text-xs text-gray-500">{d}</p>
                    })}
                </div>
                <div class="flex bg-gray-100 rounded-lg p-0.5">
                    <button
                        type="button"
                        class=move || format!(
                            "px-2 py-1 rounded text-xs font-medium transition-colors {}",
                            if !json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                        )
                        on:click=move |_| {
                            if json_mode.get() {
                                // Parse JSON and update properties
                                if let Ok(val) = serde_json::from_str::<Value>(&json_text.get()) {
                                    set_properties.set(schema_to_properties(&val));
                                    set_metadata.set(schema_to_metadata(&val));
                                    set_definitions.set(schema_to_definitions(&val));
                                    set_schema.set(val);
                                }
                            }
                            set_json_mode.set(false);
                        }
                    >
                        "Visual"
                    </button>
                    <button
                        type="button"
                        class=move || format!(
                            "px-2 py-1 rounded text-xs font-medium transition-colors {}",
                            if json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                        )
                        on:click=move |_| {
                            if !json_mode.get() {
                                // Serialize to JSON
                                let s = if show_definitions {
                                    full_schema_to_value(&metadata.get(), &definitions.get(), &properties.get())
                                } else {
                                    properties_to_schema(&properties.get())
                                };
                                set_json_text.set(serde_json::to_string_pretty(&s).unwrap_or_default());
                            }
                            set_json_mode.set(true);
                        }
                    >
                        "JSON"
                    </button>
                </div>
            </div>

            // Content area - use if/else to avoid lifetime issues with fallback closures
            {move || {
                if json_mode.get() {
                    view! {
                        <textarea
                            class=format!("w-full h-64 px-3 py-2 border border-gray-300 rounded-lg font-mono text-xs {}", ring_color)
                            placeholder=r#"{"type": "object", "properties": { ... }}"#
                            prop:value=move || json_text.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                let value = textarea.value();
                                set_json_text.set(value.clone());
                                // Try to parse and update schema
                                if let Ok(val) = serde_json::from_str::<Value>(&value) {
                                    set_schema.set(val);
                                }
                            }
                        />
                    }.into_any()
                } else {
                    view! {
                        <div>
                            // Metadata section (optional)
                            <Show when=move || show_metadata_section>
                                <details class="mb-3" open=show_meta_panel>
                                    <summary
                                        class="text-xs font-medium text-gray-600 cursor-pointer hover:text-gray-800 select-none"
                                        on:click=move |_| set_show_meta_panel.update(|v| *v = !*v)
                                    >
                                        "▶ Schema Metadata"
                                    </summary>
                                    <div class="mt-2 p-2 bg-blue-50 rounded border border-blue-200 space-y-2">
                                        <div class="grid grid-cols-2 gap-2">
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"$schema"</label>
                                                <select
                                                    class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                                    prop:value=move || metadata.get().schema_uri
                                                    on:change=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                        set_metadata.update(|m| m.schema_uri = select.value());
                                                    }
                                                >
                                                    <option value="">"(none)"</option>
                                                    <option value="http://json-schema.org/draft-07/schema#">"Draft-07"</option>
                                                    <option value="https://json-schema.org/draft/2019-09/schema">"Draft 2019-09"</option>
                                                    <option value="https://json-schema.org/draft/2020-12/schema">"Draft 2020-12"</option>
                                                </select>
                                            </div>
                                            <div>
                                                <label class="block text-xs text-gray-600 mb-1">"Title"</label>
                                                <input
                                                    type="text"
                                                    class=format!("w-full px-2 py-1 text-xs border border-gray-300 rounded {}", ring_color)
                                                    placeholder="Schema Title"
                                                    prop:value=move || metadata.get().title
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                        set_metadata.update(|m| m.title = input.value());
                                                    }
                                                />
                                            </div>
                                        </div>
                                    </div>
                                </details>
                            </Show>

                            // Definitions section (optional)
                            <Show when=move || show_definitions>
                                <div class="mb-3">
                                    <DefinitionsEditor
                                        definitions=definitions
                                        set_definitions=set_definitions
                                        set_properties=set_properties
                                    />
                                </div>
                            </Show>

                            // Properties editor
                            <JsonSchemaEditor
                                properties=properties
                                set_properties=set_properties
                                label="Properties"
                                color=color_static
                                available_schemas=available_def_refs
                            />

                            // Preview
                            <SchemaPreview properties=properties />
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
