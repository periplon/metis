//! Data Lake CRUD Strategy Editor Component
//!
//! Provides UI for configuring DataLakeCrud mock strategy:
//! - Data lake selection
//! - Schema selection (filtered by data lake)
//! - Operation selection (Create/ReadById/ReadAll/ReadFilter/Update/Delete)
//! - Auto-populate schemas button
//! - Filter template editor for ReadFilter

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::{json, Value};
use crate::api;
use crate::types::{DataLakeCrudConfig, DataLakeCrudOperation};

/// Data Lake CRUD Strategy Editor component
#[component]
pub fn DataLakeCrudEditor(
    /// DataLakeCrud configuration signal
    config: RwSignal<DataLakeCrudConfig>,
    /// Callback when input schema should be auto-populated
    #[prop(optional)]
    on_populate_input_schema: Option<Callback<Value>>,
    /// Callback when output schema should be auto-populated
    #[prop(optional)]
    on_populate_output_schema: Option<Callback<Value>>,
) -> impl IntoView {
    // Load data lakes
    let data_lakes = LocalResource::new(|| async {
        api::list_data_lakes().await.unwrap_or_default()
    });

    // Track schema info loading state
    let (loading_schema, set_loading_schema) = signal(false);
    let (schema_info, set_schema_info) = signal(Option::<api::SchemaInfoResponse>::None);

    // Derived signals from config
    let data_lake = Memo::new(move |_| config.get().data_lake.clone());
    let schema_name = Memo::new(move |_| config.get().schema_name.clone());
    let operation = Memo::new(move |_| config.get().operation.clone());

    // Get available schemas for selected data lake
    let available_schemas = Memo::new(move |_| {
        let selected_lake = data_lake.get();
        if selected_lake.is_empty() {
            return Vec::new();
        }
        data_lakes.get()
            .map(|lakes| {
                lakes.iter()
                    .find(|dl| dl.name == selected_lake)
                    .map(|dl| dl.schemas.iter().map(|s| s.schema_name.clone()).collect::<Vec<_>>())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    });

    // Fetch schema info when data lake + schema changes
    Effect::new(move |_| {
        let dl = data_lake.get();
        let sn = schema_name.get();
        if dl.is_empty() || sn.is_empty() {
            set_schema_info.set(None);
            return;
        }

        set_loading_schema.set(true);
        let dl_clone = dl.clone();
        let sn_clone = sn.clone();
        spawn_local(async move {
            match api::get_schema_info(&dl_clone, &sn_clone).await {
                Ok(info) => set_schema_info.set(Some(info)),
                Err(_) => set_schema_info.set(None),
            }
            set_loading_schema.set(false);
        });
    });

    // Generate input/output schemas based on operation
    let auto_populate_schemas = move |_| {
        let op = operation.get();
        let schema_def = schema_info.get().and_then(|s| s.schema_definition);

        let (input_schema, output_schema) = generate_schemas_for_operation(&op, schema_def);

        if let Some(cb) = &on_populate_input_schema {
            cb.run(input_schema);
        }
        if let Some(cb) = &on_populate_output_schema {
            cb.run(output_schema);
        }
    };

    view! {
        <div class="space-y-4">
            <div class="p-3 bg-teal-50 border border-teal-200 rounded-lg">
                <p class="text-sm text-teal-800">
                    <strong>"Data Lake CRUD"</strong>" enables Create, Read, Update, Delete operations on data lake records. Select a data lake, schema, and operation type."
                </p>
            </div>

            // Data Lake Selection
            <div>
                <label class="block text-sm font-medium text-gray-700 mb-1">"Data Lake *"</label>
                <select
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                    prop:value=move || data_lake.get()
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        config.update(|c| {
                            c.data_lake = val;
                            c.schema_name = String::new();
                        });
                    }
                >
                    <option value="">"Select a data lake..."</option>
                    {move || {
                        data_lakes.get().map(|lakes| {
                            lakes.into_iter().map(|dl| {
                                let name = dl.name.clone();
                                let name2 = name.clone();
                                view! {
                                    <option value=name>{name2}</option>
                                }
                            }).collect::<Vec<_>>()
                        }).unwrap_or_default()
                    }}
                </select>
            </div>

            // Schema Selection
            <Show when=move || !data_lake.get().is_empty()>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">"Schema *"</label>
                    <select
                        class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                        prop:value=move || schema_name.get()
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            config.update(|c| c.schema_name = val);
                        }
                    >
                        <option value="">"Select a schema..."</option>
                        {move || available_schemas.get().into_iter().map(|s| {
                            let s2 = s.clone();
                            view! {
                                <option value=s>{s2}</option>
                            }
                        }).collect::<Vec<_>>()}
                    </select>
                </div>

                // Schema info display
                <Show when=move || schema_info.get().is_some()>
                    <div class="p-3 bg-gray-50 rounded-lg text-sm">
                        <div class="flex items-center justify-between">
                            <span class="text-gray-600 font-medium">"Schema Info"</span>
                            <span class="text-gray-500 text-xs">
                                {move || schema_info.get().map(|s| format!("{} records", s.record_count)).unwrap_or_default()}
                            </span>
                        </div>
                    </div>
                </Show>
            </Show>

            // Operation Selection
            <Show when=move || !schema_name.get().is_empty()>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">"Operation *"</label>
                    <div class="grid grid-cols-3 gap-2">
                        {[
                            ("Create", DataLakeCrudOperation::Create),
                            ("Read by ID", DataLakeCrudOperation::ReadById),
                            ("Read All", DataLakeCrudOperation::ReadAll),
                            ("Read Filter", DataLakeCrudOperation::ReadFilter),
                            ("Update", DataLakeCrudOperation::Update),
                            ("Delete", DataLakeCrudOperation::Delete),
                        ].into_iter().map(|(label, op_val)| {
                            let op_val_check = op_val.clone();
                            let op_val_set = op_val.clone();
                            view! {
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-3 py-2 text-sm font-medium rounded-lg border transition-colors {}",
                                        if operation.get() == op_val_check {
                                            "bg-teal-100 border-teal-500 text-teal-700"
                                        } else {
                                            "bg-white border-gray-300 text-gray-700 hover:bg-gray-50"
                                        }
                                    )
                                    on:click=move |_| config.update(|c| c.operation = op_val_set.clone())
                                >
                                    {label}
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>

                // Operation-specific fields

                // ID Field (for ReadById, Update, Delete)
                <Show when=move || matches!(operation.get(), DataLakeCrudOperation::ReadById | DataLakeCrudOperation::Update | DataLakeCrudOperation::Delete)>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"ID Field Name"</label>
                        <input
                            type="text"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                            placeholder="id"
                            prop:value=move || config.get().id_field.clone()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                config.update(|c| c.id_field = val);
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Name of the input field containing the record ID"</p>
                    </div>
                </Show>

                // Read Limit (for ReadAll)
                <Show when=move || matches!(operation.get(), DataLakeCrudOperation::ReadAll | DataLakeCrudOperation::ReadFilter)>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Max Records"</label>
                        <input
                            type="number"
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500"
                            placeholder="100"
                            prop:value=move || config.get().read_limit.to_string()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                config.update(|c| c.read_limit = val.parse().unwrap_or(100));
                            }
                        />
                    </div>
                </Show>

                // Filter Template (for ReadFilter)
                <Show when=move || matches!(operation.get(), DataLakeCrudOperation::ReadFilter)>
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Filter Template (Tera)"</label>
                        <textarea
                            rows=4
                            class="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-teal-500 focus:border-teal-500 font-mono text-sm"
                            placeholder=r#"{"status": "{{ status }}", "category": "{{ category }}"}"#
                            prop:value=move || config.get().filter_template.clone().unwrap_or_default()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                config.update(|c| c.filter_template = if val.is_empty() { None } else { Some(val) });
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Tera template to build filter conditions. Use ""{{ field }}"" for input values."</p>
                    </div>
                </Show>

                // Auto-populate button
                <Show when=move || on_populate_input_schema.is_some() || on_populate_output_schema.is_some()>
                    <button
                        type="button"
                        class="w-full px-4 py-2 bg-teal-100 text-teal-700 rounded-lg hover:bg-teal-200 transition-colors flex items-center justify-center gap-2"
                        on:click=auto_populate_schemas
                        disabled=move || loading_schema.get()
                    >
                        {move || if loading_schema.get() {
                            view! { <span>"Loading..."</span> }.into_any()
                        } else {
                            view! {
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                                </svg>
                                <span>"Auto-populate Input/Output Schemas"</span>
                            }.into_any()
                        }}
                    </button>
                </Show>

                // Operation description
                <div class="p-3 bg-blue-50 border border-blue-200 rounded-lg text-sm text-blue-800">
                    {move || match operation.get() {
                        DataLakeCrudOperation::Create => "Creates a new record. Input: schema fields. Output: created record with ID.".to_string(),
                        DataLakeCrudOperation::ReadById => "Reads a single record by ID. Input: { id }. Output: record or null.".to_string(),
                        DataLakeCrudOperation::ReadAll => "Lists all records with pagination. Input: { limit?, offset? }. Output: array of records.".to_string(),
                        DataLakeCrudOperation::ReadFilter => "Filters records based on conditions. Input: filter fields. Output: array of matching records.".to_string(),
                        DataLakeCrudOperation::Update => "Updates an existing record. Input: { id, ...fields }. Output: updated record.".to_string(),
                        DataLakeCrudOperation::Delete => "Deletes a record (soft delete). Input: { id }. Output: { success, deleted_id }.".to_string(),
                    }}
                </div>
            </Show>
        </div>
    }
}

/// Generate input/output schemas based on operation type
fn generate_schemas_for_operation(
    operation: &DataLakeCrudOperation,
    schema_definition: Option<Value>,
) -> (Value, Value) {
    match operation {
        DataLakeCrudOperation::Create => {
            // Input: schema fields, Output: full record
            let input = schema_definition.clone().unwrap_or(json!({
                "type": "object",
                "properties": {}
            }));
            let output = json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string"},
                    "data_lake": {"type": "string"},
                    "schema_name": {"type": "string"},
                    "data": schema_definition.clone().unwrap_or(json!({"type": "object"})),
                    "created_at": {"type": "string"},
                    "updated_at": {"type": "string"}
                }
            });
            (input, output)
        }
        DataLakeCrudOperation::ReadById => {
            let input = json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Record ID to retrieve"}
                },
                "required": ["id"]
            });
            let output = schema_definition.clone().unwrap_or(json!({
                "type": "object",
                "description": "Record data or null if not found"
            }));
            (input, output)
        }
        DataLakeCrudOperation::ReadAll => {
            let input = json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "description": "Max records to return"},
                    "offset": {"type": "integer", "description": "Number of records to skip"}
                }
            });
            let output = json!({
                "type": "array",
                "items": schema_definition.clone().unwrap_or(json!({"type": "object"}))
            });
            (input, output)
        }
        DataLakeCrudOperation::ReadFilter => {
            let input = json!({
                "type": "object",
                "properties": {},
                "description": "Filter conditions based on schema fields"
            });
            let output = json!({
                "type": "array",
                "items": schema_definition.clone().unwrap_or(json!({"type": "object"}))
            });
            (input, output)
        }
        DataLakeCrudOperation::Update => {
            let mut input_props = serde_json::Map::new();
            input_props.insert("id".to_string(), json!({"type": "string", "description": "Record ID to update"}));

            // Add schema fields
            if let Some(Value::Object(obj)) = schema_definition.clone() {
                if let Some(Value::Object(props)) = obj.get("properties") {
                    for (k, v) in props {
                        input_props.insert(k.clone(), v.clone());
                    }
                }
            }

            let input = json!({
                "type": "object",
                "properties": input_props,
                "required": ["id"]
            });
            let output = schema_definition.clone().unwrap_or(json!({
                "type": "object",
                "description": "Updated record"
            }));
            (input, output)
        }
        DataLakeCrudOperation::Delete => {
            let input = json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Record ID to delete"}
                },
                "required": ["id"]
            });
            let output = json!({
                "type": "object",
                "properties": {
                    "success": {"type": "boolean"},
                    "deleted_id": {"type": "string"}
                }
            });
            (input, output)
        }
    }
}
