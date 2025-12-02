//! Array Field Component
//!
//! Handles dynamic array editing with add/remove item functionality.

use leptos::prelude::*;
use serde_json::{json, Value};

use super::types::*;
use super::generator::SchemaField;

// ============================================================================
// Array Field Editor
// ============================================================================

/// Editor for array fields with dynamic add/remove
#[component]
pub fn ArrayFieldEditor(
    path: PropertyPath,
    items_schema: ResolvedSchemaNode,
    mode: SchemaFormMode,
    min_items: Option<u64>,
    max_items: Option<u64>,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    let path_str = path.to_string();
    let count_key = format!("{}.__count", path_str);
    let min_items_val = min_items.unwrap_or(0);
    let max_items_val = max_items;

    // Store count in a signal for reactivity
    let count_key_signal = count_key.clone();
    let item_count = Memo::new(move |_| {
        form_values.get()
            .get(&count_key_signal)
            .and_then(|v| v.as_u64())
            .unwrap_or(min_items_val) as usize
    });

    let count_key_for_add = count_key.clone();
    let path_for_add = path.clone();
    let items_schema_for_add = items_schema.clone();
    let on_add = move |_| {
        let current_count = item_count.get();
        let new_count = current_count + 1;

        // Update count
        form_values.update(|v| {
            v.insert(count_key_for_add.clone(), json!(new_count));
        });

        // Initialize new item (for static mode)
        if matches!(mode, SchemaFormMode::StaticValue) {
            let new_item_path = path_for_add.push_index(current_count);
            initialize_item(&new_item_path, &items_schema_for_add, &form_values);
        }
    };

    let count_key_for_remove = count_key.clone();
    let path_for_remove = path.clone();
    let _on_remove_last = move |_: leptos::web_sys::MouseEvent| {
        let current_count = item_count.get();
        if current_count > 0 {
            let new_count = current_count - 1;

            // Update count
            form_values.update(|v| {
                v.insert(count_key_for_remove.clone(), json!(new_count));
            });

            // Remove item data
            let removed_path = path_for_remove.push_index(new_count);
            remove_item_data(&removed_path, &form_values);
        }
    };

    let focus_ring = match color.as_str() {
        "purple" => "focus:ring-purple-500",
        "blue" => "focus:ring-blue-500",
        _ => "focus:ring-green-500",
    };
    let add_btn_color = match color.as_str() {
        "purple" => "text-purple-600 hover:bg-purple-50",
        "blue" => "text-blue-600 hover:bg-blue-50",
        _ => "text-green-600 hover:bg-green-50",
    };

    view! {
        <div class="border border-gray-200 rounded-lg p-3 bg-gray-50">
            // Header with item count and add button
            <div class="flex items-center justify-between mb-2">
                <span class="text-xs text-gray-500">
                    {move || format!("Items: {}", item_count.get())}
                    {min_items.map(|min| format!(" (min: {})", min)).unwrap_or_default()}
                    {max_items.map(|max| format!(" (max: {})", max)).unwrap_or_default()}
                </span>
                <button
                    type="button"
                    class=format!("px-2 py-1 text-xs rounded flex items-center gap-1 {}", add_btn_color)
                    on:click=on_add
                    disabled=move || max_items_val.map(|max| item_count.get() >= max as usize).unwrap_or(false)
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "Add Item"
                </button>
            </div>

            // Items list
            {move || {
                let count = item_count.get();
                match mode {
                    SchemaFormMode::StaticValue => {
                        if count == 0 {
                            view! {
                                <div class="text-sm text-gray-400 italic p-3 text-center">
                                    "No items. Click \"Add Item\" to add one."
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <div class="space-y-2">
                                    {(0..count).map(|idx| {
                                        let item_path = path.push_index(idx);
                                        let path_for_this_remove = path.clone();
                                        let can_remove = min_items.map(|min| count > min as usize).unwrap_or(count > 0);

                                        view! {
                                            <div class="border border-gray-200 rounded-lg p-2 bg-white">
                                                <div class="flex items-center justify-between mb-2">
                                                    <span class="text-xs font-medium text-gray-600">
                                                        {format!("#{}", idx + 1)}
                                                    </span>
                                                    <button
                                                        type="button"
                                                        class="text-red-500 hover:bg-red-50 p-1 rounded"
                                                        on:click=move |_| {
                                                            remove_item_at_index(idx, &path_for_this_remove, &form_values);
                                                        }
                                                        disabled=!can_remove
                                                    >
                                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                                        </svg>
                                                    </button>
                                                </div>
                                                <ArrayItemContent
                                                    path=item_path
                                                    schema=items_schema.clone()
                                                    mode=mode
                                                    form_values=form_values
                                                    faker_configs=faker_configs
                                                    color=color.clone()
                                                />
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }
                    }
                    SchemaFormMode::FakerConfig => {
                        // For faker mode, show item template config + array size config
                        view! {
                            <div class="space-y-3">
                                // Array size configuration
                                <ArraySizeConfig
                                    path=path.clone()
                                    min_items=min_items
                                    max_items=max_items
                                    faker_configs=faker_configs
                                    focus_ring=focus_ring.to_string()
                                />

                                // Item template
                                <div class="border border-gray-200 rounded-lg p-2 bg-white">
                                    <div class="text-xs font-medium text-gray-600 mb-2">
                                        "Item Template (applies to all generated items)"
                                    </div>
                                    <ArrayItemContent
                                        path=path.push_wildcard()
                                        schema=items_schema.clone()
                                        mode=mode
                                        form_values=form_values
                                        faker_configs=faker_configs
                                        color=color.clone()
                                    />
                                </div>
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

// ============================================================================
// Array Item Content
// ============================================================================

/// Content of a single array item (scalar or nested object)
#[component]
fn ArrayItemContent(
    path: PropertyPath,
    schema: ResolvedSchemaNode,
    mode: SchemaFormMode,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    // If items are objects, render nested fields
    // Otherwise, render a single scalar field
    match &schema.node_type {
        SchemaNodeType::Object { properties, .. } => {
            view! {
                <div class="space-y-2">
                    {properties.iter().map(|(name, prop)| {
                        let child_path = path.push_property(name);
                        view! {
                            <SchemaField
                                path=child_path
                                schema=prop.clone()
                                mode=mode
                                depth=2
                                form_values=form_values
                                faker_configs=faker_configs
                                color=color.clone()
                            />
                        }
                    }).collect_view()}
                </div>
            }.into_any()
        }
        _ => {
            // Scalar item
            view! {
                <SchemaField
                    path=path
                    schema=schema
                    mode=mode
                    depth=2
                    form_values=form_values
                    faker_configs=faker_configs
                    color=color
                />
            }.into_any()
        }
    }
}

// ============================================================================
// Array Size Config (for Faker mode)
// ============================================================================

#[component]
fn ArraySizeConfig(
    path: PropertyPath,
    min_items: Option<u64>,
    max_items: Option<u64>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    focus_ring: String,
) -> impl IntoView {
    let path_str = path.to_string();
    let path_for_min = path_str.clone();
    let path_for_max = path_str.clone();
    let default_min = min_items.unwrap_or(1) as usize;
    let default_max = max_items.unwrap_or(3) as usize;

    // Initialize array config if not present
    Effect::new(move || {
        faker_configs.update(|configs| {
            configs.arrays.entry(path_str.clone()).or_insert_with(|| {
                FakerArrayConfig {
                    min_items: default_min,
                    max_items: default_max,
                }
            });
        });
    });

    let on_min_change = move |ev: leptos::web_sys::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let input: leptos::web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value().parse::<usize>().unwrap_or(1);

        faker_configs.update(|configs| {
            if let Some(config) = configs.arrays.get_mut(&path_for_min) {
                config.min_items = value;
            }
        });
    };

    let on_max_change = move |ev: leptos::web_sys::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let input: leptos::web_sys::HtmlInputElement = target.dyn_into().unwrap();
        let value = input.value().parse::<usize>().unwrap_or(3);

        faker_configs.update(|configs| {
            if let Some(config) = configs.arrays.get_mut(&path_for_max) {
                config.max_items = value;
            }
        });
    };

    let path_for_min_val = path.to_string();
    let path_for_max_val = path.to_string();

    view! {
        <div class="bg-purple-50 rounded p-2">
            <div class="text-xs font-medium text-purple-700 mb-2">"Array Generation Settings"</div>
            <div class="flex items-center gap-2">
                <div class="flex-1">
                    <label class="block text-xs text-gray-600 mb-1">"Min Items"</label>
                    <input
                        type="number"
                        min="0"
                        class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                        prop:value=move || {
                            faker_configs.get()
                                .arrays
                                .get(&path_for_min_val)
                                .map(|c| c.min_items.to_string())
                                .unwrap_or_else(|| "1".to_string())
                        }
                        on:input=on_min_change
                    />
                </div>
                <div class="flex-1">
                    <label class="block text-xs text-gray-600 mb-1">"Max Items"</label>
                    <input
                        type="number"
                        min="1"
                        class=format!("w-full px-2 py-1 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 {}", focus_ring)
                        prop:value=move || {
                            faker_configs.get()
                                .arrays
                                .get(&path_for_max_val)
                                .map(|c| c.max_items.to_string())
                                .unwrap_or_else(|| "3".to_string())
                        }
                        on:input=on_max_change
                    />
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Initialize a new array item
fn initialize_item(
    path: &PropertyPath,
    schema: &ResolvedSchemaNode,
    form_values: &RwSignal<std::collections::HashMap<String, Value>>,
) {
    match &schema.node_type {
        SchemaNodeType::Object { properties, .. } => {
            for (name, prop) in properties {
                let child_path = path.push_property(name);
                initialize_item(&child_path, prop, form_values);
            }
        }
        _ => {
            let default_val = schema.default.clone().unwrap_or_else(|| {
                match &schema.node_type {
                    SchemaNodeType::String => Value::String(String::new()),
                    SchemaNodeType::Integer => serde_json::json!(0),
                    SchemaNodeType::Number => serde_json::json!(0.0),
                    SchemaNodeType::Boolean => Value::Bool(false),
                    _ => Value::Null,
                }
            });
            form_values.update(|v| {
                v.insert(path.to_string(), default_val);
            });
        }
    }
}

/// Remove item data from form values
fn remove_item_data(
    path: &PropertyPath,
    form_values: &RwSignal<std::collections::HashMap<String, Value>>,
) {
    let path_prefix = path.to_string();
    form_values.update(|v| {
        v.retain(|k, _| !k.starts_with(&path_prefix));
    });
}

/// Remove item at index and shift subsequent items
fn remove_item_at_index(
    idx: usize,
    array_path: &PropertyPath,
    form_values: &RwSignal<std::collections::HashMap<String, Value>>,
) {
    let count_key = format!("{}.__count", array_path);

    form_values.update(|v| {
        // Get current count
        let current_count = v.get(&count_key)
            .and_then(|val| val.as_u64())
            .unwrap_or(0) as usize;

        if idx >= current_count {
            return;
        }

        // Collect keys that need to be shifted
        let array_prefix = array_path.to_string();
        let mut keys_to_update: Vec<(String, String, Value)> = Vec::new();

        for (key, value) in v.iter() {
            // Check if key belongs to items after the removed one
            if key.starts_with(&format!("{}[", array_prefix)) {
                // Parse the index from the key
                if let Some(bracket_start) = key.find('[') {
                    if let Some(bracket_end) = key[bracket_start..].find(']') {
                        let idx_str = &key[bracket_start + 1..bracket_start + bracket_end];
                        if let Ok(key_idx) = idx_str.parse::<usize>() {
                            if key_idx > idx {
                                // This item needs to be shifted down
                                let new_idx = key_idx - 1;
                                let old_prefix = format!("{}[{}]", array_prefix, key_idx);
                                let new_prefix = format!("{}[{}]", array_prefix, new_idx);
                                let new_key = key.replace(&old_prefix, &new_prefix);
                                keys_to_update.push((key.clone(), new_key, value.clone()));
                            } else if key_idx == idx {
                                // This is the item being removed - mark for deletion
                                keys_to_update.push((key.clone(), String::new(), Value::Null));
                            }
                        }
                    }
                }
            }
        }

        // Remove old keys and insert updated ones
        for (old_key, new_key, value) in keys_to_update {
            v.remove(&old_key);
            if !new_key.is_empty() {
                v.insert(new_key, value);
            }
        }

        // Update count
        v.insert(count_key, serde_json::json!(current_count - 1));
    });
}
