//! JSON Schema Editor Component
//!
//! A hierarchical, guided editor for JSON Schema definitions.
//! Supports arbitrary nested objects, arrays, and all primitive types.

use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use serde_json::{json, Value, Map};

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
}

static NEXT_PROP_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

fn next_prop_id() -> u32 {
    NEXT_PROP_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
        }
    }

    /// Convert to JSON Schema Value
    pub fn to_schema_value(&self) -> Value {
        let mut prop = Map::new();
        prop.insert("type".to_string(), json!(self.prop_type));

        if !self.description.is_empty() {
            prop.insert("description".to_string(), json!(self.description));
        }

        match self.prop_type.as_str() {
            "array" => {
                let items = if self.items_type == "object" && !self.nested_properties.is_empty() {
                    let nested_schema = properties_to_schema(&self.nested_properties);
                    nested_schema
                } else {
                    json!({"type": self.items_type})
                };
                prop.insert("items".to_string(), items);
            }
            "object" if !self.nested_properties.is_empty() => {
                let nested_schema = properties_to_schema(&self.nested_properties);
                if let Some(nested_props) = nested_schema.get("properties") {
                    prop.insert("properties".to_string(), nested_props.clone());
                }
                if let Some(required) = nested_schema.get("required") {
                    prop.insert("required".to_string(), required.clone());
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
            let prop_type = prop.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string")
                .to_string();

            let description = prop.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();

            let (items_type, nested_properties) = if prop_type == "array" {
                if let Some(items) = prop.get("items") {
                    let items_t = items.get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("string")
                        .to_string();
                    let nested = if items_t == "object" {
                        schema_to_properties(items)
                    } else {
                        Vec::new()
                    };
                    (items_t, nested)
                } else {
                    ("string".to_string(), Vec::new())
                }
            } else if prop_type == "object" {
                ("string".to_string(), schema_to_properties(prop))
            } else {
                ("string".to_string(), Vec::new())
            };

            properties.push(SchemaProperty {
                id: next_prop_id(),
                name: name.clone(),
                prop_type,
                description,
                required: required_fields.contains(name),
                items_type,
                nested_properties,
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
                                />
                            }
                        }
                    />
                </Show>
            </div>
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
) -> AnyView {
    let (expanded, set_expanded) = signal(depth == 0); // Auto-expand top level

    // Store path for all derived signals and closures
    let path_stored = StoredValue::new(path.clone());

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
    let has_nested = move || {
        let props = properties.get();
        let path = path_stored.get_value();
        let pt = get_property_at_path(&props, &path).map(|p| p.prop_type.clone()).unwrap_or_default();
        let it = get_property_at_path(&props, &path).map(|p| p.items_type.clone()).unwrap_or_default();
        pt == "object" || (pt == "array" && it == "object")
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
                                });
                            });
                        }
                    >
                        <option value="string">"string"</option>
                        <option value="number">"number"</option>
                        <option value="integer">"integer"</option>
                        <option value="boolean">"boolean"</option>
                        <option value="array">"array"</option>
                        <option value="object">"object"</option>
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
                                    });
                                });
                            }
                        >
                            <option value="string">"[string]"</option>
                            <option value="number">"[number]"</option>
                            <option value="integer">"[integer]"</option>
                            <option value="boolean">"[boolean]"</option>
                            <option value="object">"[object]"</option>
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
