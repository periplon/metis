//! Faker Selector Component
//!
//! Allows users to select and configure faker generators per field.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use serde_json::Value;

use super::types::*;

// ============================================================================
// Faker Field Selector
// ============================================================================

/// Selector for configuring a faker generator for a field
#[component]
pub fn FakerFieldSelector(
    path: PropertyPath,
    schema: ResolvedSchemaNode,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    let path_str = path.to_string();

    // Get available faker types for this schema type
    let _available_types = FakerType::for_schema_type(schema.type_name(), schema.format.as_deref());

    // Use Memo for current faker type to make it copyable/clonable
    let path_for_type_memo = path_str.clone();
    let current_type = Memo::new(move |_| {
        faker_configs.get()
            .fields
            .get(&path_for_type_memo)
            .map(|c| c.faker_type.clone())
            .unwrap_or(FakerType::Lorem)
    });

    let focus_ring = match color.as_str() {
        "purple" => "focus:ring-purple-500",
        "blue" => "focus:ring-blue-500",
        _ => "focus:ring-green-500",
    };

    let path_for_type_change = path_str.clone();
    let on_type_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
        let type_str = select.value();

        // Parse the string into FakerType
        let new_type = match type_str.as_str() {
            "full_name" => FakerType::FullName,
            "first_name" => FakerType::FirstName,
            "last_name" => FakerType::LastName,
            "username" => FakerType::Username,
            "email" => FakerType::Email,
            "phone" => FakerType::Phone,
            "street_address" => FakerType::StreetAddress,
            "city" => FakerType::City,
            "state" => FakerType::State,
            "country" => FakerType::Country,
            "postal_code" => FakerType::PostalCode,
            "uuid" => FakerType::Uuid,
            "word" => FakerType::Word,
            "sentence" => FakerType::Sentence,
            "paragraph" => FakerType::Paragraph,
            "lorem" => FakerType::Lorem,
            "integer" => FakerType::Integer,
            "float" => FakerType::Float,
            "enum" => FakerType::EnumValue,
            "constant" => FakerType::Constant,
            "pattern" => FakerType::Pattern,
            _ => FakerType::Lorem,
        };

        faker_configs.update(|configs| {
            let config = configs.fields.entry(path_for_type_change.clone()).or_insert_with(|| {
                FakerFieldConfig {
                    faker_type: FakerType::Lorem,
                    min: None,
                    max: None,
                    pattern: None,
                    enum_values: None,
                    constant: None,
                }
            });
            config.faker_type = new_type;
        });
    };

    // Check if we need constraint inputs (these now use Memo which is Copy)
    let needs_number_constraints = move || {
        matches!(current_type.get(), FakerType::Integer | FakerType::Float)
    };

    let needs_pattern = move || {
        matches!(current_type.get(), FakerType::Pattern)
    };

    let needs_enum_values = move || {
        matches!(current_type.get(), FakerType::EnumValue)
    };

    let needs_constant = move || {
        matches!(current_type.get(), FakerType::Constant)
    };

    // Clone for various usages in view
    let path_for_number = path_str.clone();
    let path_for_pattern = path_str.clone();
    let path_for_enum = path_str.clone();
    let path_for_const = path_str.clone();
    let schema_enum_values = schema.enum_values.clone();

    // Clone focus_ring for different closures
    let focus_ring_main = focus_ring.to_string();
    let focus_ring_number = focus_ring.to_string();
    let focus_ring_pattern = focus_ring.to_string();
    let focus_ring_enum = focus_ring.to_string();
    let focus_ring_const = focus_ring.to_string();

    view! {
        <div class="space-y-2">
            // Faker type dropdown
            <div class="flex items-center gap-2">
                <span class="text-xs text-gray-500 w-20">"Generator:"</span>
                <select
                    class=format!("flex-1 px-2 py-1 text-sm border border-gray-300 rounded-md bg-purple-50 focus:outline-none focus:ring-2 {}", focus_ring_main)
                    prop:value=move || current_type.get().to_backend_string()
                    on:change=on_type_change
                >
                    <optgroup label="Personal">
                        <option value="full_name">"Full Name"</option>
                        <option value="first_name">"First Name"</option>
                        <option value="last_name">"Last Name"</option>
                        <option value="username">"Username"</option>
                    </optgroup>
                    <optgroup label="Contact">
                        <option value="email">"Email"</option>
                        <option value="phone">"Phone"</option>
                    </optgroup>
                    <optgroup label="Address">
                        <option value="street_address">"Street Address"</option>
                        <option value="city">"City"</option>
                        <option value="state">"State"</option>
                        <option value="country">"Country"</option>
                        <option value="postal_code">"Postal Code"</option>
                    </optgroup>
                    <optgroup label="Text">
                        <option value="lorem">"Lorem (Default)"</option>
                        <option value="word">"Word"</option>
                        <option value="sentence">"Sentence"</option>
                        <option value="paragraph">"Paragraph"</option>
                    </optgroup>
                    <optgroup label="Numbers">
                        <option value="integer">"Integer"</option>
                        <option value="float">"Float"</option>
                    </optgroup>
                    <optgroup label="Identifiers">
                        <option value="uuid">"UUID"</option>
                    </optgroup>
                    <optgroup label="Special">
                        <option value="enum">"From Enum"</option>
                        <option value="constant">"Constant"</option>
                        <option value="pattern">"Pattern"</option>
                    </optgroup>
                </select>
            </div>

            // Number constraints (min/max)
            {move || needs_number_constraints().then(|| {
                view! {
                    <NumberConstraints
                        path_str=path_for_number.clone()
                        faker_configs=faker_configs
                        focus_ring=focus_ring_number.clone()
                    />
                }
            })}

            // Pattern input
            {move || needs_pattern().then(|| {
                view! {
                    <PatternInput
                        path_str=path_for_pattern.clone()
                        faker_configs=faker_configs
                        focus_ring=focus_ring_pattern.clone()
                    />
                }
            })}

            // Enum values input
            {move || needs_enum_values().then(|| {
                view! {
                    <EnumValuesInput
                        path_str=path_for_enum.clone()
                        schema_enum_values=schema_enum_values.clone()
                        faker_configs=faker_configs
                        focus_ring=focus_ring_enum.clone()
                    />
                }
            })}

            // Constant value input
            {move || needs_constant().then(|| {
                view! {
                    <ConstantInput
                        path_str=path_for_const.clone()
                        faker_configs=faker_configs
                        focus_ring=focus_ring_const.clone()
                    />
                }
            })}

            // Faker badge
            <div class="flex items-center gap-1">
                <span class="inline-flex items-center gap-1 px-2 py-0.5 bg-purple-100 text-purple-700 text-xs rounded-full">
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z"/>
                    </svg>
                    {move || current_type.get().display_name()}
                </span>
            </div>
        </div>
    }
}

// ============================================================================
// Constraint Components
// ============================================================================

#[component]
fn NumberConstraints(
    path_str: String,
    faker_configs: RwSignal<FakerSchemaConfig>,
    focus_ring: String,
) -> impl IntoView {
    let path_for_min = path_str.clone();

    let on_min_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value().parse::<f64>().ok();

        faker_configs.update(|configs| {
            if let Some(config) = configs.fields.get_mut(&path_for_min) {
                config.min = value;
            }
        });
    };

    let path_for_max_change = path_str.clone();
    let on_max_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value().parse::<f64>().ok();

        faker_configs.update(|configs| {
            if let Some(config) = configs.fields.get_mut(&path_for_max_change) {
                config.max = value;
            }
        });
    };

    let path_for_min_val = path_str.clone();
    let path_for_max_val = path_str.clone();

    view! {
        <div class="flex items-center gap-2">
            <div class="flex-1">
                <label class="block text-xs text-gray-500 mb-1">"Min"</label>
                <input
                    type="number"
                    step="any"
                    class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                    prop:value=move || {
                        faker_configs.get()
                            .fields
                            .get(&path_for_min_val)
                            .and_then(|c| c.min)
                            .map(|v| v.to_string())
                            .unwrap_or_default()
                    }
                    on:input=on_min_change
                />
            </div>
            <div class="flex-1">
                <label class="block text-xs text-gray-500 mb-1">"Max"</label>
                <input
                    type="number"
                    step="any"
                    class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                    prop:value=move || {
                        faker_configs.get()
                            .fields
                            .get(&path_for_max_val)
                            .and_then(|c| c.max)
                            .map(|v| v.to_string())
                            .unwrap_or_default()
                    }
                    on:input=on_max_change
                />
            </div>
        </div>
    }
}

#[component]
fn PatternInput(
    path_str: String,
    faker_configs: RwSignal<FakerSchemaConfig>,
    focus_ring: String,
) -> impl IntoView {
    let path_for_change = path_str.clone();
    let on_pattern_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value();

        faker_configs.update(|configs| {
            if let Some(config) = configs.fields.get_mut(&path_for_change) {
                config.pattern = if value.is_empty() { None } else { Some(value) };
            }
        });
    };

    let path_for_val = path_str.clone();

    view! {
        <div>
            <label class="block text-xs text-gray-500 mb-1">"Pattern (# = digit, ? = letter)"</label>
            <input
                type="text"
                class=format!("w-full px-2 py-1 text-sm font-mono border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                placeholder="###-???-####"
                prop:value=move || {
                    faker_configs.get()
                        .fields
                        .get(&path_for_val)
                        .and_then(|c| c.pattern.clone())
                        .unwrap_or_default()
                }
                on:input=on_pattern_change
            />
        </div>
    }
}

#[component]
fn EnumValuesInput(
    path_str: String,
    schema_enum_values: Vec<String>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    focus_ring: String,
) -> impl IntoView {
    // If schema has enum values, pre-populate them
    let path_for_init = path_str.clone();
    let schema_enum_for_init = schema_enum_values.clone();
    Effect::new(move || {
        if !schema_enum_for_init.is_empty() {
            faker_configs.update(|configs| {
                if let Some(config) = configs.fields.get_mut(&path_for_init) {
                    if config.enum_values.is_none() {
                        config.enum_values = Some(schema_enum_for_init.clone());
                    }
                }
            });
        }
    });

    let path_for_change = path_str.clone();
    let on_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value();

        // Split by comma
        let values: Vec<String> = value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        faker_configs.update(|configs| {
            if let Some(config) = configs.fields.get_mut(&path_for_change) {
                config.enum_values = if values.is_empty() { None } else { Some(values) };
            }
        });
    };

    let path_for_val = path_str.clone();

    view! {
        <div>
            <label class="block text-xs text-gray-500 mb-1">"Enum Values (comma-separated)"</label>
            <input
                type="text"
                class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                placeholder="value1, value2, value3"
                prop:value=move || {
                    faker_configs.get()
                        .fields
                        .get(&path_for_val)
                        .and_then(|c| c.enum_values.as_ref())
                        .map(|v| v.join(", "))
                        .unwrap_or_default()
                }
                on:input=on_change
            />
        </div>
    }
}

#[component]
fn ConstantInput(
    path_str: String,
    faker_configs: RwSignal<FakerSchemaConfig>,
    focus_ring: String,
) -> impl IntoView {
    let path_for_change = path_str.clone();
    let on_change = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value();

        faker_configs.update(|configs| {
            if let Some(config) = configs.fields.get_mut(&path_for_change) {
                // Try to parse as JSON, fallback to string
                let json_value = serde_json::from_str::<Value>(&value)
                    .unwrap_or_else(|_| Value::String(value.clone()));
                config.constant = if value.is_empty() { None } else { Some(json_value) };
            }
        });
    };

    let path_for_val = path_str.clone();

    view! {
        <div>
            <label class="block text-xs text-gray-500 mb-1">"Constant Value"</label>
            <input
                type="text"
                class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                placeholder="fixed value or JSON"
                prop:value=move || {
                    faker_configs.get()
                        .fields
                        .get(&path_for_val)
                        .and_then(|c| c.constant.as_ref())
                        .map(|v| {
                            match v {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            }
                        })
                        .unwrap_or_default()
                }
                on:input=on_change
            />
        </div>
    }
}
