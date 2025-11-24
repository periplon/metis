use leptos::prelude::*;
use leptos::web_sys;
use wasm_bindgen::JsCast;
use crate::api;
use crate::types::{
    Tool, MockConfig, MockStrategyType, StatefulConfig, StateOperation,
    FileConfig, ScriptLang, LLMConfig, LLMProvider, DatabaseConfig,
};

#[component]
pub fn Tools() -> impl IntoView {
    let tools = LocalResource::new(|| async move {
        api::list_tools().await.ok()
    });

    view! {
        <div class="p-6">
            <div class="flex justify-between items-center mb-6">
                <h2 class="text-2xl font-bold">"Tools"</h2>
                <a href="/tools/new" class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded">
                    "+ New Tool"
                </a>
            </div>

            <Suspense fallback=move || view! { <div class="text-gray-500">"Loading..."</div> }>
                {move || {
                    tools.get().map(|data| {
                        match data {
                            Some(list) if !list.is_empty() => view! {
                                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                                    {list.into_iter().map(|tool| {
                                        view! { <ToolCard tool=tool /> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any(),
                            Some(_) => view! {
                                <div class="text-center py-12 bg-white rounded-lg shadow">
                                    <p class="text-gray-500 mb-4">"No tools configured"</p>
                                    <a href="/tools/new" class="text-green-500 hover:underline">"Create your first tool"</a>
                                </div>
                            }.into_any(),
                            None => view! {
                                <div class="text-red-500">"Failed to load tools"</div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn ToolCard(tool: Tool) -> impl IntoView {
    let strategy = tool.mock.as_ref()
        .map(|m| format!("{:?}", m.strategy))
        .unwrap_or_else(|| if tool.static_response.is_some() { "Static".to_string() } else { "-".to_string() });

    view! {
        <div class="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
            <div class="flex justify-between items-start mb-2">
                <h3 class="font-bold text-lg text-gray-900">{tool.name.clone()}</h3>
                <span class="px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800">
                    {strategy}
                </span>
            </div>
            <p class="text-gray-600 text-sm mb-4">{tool.description.clone()}</p>
            <div class="flex justify-end space-x-2">
                <button class="text-sm text-blue-600 hover:text-blue-900">"Edit"</button>
                <button class="text-sm text-red-600 hover:text-red-900">"Delete"</button>
            </div>
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
