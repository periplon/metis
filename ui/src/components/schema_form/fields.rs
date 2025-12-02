//! Scalar Field Renderers
//!
//! Components for rendering string, number, boolean, and enum fields.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use serde_json::{json, Value};

use super::types::*;
use super::faker_selector::FakerFieldSelector;

// ============================================================================
// Scalar Field Component
// ============================================================================

/// Renders a scalar field (string, number, boolean, enum, const)
#[component]
pub fn ScalarField(
    path: PropertyPath,
    schema: ResolvedSchemaNode,
    mode: SchemaFormMode,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    let _path_str = path.to_string();
    let focus_ring = match color.as_str() {
        "purple" => "focus:ring-purple-500",
        "blue" => "focus:ring-blue-500",
        _ => "focus:ring-green-500",
    };

    match mode {
        SchemaFormMode::StaticValue => {
            view! {
                <StaticValueInput
                    path=path
                    schema=schema
                    form_values=form_values
                    focus_ring=focus_ring.to_string()
                />
            }.into_any()
        }
        SchemaFormMode::FakerConfig => {
            view! {
                <FakerFieldSelector
                    path=path
                    schema=schema
                    faker_configs=faker_configs
                    color=color
                />
            }.into_any()
        }
    }
}

// ============================================================================
// Static Value Input
// ============================================================================

/// Input for static values based on schema type
#[component]
fn StaticValueInput(
    path: PropertyPath,
    schema: ResolvedSchemaNode,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    focus_ring: String,
) -> impl IntoView {
    let _path_str = path.to_string();

    match &schema.node_type {
        SchemaNodeType::String => {
            let format = schema.format.clone();
            let has_enum = !schema.enum_values.is_empty();
            let enum_values = schema.enum_values.clone();

            if has_enum {
                // Enum as dropdown
                view! {
                    <EnumSelect
                        path=path
                        values=enum_values
                        form_values=form_values
                        focus_ring=focus_ring
                    />
                }.into_any()
            } else {
                // Text input (possibly with format-specific type)
                let input_type = match format.as_deref() {
                    Some("email") => "email",
                    Some("uri") | Some("url") => "url",
                    Some("date") => "date",
                    Some("date-time") => "datetime-local",
                    Some("time") => "time",
                    _ => "text",
                };

                view! {
                    <StringInput
                        path=path
                        input_type=input_type.to_string()
                        form_values=form_values
                        focus_ring=focus_ring
                        placeholder=schema.examples.first().and_then(|v| v.as_str()).map(String::from)
                        pattern=schema.pattern.clone()
                        min_length=schema.min_length
                        max_length=schema.max_length
                    />
                }.into_any()
            }
        }
        SchemaNodeType::Integer => {
            view! {
                <NumberInput
                    path=path
                    is_integer=true
                    form_values=form_values
                    focus_ring=focus_ring
                    minimum=schema.minimum
                    maximum=schema.maximum
                />
            }.into_any()
        }
        SchemaNodeType::Number => {
            view! {
                <NumberInput
                    path=path
                    is_integer=false
                    form_values=form_values
                    focus_ring=focus_ring
                    minimum=schema.minimum
                    maximum=schema.maximum
                />
            }.into_any()
        }
        SchemaNodeType::Boolean => {
            view! {
                <BooleanSelect
                    path=path
                    form_values=form_values
                    focus_ring=focus_ring
                />
            }.into_any()
        }
        SchemaNodeType::Enum(values) => {
            view! {
                <EnumSelect
                    path=path
                    values=values.clone()
                    form_values=form_values
                    focus_ring=focus_ring
                />
            }.into_any()
        }
        SchemaNodeType::Const(val) => {
            view! {
                <ConstDisplay value=val.clone() />
            }.into_any()
        }
        SchemaNodeType::Null => {
            view! {
                <div class="text-sm text-gray-400 italic">"null"</div>
            }.into_any()
        }
        _ => {
            view! {
                <div class="text-sm text-gray-500">"Unsupported type"</div>
            }.into_any()
        }
    }
}

// ============================================================================
// String Input
// ============================================================================

#[component]
fn StringInput(
    path: PropertyPath,
    input_type: String,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    focus_ring: String,
    placeholder: Option<String>,
    pattern: Option<String>,
    min_length: Option<u64>,
    max_length: Option<u64>,
) -> impl IntoView {
    let path_str = path.to_string();
    let path_for_change = path_str.clone();

    let on_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value();

        form_values.update(|v| {
            v.insert(path_for_change.clone(), Value::String(value));
        });
    };

    // Only set pattern/minlength/maxlength attributes when they have values
    // Setting pattern="" causes HTML validation to fail on any input
    let pattern_attr = pattern.filter(|p| !p.is_empty());
    let minlength_attr = min_length.map(|v| v.to_string());
    let maxlength_attr = max_length.map(|v| v.to_string());

    view! {
        <input
            type=input_type
            class=format!("w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 {}", focus_ring)
            placeholder=placeholder.unwrap_or_default()
            pattern=pattern_attr
            minlength=minlength_attr
            maxlength=maxlength_attr
            prop:value=move || {
                form_values.get()
                    .get(&path_str)
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default()
            }
            on:input=on_change
        />
    }
}

// ============================================================================
// Number Input
// ============================================================================

#[component]
fn NumberInput(
    path: PropertyPath,
    is_integer: bool,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    focus_ring: String,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> impl IntoView {
    let path_str = path.to_string();
    let path_for_change = path_str.clone();

    let on_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value_str = input.value();

        let json_value = if is_integer {
            value_str.parse::<i64>().ok().map(|n| json!(n))
        } else {
            value_str.parse::<f64>().ok().map(|n| json!(n))
        };

        if let Some(val) = json_value {
            form_values.update(|v| {
                v.insert(path_for_change.clone(), val);
            });
        }
    };

    let step = if is_integer { "1" } else { "any" };

    // Only set min/max attributes when they have values
    let min_attr = minimum.map(|v| v.to_string());
    let max_attr = maximum.map(|v| v.to_string());

    view! {
        <input
            type="number"
            step=step
            min=min_attr
            max=max_attr
            class=format!("w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 {}", focus_ring)
            prop:value=move || {
                form_values.get()
                    .get(&path_str)
                    .and_then(|v| v.as_f64())
                    .map(|n| n.to_string())
                    .unwrap_or_default()
            }
            on:input=on_change
        />
    }
}

// ============================================================================
// Boolean Select
// ============================================================================

#[component]
fn BooleanSelect(
    path: PropertyPath,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    focus_ring: String,
) -> impl IntoView {
    let path_str = path.to_string();
    let path_for_change = path_str.clone();

    let on_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
        let value = select.value();

        let bool_value = match value.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::Null,
        };

        form_values.update(|v| {
            v.insert(path_for_change.clone(), bool_value);
        });
    };

    view! {
        <select
            class=format!("w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 {}", focus_ring)
            prop:value=move || {
                form_values.get()
                    .get(&path_str)
                    .and_then(|v| v.as_bool())
                    .map(|b| if b { "true" } else { "false" })
                    .unwrap_or("")
            }
            on:change=on_change
        >
            <option value="">"-- Select --"</option>
            <option value="true">"true"</option>
            <option value="false">"false"</option>
        </select>
    }
}

// ============================================================================
// Enum Select
// ============================================================================

#[component]
fn EnumSelect(
    path: PropertyPath,
    values: Vec<String>,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    focus_ring: String,
) -> impl IntoView {
    let path_str = path.to_string();
    let path_for_change = path_str.clone();

    let on_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
        let value = select.value();

        form_values.update(|v| {
            v.insert(path_for_change.clone(), Value::String(value));
        });
    };

    view! {
        <select
            class=format!("w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 bg-yellow-50 {}", focus_ring)
            prop:value=move || {
                form_values.get()
                    .get(&path_str)
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default()
            }
            on:change=on_change
        >
            <option value="">"-- Select --"</option>
            {values.into_iter().map(|val| {
                let val_clone = val.clone();
                view! {
                    <option value=val_clone.clone()>{val}</option>
                }
            }).collect_view()}
        </select>
    }
}

// ============================================================================
// Const Display
// ============================================================================

#[component]
fn ConstDisplay(value: Value) -> impl IntoView {
    let display = match &value {
        Value::String(s) => format!("\"{}\"", s),
        other => other.to_string(),
    };

    view! {
        <div class="px-3 py-2 text-sm bg-gray-100 text-gray-600 rounded-md font-mono">
            {display}
        </div>
    }
}
