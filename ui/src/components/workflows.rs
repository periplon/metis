use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::api;
use crate::types::{Workflow, ErrorStrategy};
use crate::components::schema_editor::{
    JsonSchemaEditor, SchemaPreview, SchemaProperty,
    properties_to_schema, schema_to_properties,
};
use crate::components::step_editor::{
    WorkflowStepsEditor, StepData,
    steps_data_to_workflow_steps, workflow_steps_to_steps_data,
};

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Table,
    Card,
}

/// Schema field information for test forms
#[derive(Clone, Debug)]
struct SchemaFieldInfo {
    name: String,
    field_type: String,
    description: Option<String>,
    required: bool,
    enum_values: Vec<String>,
}

/// Extract properties from a JSON Schema
fn extract_schema_properties(schema: &serde_json::Value) -> Vec<SchemaFieldInfo> {
    let mut props = Vec::new();

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        let required_fields: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        for (name, prop) in properties {
            let prop_type = prop.get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("string")
                .to_string();
            let description = prop.get("description")
                .and_then(|d| d.as_str())
                .map(|s| s.to_string());
            let is_required = required_fields.contains(name);

            // Extract enum values if present
            let enum_values: Vec<String> = prop.get("enum")
                .and_then(|e| e.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            props.push(SchemaFieldInfo {
                name: name.clone(),
                field_type: prop_type,
                description,
                required: is_required,
                enum_values,
            });
        }
    }

    props
}

/// Build JSON value from form field values based on schema types
fn build_json_from_fields(
    fields: &HashMap<String, String>,
    schema: &serde_json::Value,
) -> serde_json::Value {
    let mut obj = serde_json::Map::new();

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, prop) in properties {
            if let Some(value) = fields.get(name) {
                if value.is_empty() {
                    continue;
                }

                let prop_type = prop.get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("string");

                let json_value = match prop_type {
                    "number" => value.parse::<f64>()
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null),
                    "integer" => value.parse::<i64>()
                        .map(serde_json::Value::from)
                        .unwrap_or(serde_json::Value::Null),
                    "boolean" => serde_json::Value::Bool(value == "true"),
                    "array" | "object" => serde_json::from_str(value)
                        .unwrap_or(serde_json::Value::Null),
                    _ => serde_json::Value::String(value.clone()),
                };

                if !json_value.is_null() {
                    obj.insert(name.clone(), json_value);
                }
            }
        }
    }

    serde_json::Value::Object(obj)
}

#[component]
pub fn Workflows() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    // Test modal state - stores full Workflow to access input_schema
    let (test_target, set_test_target) = signal(Option::<Workflow>::None);
    let (test_form_fields, set_test_form_fields) = signal(HashMap::<String, String>::new());
    let (test_result, set_test_result) = signal(Option::<Result<crate::types::TestResult, String>>::None);
    let (testing, set_testing) = signal(false);

    let workflows = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_workflows().await.ok() }
    });

    let on_delete_confirm = move |_| {
        if let Some(name) = delete_target.get() {
            set_deleting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_workflow(&name).await {
                    Ok(_) => {
                        set_delete_target.set(None);
                        set_refresh_trigger.update(|n| *n += 1);
                    }
                    Err(e) => {
                        web_sys::window()
                            .and_then(|w| w.alert_with_message(&format!("Failed to delete: {}", e)).ok());
                    }
                }
                set_deleting.set(false);
            });
        }
    };

    let on_test_run = move |_| {
        if let Some(workflow) = test_target.get() {
            set_testing.set(true);
            set_test_result.set(None);
            let fields = test_form_fields.get();
            let schema = workflow.input_schema.clone();
            let workflow_name = workflow.name.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let args = build_json_from_fields(&fields, &schema);
                let result = api::test_workflow(&workflow_name, &args).await;
                set_test_result.set(Some(result));
                set_testing.set(false);
            });
        }
    };

    let on_test_close = move |_| {
        set_test_target.set(None);
        set_test_form_fields.set(HashMap::new());
        set_test_result.set(None);
    };

    view! {
        <div class="p-6">
            // Header with title, view toggle, and new button
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Workflows"</h2>
                <div class="flex items-center gap-4">
                    // View mode toggle
                    <div class="flex bg-gray-100 rounded-lg p-1">
                        <button
                            class=move || format!(
                                "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                if view_mode.get() == ViewMode::Table { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                            )
                            on:click=move |_| set_view_mode.set(ViewMode::Table)
                        >
                            <span class="flex items-center gap-1">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16"/>
                                </svg>
                                "Table"
                            </span>
                        </button>
                        <button
                            class=move || format!(
                                "px-3 py-1 rounded text-sm font-medium transition-colors {}",
                                if view_mode.get() == ViewMode::Card { "bg-white shadow text-gray-900" } else { "text-gray-600 hover:text-gray-900" }
                            )
                            on:click=move |_| set_view_mode.set(ViewMode::Card)
                        >
                            <span class="flex items-center gap-1">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/>
                                </svg>
                                "Cards"
                            </span>
                        </button>
                    </div>
                    <a href="/workflows/new" class="bg-orange-500 hover:bg-orange-600 text-white px-4 py-2 rounded flex items-center gap-2">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Workflow"
                    </a>
                </div>
            </div>

            // Delete confirmation modal
            {move || delete_target.get().map(|name| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"Delete Workflow?"</h3>
                        <p class="text-gray-600 mb-4">
                            "Are you sure you want to delete "
                            <span class="font-mono text-sm bg-gray-100 px-1 rounded">{name.clone()}</span>
                            "? This action cannot be undone."
                        </p>
                        <div class="flex justify-end gap-3">
                            <button
                                class="px-4 py-2 text-gray-700 border border-gray-300 rounded hover:bg-gray-50"
                                on:click=move |_| set_delete_target.set(None)
                                disabled=move || deleting.get()
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 disabled:opacity-50"
                                on:click=on_delete_confirm
                                disabled=move || deleting.get()
                            >
                                {move || if deleting.get() { "Deleting..." } else { "Delete" }}
                            </button>
                        </div>
                    </div>
                </div>
            })}

            // Test modal
            {move || test_target.get().map(|workflow| {
                let schema_props = extract_schema_properties(&workflow.input_schema);
                let has_no_properties = schema_props.is_empty();

                view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-semibold text-gray-900">
                                "Test Workflow: "
                                <span class="font-mono text-orange-600">{workflow.name.clone()}</span>
                            </h3>
                            <button
                                class="text-gray-400 hover:text-gray-600"
                                on:click=on_test_close
                            >
                                <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        // Dynamic form fields based on input_schema
                        <div class="mb-4 space-y-4">
                            {if has_no_properties {
                                view! {
                                    <div class="text-sm text-gray-500 italic p-3 bg-gray-50 rounded-md">
                                        "This workflow has no input parameters defined."
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="space-y-4">
                                        {schema_props.into_iter().map(|field_info| {
                                            let field_name = field_info.name.clone();
                                            let field_name_for_handler = field_info.name.clone();
                                            let label = format!("{}{}", field_info.name, if field_info.required { " *" } else { "" });
                                            let field_type = field_info.field_type.clone();
                                            let has_enum = !field_info.enum_values.is_empty();
                                            let enum_values = field_info.enum_values.clone();

                                            view! {
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 mb-1">
                                                        {label}
                                                        <span class="ml-2 text-xs text-gray-400 font-normal">
                                                            {if has_enum { "(enum)".to_string() } else { format!("({})", field_type) }}
                                                        </span>
                                                    </label>
                                                    {if has_enum {
                                                        // Enum type - render as select dropdown
                                                        let field_name_for_enum = field_name_for_handler.clone();
                                                        view! {
                                                            <select
                                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 bg-yellow-50"
                                                                on:change=move |ev| {
                                                                    let target = ev.target().unwrap();
                                                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                                    set_test_form_fields.update(|fields| {
                                                                        fields.insert(field_name_for_enum.clone(), select.value());
                                                                    });
                                                                }
                                                            >
                                                                <option value="">"-- Select --"</option>
                                                                {enum_values.iter().map(|v| {
                                                                    let value = v.clone();
                                                                    let display_value = v.clone();
                                                                    view! { <option value=value>{display_value}</option> }
                                                                }).collect::<Vec<_>>()}
                                                            </select>
                                                        }.into_any()
                                                    } else {
                                                        match field_type.as_str() {
                                                        "boolean" => view! {
                                                            <select
                                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                                                                on:change=move |ev| {
                                                                    let target = ev.target().unwrap();
                                                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                                    set_test_form_fields.update(|fields| {
                                                                        fields.insert(field_name_for_handler.clone(), select.value());
                                                                    });
                                                                }
                                                            >
                                                                <option value="">"-- Select --"</option>
                                                                <option value="true">"true"</option>
                                                                <option value="false">"false"</option>
                                                            </select>
                                                        }.into_any(),
                                                        "number" | "integer" => view! {
                                                            <input
                                                                type="number"
                                                                step=if field_type == "integer" { "1" } else { "any" }
                                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                                                                placeholder=format!("Enter {}", field_type)
                                                                on:input=move |ev| {
                                                                    let target = ev.target().unwrap();
                                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                                    set_test_form_fields.update(|fields| {
                                                                        fields.insert(field_name_for_handler.clone(), input.value());
                                                                    });
                                                                }
                                                            />
                                                        }.into_any(),
                                                        "array" | "object" => view! {
                                                            <textarea
                                                                rows=3
                                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500 font-mono text-sm"
                                                                placeholder=if field_type == "array" { "[...]" } else { "{...}" }
                                                                on:input=move |ev| {
                                                                    let target = ev.target().unwrap();
                                                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                                    set_test_form_fields.update(|fields| {
                                                                        fields.insert(field_name_for_handler.clone(), textarea.value());
                                                                    });
                                                                }
                                                            />
                                                        }.into_any(),
                                                        _ => view! {
                                                            <input
                                                                type="text"
                                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                                                                placeholder=format!("Enter {}", field_name)
                                                                on:input=move |ev| {
                                                                    let target = ev.target().unwrap();
                                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                                    set_test_form_fields.update(|fields| {
                                                                        fields.insert(field_name_for_handler.clone(), input.value());
                                                                    });
                                                                }
                                                            />
                                                        }.into_any(),
                                                    }}}
                                                    {field_info.description.clone().map(|desc| view! {
                                                        <p class="mt-1 text-xs text-gray-500">{desc}</p>
                                                    })}
                                                </div>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </div>
                                }.into_any()
                            }}
                        </div>

                        <div class="flex gap-3 mb-4">
                            <button
                                class="px-4 py-2 bg-orange-500 text-white rounded hover:bg-orange-600 disabled:opacity-50 flex items-center gap-2"
                                on:click=on_test_run
                                disabled=move || testing.get()
                            >
                                {move || if testing.get() {
                                    view! {
                                        <span class="flex items-center gap-2">
                                            <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                                            </svg>
                                            "Running..."
                                        </span>
                                    }.into_any()
                                } else {
                                    view! {
                                        <span class="flex items-center gap-2">
                                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                            </svg>
                                            "Run Test"
                                        </span>
                                    }.into_any()
                                }}
                            </button>
                            <button
                                class="px-4 py-2 text-gray-700 border border-gray-300 rounded hover:bg-gray-50"
                                on:click=on_test_close
                            >
                                "Close"
                            </button>
                        </div>

                        // Test result display
                        {move || test_result.get().map(|result| {
                            match result {
                                Ok(res) => view! {
                                    <div class="border-t pt-4">
                                        <div class="flex items-center justify-between mb-2">
                                            <h4 class="font-medium text-gray-900">"Output"</h4>
                                            <span class="text-sm text-gray-500">
                                                {format!("{}ms", res.execution_time_ms)}
                                            </span>
                                        </div>
                                        {res.error.clone().map(|err| view! {
                                            <div class="mb-2 p-2 bg-red-50 border border-red-200 rounded text-red-700 text-sm">
                                                {err}
                                            </div>
                                        })}
                                        <pre class="bg-gray-900 text-orange-400 p-4 rounded-lg overflow-x-auto text-sm font-mono">
                                            {serde_json::to_string_pretty(&res.output).unwrap_or_else(|_| res.output.to_string())}
                                        </pre>
                                    </div>
                                }.into_any(),
                                Err(e) => view! {
                                    <div class="border-t pt-4">
                                        <div class="p-3 bg-red-50 border border-red-200 rounded-lg">
                                            <h4 class="font-medium text-red-800 mb-1">"Error"</h4>
                                            <p class="text-red-600 text-sm">{e}</p>
                                        </div>
                                    </div>
                                }.into_any(),
                            }
                        })}
                    </div>
                </div>
            }
            })}

            <Suspense fallback=move || view! { <LoadingState /> }>
                {move || {
                    workflows.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => {
                                if view_mode.get() == ViewMode::Table {
                                    view! { <WorkflowTable workflows=list set_delete_target=set_delete_target set_test_target=set_test_target /> }.into_any()
                                } else {
                                    view! { <WorkflowCards workflows=list set_delete_target=set_delete_target set_test_target=set_test_target /> }.into_any()
                                }
                            },
                            Some(_) => view! { <EmptyState /> }.into_any(),
                            None => view! { <ErrorState /> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn LoadingState() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center py-12">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-500"></div>
            <span class="ml-3 text-gray-500">"Loading workflows..."</span>
        </div>
    }
}

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-white rounded-lg shadow">
            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No workflows"</h3>
            <p class="mt-1 text-sm text-gray-500">"Get started by creating a new workflow."</p>
            <div class="mt-6">
                <a href="/workflows/new" class="inline-flex items-center px-4 py-2 bg-orange-500 text-white rounded hover:bg-orange-600">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New Workflow"
                </a>
            </div>
        </div>
    }
}

#[component]
fn ErrorState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-red-50 rounded-lg border border-red-200">
            <svg class="mx-auto h-12 w-12 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-red-800">"Failed to load workflows"</h3>
            <p class="mt-1 text-sm text-red-600">"Please check your connection and try again."</p>
        </div>
    }
}

#[component]
fn WorkflowTable(
    workflows: Vec<Workflow>,
    set_delete_target: WriteSignal<Option<String>>,
    set_test_target: WriteSignal<Option<Workflow>>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Description"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Steps"</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                    </tr>
                </thead>
                <tbody class="bg-white divide-y divide-gray-200">
                    {workflows.into_iter().map(|workflow| {
                        let name_for_edit = workflow.name.clone();
                        let name_for_delete = workflow.name.clone();
                        let workflow_for_test = workflow.clone();
                        let steps_count = workflow.steps.len();

                        view! {
                            <tr class="hover:bg-gray-50">
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <div class="font-medium text-gray-900">{workflow.name.clone()}</div>
                                </td>
                                <td class="px-6 py-4">
                                    <div class="text-sm text-gray-500 truncate max-w-md">{workflow.description.clone()}</div>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <span class="px-2 py-1 text-xs font-semibold rounded-full bg-orange-100 text-orange-800">
                                        {format!("{} steps", steps_count)}
                                    </span>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                    <button
                                        class="text-orange-600 hover:text-orange-900 mr-3"
                                        on:click=move |_| set_test_target.set(Some(workflow_for_test.clone()))
                                    >
                                        "Test"
                                    </button>
                                    <a
                                        href=format!("/workflows/edit/{}", name_for_edit)
                                        class="text-blue-600 hover:text-blue-900 mr-3"
                                    >
                                        "Edit"
                                    </a>
                                    <button
                                        class="text-red-600 hover:text-red-900"
                                        on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                                    >
                                        "Delete"
                                    </button>
                                </td>
                            </tr>
                        }
                    }).collect::<Vec<_>>()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn WorkflowCards(
    workflows: Vec<Workflow>,
    set_delete_target: WriteSignal<Option<String>>,
    set_test_target: WriteSignal<Option<Workflow>>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {workflows.into_iter().map(|workflow| {
                let name_for_edit = workflow.name.clone();
                let name_for_delete = workflow.name.clone();
                let workflow_for_test = workflow.clone();
                let steps_count = workflow.steps.len();

                view! {
                    <div class="bg-white rounded-lg shadow hover:shadow-md transition-shadow p-4">
                        <div class="flex justify-between items-start mb-3">
                            <div class="flex-1 min-w-0">
                                <h3 class="font-semibold text-gray-900 truncate">{workflow.name.clone()}</h3>
                                <p class="text-sm text-gray-500 line-clamp-2">{workflow.description.clone()}</p>
                            </div>
                            <span class="ml-2 px-2 py-1 text-xs font-semibold rounded-full bg-orange-100 text-orange-800 flex-shrink-0">
                                {format!("{} steps", steps_count)}
                            </span>
                        </div>
                        // Visual step flow
                        <div class="flex items-center space-x-1 overflow-x-auto pb-2 mb-3">
                            {workflow.steps.iter().enumerate().map(|(i, step)| {
                                let is_last = i == steps_count - 1;
                                view! {
                                    <div class="flex items-center">
                                        <div class="px-2 py-0.5 bg-gray-100 rounded text-xs font-mono">
                                            {step.tool.clone()}
                                        </div>
                                        {if !is_last {
                                            view! { <span class="mx-0.5 text-gray-400 text-xs">"→"</span> }.into_any()
                                        } else {
                                            view! { <span></span> }.into_any()
                                        }}
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                        <div class="flex justify-end gap-2 pt-3 border-t border-gray-100">
                            <button
                                class="px-3 py-1 text-sm text-orange-600 hover:bg-orange-50 rounded"
                                on:click=move |_| set_test_target.set(Some(workflow_for_test.clone()))
                            >
                                "Test"
                            </button>
                            <a
                                href=format!("/workflows/edit/{}", name_for_edit)
                                class="px-3 py-1 text-sm text-blue-600 hover:bg-blue-50 rounded"
                            >
                                "Edit"
                            </a>
                            <button
                                class="px-3 py-1 text-sm text-red-600 hover:bg-red-50 rounded"
                                on:click=move |_| set_delete_target.set(Some(name_for_delete.clone()))
                            >
                                "Delete"
                            </button>
                        </div>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

#[component]
pub fn WorkflowForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (schema_properties, set_schema_properties) = signal(Vec::<SchemaProperty>::new());
    let (output_schema_properties, set_output_schema_properties) = signal(Vec::<SchemaProperty>::new());
    let (steps_data, set_steps_data) = signal(Vec::<StepData>::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);

    // Load available tools for dropdown
    let available_tools = LocalResource::new(move || async move {
        api::list_tools().await.ok().map(|tools| {
            tools.into_iter().map(|t| t.name).collect::<Vec<_>>()
        }).unwrap_or_default()
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let input_schema = properties_to_schema(&schema_properties.get());
        let output_props = output_schema_properties.get();
        let output_schema = if output_props.is_empty() {
            None
        } else {
            Some(properties_to_schema(&output_props))
        };

        let steps = match steps_data_to_workflow_steps(&steps_data.get()) {
            Ok(s) => s,
            Err(e) => {
                set_error.set(Some(e));
                set_saving.set(false);
                return;
            }
        };

        if steps.is_empty() {
            set_error.set(Some("At least one step is required".to_string()));
            set_saving.set(false);
            return;
        }

        let workflow = Workflow {
            name: name.get(),
            description: description.get(),
            input_schema,
            output_schema,
            steps,
            on_error: ErrorStrategy::Fail,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_workflow(&workflow).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/workflows");
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6">
            <div class="mb-6">
                <a href="/workflows" class="text-orange-500 hover:underline flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                    "Back to Workflows"
                </a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"New Workflow"</h2>

            <form on:submit=on_submit class="bg-white rounded-lg shadow p-6 max-w-2xl">
                {move || error.get().map(|e| view! {
                    <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                        {e}
                    </div>
                })}

                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                        <input
                            type="text"
                            required=true
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                            placeholder="my-workflow"
                            prop:value=move || name.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_name.set(input.value());
                            }
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Description *"</label>
                        <input
                            type="text"
                            required=true
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                            placeholder="What this workflow does"
                            prop:value=move || description.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                set_description.set(input.value());
                            }
                        />
                    </div>

                    <div>
                        <JsonSchemaEditor
                            properties=schema_properties
                            set_properties=set_schema_properties
                            label="Input Schema"
                            color="orange"
                        />
                        <SchemaPreview properties=schema_properties />
                    </div>

                    <div>
                        <JsonSchemaEditor
                            properties=output_schema_properties
                            set_properties=set_output_schema_properties
                            label="Output Schema"
                            color="blue"
                        />
                        <SchemaPreview properties=output_schema_properties />
                    </div>

                    <div>
                        {move || {
                            let tools = available_tools.get().unwrap_or_default();
                            view! {
                                <WorkflowStepsEditor
                                    steps=steps_data
                                    set_steps=set_steps_data
                                    available_tools=tools
                                />
                            }
                        }}
                    </div>
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-orange-500 text-white rounded hover:bg-orange-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Workflow" }}
                    </button>
                    <a
                        href="/workflows"
                        class="px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                    >
                        "Cancel"
                    </a>
                </div>
            </form>
        </div>
    }
}

#[component]
pub fn WorkflowEditForm() -> impl IntoView {
    let params = use_params_map();
    let workflow_name = move || params.read().get("name").unwrap_or_default();

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (schema_properties, set_schema_properties) = signal(Vec::<SchemaProperty>::new());
    let (output_schema_properties, set_output_schema_properties) = signal(Vec::<SchemaProperty>::new());
    let (steps_data, set_steps_data) = signal(Vec::<StepData>::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);
    let (loading, set_loading) = signal(true);
    let (original_name, set_original_name) = signal(String::new());

    // Load available tools for dropdown
    let available_tools = LocalResource::new(move || async move {
        api::list_tools().await.ok().map(|tools| {
            tools.into_iter().map(|t| t.name).collect::<Vec<_>>()
        }).unwrap_or_default()
    });

    // Load existing workflow
    Effect::new(move |_| {
        let name_param = workflow_name();
        // Skip if name is empty (params not ready yet)
        if name_param.is_empty() {
            return;
        }
        set_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_workflow(&name_param).await {
                Ok(workflow) => {
                    set_original_name.set(workflow.name.clone());
                    set_name.set(workflow.name.clone());
                    set_description.set(workflow.description.clone());
                    set_schema_properties.set(schema_to_properties(&workflow.input_schema));
                    // Load output schema if present
                    if let Some(output_schema) = &workflow.output_schema {
                        set_output_schema_properties.set(schema_to_properties(output_schema));
                    }
                    set_steps_data.set(workflow_steps_to_steps_data(&workflow.steps));
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load workflow: {}", e)));
                }
            }
            set_loading.set(false);
        });
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let orig_name = original_name.get();

        let input_schema = properties_to_schema(&schema_properties.get());
        let output_props = output_schema_properties.get();
        let output_schema = if output_props.is_empty() {
            None
        } else {
            Some(properties_to_schema(&output_props))
        };

        let steps = match steps_data_to_workflow_steps(&steps_data.get()) {
            Ok(s) => s,
            Err(e) => {
                set_error.set(Some(e));
                set_saving.set(false);
                return;
            }
        };

        if steps.is_empty() {
            set_error.set(Some("At least one step is required".to_string()));
            set_saving.set(false);
            return;
        }

        let workflow = Workflow {
            name: name.get(),
            description: description.get(),
            input_schema,
            output_schema,
            steps,
            on_error: ErrorStrategy::Fail,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_workflow(&orig_name, &workflow).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/workflows");
                    }
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_saving.set(false);
                }
            }
        });
    };

    view! {
        <div class="p-6">
            <div class="mb-6">
                <a href="/workflows" class="text-orange-500 hover:underline flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                    "Back to Workflows"
                </a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"Edit Workflow"</h2>

            {move || if loading.get() {
                view! {
                    <div class="flex items-center justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-500"></div>
                        <span class="ml-3 text-gray-500">"Loading workflow..."</span>
                    </div>
                }.into_any()
            } else {
                view! {
                    <form on:submit=on_submit class="bg-white rounded-lg shadow p-6 max-w-2xl">
                        {move || error.get().map(|e| view! {
                            <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                                {e}
                            </div>
                        })}

                        <div class="space-y-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                                <input
                                    type="text"
                                    required=true
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                                    placeholder="my-workflow"
                                    prop:value=move || name.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_name.set(input.value());
                                    }
                                />
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Description *"</label>
                                <input
                                    type="text"
                                    required=true
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-orange-500"
                                    placeholder="What this workflow does"
                                    prop:value=move || description.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_description.set(input.value());
                                    }
                                />
                            </div>

                            <div>
                                <JsonSchemaEditor
                                    properties=schema_properties
                                    set_properties=set_schema_properties
                                    label="Input Schema"
                                    color="orange"
                                />
                                <SchemaPreview properties=schema_properties />
                            </div>

                            <div>
                                <JsonSchemaEditor
                                    properties=output_schema_properties
                                    set_properties=set_output_schema_properties
                                    label="Output Schema"
                                    color="blue"
                                />
                                <SchemaPreview properties=output_schema_properties />
                            </div>

                            <div>
                                {move || {
                                    let tools = available_tools.get().unwrap_or_default();
                                    view! {
                                        <WorkflowStepsEditor
                                            steps=steps_data
                                            set_steps=set_steps_data
                                            available_tools=tools
                                        />
                                    }
                                }}
                            </div>
                        </div>

                        <div class="mt-6 flex gap-3">
                            <button
                                type="submit"
                                disabled=move || saving.get()
                                class="px-4 py-2 bg-orange-500 text-white rounded hover:bg-orange-600 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                            </button>
                            <a
                                href="/workflows"
                                class="px-4 py-2 border border-gray-300 text-gray-700 rounded hover:bg-gray-50"
                            >
                                "Cancel"
                            </a>
                        </div>
                    </form>
                }.into_any()
            }}
        </div>
    }
}
