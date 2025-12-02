//! Core types for schema-driven form generation

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Form Mode
// ============================================================================

/// Mode for schema form generation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaFormMode {
    /// Static mode: Users enter actual values for each property
    StaticValue,
    /// Faker mode: Users configure faker generators per property
    FakerConfig,
}

// ============================================================================
// Property Path
// ============================================================================

/// Segment of a property path
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Object property access: .fieldName
    Property(String),
    /// Array index access: [0], [1], etc.
    Index(usize),
    /// Array wildcard for faker configs: [*]
    ArrayWildcard,
}

/// Property path for nested access (e.g., "user.address.city" or "items[0].name")
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PropertyPath {
    segments: Vec<PathSegment>,
}

#[allow(dead_code)]
impl PropertyPath {
    /// Create a root path (empty)
    pub fn root() -> Self {
        Self { segments: vec![] }
    }

    /// Check if this is the root path
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the depth (number of segments)
    pub fn depth(&self) -> usize {
        self.segments.len()
    }

    /// Push a property segment
    pub fn push_property(&self, name: &str) -> Self {
        let mut new = self.clone();
        new.segments.push(PathSegment::Property(name.to_string()));
        new
    }

    /// Push an array index segment
    pub fn push_index(&self, idx: usize) -> Self {
        let mut new = self.clone();
        new.segments.push(PathSegment::Index(idx));
        new
    }

    /// Push an array wildcard segment (for faker configs)
    pub fn push_wildcard(&self) -> Self {
        let mut new = self.clone();
        new.segments.push(PathSegment::ArrayWildcard);
        new
    }

    /// Get the last segment
    pub fn last(&self) -> Option<&PathSegment> {
        self.segments.last()
    }

    /// Get the parent path (without the last segment)
    pub fn parent(&self) -> Self {
        let mut new = self.clone();
        new.segments.pop();
        new
    }

    /// Get segments iterator
    pub fn segments(&self) -> impl Iterator<Item = &PathSegment> {
        self.segments.iter()
    }

    /// Convert to dot-notation string: "user.address.city" or "items[*].name"
    pub fn to_string(&self) -> String {
        self.segments
            .iter()
            .enumerate()
            .map(|(i, seg)| match seg {
                PathSegment::Property(name) => {
                    if i == 0 {
                        name.clone()
                    } else {
                        format!(".{}", name)
                    }
                }
                PathSegment::Index(idx) => format!("[{}]", idx),
                PathSegment::ArrayWildcard => "[*]".to_string(),
            })
            .collect()
    }

    /// Parse a path string into PropertyPath
    pub fn parse(s: &str) -> Self {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Property(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Property(current.clone()));
                        current.clear();
                    }
                    // Read until ]
                    let mut index_str = String::new();
                    while let Some(&c) = chars.peek() {
                        if c == ']' {
                            chars.next();
                            break;
                        }
                        index_str.push(chars.next().unwrap());
                    }
                    if index_str == "*" {
                        segments.push(PathSegment::ArrayWildcard);
                    } else if let Ok(idx) = index_str.parse::<usize>() {
                        segments.push(PathSegment::Index(idx));
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.is_empty() {
            segments.push(PathSegment::Property(current));
        }

        Self { segments }
    }
}

impl fmt::Display for PropertyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

// ============================================================================
// Faker Configuration
// ============================================================================

/// Faker generator types (must match backend FakerFieldType)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FakerType {
    // Personal
    #[default]
    #[serde(alias = "name")]
    FullName,
    FirstName,
    LastName,
    Username,

    // Contact
    Email,
    Phone,

    // Address
    StreetAddress,
    City,
    State,
    Country,
    PostalCode,

    // Identifiers
    Uuid,

    // Text
    Word,
    Sentence,
    Paragraph,
    Lorem,

    // Numbers
    Integer,
    Float,

    // Special
    /// Pick from enum values
    #[serde(rename = "enum")]
    EnumValue,
    /// Fixed/constant value
    Constant,
    /// Pattern-based generation (regex-like)
    Pattern,
}

impl FakerType {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            FakerType::FullName => "Full Name",
            FakerType::FirstName => "First Name",
            FakerType::LastName => "Last Name",
            FakerType::Username => "Username",
            FakerType::Email => "Email",
            FakerType::Phone => "Phone Number",
            FakerType::StreetAddress => "Street Address",
            FakerType::City => "City",
            FakerType::State => "State/Province",
            FakerType::Country => "Country",
            FakerType::PostalCode => "Postal Code",
            FakerType::Uuid => "UUID",
            FakerType::Word => "Word",
            FakerType::Sentence => "Sentence",
            FakerType::Paragraph => "Paragraph",
            FakerType::Lorem => "Lorem Text",
            FakerType::Integer => "Integer",
            FakerType::Float => "Float Number",
            FakerType::EnumValue => "Enum Value",
            FakerType::Constant => "Constant Value",
            FakerType::Pattern => "Pattern (Regex)",
        }
    }

    /// Convert to backend string format (snake_case)
    pub fn to_backend_string(&self) -> String {
        match self {
            FakerType::FullName => "full_name",
            FakerType::FirstName => "first_name",
            FakerType::LastName => "last_name",
            FakerType::Username => "username",
            FakerType::Email => "email",
            FakerType::Phone => "phone",
            FakerType::StreetAddress => "street_address",
            FakerType::City => "city",
            FakerType::State => "state",
            FakerType::Country => "country",
            FakerType::PostalCode => "postal_code",
            FakerType::Uuid => "uuid",
            FakerType::Word => "word",
            FakerType::Sentence => "sentence",
            FakerType::Paragraph => "paragraph",
            FakerType::Lorem => "lorem",
            FakerType::Integer => "integer",
            FakerType::Float => "float",
            FakerType::EnumValue => "enum",
            FakerType::Constant => "constant",
            FakerType::Pattern => "pattern",
        }
        .to_string()
    }

    /// Get faker types appropriate for a JSON Schema type
    pub fn for_schema_type(schema_type: &str, format: Option<&str>) -> Vec<FakerType> {
        // Check format first
        if let Some(fmt) = format {
            match fmt {
                "email" => return vec![FakerType::Email],
                "uuid" => return vec![FakerType::Uuid],
                _ => {}
            }
        }

        match schema_type {
            "string" => vec![
                FakerType::Lorem,
                FakerType::Sentence,
                FakerType::Word,
                FakerType::Paragraph,
                FakerType::FullName,
                FakerType::FirstName,
                FakerType::LastName,
                FakerType::Email,
                FakerType::Username,
                FakerType::Phone,
                FakerType::Uuid,
                FakerType::City,
                FakerType::Country,
                FakerType::StreetAddress,
                FakerType::PostalCode,
                FakerType::Constant,
                FakerType::Pattern,
                FakerType::EnumValue,
            ],
            "integer" => vec![FakerType::Integer, FakerType::Constant],
            "number" => vec![FakerType::Float, FakerType::Integer, FakerType::Constant],
            _ => vec![FakerType::Constant],
        }
    }
}

/// Faker configuration per field (matches backend FakerFieldConfig)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FakerFieldConfig {
    /// Type of faker generator
    pub faker_type: FakerType,
    /// Minimum value for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum value for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Regex pattern for pattern-based generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Allowed enum values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    /// Constant value (for constant faker type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constant: Option<serde_json::Value>,
}

/// Array configuration for faker (matches backend FakerArrayConfig)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FakerArrayConfig {
    pub min_items: usize,
    pub max_items: usize,
}

/// Complete faker schema configuration (sent to backend, matches backend FakerSchemaConfig)
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FakerSchemaConfig {
    /// Field path -> faker configuration
    #[serde(default)]
    pub fields: HashMap<String, FakerFieldConfig>,
    /// Array path -> array size configuration
    #[serde(default)]
    pub arrays: HashMap<String, FakerArrayConfig>,
}

#[allow(dead_code)]
impl FakerSchemaConfig {
    /// Create empty config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set field configuration
    pub fn set_field(&mut self, path: &PropertyPath, config: FakerFieldConfig) {
        self.fields.insert(path.to_string(), config);
    }

    /// Get field configuration
    pub fn get_field(&self, path: &PropertyPath) -> Option<&FakerFieldConfig> {
        self.fields.get(&path.to_string())
    }

    /// Set array configuration
    pub fn set_array(&mut self, path: &PropertyPath, config: FakerArrayConfig) {
        self.arrays.insert(path.to_string(), config);
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================================
// Resolved Schema Node (after $ref resolution)
// ============================================================================

/// A schema node with all $refs resolved
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSchemaNode {
    /// The type of this schema node
    pub node_type: SchemaNodeType,
    /// Property name (if this is a property)
    pub name: Option<String>,
    /// Description from schema
    pub description: Option<String>,
    /// Whether this property is required
    pub required: bool,
    /// Format hint (e.g., "email", "uri", "date-time")
    pub format: Option<String>,
    /// Regex pattern for validation
    pub pattern: Option<String>,
    /// Minimum value for numbers
    pub minimum: Option<f64>,
    /// Maximum value for numbers
    pub maximum: Option<f64>,
    /// Minimum string length
    pub min_length: Option<u64>,
    /// Maximum string length
    pub max_length: Option<u64>,
    /// Enum values
    pub enum_values: Vec<String>,
    /// Default value
    pub default: Option<Value>,
    /// Example values
    pub examples: Vec<Value>,
}

impl Default for ResolvedSchemaNode {
    fn default() -> Self {
        Self {
            node_type: SchemaNodeType::String,
            name: None,
            description: None,
            required: false,
            format: None,
            pattern: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            enum_values: Vec::new(),
            default: None,
            examples: Vec::new(),
        }
    }
}

/// Type of a schema node
#[derive(Clone, Debug, PartialEq)]
pub enum SchemaNodeType {
    Null,
    Boolean,
    Integer,
    Number,
    String,
    Array {
        items: Box<ResolvedSchemaNode>,
        min_items: Option<u64>,
        max_items: Option<u64>,
        unique_items: bool,
    },
    Object {
        properties: Vec<(String, ResolvedSchemaNode)>,
        additional_properties: Option<Box<ResolvedSchemaNode>>,
    },
    OneOf {
        variants: Vec<ResolvedSchemaNode>,
        labels: Vec<String>,
    },
    AnyOf {
        variants: Vec<ResolvedSchemaNode>,
        labels: Vec<String>,
    },
    Enum(Vec<String>),
    Const(Value),
}

#[allow(dead_code)]
impl ResolvedSchemaNode {
    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match &self.node_type {
            SchemaNodeType::Null => "null",
            SchemaNodeType::Boolean => "boolean",
            SchemaNodeType::Integer => "integer",
            SchemaNodeType::Number => "number",
            SchemaNodeType::String => "string",
            SchemaNodeType::Array { .. } => "array",
            SchemaNodeType::Object { .. } => "object",
            SchemaNodeType::OneOf { .. } => "oneOf",
            SchemaNodeType::AnyOf { .. } => "anyOf",
            SchemaNodeType::Enum(_) => "enum",
            SchemaNodeType::Const(_) => "const",
        }
    }

    /// Check if this is a scalar type
    pub fn is_scalar(&self) -> bool {
        matches!(
            &self.node_type,
            SchemaNodeType::Null
                | SchemaNodeType::Boolean
                | SchemaNodeType::Integer
                | SchemaNodeType::Number
                | SchemaNodeType::String
                | SchemaNodeType::Enum(_)
                | SchemaNodeType::Const(_)
        )
    }

    /// Check if this is an object type
    pub fn is_object(&self) -> bool {
        matches!(&self.node_type, SchemaNodeType::Object { .. })
    }

    /// Check if this is an array type
    pub fn is_array(&self) -> bool {
        matches!(&self.node_type, SchemaNodeType::Array { .. })
    }

    /// Get object properties if this is an object
    pub fn properties(&self) -> Option<&Vec<(String, ResolvedSchemaNode)>> {
        match &self.node_type {
            SchemaNodeType::Object { properties, .. } => Some(properties),
            _ => None,
        }
    }

    /// Get array items schema if this is an array
    pub fn items(&self) -> Option<&ResolvedSchemaNode> {
        match &self.node_type {
            SchemaNodeType::Array { items, .. } => Some(items),
            _ => None,
        }
    }

    /// Infer the best faker config for this schema node
    pub fn infer_faker_config(&self) -> FakerFieldConfig {
        let mut config = FakerFieldConfig::default();

        // Set faker type based on schema type and format
        let faker_types = FakerType::for_schema_type(self.type_name(), self.format.as_deref());
        if let Some(first) = faker_types.first() {
            config.faker_type = first.clone();
        }

        // Check for enum
        if !self.enum_values.is_empty() {
            config.faker_type = FakerType::EnumValue;
            config.enum_values = Some(self.enum_values.clone());
        }

        // Copy constraints
        config.min = self.minimum;
        config.max = self.maximum;
        config.pattern = self.pattern.clone();

        config
    }
}

// ============================================================================
// Form Field State
// ============================================================================

/// State for a single form field
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FormFieldState {
    /// Path to this field
    pub path: PropertyPath,
    /// The resolved schema for this field
    pub schema: ResolvedSchemaNode,
    /// Current static value (for static mode)
    pub static_value: Value,
    /// Faker configuration (for faker mode)
    pub faker_config: FakerFieldConfig,
    /// For oneOf/anyOf: which variant is selected
    pub selected_variant: Option<usize>,
    /// Validation errors
    pub errors: Vec<String>,
}

#[allow(dead_code)]
impl FormFieldState {
    /// Create a new field state with default values
    pub fn new(path: PropertyPath, schema: ResolvedSchemaNode) -> Self {
        let default_value = schema.default.clone().unwrap_or(Value::Null);
        let faker_config = schema.infer_faker_config();

        Self {
            path,
            schema,
            static_value: default_value,
            faker_config,
            selected_variant: None,
            errors: Vec::new(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_path_building() {
        let path = PropertyPath::root()
            .push_property("user")
            .push_property("address")
            .push_property("city");

        assert_eq!(path.to_string(), "user.address.city");
        assert_eq!(path.depth(), 3);
    }

    #[test]
    fn test_property_path_with_array() {
        let path = PropertyPath::root()
            .push_property("items")
            .push_index(0)
            .push_property("name");

        assert_eq!(path.to_string(), "items[0].name");
    }

    #[test]
    fn test_property_path_with_wildcard() {
        let path = PropertyPath::root()
            .push_property("items")
            .push_wildcard()
            .push_property("name");

        assert_eq!(path.to_string(), "items[*].name");
    }

    #[test]
    fn test_property_path_parse() {
        let path = PropertyPath::parse("user.address[0].city");
        assert_eq!(path.depth(), 4);
        assert_eq!(path.to_string(), "user.address[0].city");
    }

    #[test]
    fn test_faker_schema_config_serialization() {
        let mut config = FakerSchemaConfig::new();
        config.set_field(
            &PropertyPath::parse("user.email"),
            FakerFieldConfig {
                faker_type: FakerType::Email,
                ..Default::default()
            },
        );

        let json = config.to_json().unwrap();
        let parsed = FakerSchemaConfig::from_json(&json).unwrap();
        assert_eq!(parsed.fields.len(), 1);
    }
}
