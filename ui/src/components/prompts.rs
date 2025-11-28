use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::api;
use crate::types::{Prompt, PromptArgument, PromptMessage};
use crate::components::schema_editor::{JsonSchemaEditor, SchemaPreview, SchemaProperty, properties_to_schema};

/// Convert SchemaProperty to PromptArgument
fn schema_property_to_argument(prop: &SchemaProperty) -> PromptArgument {
    PromptArgument {
        name: prop.name.clone(),
        description: if prop.description.is_empty() { None } else { Some(prop.description.clone()) },
        required: prop.required,
    }
}

/// Convert PromptArgument to SchemaProperty
fn argument_to_schema_property(arg: &PromptArgument) -> SchemaProperty {
    let mut prop = SchemaProperty::new();
    prop.name = arg.name.clone();
    prop.description = arg.description.clone().unwrap_or_default();
    prop.required = arg.required;
    prop.prop_type = "string".to_string(); // MCP prompt arguments are strings
    prop
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Table,
    Card,
}

/// Build JSON object from prompt form fields (all string values)
fn build_json_from_prompt_fields(fields: &HashMap<String, String>) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (name, value) in fields {
        if !value.is_empty() {
            obj.insert(name.clone(), serde_json::Value::String(value.clone()));
        }
    }
    serde_json::Value::Object(obj)
}

#[component]
pub fn Prompts() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    // Test modal state - stores full Prompt to access arguments
    let (test_target, set_test_target) = signal(Option::<Prompt>::None);
    let (test_form_fields, set_test_form_fields) = signal(HashMap::<String, String>::new());
    let (test_result, set_test_result) = signal(Option::<Result<crate::types::TestResult, String>>::None);
    let (testing, set_testing) = signal(false);

    let prompts = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_prompts().await.ok() }
    });

    let on_delete_confirm = move |_| {
        if let Some(name) = delete_target.get() {
            set_deleting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_prompt(&name).await {
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
        if let Some(prompt) = test_target.get() {
            set_testing.set(true);
            set_test_result.set(None);
            let fields = test_form_fields.get();
            let prompt_name = prompt.name.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let args = build_json_from_prompt_fields(&fields);
                let result = api::test_prompt(&prompt_name, &args).await;
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
                <h2 class="text-2xl font-bold">"Prompts"</h2>
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
                    <a href="/prompts/new" class="bg-purple-500 hover:bg-purple-600 text-white px-4 py-2 rounded flex items-center gap-2">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Prompt"
                    </a>
                </div>
            </div>

            // Delete confirmation modal
            {move || delete_target.get().map(|name| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"Delete Prompt?"</h3>
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
            {move || test_target.get().map(|prompt| {
                let args = prompt.arguments.clone().unwrap_or_default();
                let has_no_args = args.is_empty();

                view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-2xl w-full mx-4 max-h-[90vh] overflow-y-auto">
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-semibold text-gray-900">
                                "Test Prompt: "
                                <span class="font-mono text-purple-600">{prompt.name.clone()}</span>
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

                        // Dynamic form fields based on prompt arguments
                        <div class="mb-4 space-y-4">
                            {if has_no_args {
                                view! {
                                    <div class="text-sm text-gray-500 italic p-3 bg-gray-50 rounded-md">
                                        "This prompt has no arguments defined."
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="space-y-4">
                                        {args.into_iter().map(|arg| {
                                            let field_name = arg.name.clone();
                                            let field_name_for_handler = arg.name.clone();
                                            let label = format!("{}{}", arg.name, if arg.required { " *" } else { "" });

                                            view! {
                                                <div>
                                                    <label class="block text-sm font-medium text-gray-700 mb-1">
                                                        {label}
                                                    </label>
                                                    <input
                                                        type="text"
                                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                                        placeholder=format!("Enter {}", field_name)
                                                        on:input=move |ev| {
                                                            let target = ev.target().unwrap();
                                                            let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                            set_test_form_fields.update(|fields| {
                                                                fields.insert(field_name_for_handler.clone(), input.value());
                                                            });
                                                        }
                                                    />
                                                    {arg.description.clone().map(|desc| view! {
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
                                class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 flex items-center gap-2"
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
                                        <pre class="bg-gray-900 text-purple-400 p-4 rounded-lg overflow-x-auto text-sm font-mono">
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
                    prompts.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => {
                                if view_mode.get() == ViewMode::Table {
                                    view! { <PromptTable prompts=list set_delete_target=set_delete_target set_test_target=set_test_target /> }.into_any()
                                } else {
                                    view! { <PromptCards prompts=list set_delete_target=set_delete_target set_test_target=set_test_target /> }.into_any()
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
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500"></div>
            <span class="ml-3 text-gray-500">"Loading prompts..."</span>
        </div>
    }
}

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-white rounded-lg shadow">
            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No prompts"</h3>
            <p class="mt-1 text-sm text-gray-500">"Get started by creating a new prompt."</p>
            <div class="mt-6">
                <a href="/prompts/new" class="inline-flex items-center px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New Prompt"
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
            <h3 class="mt-2 text-sm font-medium text-red-800">"Failed to load prompts"</h3>
            <p class="mt-1 text-sm text-red-600">"Please check your connection and try again."</p>
        </div>
    }
}

#[component]
fn PromptTable(
    prompts: Vec<Prompt>,
    set_delete_target: WriteSignal<Option<String>>,
    set_test_target: WriteSignal<Option<Prompt>>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Description"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Arguments"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Messages"</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                    </tr>
                </thead>
                <tbody class="bg-white divide-y divide-gray-200">
                    {prompts.into_iter().map(|prompt| {
                        let name_for_edit = prompt.name.clone();
                        let name_for_delete = prompt.name.clone();
                        let prompt_for_test = prompt.clone();
                        let args_count = prompt.arguments.as_ref().map(|a| a.len()).unwrap_or(0);
                        let msgs_count = prompt.messages.as_ref().map(|m| m.len()).unwrap_or(0);

                        view! {
                            <tr class="hover:bg-gray-50">
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <div class="font-medium text-gray-900">{prompt.name.clone()}</div>
                                </td>
                                <td class="px-6 py-4">
                                    <div class="text-sm text-gray-500 truncate max-w-md">{prompt.description.clone()}</div>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <span class="px-2 py-1 text-xs font-semibold rounded-full bg-purple-100 text-purple-800">
                                        {format!("{} args", args_count)}
                                    </span>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <span class="px-2 py-1 text-xs font-semibold rounded-full bg-gray-100 text-gray-800">
                                        {format!("{} msgs", msgs_count)}
                                    </span>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                    <button
                                        class="text-purple-600 hover:text-purple-900 mr-3"
                                        on:click=move |_| set_test_target.set(Some(prompt_for_test.clone()))
                                    >
                                        "Test"
                                    </button>
                                    <a
                                        href=format!("/prompts/edit/{}", name_for_edit)
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
fn PromptCards(
    prompts: Vec<Prompt>,
    set_delete_target: WriteSignal<Option<String>>,
    set_test_target: WriteSignal<Option<Prompt>>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {prompts.into_iter().map(|prompt| {
                let name_for_edit = prompt.name.clone();
                let name_for_delete = prompt.name.clone();
                let prompt_for_test = prompt.clone();
                let args_count = prompt.arguments.as_ref().map(|a| a.len()).unwrap_or(0);
                let msgs_count = prompt.messages.as_ref().map(|m| m.len()).unwrap_or(0);

                view! {
                    <div class="bg-white rounded-lg shadow hover:shadow-md transition-shadow p-4">
                        <div class="flex justify-between items-start mb-3">
                            <div class="flex-1 min-w-0">
                                <h3 class="font-semibold text-gray-900 truncate">{prompt.name.clone()}</h3>
                                <p class="text-sm text-gray-500 line-clamp-2">{prompt.description.clone()}</p>
                            </div>
                        </div>
                        <div class="flex gap-2 mb-4">
                            <span class="px-2 py-1 text-xs font-semibold rounded-full bg-purple-100 text-purple-800">
                                {format!("{} args", args_count)}
                            </span>
                            <span class="px-2 py-1 text-xs font-semibold rounded-full bg-gray-100 text-gray-800">
                                {format!("{} msgs", msgs_count)}
                            </span>
                        </div>
                        <div class="flex justify-end gap-2 pt-3 border-t border-gray-100">
                            <button
                                class="px-3 py-1 text-sm text-purple-600 hover:bg-purple-50 rounded"
                                on:click=move |_| set_test_target.set(Some(prompt_for_test.clone()))
                            >
                                "Test"
                            </button>
                            <a
                                href=format!("/prompts/edit/{}", name_for_edit)
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
pub fn PromptForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (args_properties, set_args_properties) = signal(Vec::<SchemaProperty>::new());
    let (messages_json, set_messages_json) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        // Validate name
        let name_val = name.get();
        let name_trimmed = name_val.trim();
        if name_trimmed.is_empty() {
            set_error.set(Some("Name cannot be blank".to_string()));
            set_saving.set(false);
            return;
        }
        // Validate name format (alphanumeric, underscore, hyphen only)
        if !name_trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            set_error.set(Some("Name can only contain letters, numbers, underscores, and hyphens".to_string()));
            set_saving.set(false);
            return;
        }

        // Convert schema properties to prompt arguments
        let props = args_properties.get();
        let arguments: Option<Vec<PromptArgument>> = if props.is_empty() {
            None
        } else {
            Some(props.iter()
                .filter(|p| !p.name.is_empty())
                .map(schema_property_to_argument)
                .collect())
        };

        // Also generate input_schema from the properties
        let input_schema = if props.is_empty() {
            None
        } else {
            let schema = properties_to_schema(&props);
            Some(schema)
        };

        let messages: Option<Vec<PromptMessage>> = if messages_json.get().is_empty() {
            None
        } else {
            match serde_json::from_str(&messages_json.get()) {
                Ok(msgs) => Some(msgs),
                Err(e) => {
                    set_error.set(Some(format!("Invalid messages JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        };

        let prompt = Prompt {
            name: name.get(),
            description: description.get(),
            arguments,
            input_schema,
            messages,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_prompt(&prompt).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/prompts");
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
                <a href="/prompts" class="text-purple-500 hover:underline">"← Back to Prompts"</a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"New Prompt"</h2>

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
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="my-prompt"
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
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                            placeholder="What this prompt does"
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
                            properties=args_properties
                            set_properties=set_args_properties
                            label="Arguments"
                            color="purple"
                        />
                        <SchemaPreview properties=args_properties />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-gray-700 mb-1">"Messages (JSON Array)"</label>
                        <textarea
                            rows=6
                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                            placeholder=r#"[{"role": "user", "content": "Hello"}]"#
                            prop:value=move || messages_json.get()
                            on:input=move |ev| {
                                let target = ev.target().unwrap();
                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                set_messages_json.set(textarea.value());
                            }
                        />
                        <p class="mt-1 text-xs text-gray-500">"Optional array of prompt messages"</p>
                    </div>
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Prompt" }}
                    </button>
                    <a
                        href="/prompts"
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
pub fn PromptEditForm() -> impl IntoView {
    let params = use_params_map();
    let prompt_name = move || params.read().get("name").unwrap_or_default();

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (args_properties, set_args_properties) = signal(Vec::<SchemaProperty>::new());
    let (messages_json, set_messages_json) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);
    let (loading, set_loading) = signal(true);
    let (original_name, set_original_name) = signal(String::new());

    // Load existing prompt
    Effect::new(move |_| {
        let name_param = prompt_name();
        // Skip if name is empty (params not ready yet)
        if name_param.is_empty() {
            return;
        }
        set_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_prompt(&name_param).await {
                Ok(prompt) => {
                    set_original_name.set(prompt.name.clone());
                    set_name.set(prompt.name.clone());
                    set_description.set(prompt.description.clone());
                    // Convert arguments to schema properties
                    if let Some(args) = &prompt.arguments {
                        let properties: Vec<SchemaProperty> = args.iter()
                            .map(argument_to_schema_property)
                            .collect();
                        set_args_properties.set(properties);
                    }
                    if let Some(msgs) = &prompt.messages {
                        set_messages_json.set(serde_json::to_string_pretty(msgs).unwrap_or_default());
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load prompt: {}", e)));
                }
            }
            set_loading.set(false);
        });
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        // Validate name
        let name_val = name.get();
        let name_trimmed = name_val.trim();
        if name_trimmed.is_empty() {
            set_error.set(Some("Name cannot be blank".to_string()));
            set_saving.set(false);
            return;
        }
        if !name_trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            set_error.set(Some("Name can only contain letters, numbers, underscores, and hyphens".to_string()));
            set_saving.set(false);
            return;
        }

        let orig_name = original_name.get();

        // Convert schema properties to prompt arguments
        let props = args_properties.get();
        let arguments: Option<Vec<PromptArgument>> = if props.is_empty() {
            None
        } else {
            Some(props.iter()
                .filter(|p| !p.name.is_empty())
                .map(schema_property_to_argument)
                .collect())
        };

        // Also generate input_schema from the properties
        let input_schema = if props.is_empty() {
            None
        } else {
            let schema = properties_to_schema(&props);
            Some(schema)
        };

        let messages: Option<Vec<PromptMessage>> = if messages_json.get().is_empty() {
            None
        } else {
            match serde_json::from_str(&messages_json.get()) {
                Ok(msgs) => Some(msgs),
                Err(e) => {
                    set_error.set(Some(format!("Invalid messages JSON: {}", e)));
                    set_saving.set(false);
                    return;
                }
            }
        };

        let prompt = Prompt {
            name: name.get(),
            description: description.get(),
            arguments,
            input_schema,
            messages,
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_prompt(&orig_name, &prompt).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/prompts");
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
                <a href="/prompts" class="text-purple-500 hover:underline flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                    "Back to Prompts"
                </a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"Edit Prompt"</h2>

            {move || if loading.get() {
                view! {
                    <div class="flex items-center justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500"></div>
                        <span class="ml-3 text-gray-500">"Loading prompt..."</span>
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
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                    placeholder="my-prompt"
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
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500"
                                    placeholder="What this prompt does"
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
                                    properties=args_properties
                                    set_properties=set_args_properties
                                    label="Arguments"
                                    color="purple"
                                />
                                <SchemaPreview properties=args_properties />
                            </div>

                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Messages (JSON Array)"</label>
                                <textarea
                                    rows=6
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-500 font-mono text-sm"
                                    placeholder=r#"[{"role": "user", "content": "Hello"}]"#
                                    prop:value=move || messages_json.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                        set_messages_json.set(textarea.value());
                                    }
                                />
                                <p class="mt-1 text-xs text-gray-500">"Optional array of prompt messages"</p>
                            </div>
                        </div>

                        <div class="mt-6 flex gap-3">
                            <button
                                type="submit"
                                disabled=move || saving.get()
                                class="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                            </button>
                            <a
                                href="/prompts"
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
