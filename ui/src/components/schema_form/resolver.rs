//! JSON Schema resolution with $ref support
//!
//! Resolves JSON Schema definitions into ResolvedSchemaNode tree,
//! handling $ref references, oneOf/anyOf, and nested structures.

use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

use super::types::{FakerType, FakeStrategyExtendedConfig, ResolvedSchemaNode, SchemaNodeType};

// ============================================================================
// Resolution Context
// ============================================================================

/// Context for schema resolution, carrying available definitions
pub struct SchemaResolutionContext {
    /// Local definitions from current schema (#/$defs/* or #/definitions/*)
    pub definitions: HashMap<String, Value>,
    /// Track visited refs to detect cycles
    visited_refs: HashSet<String>,
    /// Maximum recursion depth
    pub max_depth: usize,
}

impl Default for SchemaResolutionContext {
    fn default() -> Self {
        Self {
            definitions: HashMap::new(),
            visited_refs: HashSet::new(),
            max_depth: 20,
        }
    }
}

impl SchemaResolutionContext {
    /// Create a new context with definitions extracted from a schema
    pub fn from_schema(schema: &Value) -> Self {
        let mut ctx = Self::default();

        // Extract definitions from schema
        if let Some(defs) = schema.get("definitions").or_else(|| schema.get("$defs")) {
            if let Some(defs_obj) = defs.as_object() {
                for (name, def) in defs_obj {
                    ctx.definitions.insert(name.clone(), def.clone());
                }
            }
        }

        ctx
    }

    /// Check if a ref has been visited (cycle detection)
    fn enter_ref(&mut self, ref_path: &str) -> bool {
        if self.visited_refs.contains(ref_path) {
            return false; // Cycle detected
        }
        self.visited_refs.insert(ref_path.to_string());
        true
    }

    /// Mark ref as no longer being processed
    fn exit_ref(&mut self, ref_path: &str) {
        self.visited_refs.remove(ref_path);
    }
}

// ============================================================================
// Schema Resolution
// ============================================================================

/// Resolve a JSON Schema Value into a ResolvedSchemaNode tree
pub fn resolve_schema(schema: &Value, ctx: &mut SchemaResolutionContext, depth: usize) -> ResolvedSchemaNode {
    if depth > ctx.max_depth {
        return ResolvedSchemaNode {
            node_type: SchemaNodeType::String,
            description: Some("Max depth exceeded".to_string()),
            ..Default::default()
        };
    }

    // Handle $ref first
    if let Some(ref_value) = schema.get("$ref").and_then(|v| v.as_str()) {
        return resolve_ref(ref_value, ctx, depth);
    }

    // Extract common properties
    let mut node = extract_common_props(schema);

    // Check for oneOf
    if let Some(one_of) = schema.get("oneOf").and_then(|v| v.as_array()) {
        let variants: Vec<ResolvedSchemaNode> = one_of
            .iter()
            .map(|v| resolve_schema(v, ctx, depth + 1))
            .collect();
        let labels = generate_variant_labels(&variants);
        node.node_type = SchemaNodeType::OneOf { variants, labels };
        return node;
    }

    // Check for anyOf
    if let Some(any_of) = schema.get("anyOf").and_then(|v| v.as_array()) {
        let variants: Vec<ResolvedSchemaNode> = any_of
            .iter()
            .map(|v| resolve_schema(v, ctx, depth + 1))
            .collect();
        let labels = generate_variant_labels(&variants);
        node.node_type = SchemaNodeType::AnyOf { variants, labels };
        return node;
    }

    // Check for const
    if let Some(const_val) = schema.get("const") {
        node.node_type = SchemaNodeType::Const(const_val.clone());
        return node;
    }

    // Check for enum (before type handling)
    if let Some(enum_values) = schema.get("enum").and_then(|v| v.as_array()) {
        let values: Vec<String> = enum_values
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect();
        node.enum_values = values.clone();
        node.node_type = SchemaNodeType::Enum(values);
        return node;
    }

    // Handle type-based resolution
    let type_str = schema
        .get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("object");

    node.node_type = match type_str {
        "null" => SchemaNodeType::Null,
        "boolean" => SchemaNodeType::Boolean,
        "integer" => SchemaNodeType::Integer,
        "number" => SchemaNodeType::Number,
        "string" => SchemaNodeType::String,
        "array" => resolve_array_type(schema, ctx, depth),
        "object" => resolve_object_type(schema, ctx, depth),
        _ => SchemaNodeType::String, // Default fallback
    };

    node
}

/// Resolve a $ref reference
fn resolve_ref(ref_value: &str, ctx: &mut SchemaResolutionContext, depth: usize) -> ResolvedSchemaNode {
    // Check for cycles
    if !ctx.enter_ref(ref_value) {
        return ResolvedSchemaNode {
            node_type: SchemaNodeType::String,
            description: Some(format!("Circular reference: {}", ref_value)),
            ..Default::default()
        };
    }

    let result = if ref_value.starts_with("#/definitions/") {
        let def_name = ref_value.strip_prefix("#/definitions/").unwrap();
        resolve_definition(def_name, ctx, depth)
    } else if ref_value.starts_with("#/$defs/") {
        let def_name = ref_value.strip_prefix("#/$defs/").unwrap();
        resolve_definition(def_name, ctx, depth)
    } else {
        // Unknown ref format
        ResolvedSchemaNode {
            node_type: SchemaNodeType::String,
            description: Some(format!("Unknown ref: {}", ref_value)),
            ..Default::default()
        }
    };

    ctx.exit_ref(ref_value);
    result
}

/// Resolve a definition by name
fn resolve_definition(def_name: &str, ctx: &mut SchemaResolutionContext, depth: usize) -> ResolvedSchemaNode {
    if let Some(def_schema) = ctx.definitions.get(def_name).cloned() {
        resolve_schema(&def_schema, ctx, depth + 1)
    } else {
        ResolvedSchemaNode {
            node_type: SchemaNodeType::String,
            description: Some(format!("Definition not found: {}", def_name)),
            ..Default::default()
        }
    }
}

/// Resolve array type
fn resolve_array_type(schema: &Value, ctx: &mut SchemaResolutionContext, depth: usize) -> SchemaNodeType {
    let items = if let Some(items_schema) = schema.get("items") {
        Box::new(resolve_schema(items_schema, ctx, depth + 1))
    } else {
        Box::new(ResolvedSchemaNode::default())
    };

    let min_items = schema.get("minItems").and_then(|v| v.as_u64());
    let max_items = schema.get("maxItems").and_then(|v| v.as_u64());
    let unique_items = schema
        .get("uniqueItems")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    SchemaNodeType::Array {
        items,
        min_items,
        max_items,
        unique_items,
    }
}

/// Resolve object type
fn resolve_object_type(schema: &Value, ctx: &mut SchemaResolutionContext, depth: usize) -> SchemaNodeType {
    let mut properties = Vec::new();

    // Get required fields
    let required: HashSet<String> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // Process properties
    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
        for (name, prop_schema) in props {
            let mut resolved = resolve_schema(prop_schema, ctx, depth + 1);
            resolved.name = Some(name.clone());
            resolved.required = required.contains(name);
            properties.push((name.clone(), resolved));
        }
    }

    // Handle additionalProperties
    let additional_properties = schema.get("additionalProperties").and_then(|v| {
        if v.is_boolean() {
            None // We don't need to track boolean additionalProperties for form generation
        } else {
            Some(Box::new(resolve_schema(v, ctx, depth + 1)))
        }
    });

    SchemaNodeType::Object {
        properties,
        additional_properties,
    }
}

/// Extract common properties from a schema
fn extract_common_props(schema: &Value) -> ResolvedSchemaNode {
    // Parse x-fake-strategy extension attribute (supports both string and object form)
    let (fake_strategy, fake_strategy_config) = if let Some(strategy_val) = schema.get("x-fake-strategy") {
        match strategy_val {
            // Simple string form: "email"
            Value::String(s) => (FakerType::from_strategy_string(s), None),
            // Object form: { "type": "pattern", "pattern": "[A-Z]+" }
            Value::Object(obj) => {
                let strategy = obj.get("type")
                    .and_then(|v| v.as_str())
                    .and_then(FakerType::from_strategy_string);
                let ext_config = FakeStrategyExtendedConfig {
                    pattern: obj.get("pattern").and_then(|v| v.as_str()).map(String::from),
                    min: obj.get("min").and_then(|v| v.as_f64()),
                    max: obj.get("max").and_then(|v| v.as_f64()),
                    constant: obj.get("constant").cloned(),
                };
                // Only include extended config if at least one field is set
                let has_config = ext_config.pattern.is_some()
                    || ext_config.min.is_some()
                    || ext_config.max.is_some()
                    || ext_config.constant.is_some();
                (strategy, if has_config { Some(ext_config) } else { None })
            }
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    ResolvedSchemaNode {
        node_type: SchemaNodeType::String, // Will be overwritten
        name: None,
        description: schema.get("description").and_then(|v| v.as_str()).map(String::from),
        required: false,
        format: schema.get("format").and_then(|v| v.as_str()).map(String::from),
        pattern: schema.get("pattern").and_then(|v| v.as_str()).map(String::from),
        minimum: schema.get("minimum").and_then(|v| v.as_f64()),
        maximum: schema.get("maximum").and_then(|v| v.as_f64()),
        min_length: schema.get("minLength").and_then(|v| v.as_u64()),
        max_length: schema.get("maxLength").and_then(|v| v.as_u64()),
        enum_values: schema
            .get("enum")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        default: schema.get("default").cloned(),
        examples: schema
            .get("examples")
            .and_then(|v| v.as_array())
            .map(|arr| arr.clone())
            .unwrap_or_default(),
        fake_strategy,
        fake_strategy_config,
    }
}

/// Generate labels for oneOf/anyOf variants
fn generate_variant_labels(variants: &[ResolvedSchemaNode]) -> Vec<String> {
    variants
        .iter()
        .enumerate()
        .map(|(i, v)| {
            // Try to use description, const value, or type name
            if let Some(desc) = &v.description {
                desc.clone()
            } else if let SchemaNodeType::Const(val) = &v.node_type {
                match val {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                }
            } else {
                format!("Option {}", i + 1)
            }
        })
        .collect()
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if a schema has properties (is a non-empty object schema)
#[allow(dead_code)]
pub fn has_properties(schema: &Value) -> bool {
    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
        !props.is_empty()
    } else {
        false
    }
}

/// Convert a ResolvedSchemaNode back to a JSON Schema Value (for debugging)
#[allow(dead_code)]
pub fn schema_node_to_json(node: &ResolvedSchemaNode) -> Value {
    let mut obj = Map::new();

    match &node.node_type {
        SchemaNodeType::Null => {
            obj.insert("type".to_string(), Value::String("null".to_string()));
        }
        SchemaNodeType::Boolean => {
            obj.insert("type".to_string(), Value::String("boolean".to_string()));
        }
        SchemaNodeType::Integer => {
            obj.insert("type".to_string(), Value::String("integer".to_string()));
        }
        SchemaNodeType::Number => {
            obj.insert("type".to_string(), Value::String("number".to_string()));
        }
        SchemaNodeType::String => {
            obj.insert("type".to_string(), Value::String("string".to_string()));
        }
        SchemaNodeType::Enum(values) => {
            obj.insert("type".to_string(), Value::String("string".to_string()));
            obj.insert(
                "enum".to_string(),
                Value::Array(values.iter().map(|v| Value::String(v.clone())).collect()),
            );
        }
        SchemaNodeType::Const(val) => {
            obj.insert("const".to_string(), val.clone());
        }
        SchemaNodeType::Array { items, min_items, max_items, .. } => {
            obj.insert("type".to_string(), Value::String("array".to_string()));
            obj.insert("items".to_string(), schema_node_to_json(items));
            if let Some(min) = min_items {
                obj.insert("minItems".to_string(), Value::Number((*min).into()));
            }
            if let Some(max) = max_items {
                obj.insert("maxItems".to_string(), Value::Number((*max).into()));
            }
        }
        SchemaNodeType::Object { properties, .. } => {
            obj.insert("type".to_string(), Value::String("object".to_string()));
            let mut props = Map::new();
            let mut required = Vec::new();
            for (name, prop) in properties {
                props.insert(name.clone(), schema_node_to_json(prop));
                if prop.required {
                    required.push(Value::String(name.clone()));
                }
            }
            obj.insert("properties".to_string(), Value::Object(props));
            if !required.is_empty() {
                obj.insert("required".to_string(), Value::Array(required));
            }
        }
        SchemaNodeType::OneOf { variants, .. } => {
            obj.insert(
                "oneOf".to_string(),
                Value::Array(variants.iter().map(schema_node_to_json).collect()),
            );
        }
        SchemaNodeType::AnyOf { variants, .. } => {
            obj.insert(
                "anyOf".to_string(),
                Value::Array(variants.iter().map(schema_node_to_json).collect()),
            );
        }
    }

    // Add common properties
    if let Some(desc) = &node.description {
        obj.insert("description".to_string(), Value::String(desc.clone()));
    }
    if let Some(format) = &node.format {
        obj.insert("format".to_string(), Value::String(format.clone()));
    }
    if let Some(pattern) = &node.pattern {
        obj.insert("pattern".to_string(), Value::String(pattern.clone()));
    }

    Value::Object(obj)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolve_simple_object() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        });

        let mut ctx = SchemaResolutionContext::from_schema(&schema);
        let resolved = resolve_schema(&schema, &mut ctx, 0);

        assert!(resolved.is_object());
        if let SchemaNodeType::Object { properties, .. } = &resolved.node_type {
            assert_eq!(properties.len(), 2);
            assert!(properties.iter().any(|(n, p)| n == "name" && p.required));
            assert!(properties.iter().any(|(n, p)| n == "age" && !p.required));
        }
    }

    #[test]
    fn test_resolve_with_ref() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": { "$ref": "#/definitions/User" }
            },
            "definitions": {
                "User": {
                    "type": "object",
                    "properties": {
                        "email": { "type": "string", "format": "email" }
                    }
                }
            }
        });

        let mut ctx = SchemaResolutionContext::from_schema(&schema);
        let resolved = resolve_schema(&schema, &mut ctx, 0);

        if let SchemaNodeType::Object { properties, .. } = &resolved.node_type {
            let (_, user_prop) = properties.iter().find(|(n, _)| n == "user").unwrap();
            assert!(user_prop.is_object());
        }
    }

    #[test]
    fn test_resolve_array() {
        let schema = json!({
            "type": "array",
            "items": { "type": "string" },
            "minItems": 1,
            "maxItems": 10
        });

        let mut ctx = SchemaResolutionContext::default();
        let resolved = resolve_schema(&schema, &mut ctx, 0);

        if let SchemaNodeType::Array { items, min_items, max_items, .. } = &resolved.node_type {
            assert!(matches!(items.node_type, SchemaNodeType::String));
            assert_eq!(*min_items, Some(1));
            assert_eq!(*max_items, Some(10));
        } else {
            panic!("Expected array type");
        }
    }

    #[test]
    fn test_resolve_one_of() {
        let schema = json!({
            "oneOf": [
                { "type": "string" },
                { "type": "number" }
            ]
        });

        let mut ctx = SchemaResolutionContext::default();
        let resolved = resolve_schema(&schema, &mut ctx, 0);

        if let SchemaNodeType::OneOf { variants, labels } = &resolved.node_type {
            assert_eq!(variants.len(), 2);
            assert_eq!(labels.len(), 2);
        } else {
            panic!("Expected oneOf type");
        }
    }
}
