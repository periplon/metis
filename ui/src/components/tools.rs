use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_params_map;
use wasm_bindgen::JsCast;
use crate::api;
use crate::types::{
    Tool, MockConfig, MockStrategyType, StatefulConfig, StateOperation,
    FileConfig, ScriptLang, LLMConfig, LLMProvider, DatabaseConfig,
};

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Table,
    Card,
}

#[component]
pub fn Tools() -> impl IntoView {
    let (view_mode, set_view_mode) = signal(ViewMode::Table);
    let (refresh_trigger, set_refresh_trigger) = signal(0u32);
    let (delete_target, set_delete_target) = signal(Option::<String>::None);
    let (deleting, set_deleting) = signal(false);

    let tools = LocalResource::new(move || {
        let _ = refresh_trigger.get();
        async move { api::list_tools().await.ok() }
    });

    let on_delete_confirm = move |_| {
        if let Some(name) = delete_target.get() {
            set_deleting.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match api::delete_tool(&name).await {
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

    view! {
        <div class="p-6">
            // Header with title, view toggle, and new button
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Tools"</h2>
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
                    <a href="/tools/new" class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded flex items-center gap-2">
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                        "New Tool"
                    </a>
                </div>
            </div>

            // Delete confirmation modal
            {move || delete_target.get().map(|name| view! {
                <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                    <div class="bg-white rounded-lg shadow-xl p-6 max-w-md w-full mx-4">
                        <h3 class="text-lg font-semibold text-gray-900 mb-2">"Delete Tool?"</h3>
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

            <Suspense fallback=move || view! { <LoadingState /> }>
                {move || {
                    tools.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => {
                                if view_mode.get() == ViewMode::Table {
                                    view! { <ToolTable tools=list set_delete_target=set_delete_target /> }.into_any()
                                } else {
                                    view! { <ToolCards tools=list set_delete_target=set_delete_target /> }.into_any()
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
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-green-500"></div>
            <span class="ml-3 text-gray-500">"Loading tools..."</span>
        </div>
    }
}

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="text-center py-12 bg-white rounded-lg shadow">
            <svg class="mx-auto h-12 w-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
            </svg>
            <h3 class="mt-2 text-sm font-medium text-gray-900">"No tools"</h3>
            <p class="mt-1 text-sm text-gray-500">"Get started by creating a new tool."</p>
            <div class="mt-6">
                <a href="/tools/new" class="inline-flex items-center px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                    </svg>
                    "New Tool"
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
            <h3 class="mt-2 text-sm font-medium text-red-800">"Failed to load tools"</h3>
            <p class="mt-1 text-sm text-red-600">"Please check your connection and try again."</p>
        </div>
    }
}

#[component]
fn ToolTable(
    tools: Vec<Tool>,
    set_delete_target: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="bg-white rounded-lg shadow overflow-hidden">
            <table class="min-w-full divide-y divide-gray-200">
                <thead class="bg-gray-50">
                    <tr>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Name"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Description"</th>
                        <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">"Strategy"</th>
                        <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">"Actions"</th>
                    </tr>
                </thead>
                <tbody class="bg-white divide-y divide-gray-200">
                    {tools.into_iter().map(|tool| {
                        let name_for_edit = tool.name.clone();
                        let name_for_delete = tool.name.clone();
                        let strategy = tool.mock.as_ref()
                            .map(|m| format!("{:?}", m.strategy))
                            .unwrap_or_else(|| if tool.static_response.is_some() { "Static".to_string() } else { "-".to_string() });

                        view! {
                            <tr class="hover:bg-gray-50">
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <div class="font-medium text-gray-900">{tool.name.clone()}</div>
                                </td>
                                <td class="px-6 py-4">
                                    <div class="text-sm text-gray-500 truncate max-w-md">{tool.description.clone()}</div>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap">
                                    <span class="px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800">
                                        {strategy}
                                    </span>
                                </td>
                                <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                    <a
                                        href=format!("/tools/edit/{}", name_for_edit)
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
fn ToolCards(
    tools: Vec<Tool>,
    set_delete_target: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {tools.into_iter().map(|tool| {
                let name_for_edit = tool.name.clone();
                let name_for_delete = tool.name.clone();
                let strategy = tool.mock.as_ref()
                    .map(|m| format!("{:?}", m.strategy))
                    .unwrap_or_else(|| if tool.static_response.is_some() { "Static".to_string() } else { "-".to_string() });

                view! {
                    <div class="bg-white rounded-lg shadow hover:shadow-md transition-shadow p-4">
                        <div class="flex justify-between items-start mb-3">
                            <div class="flex-1 min-w-0">
                                <h3 class="font-semibold text-gray-900 truncate">{tool.name.clone()}</h3>
                                <p class="text-sm text-gray-500 line-clamp-2">{tool.description.clone()}</p>
                            </div>
                            <span class="ml-2 px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800 flex-shrink-0">
                                {strategy}
                            </span>
                        </div>
                        <div class="flex justify-end gap-2 pt-3 border-t border-gray-100">
                            <a
                                href=format!("/tools/edit/{}", name_for_edit)
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
pub fn ToolForm() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (input_schema, set_input_schema) = signal(String::from("{}"));
    let (static_response, set_static_response) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);

    // Mock strategy signals
    let (mock_strategy, set_mock_strategy) = signal("none".to_string());
    let (mock_template, set_mock_template) = signal(String::new());
    let (mock_faker_type, set_mock_faker_type) = signal(String::new());
    let (mock_state_key, set_mock_state_key) = signal(String::new());
    let (mock_state_operation, set_mock_state_operation) = signal("get".to_string());
    let (mock_script, set_mock_script) = signal(String::new());
    let (mock_script_lang, set_mock_script_lang) = signal("rhai".to_string());
    let (mock_file_path, set_mock_file_path) = signal(String::new());
    let (mock_file_selection, set_mock_file_selection) = signal("random".to_string());
    let (mock_pattern, set_mock_pattern) = signal(String::new());
    let (mock_llm_provider, set_mock_llm_provider) = signal("openai".to_string());
    let (mock_llm_model, set_mock_llm_model) = signal(String::new());
    let (mock_llm_api_key_env, set_mock_llm_api_key_env) = signal(String::new());
    let (mock_llm_system_prompt, set_mock_llm_system_prompt) = signal(String::new());
    let (mock_db_url, set_mock_db_url) = signal(String::new());
    let (mock_db_query, set_mock_db_query) = signal(String::new());

    let build_mock_config = move || -> Option<MockConfig> {
        let strategy = mock_strategy.get();
        if strategy == "none" {
            return None;
        }

        let strategy_type = match strategy.as_str() {
            "static" => MockStrategyType::Static,
            "template" => MockStrategyType::Template,
            "random" => MockStrategyType::Random,
            "stateful" => MockStrategyType::Stateful,
            "script" => MockStrategyType::Script,
            "file" => MockStrategyType::File,
            "pattern" => MockStrategyType::Pattern,
            "llm" => MockStrategyType::LLM,
            "database" => MockStrategyType::Database,
            _ => return None,
        };

        let mut config = MockConfig {
            strategy: strategy_type,
            ..Default::default()
        };

        match strategy.as_str() {
            "template" => {
                config.template = Some(mock_template.get());
            }
            "random" => {
                if !mock_faker_type.get().is_empty() {
                    config.faker_type = Some(mock_faker_type.get());
                }
            }
            "stateful" => {
                config.stateful = Some(StatefulConfig {
                    state_key: mock_state_key.get(),
                    operation: match mock_state_operation.get().as_str() {
                        "set" => StateOperation::Set,
                        "increment" => StateOperation::Increment,
                        _ => StateOperation::Get,
                    },
                    template: if mock_template.get().is_empty() { None } else { Some(mock_template.get()) },
                });
            }
            "script" => {
                config.script = Some(mock_script.get());
                config.script_lang = Some(match mock_script_lang.get().as_str() {
                    "lua" => ScriptLang::Lua,
                    "js" => ScriptLang::Js,
                    "python" => ScriptLang::Python,
                    _ => ScriptLang::Rhai,
                });
            }
            "file" => {
                config.file = Some(FileConfig {
                    path: mock_file_path.get(),
                    selection: mock_file_selection.get(),
                });
            }
            "pattern" => {
                config.pattern = Some(mock_pattern.get());
            }
            "llm" => {
                config.llm = Some(LLMConfig {
                    provider: match mock_llm_provider.get().as_str() {
                        "anthropic" => LLMProvider::Anthropic,
                        _ => LLMProvider::OpenAI,
                    },
                    api_key_env: if mock_llm_api_key_env.get().is_empty() { None } else { Some(mock_llm_api_key_env.get()) },
                    model: mock_llm_model.get(),
                    system_prompt: if mock_llm_system_prompt.get().is_empty() { None } else { Some(mock_llm_system_prompt.get()) },
                    temperature: None,
                    max_tokens: None,
                    stream: false,
                });
            }
            "database" => {
                config.database = Some(DatabaseConfig {
                    url: mock_db_url.get(),
                    query: mock_db_query.get(),
                    params: vec![],
                });
            }
            _ => {}
        }

        Some(config)
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let schema: serde_json::Value = serde_json::from_str(&input_schema.get())
            .unwrap_or(serde_json::json!({}));

        let static_resp: Option<serde_json::Value> = if static_response.get().is_empty() {
            None
        } else {
            serde_json::from_str(&static_response.get()).ok()
        };

        let tool = Tool {
            name: name.get(),
            description: description.get(),
            input_schema: schema,
            static_response: static_resp,
            mock: build_mock_config(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::create_tool(&tool).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/tools");
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
                <a href="/tools" class="text-green-500 hover:underline">"← Back to Tools"</a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"New Tool"</h2>

            <form on:submit=on_submit class="bg-white rounded-lg shadow p-6 max-w-3xl">
                {move || error.get().map(|e| view! {
                    <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                        {e}
                    </div>
                })}

                <div class="space-y-6">
                    // Basic Info Section
                    <div class="border-b pb-4">
                        <h3 class="text-lg font-semibold text-gray-800 mb-4">"Basic Information"</h3>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                                <input
                                    type="text"
                                    required=true
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                    placeholder="my-tool"
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
                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                    placeholder="What this tool does"
                                    prop:value=move || description.get()
                                    on:input=move |ev| {
                                        let target = ev.target().unwrap();
                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                        set_description.set(input.value());
                                    }
                                />
                            </div>
                        </div>
                        <div class="mt-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Input Schema (JSON)"</label>
                            <textarea
                                rows=6
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                placeholder=r#"{"type": "object", "properties": {"query": {"type": "string"}}}"#
                                prop:value=move || input_schema.get()
                                on:input=move |ev| {
                                    let target = ev.target().unwrap();
                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                    set_input_schema.set(textarea.value());
                                }
                            />
                            <p class="mt-1 text-xs text-gray-500">"JSON Schema for tool input parameters"</p>
                        </div>
                    </div>

                    // Response Strategy Section
                    <div class="border-b pb-4">
                        <h3 class="text-lg font-semibold text-gray-800 mb-4">"Response Strategy"</h3>

                        <div class="mb-4">
                            <label class="block text-sm font-medium text-gray-700 mb-1">"Strategy Type"</label>
                            <select
                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                prop:value=move || mock_strategy.get()
                                on:change=move |ev| {
                                    let target = ev.target().unwrap();
                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                    set_mock_strategy.set(select.value());
                                }
                            >
                                <option value="none">"No Mock (Use Static Response)"</option>
                                <option value="static">"Static"</option>
                                <option value="template">"Template (Handlebars)"</option>
                                <option value="random">"Random (Faker)"</option>
                                <option value="stateful">"Stateful"</option>
                                <option value="script">"Script"</option>
                                <option value="file">"File"</option>
                                <option value="pattern">"Pattern (Regex)"</option>
                                <option value="llm">"LLM (AI Generated)"</option>
                                <option value="database">"Database Query"</option>
                            </select>
                            <p class="mt-1 text-xs text-gray-500">"Choose how the tool response should be generated"</p>
                        </div>

                        // Strategy-specific fields
                        {move || {
                            let strategy = mock_strategy.get();
                            match strategy.as_str() {
                                "none" | "static" => view! {
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"Static Response (JSON)"</label>
                                        <textarea
                                            rows=6
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                            placeholder=r#"{"result": "success", "data": {}}"#
                                            prop:value=move || static_response.get()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                set_static_response.set(textarea.value());
                                            }
                                        />
                                    </div>
                                }.into_any(),
                                "template" => view! {
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"Handlebars Template *"</label>
                                        <textarea
                                            rows=6
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                            placeholder=r#"{"id": "{{uuid}}", "query": "{{input.query}}", "timestamp": "{{now '%Y-%m-%d'}}"}"#
                                            prop:value=move || mock_template.get()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                set_mock_template.set(textarea.value());
                                            }
                                        />
                                        <p class="mt-1 text-xs text-gray-500">"Use {{input.field}} to access tool arguments. Helpers: now, uuid, random_int, random_float, random_bool, random_string, json_encode"</p>
                                    </div>
                                }.into_any(),
                                "random" => view! {
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"Faker Type"</label>
                                        <select
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                            prop:value=move || mock_faker_type.get()
                                            on:change=move |ev| {
                                                let target = ev.target().unwrap();
                                                let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                set_mock_faker_type.set(select.value());
                                            }
                                        >
                                            <option value="">"Default (lorem ipsum)"</option>
                                            <option value="name">"Name"</option>
                                            <option value="email">"Email"</option>
                                            <option value="phone">"Phone"</option>
                                            <option value="address">"Address"</option>
                                            <option value="company">"Company"</option>
                                            <option value="uuid">"UUID"</option>
                                            <option value="sentence">"Sentence"</option>
                                            <option value="paragraph">"Paragraph"</option>
                                        </select>
                                        <p class="mt-1 text-xs text-gray-500">"Generate random fake data"</p>
                                    </div>
                                }.into_any(),
                                "stateful" => view! {
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"State Key *"</label>
                                            <input
                                                type="text"
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                placeholder="tool-call-counter"
                                                prop:value=move || mock_state_key.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                    set_mock_state_key.set(input.value());
                                                }
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"Operation"</label>
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                prop:value=move || mock_state_operation.get()
                                                on:change=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                    set_mock_state_operation.set(select.value());
                                                }
                                            >
                                                <option value="get">"Get"</option>
                                                <option value="set">"Set"</option>
                                                <option value="increment">"Increment"</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"Response Template (optional)"</label>
                                            <textarea
                                                rows=3
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                                placeholder=r#"{"call_count": {{state}}, "input": {{json_encode input}}}"#
                                                prop:value=move || mock_template.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                    set_mock_template.set(textarea.value());
                                                }
                                            />
                                        </div>
                                    </div>
                                }.into_any(),
                                "script" => view! {
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"Script Language"</label>
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                prop:value=move || mock_script_lang.get()
                                                on:change=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                    set_mock_script_lang.set(select.value());
                                                }
                                            >
                                                <option value="rhai">"Rhai"</option>
                                                <option value="lua">"Lua"</option>
                                                <option value="js">"JavaScript"</option>
                                                <option value="python">"Python"</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"Script *"</label>
                                            <textarea
                                                rows=8
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                                placeholder="// Process tool input\nlet result = #{\"processed\": true, \"input\": input};\nto_json(result)"
                                                prop:value=move || mock_script.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                    set_mock_script.set(textarea.value());
                                                }
                                            />
                                            <p class="mt-1 text-xs text-gray-500">"Access tool arguments via 'input' object, return JSON string"</p>
                                        </div>
                                    </div>
                                }.into_any(),
                                "file" => view! {
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"File Path *"</label>
                                            <input
                                                type="text"
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                placeholder="/path/to/tool-responses.json"
                                                prop:value=move || mock_file_path.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                    set_mock_file_path.set(input.value());
                                                }
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"Selection Mode"</label>
                                            <select
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                prop:value=move || mock_file_selection.get()
                                                on:change=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                    set_mock_file_selection.set(select.value());
                                                }
                                            >
                                                <option value="random">"Random"</option>
                                                <option value="sequential">"Sequential"</option>
                                                <option value="first">"First"</option>
                                                <option value="last">"Last"</option>
                                            </select>
                                            <p class="mt-1 text-xs text-gray-500">"How to select from multiple responses in the file"</p>
                                        </div>
                                    </div>
                                }.into_any(),
                                "pattern" => view! {
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"Pattern *"</label>
                                        <input
                                            type="text"
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono"
                                            placeholder="RES-[A-Z]{4}-[0-9]{6}"
                                            prop:value=move || mock_pattern.get()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                set_mock_pattern.set(input.value());
                                            }
                                        />
                                        <p class="mt-1 text-xs text-gray-500">"Regex-like pattern to generate random strings"</p>
                                    </div>
                                }.into_any(),
                                "llm" => view! {
                                    <div class="space-y-4">
                                        <div class="grid grid-cols-2 gap-4">
                                            <div>
                                                <label class="block text-sm font-medium text-gray-700 mb-1">"Provider"</label>
                                                <select
                                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                    prop:value=move || mock_llm_provider.get()
                                                    on:change=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                                        set_mock_llm_provider.set(select.value());
                                                    }
                                                >
                                                    <option value="openai">"OpenAI"</option>
                                                    <option value="anthropic">"Anthropic"</option>
                                                </select>
                                            </div>
                                            <div>
                                                <label class="block text-sm font-medium text-gray-700 mb-1">"Model *"</label>
                                                <input
                                                    type="text"
                                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                    placeholder="gpt-4"
                                                    prop:value=move || mock_llm_model.get()
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                        set_mock_llm_model.set(input.value());
                                                    }
                                                />
                                            </div>
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"API Key Environment Variable"</label>
                                            <input
                                                type="text"
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                placeholder="OPENAI_API_KEY"
                                                prop:value=move || mock_llm_api_key_env.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                    set_mock_llm_api_key_env.set(input.value());
                                                }
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"System Prompt"</label>
                                            <textarea
                                                rows=4
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                                placeholder="You are a tool simulator. Generate realistic responses based on the input..."
                                                prop:value=move || mock_llm_system_prompt.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                    set_mock_llm_system_prompt.set(textarea.value());
                                                }
                                            />
                                        </div>
                                    </div>
                                }.into_any(),
                                "database" => view! {
                                    <div class="space-y-4">
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"Database URL *"</label>
                                            <input
                                                type="text"
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono"
                                                placeholder="postgres://user:pass@host/db"
                                                prop:value=move || mock_db_url.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                    set_mock_db_url.set(input.value());
                                                }
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-sm font-medium text-gray-700 mb-1">"SQL Query *"</label>
                                            <textarea
                                                rows=4
                                                class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                                placeholder="SELECT * FROM results WHERE query = $1"
                                                prop:value=move || mock_db_query.get()
                                                on:input=move |ev| {
                                                    let target = ev.target().unwrap();
                                                    let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                    set_mock_db_query.set(textarea.value());
                                                }
                                            />
                                            <p class="mt-1 text-xs text-gray-500">"Use $1, $2, etc. for parameters from tool input"</p>
                                        </div>
                                    </div>
                                }.into_any(),
                                _ => view! { <div></div> }.into_any(),
                            }
                        }}
                    </div>
                </div>

                <div class="mt-6 flex gap-3">
                    <button
                        type="submit"
                        disabled=move || saving.get()
                        class="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                        {move || if saving.get() { "Creating..." } else { "Create Tool" }}
                    </button>
                    <a
                        href="/tools"
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
pub fn ToolEditForm() -> impl IntoView {
    let params = use_params_map();
    let tool_name = move || params.read().get("name").unwrap_or_default();

    let (name, set_name) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (input_schema, set_input_schema) = signal(String::from("{}"));
    let (static_response, set_static_response) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (saving, set_saving) = signal(false);
    let (loading, set_loading) = signal(true);
    let (original_name, set_original_name) = signal(String::new());

    // Mock strategy signals
    let (mock_strategy, set_mock_strategy) = signal("none".to_string());
    let (mock_template, set_mock_template) = signal(String::new());
    let (mock_faker_type, set_mock_faker_type) = signal(String::new());
    let (mock_state_key, set_mock_state_key) = signal(String::new());
    let (mock_state_operation, set_mock_state_operation) = signal("get".to_string());
    let (mock_script, set_mock_script) = signal(String::new());
    let (mock_script_lang, set_mock_script_lang) = signal("rhai".to_string());
    let (mock_file_path, set_mock_file_path) = signal(String::new());
    let (mock_file_selection, set_mock_file_selection) = signal("random".to_string());
    let (mock_pattern, set_mock_pattern) = signal(String::new());
    let (mock_llm_provider, set_mock_llm_provider) = signal("openai".to_string());
    let (mock_llm_model, set_mock_llm_model) = signal(String::new());
    let (mock_llm_api_key_env, set_mock_llm_api_key_env) = signal(String::new());
    let (mock_llm_system_prompt, set_mock_llm_system_prompt) = signal(String::new());
    let (mock_db_url, set_mock_db_url) = signal(String::new());
    let (mock_db_query, set_mock_db_query) = signal(String::new());

    // Load existing tool
    Effect::new(move |_| {
        let name_param = tool_name();
        set_loading.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match api::get_tool(&name_param).await {
                Ok(tool) => {
                    set_original_name.set(tool.name.clone());
                    set_name.set(tool.name.clone());
                    set_description.set(tool.description.clone());
                    set_input_schema.set(serde_json::to_string_pretty(&tool.input_schema).unwrap_or_default());
                    if let Some(resp) = &tool.static_response {
                        set_static_response.set(serde_json::to_string_pretty(resp).unwrap_or_default());
                    }

                    // Load mock config
                    if let Some(mock) = &tool.mock {
                        let strategy = match mock.strategy {
                            MockStrategyType::Static => "static",
                            MockStrategyType::Template => "template",
                            MockStrategyType::Random => "random",
                            MockStrategyType::Stateful => "stateful",
                            MockStrategyType::Script => "script",
                            MockStrategyType::File => "file",
                            MockStrategyType::Pattern => "pattern",
                            MockStrategyType::LLM => "llm",
                            MockStrategyType::Database => "database",
                        };
                        set_mock_strategy.set(strategy.to_string());

                        if let Some(template) = &mock.template {
                            set_mock_template.set(template.clone());
                        }
                        if let Some(faker_type) = &mock.faker_type {
                            set_mock_faker_type.set(faker_type.clone());
                        }
                        if let Some(stateful) = &mock.stateful {
                            set_mock_state_key.set(stateful.state_key.clone());
                            let op = match stateful.operation {
                                StateOperation::Get => "get",
                                StateOperation::Set => "set",
                                StateOperation::Increment => "increment",
                            };
                            set_mock_state_operation.set(op.to_string());
                            if let Some(template) = &stateful.template {
                                set_mock_template.set(template.clone());
                            }
                        }
                        if let Some(script) = &mock.script {
                            set_mock_script.set(script.clone());
                        }
                        if let Some(lang) = &mock.script_lang {
                            let l = match lang {
                                ScriptLang::Rhai => "rhai",
                                ScriptLang::Lua => "lua",
                                ScriptLang::Js => "js",
                                ScriptLang::Python => "python",
                            };
                            set_mock_script_lang.set(l.to_string());
                        }
                        if let Some(file) = &mock.file {
                            set_mock_file_path.set(file.path.clone());
                            set_mock_file_selection.set(file.selection.clone());
                        }
                        if let Some(pattern) = &mock.pattern {
                            set_mock_pattern.set(pattern.clone());
                        }
                        if let Some(llm) = &mock.llm {
                            let provider = match llm.provider {
                                LLMProvider::OpenAI => "openai",
                                LLMProvider::Anthropic => "anthropic",
                            };
                            set_mock_llm_provider.set(provider.to_string());
                            set_mock_llm_model.set(llm.model.clone());
                            if let Some(env) = &llm.api_key_env {
                                set_mock_llm_api_key_env.set(env.clone());
                            }
                            if let Some(prompt) = &llm.system_prompt {
                                set_mock_llm_system_prompt.set(prompt.clone());
                            }
                        }
                        if let Some(db) = &mock.database {
                            set_mock_db_url.set(db.url.clone());
                            set_mock_db_query.set(db.query.clone());
                        }
                    }
                }
                Err(e) => {
                    set_error.set(Some(format!("Failed to load tool: {}", e)));
                }
            }
            set_loading.set(false);
        });
    });

    let build_mock_config = move || -> Option<MockConfig> {
        let strategy = mock_strategy.get();
        if strategy == "none" {
            return None;
        }

        let strategy_type = match strategy.as_str() {
            "static" => MockStrategyType::Static,
            "template" => MockStrategyType::Template,
            "random" => MockStrategyType::Random,
            "stateful" => MockStrategyType::Stateful,
            "script" => MockStrategyType::Script,
            "file" => MockStrategyType::File,
            "pattern" => MockStrategyType::Pattern,
            "llm" => MockStrategyType::LLM,
            "database" => MockStrategyType::Database,
            _ => return None,
        };

        let mut config = MockConfig {
            strategy: strategy_type,
            ..Default::default()
        };

        match strategy.as_str() {
            "template" => {
                config.template = Some(mock_template.get());
            }
            "random" => {
                if !mock_faker_type.get().is_empty() {
                    config.faker_type = Some(mock_faker_type.get());
                }
            }
            "stateful" => {
                config.stateful = Some(StatefulConfig {
                    state_key: mock_state_key.get(),
                    operation: match mock_state_operation.get().as_str() {
                        "set" => StateOperation::Set,
                        "increment" => StateOperation::Increment,
                        _ => StateOperation::Get,
                    },
                    template: if mock_template.get().is_empty() { None } else { Some(mock_template.get()) },
                });
            }
            "script" => {
                config.script = Some(mock_script.get());
                config.script_lang = Some(match mock_script_lang.get().as_str() {
                    "lua" => ScriptLang::Lua,
                    "js" => ScriptLang::Js,
                    "python" => ScriptLang::Python,
                    _ => ScriptLang::Rhai,
                });
            }
            "file" => {
                config.file = Some(FileConfig {
                    path: mock_file_path.get(),
                    selection: mock_file_selection.get(),
                });
            }
            "pattern" => {
                config.pattern = Some(mock_pattern.get());
            }
            "llm" => {
                config.llm = Some(LLMConfig {
                    provider: match mock_llm_provider.get().as_str() {
                        "anthropic" => LLMProvider::Anthropic,
                        _ => LLMProvider::OpenAI,
                    },
                    api_key_env: if mock_llm_api_key_env.get().is_empty() { None } else { Some(mock_llm_api_key_env.get()) },
                    model: mock_llm_model.get(),
                    system_prompt: if mock_llm_system_prompt.get().is_empty() { None } else { Some(mock_llm_system_prompt.get()) },
                    temperature: None,
                    max_tokens: None,
                    stream: false,
                });
            }
            "database" => {
                config.database = Some(DatabaseConfig {
                    url: mock_db_url.get(),
                    query: mock_db_query.get(),
                    params: vec![],
                });
            }
            _ => {}
        }

        Some(config)
    };

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        set_saving.set(true);
        set_error.set(None);

        let orig_name = original_name.get();
        let schema: serde_json::Value = serde_json::from_str(&input_schema.get())
            .unwrap_or(serde_json::json!({}));

        let static_resp: Option<serde_json::Value> = if static_response.get().is_empty() {
            None
        } else {
            serde_json::from_str(&static_response.get()).ok()
        };

        let tool = Tool {
            name: name.get(),
            description: description.get(),
            input_schema: schema,
            static_response: static_resp,
            mock: build_mock_config(),
        };

        wasm_bindgen_futures::spawn_local(async move {
            match api::update_tool(&orig_name, &tool).await {
                Ok(_) => {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().set_href("/tools");
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
                <a href="/tools" class="text-green-500 hover:underline flex items-center gap-1">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                    "Back to Tools"
                </a>
            </div>

            <h2 class="text-2xl font-bold mb-6">"Edit Tool"</h2>

            {move || if loading.get() {
                view! {
                    <div class="flex items-center justify-center py-12">
                        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-green-500"></div>
                        <span class="ml-3 text-gray-500">"Loading tool..."</span>
                    </div>
                }.into_any()
            } else {
                view! {
                    <form on:submit=on_submit class="bg-white rounded-lg shadow p-6 max-w-3xl">
                        {move || error.get().map(|e| view! {
                            <div class="mb-4 p-3 bg-red-100 border border-red-400 text-red-700 rounded">
                                {e}
                            </div>
                        })}

                        <div class="space-y-6">
                            // Basic Info Section
                            <div class="border-b pb-4">
                                <h3 class="text-lg font-semibold text-gray-800 mb-4">"Basic Information"</h3>
                                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div>
                                        <label class="block text-sm font-medium text-gray-700 mb-1">"Name *"</label>
                                        <input
                                            type="text"
                                            required=true
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                            placeholder="my-tool"
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
                                            class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                            placeholder="What this tool does"
                                            prop:value=move || description.get()
                                            on:input=move |ev| {
                                                let target = ev.target().unwrap();
                                                let input: web_sys::HtmlInputElement = target.dyn_into().unwrap();
                                                set_description.set(input.value());
                                            }
                                        />
                                    </div>
                                </div>
                                <div class="mt-4">
                                    <label class="block text-sm font-medium text-gray-700 mb-1">"Input Schema (JSON)"</label>
                                    <textarea
                                        rows=6
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                        placeholder=r#"{"type": "object", "properties": {"query": {"type": "string"}}}"#
                                        prop:value=move || input_schema.get()
                                        on:input=move |ev| {
                                            let target = ev.target().unwrap();
                                            let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                            set_input_schema.set(textarea.value());
                                        }
                                    />
                                    <p class="mt-1 text-xs text-gray-500">"JSON Schema for tool input parameters"</p>
                                </div>
                            </div>

                            // Response Strategy Section
                            <div class="border-b pb-4">
                                <h3 class="text-lg font-semibold text-gray-800 mb-4">"Response Strategy"</h3>

                                <div class="mb-4">
                                    <label class="block text-sm font-medium text-gray-700 mb-1">"Strategy Type"</label>
                                    <select
                                        class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500"
                                        prop:value=move || mock_strategy.get()
                                        on:change=move |ev| {
                                            let target = ev.target().unwrap();
                                            let select: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
                                            set_mock_strategy.set(select.value());
                                        }
                                    >
                                        <option value="none">"No Mock (Use Static Response)"</option>
                                        <option value="static">"Static"</option>
                                        <option value="template">"Template (Handlebars)"</option>
                                        <option value="random">"Random (Faker)"</option>
                                        <option value="stateful">"Stateful"</option>
                                        <option value="script">"Script"</option>
                                        <option value="file">"File"</option>
                                        <option value="pattern">"Pattern (Regex)"</option>
                                        <option value="llm">"LLM (AI Generated)"</option>
                                        <option value="database">"Database Query"</option>
                                    </select>
                                </div>

                                // Static response for none/static strategy
                                {move || {
                                    let strategy = mock_strategy.get();
                                    if strategy == "none" || strategy == "static" {
                                        view! {
                                            <div>
                                                <label class="block text-sm font-medium text-gray-700 mb-1">"Static Response (JSON)"</label>
                                                <textarea
                                                    rows=6
                                                    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-green-500 font-mono text-sm"
                                                    placeholder=r#"{"result": "success", "data": {}}"#
                                                    prop:value=move || static_response.get()
                                                    on:input=move |ev| {
                                                        let target = ev.target().unwrap();
                                                        let textarea: web_sys::HtmlTextAreaElement = target.dyn_into().unwrap();
                                                        set_static_response.set(textarea.value());
                                                    }
                                                />
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }
                                }}
                            </div>
                        </div>

                        <div class="mt-6 flex gap-3">
                            <button
                                type="submit"
                                disabled=move || saving.get()
                                class="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {move || if saving.get() { "Saving..." } else { "Save Changes" }}
                            </button>
                            <a
                                href="/tools"
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
