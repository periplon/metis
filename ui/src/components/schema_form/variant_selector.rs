//! Variant Selector Component
//!
//! Handles oneOf/anyOf schema variants with a dropdown selector.

use leptos::prelude::*;
use serde_json::Value;

use super::types::*;
use super::generator::SchemaField;

// ============================================================================
// Variant Selector
// ============================================================================

/// Selector for oneOf/anyOf schema variants
#[component]
pub fn VariantSelector(
    path: PropertyPath,
    variants: Vec<ResolvedSchemaNode>,
    labels: Vec<String>,
    mode: SchemaFormMode,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    let path_str = path.to_string();
    let variant_key = format!("{}.__variant", path_str);

    // Use Memo for the selected variant index to make it reactive
    let variant_key_for_memo = variant_key.clone();
    let selected_variant = Memo::new(move |_| {
        form_values.get()
            .get(&variant_key_for_memo)
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize
    });

    let focus_ring = match color.as_str() {
        "purple" => "focus:ring-purple-500",
        "blue" => "focus:ring-blue-500",
        _ => "focus:ring-green-500",
    };

    let border_color = match color.as_str() {
        "purple" => "border-purple-200",
        "blue" => "border-blue-200",
        _ => "border-green-200",
    };

    let bg_color = match color.as_str() {
        "purple" => "bg-purple-50",
        "blue" => "bg-blue-50",
        _ => "bg-green-50",
    };

    let variant_key_for_change = variant_key.clone();
    let path_for_change = path.clone();

    let on_variant_change = move |ev: leptos::web_sys::Event| {
        use wasm_bindgen::JsCast;
        let target = ev.target().unwrap();
        let select: leptos::web_sys::HtmlSelectElement = target.dyn_into().unwrap();
        let value = select.value().parse::<usize>().unwrap_or(0);

        // Update variant selection
        form_values.update(|v| {
            v.insert(variant_key_for_change.clone(), serde_json::json!(value));
        });

        // Clear data from other variants to avoid stale data
        let path_prefix = path_for_change.to_string();
        form_values.update(|v| {
            // Keep variant key and any keys that belong to the selected variant
            // Remove keys that might belong to other variants
            let keys_to_check: Vec<String> = v.keys()
                .filter(|k| k.starts_with(&path_prefix) && !k.ends_with(".__variant"))
                .cloned()
                .collect();

            for key in keys_to_check {
                // For simplicity, clear all nested keys when variant changes
                // The new variant's fields will be initialized fresh
                if key != path_prefix {
                    v.remove(&key);
                }
            }
        });
    };

    view! {
        <div class=format!("border {} rounded-lg p-3 {}", border_color, bg_color)>
            // Variant selector dropdown
            <div class="mb-3">
                <label class="block text-xs font-medium text-gray-600 mb-1">
                    "Select variant"
                </label>
                <select
                    class=format!("w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-2 {}", focus_ring)
                    prop:value=move || selected_variant.get().to_string()
                    on:change=on_variant_change
                >
                    {labels.iter().enumerate().map(|(idx, label)| {
                        view! {
                            <option value=idx.to_string()>{label.clone()}</option>
                        }
                    }).collect_view()}
                </select>
            </div>

            // Render selected variant's fields
            {move || {
                let idx = selected_variant.get();
                if idx < variants.len() {
                    let variant = variants[idx].clone();
                    view! {
                        <VariantContent
                            path=path.clone()
                            variant=variant
                            mode=mode
                            form_values=form_values
                            faker_configs=faker_configs
                            color=color.clone()
                        />
                    }.into_any()
                } else {
                    view! {
                        <div class="text-sm text-gray-400 italic">
                            "Invalid variant selection"
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

// ============================================================================
// Variant Content
// ============================================================================

/// Renders the content of a selected variant
#[component]
fn VariantContent(
    path: PropertyPath,
    variant: ResolvedSchemaNode,
    mode: SchemaFormMode,
    form_values: RwSignal<std::collections::HashMap<String, Value>>,
    faker_configs: RwSignal<FakerSchemaConfig>,
    color: String,
) -> impl IntoView {
    match &variant.node_type {
        SchemaNodeType::Object { properties, .. } => {
            // Render object properties
            view! {
                <div class="space-y-2">
                    {properties.iter().map(|(name, prop)| {
                        let child_path = path.push_property(name);
                        view! {
                            <SchemaField
                                path=child_path
                                schema=prop.clone()
                                mode=mode
                                depth=1
                                form_values=form_values
                                faker_configs=faker_configs
                                color=color.clone()
                            />
                        }
                    }).collect_view()}
                </div>
            }.into_any()
        }
        SchemaNodeType::Const(val) => {
            // Const variant - just display the value
            let display = match val {
                Value::String(s) => format!("\"{}\"", s),
                other => other.to_string(),
            };
            view! {
                <div class="px-3 py-2 text-sm bg-gray-100 text-gray-600 rounded-md font-mono">
                    {display}
                </div>
            }.into_any()
        }
        _ => {
            // Other types - render as a single field
            view! {
                <SchemaField
                    path=path
                    schema=variant
                    mode=mode
                    depth=1
                    form_values=form_values
                    faker_configs=faker_configs
                    color=color
                />
            }.into_any()
        }
    }
}

