//! Main Schema Form Generator Component
//!
//! Generates dynamic forms from JSON Schema for static values or faker configuration.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use serde_json::{json, Value, Map};

use super::types::*;
use super::resolver::*;
use super::fields::*;
use super::array_field::*;
use super::variant_selector::*;

// ============================================================================
// Main Component
// ============================================================================

/// Schema Form Generator Component
///
/// Generates a form from a JSON Schema definition for either:
/// - Static mode: Enter concrete values per property
/// - Faker mode: Configure faker generators per property
#[component]
pub fn SchemaFormGenerator(
    /// The JSON schema to generate a form for
    schema: ReadSignal<Value>,
    /// Form mode: static values or faker configuration
    mode: SchemaFormMode,
    /// Output as JSON string (for static: the value, for faker: FakerSchemaConfig)
    output: RwSignal<String>,
    /// Color theme (green, purple, blue)
    #[prop(default = "green".to_string())]
    color: String,
    /// Whether to show the form/JSON toggle
    #[prop(default = true)]
    show_toggle: bool,
) -> impl IntoView {
    // JSON mode toggle
    let (json_mode, set_json_mode) = signal(false);

    // JSON text for raw editing
    let (json_text, set_json_text) = signal(output.get());
    let (json_error, set_json_error) = signal(Option::<String>::None);

    // Sync json_text when output changes externally
    Effect::new(move || {
        let out = output.get();
        if !json_mode.get() {
            set_json_text.set(out);
        }
    });

    // Form state: map of path -> value for static, path -> faker config for faker
    let form_values = RwSignal::new(std::collections::HashMap::<String, Value>::new());
    let faker_configs = RwSignal::new(FakerSchemaConfig::new());

    // Resolve schema
    let resolved_schema = Memo::new(move |_| {
        let schema_val = schema.get();
        let mut ctx = SchemaResolutionContext::from_schema(&schema_val);
        resolve_schema(&schema_val, &mut ctx, 0)
    });

    // Initialize form state from schema defaults, then override with existing output if present
    Effect::new(move || {
        let resolved = resolved_schema.get();
        if let SchemaNodeType::Object { properties, .. } = &resolved.node_type {
            let mut values = std::collections::HashMap::new();
            let mut configs = FakerSchemaConfig::new();

            // First, initialize with defaults from schema
            for (name, prop) in properties {
                let path = PropertyPath::root().push_property(name);
                initialize_field_state(&path, prop, &mut values, &mut configs, mode);
            }

            // Then, if there's existing output content, parse it into form state
            // Use get_untracked() to avoid creating a reactive dependency on output
            // (otherwise we'd have an infinite loop: output changes -> effect runs -> form changes -> output changes)
            let existing_output = output.get_untracked();
            if !existing_output.is_empty() {
                if let Ok(existing_value) = serde_json::from_str::<Value>(&existing_output) {
                    match mode {
                        SchemaFormMode::StaticValue => {
                            // Parse existing JSON into form values
                            parse_json_to_form(&resolved, &existing_value, &PropertyPath::root(), &mut values);
                        }
                        SchemaFormMode::FakerConfig => {
                            // Parse existing faker config
                            if let Ok(config) = serde_json::from_value::<FakerSchemaConfig>(existing_value) {
                                configs = config;
                            }
                        }
                    }
                }
            }

            form_values.set(values);
            faker_configs.set(configs);
        }
    });

    // Sync form state to output
    let sync_to_output = move || {
        match mode {
            SchemaFormMode::StaticValue => {
                let values = form_values.get();
                let resolved = resolved_schema.get();
                let json_value = build_json_from_form(&resolved, &values);
                if let Ok(json_str) = serde_json::to_string_pretty(&json_value) {
                    output.set(json_str);
                    set_json_text.set(output.get());
                }
            }
            SchemaFormMode::FakerConfig => {
                let configs = faker_configs.get();
                if let Ok(json_str) = serde_json::to_string_pretty(&configs) {
                    output.set(json_str);
                    set_json_text.set(output.get());
                }
            }
        }
    };

    // Watch form values and sync
    Effect::new(move || {
        let _ = form_values.get();
        let _ = faker_configs.get();
        if !json_mode.get() {
            sync_to_output();
        }
    });

    // Handle JSON text change
    let on_json_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
        let new_text = textarea.value();
        set_json_text.set(new_text.clone());

        // Validate JSON
        match serde_json::from_str::<Value>(&new_text) {
            Ok(val) => {
                set_json_error.set(None);
                output.set(new_text);

                // Update form state from JSON
                match mode {
                    SchemaFormMode::StaticValue => {
                        let mut values = std::collections::HashMap::new();
                        let resolved = resolved_schema.get();
                        parse_json_to_form(&resolved, &val, &PropertyPath::root(), &mut values);
                        form_values.set(values);
                    }
                    SchemaFormMode::FakerConfig => {
                        if let Ok(config) = serde_json::from_value::<FakerSchemaConfig>(val) {
                            faker_configs.set(config);
                        }
                    }
                }
            }
            Err(e) => {
                set_json_error.set(Some(e.to_string()));
            }
        }
    };

    // Color classes
    let focus_ring = match color.as_str() {
        "purple" => "focus:ring-purple-500",
        "blue" => "focus:ring-blue-500",
        _ => "focus:ring-green-500",
    };
    let header_color = match color.as_str() {
        "purple" => "text-purple-700",
        "blue" => "text-blue-700",
        _ => "text-green-700",
    };

    view! {
        <div class="schema-form-generator">
            // Header with mode indicator and toggle
            {show_toggle.then(|| view! {
                <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center gap-2">
                        <span class=format!("text-sm font-medium {}", header_color)>
                            {match mode {
                                SchemaFormMode::StaticValue => "Static Values",
                                SchemaFormMode::FakerConfig => "Faker Configuration",
                            }}
                        </span>
                    </div>
                    <div class="flex items-center gap-2">
                        <span class="text-xs text-gray-500">"View:"</span>
                        <div class="inline-flex bg-gray-100 rounded-lg p-0.5">
                            <button
                                type="button"
                                class=move || format!(
                                    "px-2 py-1 text-xs font-medium rounded {}",
                                    if !json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                                )
                                on:click=move |_| set_json_mode.set(false)
                            >
                                "Form"
                            </button>
                            <button
                                type="button"
                                class=move || format!(
                                    "px-2 py-1 text-xs font-medium rounded {}",
                                    if json_mode.get() { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                                )
                                on:click=move |_| set_json_mode.set(true)
                            >
                                "JSON"
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Form view
            <div style=move || if json_mode.get() { "display: none" } else { "display: block" }>
                {move || {
                    let resolved = resolved_schema.get();

                    if let SchemaNodeType::Object { properties, .. } = &resolved.node_type {
                        view! {
                            <div class="space-y-3">
                                {properties.iter().map(|(name, prop)| {
                                    let path = PropertyPath::root().push_property(name);
                                    view! {
                                        <SchemaField
                                            path=path
                                            schema=prop.clone()
                                            mode=mode
                                            depth=0
                                            form_values=form_values
                                            faker_configs=faker_configs
                                            color=color.clone()
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="text-sm text-gray-500 italic p-4 bg-gray-50 rounded">
                                "Schema does not have properties. Use JSON mode to edit."
                            </div>
                        }.into_any()
                    }
                }}
            </div>

            // JSON view
            <div style=move || if json_mode.get() { "display: block" } else { "display: none" }>
                <div class="relative">
                    <textarea
                        rows=12
                        class=format!(
                            "w-full px-3 py-2 font-mono text-sm border rounded-md {} {}",
                            if json_error.get().is_some() { "border-red-300 bg-red-50" } else { "border-gray-300" },
                            focus_ring
                        )
                        prop:value=move || json_text.get()
                        on:input=on_json_change
                    />
                    {move || json_error.get().map(|err| view! {
                        <p class="mt-1 text-xs text-red-500">{err}</p>
                    })}
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Schema Field Component (recursive)
// ============================================================================

/// Renders a single schema field (may be recursive for objects/arrays)
#[component]
pub fn SchemaField(
    path: PropertyPath,
    schema: ResolvedSchemaNode,
    mode: SchemaFormMode,
    depth: usize,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    let path_str = path.to_string();
    let name = schema.name.clone().unwrap_or_else(|| path_str.clone());
    let is_required = schema.required;

    // Indentation and styling based on depth
    let indent_class = match depth {
        0 => "",
        1 => "ml-4 border-l-2 border-gray-200 pl-3",
        _ => "ml-4 border-l-2 border-gray-200 pl-3",
    };
    let bg_class = if depth % 2 == 0 { "bg-gray-50" } else { "bg-white" };

    view! {
        <div class=format!("rounded-lg p-3 {} {}", bg_class, indent_class)>
            // Field header
            <div class="flex items-center justify-between mb-2">
                <div class="flex items-center gap-2">
                    <span class="text-sm font-medium text-gray-700">{name.clone()}</span>
                    <span class="text-xs text-gray-400">{format!("({})", schema.type_name())}</span>
                    {is_required.then(|| view! {
                        <span class="text-xs text-red-500 font-medium">"*"</span>
                    })}
                </div>
                {schema.description.as_ref().map(|desc| view! {
                    <span class="text-xs text-gray-500 truncate max-w-xs" title=desc.clone()>{desc.clone()}</span>
                })}
            </div>

            // Field content based on type
            {move || {
                let schema_clone = schema.clone();
                let path_clone = path.clone();
                let color_clone = color.clone();

                match &schema_clone.node_type {
                    SchemaNodeType::Object { properties, .. } => {
                        view! {
                            <div class="space-y-2">
                                {properties.iter().map(|(prop_name, prop_schema)| {
                                    let child_path = path_clone.push_property(prop_name);
                                    view! {
                                        <SchemaField
                                            path=child_path
                                            schema=prop_schema.clone()
                                            mode=mode
                                            depth=depth + 1
                                            form_values=form_values
                                            faker_configs=faker_configs
                                            color=color_clone.clone()
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                    SchemaNodeType::Array { items, min_items, max_items, .. } => {
                        view! {
                            <ArrayFieldEditor
                                path=path_clone
                                items_schema=(**items).clone()
                                mode=mode
                                min_items=*min_items
                                max_items=*max_items
                                form_values=form_values
                                faker_configs=faker_configs
                                color=color_clone
                            />
                        }.into_any()
                    }
                    SchemaNodeType::OneOf { variants, labels } | SchemaNodeType::AnyOf { variants, labels } => {
                        view! {
                            <VariantSelector
                                path=path_clone
                                variants=variants.clone()
                                labels=labels.clone()
                                mode=mode
                                form_values=form_values
                                faker_configs=faker_configs
                                color=color_clone
                            />
                        }.into_any()
                    }
                    // Scalar types
                    _ => {
                        view! {
                            <ScalarField
                                path=path_clone
                                schema=schema_clone
                                mode=mode
                                form_values=form_values
                                faker_configs=faker_configs
                                color=color_clone
                            />
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Initialize field state for a property
fn initialize_field_state(
    path: &PropertyPath,
    schema: &ResolvedSchemaNode,
    values: &mut std::collections::HashMap<String, Value>,
    configs: &mut FakerSchemaConfig,
    mode: SchemaFormMode,
) {
    match &schema.node_type {
        SchemaNodeType::Object { properties, .. } => {
            for (name, prop) in properties {
                let child_path = path.push_property(name);
                initialize_field_state(&child_path, prop, values, configs, mode);
            }
        }
        SchemaNodeType::Array { items, min_items, .. } => {
            // Initialize with minimum required items or 1
            let count = min_items.unwrap_or(1) as usize;
            values.insert(format!("{}.__count", path), json!(count));

            match mode {
                SchemaFormMode::StaticValue => {
                    for i in 0..count {
                        let item_path = path.push_index(i);
                        initialize_field_state(&item_path, items, values, configs, mode);
                    }
                }
                SchemaFormMode::FakerConfig => {
                    let item_path = path.push_wildcard();
                    initialize_field_state(&item_path, items, values, configs, mode);
                    configs.arrays.insert(path.to_string(), FakerArrayConfig {
                        min_items: min_items.unwrap_or(1) as usize,
                        max_items: min_items.unwrap_or(3) as usize,
                    });
                }
            }
        }
        _ => {
            // Scalar types
            let path_str = path.to_string();
            match mode {
                SchemaFormMode::StaticValue => {
                    let default_value = schema.default.clone().unwrap_or_else(|| {
                        default_value_for_type(&schema.node_type)
                    });
                    values.insert(path_str, default_value);
                }
                SchemaFormMode::FakerConfig => {
                    let faker_config = schema.infer_faker_config();
                    configs.fields.insert(path_str, faker_config);
                }
            }
        }
    }
}

/// Get default value for a schema type
fn default_value_for_type(node_type: &SchemaNodeType) -> Value {
    match node_type {
        SchemaNodeType::Null => Value::Null,
        SchemaNodeType::Boolean => Value::Bool(false),
        SchemaNodeType::Integer => json!(0),
        SchemaNodeType::Number => json!(0.0),
        SchemaNodeType::String => Value::String(String::new()),
        SchemaNodeType::Enum(values) => {
            values.first().map(|v| Value::String(v.clone())).unwrap_or(Value::Null)
        }
        SchemaNodeType::Const(val) => val.clone(),
        SchemaNodeType::Array { .. } => Value::Array(vec![]),
        SchemaNodeType::Object { .. } => Value::Object(Map::new()),
        SchemaNodeType::OneOf { variants, .. } | SchemaNodeType::AnyOf { variants, .. } => {
            variants.first().map(|v| default_value_for_type(&v.node_type)).unwrap_or(Value::Null)
        }
    }
}

/// Build JSON value from form state
fn build_json_from_form(
    schema: &ResolvedSchemaNode,
    values: &std::collections::HashMap<String, Value>,
) -> Value {
    build_json_at_path(schema, values, &PropertyPath::root())
}

fn build_json_at_path(
    schema: &ResolvedSchemaNode,
    values: &std::collections::HashMap<String, Value>,
    path: &PropertyPath,
) -> Value {
    match &schema.node_type {
        SchemaNodeType::Object { properties, .. } => {
            let mut obj = Map::new();
            for (name, prop) in properties {
                let child_path = path.push_property(name);
                let val = build_json_at_path(prop, values, &child_path);
                obj.insert(name.clone(), val);
            }
            Value::Object(obj)
        }
        SchemaNodeType::Array { items, .. } => {
            let count_key = format!("{}.__count", path);
            let count = values.get(&count_key)
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let mut arr = Vec::new();
            for i in 0..count {
                let item_path = path.push_index(i);
                let val = build_json_at_path(items, values, &item_path);
                arr.push(val);
            }
            Value::Array(arr)
        }
        _ => {
            // Scalar type - get from values map
            values.get(&path.to_string()).cloned().unwrap_or(Value::Null)
        }
    }
}

/// Parse JSON value into form state
fn parse_json_to_form(
    schema: &ResolvedSchemaNode,
    value: &Value,
    path: &PropertyPath,
    values: &mut std::collections::HashMap<String, Value>,
) {
    match (&schema.node_type, value) {
        (SchemaNodeType::Object { properties, .. }, Value::Object(obj)) => {
            for (name, prop) in properties {
                let child_path = path.push_property(name);
                if let Some(val) = obj.get(name) {
                    parse_json_to_form(prop, val, &child_path, values);
                }
            }
        }
        (SchemaNodeType::Array { items, .. }, Value::Array(arr)) => {
            let count_key = format!("{}.__count", path);
            values.insert(count_key, json!(arr.len()));

            for (i, item) in arr.iter().enumerate() {
                let item_path = path.push_index(i);
                parse_json_to_form(items, item, &item_path, values);
            }
        }
        _ => {
            // Scalar - store directly
            values.insert(path.to_string(), value.clone());
        }
    }
}
